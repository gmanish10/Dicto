use serde::{Deserialize, Serialize};

/// Static manifest of supported models. SHA-256 values are the ones published
/// alongside the GGML files on the whisper.cpp HuggingFace repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub name: &'static str,
    pub display_name: &'static str,
    pub size_mb: u32,
    pub sha256: &'static str,
    pub bundled: bool,
    pub english_only: bool,
}

pub const MODELS: &[ModelEntry] = &[
    ModelEntry {
        name: "ggml-base.en",
        display_name: "Base (English) — fastest, 150 MB",
        size_mb: 150,
        // NOTE: only `ggml-small.en` is bundled today. The other entries
        // are reserved for in-app downloads which aren't wired up yet;
        // when they are, populate `sha256` (and keep `scripts/models.sha256`
        // in sync) so `download_file` can verify post-download.
        sha256: "",
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
        bundled: true,
        english_only: true,
    },
    ModelEntry {
        name: "ggml-medium.en",
        display_name: "Medium (English) — most accurate, 1.5 GB",
        size_mb: 1500,
        sha256: "",
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

    /// Anything we ship inside the bundle must have a verifiable SHA-256.
    /// If you mark a new model `bundled: true`, populate its `sha256` (and
    /// add it to `scripts/models.sha256` so `fetch-model.sh` can check the
    /// download before it lands in `resources/models/`).
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
