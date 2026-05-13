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
        // NOTE: Real SHA-256 values must be looked up at packaging time and
        // committed alongside the model bin. Leaving empty disables verification
        // for that entry; production builds should populate these.
        sha256: "",
        bundled: false,
        english_only: true,
    },
    ModelEntry {
        name: "ggml-small.en",
        display_name: "Small (English) — recommended, 250 MB",
        size_mb: 250,
        sha256: "",
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
