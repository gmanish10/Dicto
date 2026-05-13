use super::{TranscribeError, Transcriber};
use async_trait::async_trait;
use serde::Deserialize;

const ENDPOINT: &str = "https://api.groq.com/openai/v1/audio/transcriptions";

pub struct GroqTranscriber {
    api_key: String,
    model: String,
    language: String,
}

impl GroqTranscriber {
    pub fn new(api_key: String, language: String) -> Self {
        Self {
            api_key,
            // large-v3-turbo is fastest as of May 2026 — see plan notes.
            model: "whisper-large-v3-turbo".to_string(),
            language,
        }
    }
}

#[derive(Deserialize)]
struct GroqResponse {
    text: String,
}

#[async_trait]
impl Transcriber for GroqTranscriber {
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
            .text("model", self.model.clone())
            .text("language", self.language.clone())
            .text("response_format", "json")
            .part("file", part);
        if let Some(p) = prompt {
            form = form.text("prompt", p.to_string());
        }

        let client = reqwest::Client::new();
        let resp = client
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
        let parsed: GroqResponse = resp
            .json()
            .await
            .map_err(|e| TranscribeError::Api(e.to_string()))?;
        Ok(parsed.text.trim().to_string())
    }

    fn name(&self) -> &'static str {
        "groq"
    }

    fn requires_network(&self) -> bool {
        true
    }
}

/// Encode f32 PCM samples (range [-1.0, 1.0]) into a 16-bit WAV byte stream.
pub(crate) fn encode_wav(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> anyhow::Result<Vec<u8>> {
    use hound::{SampleFormat, WavSpec, WavWriter};
    use std::io::Cursor;

    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::<u8>::new());
    {
        let mut writer = WavWriter::new(&mut cursor, spec)?;
        for &s in samples {
            let clamped = s.clamp(-1.0, 1.0);
            let v = (clamped * i16::MAX as f32) as i16;
            writer.write_sample(v)?;
        }
        writer.finalize()?;
    }
    Ok(cursor.into_inner())
}
