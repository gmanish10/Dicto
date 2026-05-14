//! Polish provider resolution + auto-fallback.
//!
//! `PolishContext` lives on `AppState` and caches expensive client handles
//! (sidecar process, loaded LLM context) that are too costly to construct
//! per utterance. `resolve()` picks the actual `Polisher` to use for a
//! single transcript, given:
//!
//! - the user's preference (from `Settings.polish_provider`)
//! - the runtime availability snapshot in `PolishContext`
//! - keychain state (for BYOK providers)
//!
//! When the user picks an explicit provider that's unavailable at runtime,
//! the resolver silently degrades to the next-best free tier and returns
//! the actual `Polisher` plus the fallback reason. The pipeline emits a
//! `pipeline:toast` so the user knows their selection didn't take effect.

use std::sync::Arc;

use super::{
    apple_intelligence::AppleIntelligencePolisher,
    bundled_llm::{manifest as bundled_llm_manifest, BundledLlmPolisher},
    claude::ClaudePolisher,
    groq_llama::GroqLlamaPolisher,
    local_lite::LocalLitePolisher,
    noop::NoOpPolisher,
    Polisher,
};
use crate::{
    config::PolishProvider,
    keychain::{self, ApiKey},
};
use tauri::AppHandle;

/// Cached, expensive-to-build polish client handles. Populated at startup
/// (or lazily on first need) and shared across utterances.
///
/// Until BundledLlm and AppleIntelligence land, both fields stay `None`
/// and `Auto` resolves directly to `LocalLite`.
#[derive(Default)]
pub struct PolishContext {
    pub apple_ai: Option<Arc<dyn Polisher>>,
    pub bundled_llm: Option<Arc<dyn Polisher>>,
}

impl PolishContext {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Set / clear the bundled-LLM client. Called at startup when the
    /// model file already exists on disk, and after a successful download.
    pub fn set_bundled_llm(&mut self, polisher: Option<Arc<dyn Polisher>>) {
        self.bundled_llm = polisher;
    }

    /// Set / clear the Apple Intelligence client. Called at startup when
    /// the sidecar binary is present and we're on macOS 26+.
    pub fn set_apple_ai(&mut self, polisher: Option<Arc<dyn Polisher>>) {
        self.apple_ai = polisher;
    }
}

/// Locate the Apple Intelligence sidecar binary. Returns `Some(polisher)`
/// when the binary is on disk and we're on macOS 26+ (the sidecar itself
/// reports availability of the underlying Foundation Models framework
/// on first spawn).
///
/// Lookup order:
/// 1. `<exe-dir>/dicto-apple-polish` — production bundle layout (Tauri
///    places `externalBin` here, stripping the platform suffix).
/// 2. `<exe-dir>/dicto-apple-polish-<target-triple>` — Tauri dev layout.
/// 3. `<workspace>/src-tauri/binaries/dicto-apple-polish-<target-triple>`
///    — dev fallback if the binary hasn't been copied next to the exe.
///
/// Returns `None` on older macOS — the sidecar would fail at runtime, so
/// we don't even register the provider.
pub fn try_construct_apple_intelligence(_app: &AppHandle) -> Option<Arc<dyn Polisher>> {
    if !is_macos_26_or_newer() {
        return None;
    }
    let bin = locate_apple_polish_binary()?;
    Some(Arc::new(AppleIntelligencePolisher::new(bin)))
}

#[cfg(target_os = "macos")]
fn is_macos_26_or_newer() -> bool {
    let info = std::process::Command::new("/usr/bin/sw_vers")
        .arg("-productVersion")
        .output()
        .ok();
    let Some(out) = info else { return false };
    let version = String::from_utf8_lossy(&out.stdout);
    let major: u32 = version.trim().split('.').next().and_then(|s| s.parse().ok()).unwrap_or(0);
    major >= 26
}

#[cfg(not(target_os = "macos"))]
fn is_macos_26_or_newer() -> bool {
    false
}

fn locate_apple_polish_binary() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;

    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    let triple_suffix = format!("dicto-apple-polish-{}", env!("DICTO_TARGET_TRIPLE"));

    let candidates = [
        exe_dir.join("dicto-apple-polish"),
        exe_dir.join(&triple_suffix),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries").join(&triple_suffix),
    ];

    candidates.into_iter().find(|p| p.exists())
}

/// Check whether the bundled-LLM model exists on disk; if so, construct
/// a Polisher for it and return it. Returns None if the file is missing.
///
/// Cheap: just an `exists()` check + a few struct allocations. Safe to
/// call at startup and after download completes.
///
/// **Disabled for v0.2.0.** The polisher itself is fully wired, but
/// llama-cpp-2 0.1.146 miscompiles tensor reads on macOS 26 (Tahoe) and
/// rejects valid GGUFs as "duplicated" — see the `bundled_llm` entry in
/// `polishLabels.ts` for the full note. Returning `None` here keeps the
/// resolver from registering a polisher that would fail at run time;
/// once upstream ships a fix, drop this early return and the feature
/// unflags itself end-to-end.
pub fn try_construct_bundled_llm(_app: &AppHandle) -> Option<Arc<dyn Polisher>> {
    None
}

#[allow(dead_code)]
fn _try_construct_bundled_llm_when_unblocked(app: &AppHandle) -> Option<Arc<dyn Polisher>> {
    let path = crate::model::resolve_file(app, bundled_llm_manifest::QWEN_FILENAME).ok()?;
    Some(Arc::new(BundledLlmPolisher::new(path)))
}

/// Result of resolution: the actual `Polisher` to run, plus the *effective*
/// provider that produced it (for telemetry / toast messages). When the
/// user's preference matched, `effective == preference`. When the resolver
/// fell back, `effective` differs.
pub struct Resolution {
    pub polisher: Box<dyn Polisher>,
    pub effective: PolishProvider,
    /// If non-None, the user's selection was unavailable. Use this for a
    /// "your preferred cleanup wasn't available, used X instead" toast.
    pub downgraded_from: Option<PolishProvider>,
}

/// Pick the best `Polisher` for this utterance given the user's preference.
///
/// Never returns Err — always falls back to at least `NoOpPolisher`.
pub fn resolve(preference: PolishProvider, ctx: &PolishContext) -> Resolution {
    match preference {
        PolishProvider::None => done(Box::new(NoOpPolisher), PolishProvider::None, None),

        PolishProvider::LocalLite => {
            done(Box::new(LocalLitePolisher), PolishProvider::LocalLite, None)
        }

        PolishProvider::Auto => auto_pick(ctx),

        PolishProvider::AppleIntelligence => match ctx.apple_ai.clone() {
            Some(client) => done(
                Box::new(ArcPolisher(client)),
                PolishProvider::AppleIntelligence,
                None,
            ),
            None => downgrade(auto_pick(ctx), PolishProvider::AppleIntelligence),
        },

        PolishProvider::BundledLlm => match ctx.bundled_llm.clone() {
            Some(client) => done(
                Box::new(ArcPolisher(client)),
                PolishProvider::BundledLlm,
                None,
            ),
            None => downgrade(auto_pick(ctx), PolishProvider::BundledLlm),
        },

        PolishProvider::Claude => match keychain::get(ApiKey::Anthropic) {
            Some(key) => done(
                Box::new(ClaudePolisher::new(key)),
                PolishProvider::Claude,
                None,
            ),
            None => downgrade(auto_pick(ctx), PolishProvider::Claude),
        },

        PolishProvider::GroqLlama => match keychain::get(ApiKey::Groq) {
            Some(key) => done(
                Box::new(GroqLlamaPolisher::new(key)),
                PolishProvider::GroqLlama,
                None,
            ),
            None => downgrade(auto_pick(ctx), PolishProvider::GroqLlama),
        },
    }
}

/// Walk the free-tier stack and return the best available.
fn auto_pick(ctx: &PolishContext) -> Resolution {
    if let Some(client) = ctx.apple_ai.clone() {
        return done(
            Box::new(ArcPolisher(client)),
            PolishProvider::AppleIntelligence,
            None,
        );
    }
    if let Some(client) = ctx.bundled_llm.clone() {
        return done(
            Box::new(ArcPolisher(client)),
            PolishProvider::BundledLlm,
            None,
        );
    }
    done(Box::new(LocalLitePolisher), PolishProvider::LocalLite, None)
}

fn done(
    polisher: Box<dyn Polisher>,
    effective: PolishProvider,
    downgraded_from: Option<PolishProvider>,
) -> Resolution {
    Resolution {
        polisher,
        effective,
        downgraded_from,
    }
}

fn downgrade(mut inner: Resolution, requested: PolishProvider) -> Resolution {
    inner.downgraded_from = Some(requested);
    inner
}

/// Thin `Polisher` wrapper around an `Arc<dyn Polisher>` so we can stash
/// cached clients in `PolishContext` and still return them as a `Box<dyn>`
/// from `resolve()`.
struct ArcPolisher(Arc<dyn Polisher>);

#[async_trait::async_trait]
impl Polisher for ArcPolisher {
    async fn polish(
        &self,
        raw: &str,
        recent: &[super::Correction],
    ) -> Result<String, super::PolishError> {
        self.0.polish(raw, recent).await
    }

    fn name(&self) -> &'static str {
        self.0.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_with_empty_context_picks_local_lite() {
        let r = resolve(PolishProvider::Auto, &PolishContext::empty());
        assert_eq!(r.effective, PolishProvider::LocalLite);
        assert!(r.downgraded_from.is_none());
    }

    #[test]
    fn none_returns_noop() {
        let r = resolve(PolishProvider::None, &PolishContext::empty());
        assert_eq!(r.effective, PolishProvider::None);
        assert_eq!(r.polisher.name(), "none");
    }

    #[test]
    fn missing_apple_intelligence_falls_back_with_reason() {
        let r = resolve(PolishProvider::AppleIntelligence, &PolishContext::empty());
        assert_eq!(r.effective, PolishProvider::LocalLite);
        assert_eq!(r.downgraded_from, Some(PolishProvider::AppleIntelligence));
    }

    #[test]
    fn missing_bundled_llm_falls_back_with_reason() {
        let r = resolve(PolishProvider::BundledLlm, &PolishContext::empty());
        assert_eq!(r.effective, PolishProvider::LocalLite);
        assert_eq!(r.downgraded_from, Some(PolishProvider::BundledLlm));
    }
}
