//! Free, on-device LLM polish — runs a small Qwen 2.5 1.5B model via
//! llama.cpp (Metal accelerated on Apple Silicon).
//!
//! The model file is **not bundled** in the .dmg; the user downloads it
//! on first use (~940 MB) and we cache it in
//! `~/Library/Application Support/com.dicto.app/models/`. Same UX as the
//! Whisper model download — see [`crate::model::download_file`].
//!
//! ## Lifecycle
//!
//! - Constructed on demand by the resolver once the user opts into
//!   `PolishProvider::BundledLlm` and the model file exists on disk.
//! - The `LlamaModel` is loaded lazily on the first polish call inside
//!   `tokio::task::spawn_blocking` (cold load ~1–2s on M-series). The
//!   loaded model is cached in `Arc<Mutex<Option<...>>>` for reuse.
//! - Each polish call creates a fresh `LlamaContext` (KV cache); we don't
//!   keep one across calls because `LlamaContext` borrows from `LlamaModel`
//!   and self-referential structs in Rust are painful. Context creation
//!   for 2k tokens is ~50 ms — well under the prompt-decode budget.
//! - Idle eviction (drop the model after 5 min unused) will be wired in
//!   Phase 3 when we add the download IPC.

pub mod manifest;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use llama_cpp_2::{
    context::params::LlamaContextParams,
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{params::LlamaModelParams, AddBos, LlamaModel},
    sampling::LlamaSampler,
    token::LlamaToken,
};
use once_cell::sync::Lazy;
use parking_lot::Mutex;

use super::{prompt, sanity_check, Correction, PolishError, Polisher};

/// LlamaBackend is meant to be initialized once per process. Stash it
/// behind a Lazy<Mutex<Option<...>>> so we can recover if init fails
/// (very unlikely).
///
/// We hold a `Mutex<Option<Arc<...>>>` instead of just `Lazy<Arc<...>>`
/// because the backend's `init()` returns a `Result` — if it ever fails
/// we want to retry on the next call rather than poison the lazy.
static BACKEND: Lazy<Mutex<Option<Arc<LlamaBackend>>>> = Lazy::new(|| Mutex::new(None));

fn backend() -> Result<Arc<LlamaBackend>, PolishError> {
    let mut guard = BACKEND.lock();
    if let Some(b) = guard.as_ref() {
        return Ok(b.clone());
    }
    let b =
        LlamaBackend::init().map_err(|e| PolishError::Api(format!("llama backend init: {e}")))?;
    let b = Arc::new(b);
    *guard = Some(b.clone());
    Ok(b)
}

pub struct BundledLlmPolisher {
    model_path: PathBuf,
    /// Cached model, populated lazily on first polish call. We hold an
    /// `Arc<LlamaModel>` so `polish()` can drop the outer lock quickly.
    model: Arc<Mutex<Option<Arc<LlamaModel>>>>,
}

impl BundledLlmPolisher {
    /// Construct a polisher pointed at a GGUF model file. The file isn't
    /// touched until the first polish call.
    pub fn new(model_path: PathBuf) -> Self {
        Self {
            model_path,
            model: Arc::new(Mutex::new(None)),
        }
    }

    /// Load + cache the LlamaModel if we haven't already. Cheap on repeat
    /// calls.
    ///
    /// macOS 26 ships a Metal toolchain that miscompiles llama.cpp's bundled
    /// shaders (same issue that hit whisper.cpp earlier this year — its
    /// `ggml_backend_metal_init` returns an unusable context and the
    /// subsequent CPU retry inherits the corrupted global state, surfacing
    /// as a bogus "tensor 'token_embd.weight' is duplicated" error). To
    /// avoid that minefield we load CPU-only unconditionally for now;
    /// inference for a typical transcript on M-series is ~1–2 s, well
    /// within the polish-budget. Revisit when upstream llama.cpp ships a
    /// fix for the macOS 26 Metal toolchain.
    fn ensure_loaded(&self) -> Result<Arc<LlamaModel>, PolishError> {
        if let Some(m) = self.model.lock().as_ref() {
            return Ok(m.clone());
        }
        if !self.model_path.exists() {
            return Err(PolishError::Api(format!(
                "bundled-llm model not downloaded ({})",
                self.model_path.display()
            )));
        }
        let backend = backend()?;

        // CPU-only + no-mmap. macOS 26 ships a Metal toolchain that
        // miscompiles llama.cpp's shaders, and llama.cpp's mmap reader on
        // Tahoe has been reported to drop bytes intermittently — both
        // surfacing as bogus "tensor 'X' is duplicated" errors (the
        // duplicate-detection map gets fooled by garbled tensor names from
        // a partial read). Forcing a full read into RAM via use_mmap(false)
        // is slower at load time (~1 GB read once) but stable.
        let params = LlamaModelParams::default()
            .with_n_gpu_layers(0)
            .with_use_mmap(false);
        let loaded = LlamaModel::load_from_file(&backend, &self.model_path, &params)
            .map_err(|e| PolishError::Api(format!("llama model load: {e}")))?;
        tracing::info!("bundled LLM loaded CPU-only (no mmap)");

        let arc = Arc::new(loaded);
        *self.model.lock() = Some(arc.clone());
        Ok(arc)
    }
}

#[async_trait]
impl Polisher for BundledLlmPolisher {
    async fn polish(&self, raw: &str, recent: &[Correction]) -> Result<String, PolishError> {
        let raw_owned = raw.to_string();
        let recent_owned = recent.to_vec();
        let model_path = self.model_path.clone();
        let model_cell = self.model.clone();

        // All llama.cpp work is CPU/GPU heavy and blocking. Hand off to a
        // blocking worker so we don't pin a tokio runtime thread.
        let out: String = tokio::task::spawn_blocking(move || {
            let polisher = BundledLlmPolisher {
                model_path,
                model: model_cell,
            };
            let model = polisher.ensure_loaded()?;
            generate(&model, &raw_owned, &recent_owned)
        })
        .await
        .map_err(|e| PolishError::Api(format!("join: {e}")))??;

        sanity_check(raw, &out).map_err(PolishError::OutputRejected)?;
        Ok(out)
    }

    fn name(&self) -> &'static str {
        "bundled_llm"
    }
}

/// One-shot inference: build the Qwen ChatML prompt, decode, sample until
/// `<|im_end|>` or the cap, return the assistant turn as a String.
///
/// Runs on a blocking thread, owns its `LlamaContext`.
fn generate(model: &LlamaModel, raw: &str, recent: &[Correction]) -> Result<String, PolishError> {
    // ChatML prompt for Qwen 2.5. The sentinels are tokenized as special
    // tokens (AddBos::Never so we don't double-add bos).
    let system = prompt::build_full_system(recent);
    let user = format!("Polish this transcript:\n\n{raw}");
    let prompt_text = format!(
        "<|im_start|>system\n{system}<|im_end|>\n\
         <|im_start|>user\n{user}<|im_end|>\n\
         <|im_start|>assistant\n",
    );

    // Context sized to fit prompt + a generous response budget.
    // Qwen 2.5 supports 32k native context; we cap to 4096 to keep KV
    // small on user machines.
    let ctx_params = LlamaContextParams::default().with_n_ctx(std::num::NonZeroU32::new(4096));
    let backend = backend()?;
    let mut ctx = model
        .new_context(&backend, ctx_params)
        .map_err(|e| PolishError::Api(format!("llama context: {e}")))?;

    let tokens = model
        .str_to_token(&prompt_text, AddBos::Never)
        .map_err(|e| PolishError::Api(format!("tokenize: {e}")))?;

    let n_tokens = tokens.len();
    if n_tokens >= 4090 {
        return Err(PolishError::OutputRejected(
            "prompt too long for bundled-llm context",
        ));
    }

    // Feed the prompt in one batch. Only the last token needs logits.
    let mut batch = LlamaBatch::new(n_tokens.max(1), 1);
    for (i, &tok) in tokens.iter().enumerate() {
        let is_last = i == n_tokens - 1;
        batch
            .add(tok, i as i32, &[0], is_last)
            .map_err(|e| PolishError::Api(format!("batch add: {e}")))?;
    }
    ctx.decode(&mut batch)
        .map_err(|e| PolishError::Api(format!("prompt decode: {e}")))?;

    // Light-touch sampling chain: temp 0.2 + top_p 0.9 + final dist.
    let mut sampler = LlamaSampler::chain_simple([
        LlamaSampler::top_p(0.9, 1),
        LlamaSampler::temp(0.2),
        LlamaSampler::dist(0xD1C70),
    ]);

    let im_end_id: Option<LlamaToken> = model
        .str_to_token("<|im_end|>", AddBos::Never)
        .ok()
        .and_then(|v| v.first().copied());

    let mut produced_bytes: Vec<u8> = Vec::with_capacity(256);
    let mut cur_pos = n_tokens as i32;
    let max_new = (4096 - n_tokens).min(512);
    // Token-count hard cap to backstop a missed EOS — proportional to
    // input length plus headroom for legitimate expansion.
    let token_cap = ((raw.len() / 3) + 128).min(max_new);

    // Using a manual counter so `cur_pos` doubles as the absolute position
    // we feed back to llama.cpp and the loop's iteration index.
    #[allow(clippy::explicit_counter_loop)]
    for _ in 0..token_cap {
        let new_token: LlamaToken = sampler.sample(&ctx, batch.n_tokens() - 1);
        if model.is_eog_token(new_token) {
            break;
        }
        if Some(new_token) == im_end_id {
            break;
        }
        // Accumulate raw bytes; we decode once at the end so partial
        // multi-byte UTF-8 sequences across token boundaries don't
        // produce intermediate decode errors.
        if let Ok(bytes) = model.token_to_piece_bytes(new_token, 8, false, None) {
            produced_bytes.extend_from_slice(&bytes);
        }

        batch.clear();
        batch
            .add(new_token, cur_pos, &[0], true)
            .map_err(|e| PolishError::Api(format!("batch step: {e}")))?;
        ctx.decode(&mut batch)
            .map_err(|e| PolishError::Api(format!("step decode: {e}")))?;
        cur_pos += 1;
    }

    // `from_utf8_lossy` replaces any invalid bytes with U+FFFD; for
    // English polishing this should be a no-op in practice.
    let output = String::from_utf8_lossy(&produced_bytes).into_owned();
    let trimmed = output.trim().to_string();
    if trimmed.is_empty() {
        return Err(PolishError::OutputRejected("empty output from bundled-llm"));
    }
    Ok(trimmed)
}

/// Internal helper for `crate::polish::resolver` to construct one of these
/// without leaking the type detail into the resolver module.
#[allow(dead_code)]
pub fn try_new(model_path: &Path) -> Option<BundledLlmPolisher> {
    if model_path.exists() {
        Some(BundledLlmPolisher::new(model_path.to_path_buf()))
    } else {
        None
    }
}
