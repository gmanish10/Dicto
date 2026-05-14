//! Static manifest of bundled-LLM model files Dicto knows how to download.
//!
//! Only one entry today: Qwen 2.5 1.5B-Instruct, Q4_K_M quantization. Apache
//! 2.0 license keeps redistribution clean. Sized for "good enough at polish
//! on any Mac without exhausting RAM".
//!
//! When adding a new model, populate `sha256` only after downloading the
//! upstream artifact and computing the hash yourself — never trust the
//! HuggingFace-served sha as the source of truth.

/// Where to fetch the GGUF model on first use.
pub const QWEN_URL: &str =
    "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf";

/// On-disk filename. Lives alongside whisper models in the user data dir.
pub const QWEN_FILENAME: &str = "qwen2.5-1.5b-instruct-q4_k_m.gguf";

/// Expected size in MB, for the download-card UI. Approximate.
pub const QWEN_SIZE_MB: u32 = 940;

/// SHA-256 of the GGUF we publish-trust. Populated after a maintainer
/// downloads + hashes the upstream artifact. Empty disables verification
/// (acceptable for dev; CI should fail if this is empty at release).
pub const QWEN_SHA256: &str = "";
