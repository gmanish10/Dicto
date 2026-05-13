pub mod groq;
pub mod local;
pub mod openai;

use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TranscribeError {
    #[error("network error: {0}")]
    Network(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("model not loaded: {0}")]
    ModelNotLoaded(String),
    #[error("inference failed: {0}")]
    Inference(String),
    #[error("missing API key for {0}")]
    MissingKey(&'static str),
    #[error("audio too short")]
    AudioTooShort,
}

#[async_trait]
pub trait Transcriber: Send + Sync {
    /// Transcribe 16kHz mono f32 PCM. `prompt` is an optional vocabulary-biasing hint.
    async fn transcribe(
        &self,
        pcm_16k_mono: &[f32],
        prompt: Option<&str>,
    ) -> Result<String, TranscribeError>;

    fn name(&self) -> &'static str;

    fn requires_network(&self) -> bool;
}
