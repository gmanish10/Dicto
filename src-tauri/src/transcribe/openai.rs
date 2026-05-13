use super::{groq::encode_wav, TranscribeError, Transcriber};
use async_trait::async_trait;
use serde::Deserialize;

const ENDPOINT: &str = "https://api.openai.com/v1/audio/transcriptions";

pub struct OpenAiTranscriber {
    api_key: String,
    language: String,
}

impl OpenAiTranscriber {
    pub fn new(api_key: String, language: String) -> Self {
        Self { api_key, language }
    }
}

#[derive(Deserialize)]
struct OpenAiResponse {
    text: String,
}

#[async_trait]
impl Transcriber for OpenAiTranscriber {
    async fn transcribe(
        &self,
        pcm_16k_mono: &[f32],
        prompt: Option<&str>,
    ) -> Result<String, TranscribeError> {
        let wav_bytes = encode_wav(pcm_16k_mono, 16_000, 1)
            .map_err(|e| TranscribeError::Api(format!("wav encode: {e}")))?;

        let part = reqwest::multipart::Part::bytes(wav_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| TranscribeError::Api(e.to_string()))?;
        let mut form = reqwest::multipart::Form::new()
            .text("model", "whisper-1")
            .text("language", self.language.clone())
            .text("response_format", "json")
            .part("file", part);
        if let Some(p) = prompt {
            form = form.text("prompt", p.to_string());
        }

        let resp = reqwest::Client::new()
            .post(ENDPOINT)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| TranscribeError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(TranscribeError::Api(format!("{status}: {body}")));
        }
        let parsed: OpenAiResponse = resp
            .json()
            .await
            .map_err(|e| TranscribeError::Api(e.to_string()))?;
        Ok(parsed.text.trim().to_string())
    }

    fn name(&self) -> &'static str {
        "openai"
    }

    fn requires_network(&self) -> bool {
        true
    }
}
