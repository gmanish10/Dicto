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

/// Resolve the on-disk path of a model file.
/// Order:
/// 1. App's user data dir (downloaded after first launch).
/// 2. Bundled resource (shipped inside the .app).
///
/// If neither exists, returns `NotFound` so the caller can trigger a download.
pub fn resolve_path(app: &AppHandle, model_name: &str) -> Result<PathBuf, ModelError> {
    let filename = format!("{model_name}.bin");

    // 1. User-downloaded location
    let user_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| ModelError::Io(std::io::Error::other(e.to_string())))?
        .join("models");
    let user_path = user_dir.join(&filename);
    if user_path.exists() {
        return Ok(user_path);
    }

    // 2. Bundled resource
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir
            .join("resources")
            .join("models")
            .join(&filename);
        if bundled.exists() {
            return Ok(bundled);
        }
        let bundled_alt = resource_dir.join("models").join(&filename);
        if bundled_alt.exists() {
            return Ok(bundled_alt);
        }
    }

    Err(ModelError::NotFound(user_path))
}

/// Verify the SHA-256 of a model file matches the expected manifest entry.
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

/// Download a model from HuggingFace into the user data dir. Reports progress
/// (bytes_downloaded, total_bytes) via the supplied callback. Total may be 0 if
/// the server doesn't send Content-Length.
pub async fn download(
    app: &AppHandle,
    model_name: &str,
    expected_sha256: &str,
    mut progress: impl FnMut(u64, u64),
) -> Result<PathBuf, ModelError> {
    let url = format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{model_name}.bin");
    let user_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| ModelError::Io(std::io::Error::other(e.to_string())))?
        .join("models");
    std::fs::create_dir_all(&user_dir)?;
    let dest = user_dir.join(format!("{model_name}.bin"));
    let tmp = user_dir.join(format!("{model_name}.bin.partial"));

    let response = reqwest::get(&url)
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

    verify(&tmp, expected_sha256)?;
    std::fs::rename(&tmp, &dest)?;
    Ok(dest)
}
