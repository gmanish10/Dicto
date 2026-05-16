use serde::{Deserialize, Serialize};

/// Static manifest of supported models. SHA-256 values are the ones published
/// alongside the GGML files on the whisper.cpp HuggingFace repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub name: &'static str,
    pub display_name: &'static str,
    pub size_mb: u32,
    pub sha256: &'static str,
    /// SHA-256 of the CoreML encoder archive (`<name>-encoder.mlmodelc.zip`).
    /// Empty when no CoreML encoder is published / verified for this model.
    pub encoder_sha256: &'static str,
    pub bundled: bool,
    pub english_only: bool,
}

pub const MODELS: &[ModelEntry] = &[
    ModelEntry {
        name: "ggml-base.en",
        display_name: "Base (English) — fastest, 150 MB",
        size_mb: 150,
        // NOTE: only `ggml-small.en` is wired for in-app download today.
        // The other entries are reserved for future selectable models;
        // when they are, populate `sha256` / `encoder_sha256` (and keep
        // `scripts/models.sha256` in sync) so `download_file` can verify.
        sha256: "",
        encoder_sha256: "",
        bundled: false,
        english_only: true,
    },
    ModelEntry {
        name: "ggml-small.en",
        display_name: "Small (English) — recommended, 250 MB",
        size_mb: 250,
        // SHA-256 of the upstream `ggml-small.en.bin` artifact published
        // by ggerganov on Hugging Face. Keep in sync with
        // `scripts/models.sha256`.
        sha256: "c6138d6d58ecc8322097e0f987c32f1be8bb0a18532a3f88f734d1bbf9c41e5d",
        // SHA-256 of the upstream `ggml-small.en-encoder.mlmodelc.zip`
        // (the CoreML encoder, ~168 MB). Computed on 2026-05-16. Keep in
        // sync with `scripts/models.sha256`.
        encoder_sha256: "b2ef1c506378b825b4b4341979a93e1656b5d6c129f17114cfb8fb78aabc2f89",
        // No longer bundled in the `.app`: the model auto-downloads on
        // first launch into the app-data models dir (see `model::mod`).
        bundled: false,
        english_only: true,
    },
    ModelEntry {
        name: "ggml-medium.en",
        display_name: "Medium (English) — most accurate, 1.5 GB",
        size_mb: 1500,
        sha256: "",
        encoder_sha256: "",
        bundled: false,
        english_only: true,
    },
];

pub fn find(name: &str) -> Option<&'static ModelEntry> {
    MODELS.iter().find(|m| m.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Anything we mark `bundled: true` must have a verifiable SHA-256.
    /// No model is bundled today — the whisper model auto-downloads on
    /// first launch — so this currently iterates to a no-op. If you ever
    /// re-introduce a bundled model, populate its `sha256` (and add it to
    /// `scripts/models.sha256`) so the artifact can be verified.
    #[test]
    fn bundled_models_have_sha256() {
        for entry in MODELS {
            if entry.bundled {
                assert!(
                    !entry.sha256.is_empty(),
                    "bundled model `{}` has no SHA-256 in manifest.rs",
                    entry.name
                );
                assert_eq!(
                    entry.sha256.len(),
                    64,
                    "bundled model `{}` SHA-256 must be 64 hex chars",
                    entry.name
                );
            }
        }
    }
}
