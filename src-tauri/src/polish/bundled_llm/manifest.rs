//! Static manifest of bundled-LLM model files Dicto knows how to download.
//!
//! Only one entry today: Qwen 2.5 1.5B-Instruct, Q4_K_M quantization. Apache
//! 2.0 license keeps redistribution clean. Sized for "good enough at polish
//! on any Mac without exhausting RAM".
//!
//! We pull from **bartowski's** requantized GGUF rather than the official
//! `Qwen/Qwen2.5-1.5B-Instruct-GGUF` repo. The official file ships both
//! `token_embd.weight` and `output.weight` even though the model config
//! sets `tie_word_embeddings: true`, which causes llama.cpp to refuse the
//! load with `tensor 'token_embd.weight' is duplicated`. Bartowski's
//! requant fixes that and is the de-facto community-standard build for
//! GGUF tooling.

/// Where to fetch the GGUF model on first use.
pub const QWEN_URL: &str =
    "https://huggingface.co/bartowski/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf";

/// On-disk filename. Lives alongside whisper models in the user data dir.
/// Capitalization matches bartowski's published asset.
pub const QWEN_FILENAME: &str = "Qwen2.5-1.5B-Instruct-Q4_K_M.gguf";

/// Expected size in MB, for the download-card UI. Approximate.
pub const QWEN_SIZE_MB: u32 = 940;

/// SHA-256 of the GGUF we publish-trust. Verified by hashing the upstream
/// LFS pointer on 2026-05-14. Empty disables verification (acceptable for
/// dev; CI should fail if this is empty at release).
pub const QWEN_SHA256: &str = "1adf0b11065d8ad2e8123ea110d1ec956dab4ab038eab665614adba04b6c3370";
