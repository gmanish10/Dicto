pub mod manifest;

use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModelError {
    #[error("model not found at {0}")]
    NotFound(PathBuf),
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("network error: {0}")]
    Network(String),
}

/// Where downloaded model files live: `~/Library/Application Support/<bundle>/models/`.
fn user_models_dir(app: &AppHandle) -> Result<PathBuf, ModelError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| ModelError::Io(std::io::Error::other(e.to_string())))?
        .join("models");
    Ok(dir)
}

/// Resolve a model file by *filename* (e.g. `"ggml-small.en.bin"` or
/// `"qwen2.5-1.5b-instruct-q4_k_m.gguf"`). Looks in the user data dir first,
/// then the bundled `.app/Contents/Resources/...` paths.
pub fn resolve_file(app: &AppHandle, filename: &str) -> Result<PathBuf, ModelError> {
    let user_dir = user_models_dir(app)?;
    let user_path = user_dir.join(filename);
    if user_path.exists() {
        return Ok(user_path);
    }
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("resources").join("models").join(filename);
        if bundled.exists() {
            return Ok(bundled);
        }
        let bundled_alt = resource_dir.join("models").join(filename);
        if bundled_alt.exists() {
            return Ok(bundled_alt);
        }
    }
    Err(ModelError::NotFound(user_path))
}

/// Back-compat wrapper: whisper-style `<model_name>.bin` lookup.
pub fn resolve_path(app: &AppHandle, model_name: &str) -> Result<PathBuf, ModelError> {
    resolve_file(app, &format!("{model_name}.bin"))
}

/// Verify the SHA-256 of a file matches the expected hex digest.
pub fn verify(path: &Path, expected_hex: &str) -> Result<(), ModelError> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let actual = hex::encode(hasher.finalize());
    if !actual.eq_ignore_ascii_case(expected_hex) {
        return Err(ModelError::HashMismatch {
            expected: expected_hex.to_string(),
            actual,
        });
    }
    Ok(())
}

/// Generic download helper. Streams a file from `url` to the user models dir
/// at `filename`, reporting (bytes_downloaded, total_bytes) via `progress`.
/// If `expected_sha256` is `Some(..)` and non-empty, the file is verified
/// after download; on mismatch the partial file is left in place for
/// debugging.
pub async fn download_file(
    app: &AppHandle,
    url: &str,
    filename: &str,
    expected_sha256: Option<&str>,
    mut progress: impl FnMut(u64, u64),
) -> Result<PathBuf, ModelError> {
    let user_dir = user_models_dir(app)?;
    std::fs::create_dir_all(&user_dir)?;
    let dest = user_dir.join(filename);
    let tmp = user_dir.join(format!("{filename}.partial"));

    let response = reqwest::get(url)
        .await
        .map_err(|e| ModelError::Network(e.to_string()))?;
    if !response.status().is_success() {
        return Err(ModelError::Network(format!("HTTP {}", response.status())));
    }
    let total = response.content_length().unwrap_or(0);

    use futures::StreamExt;
    use std::io::Write;
    let mut stream = response.bytes_stream();
    let mut file = std::fs::File::create(&tmp)?;
    let mut downloaded: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| ModelError::Network(e.to_string()))?;
        file.write_all(&bytes)?;
        downloaded += bytes.len() as u64;
        progress(downloaded, total);
    }
    drop(file);

    if let Some(sha) = expected_sha256 {
        if !sha.is_empty() {
            verify(&tmp, sha)?;
        }
    }
    std::fs::rename(&tmp, &dest)?;
    Ok(dest)
}

/// Back-compat: download a whisper.cpp model from the canonical HF mirror.
/// Convenience wrapper around `download_file` for the common case.
pub async fn download(
    app: &AppHandle,
    model_name: &str,
    expected_sha256: &str,
    progress: impl FnMut(u64, u64),
) -> Result<PathBuf, ModelError> {
    let url = format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{model_name}.bin");
    let filename = format!("{model_name}.bin");
    let sha = if expected_sha256.is_empty() {
        None
    } else {
        Some(expected_sha256)
    };
    download_file(app, &url, &filename, sha, progress).await
}
