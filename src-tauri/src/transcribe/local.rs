use super::{TranscribeError, Transcriber};
use async_trait::async_trait;
use parking_lot::Mutex;
use std::path::Path;
use std::sync::Arc;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// On-device Whisper inference via whisper.cpp.
/// Model is loaded once at startup and reused. Inference is CPU/GPU/ANE bound,
/// so callers should run `transcribe` inside `tokio::task::spawn_blocking`.
pub struct LocalWhisper {
    ctx: Arc<Mutex<WhisperContext>>,
    language: String,
}

impl LocalWhisper {
    pub fn load(model_path: &Path, language: &str) -> Result<Self, TranscribeError> {
        if !model_path.exists() {
            return Err(TranscribeError::ModelNotLoaded(format!(
                "model file missing at {}",
                model_path.display()
            )));
        }
        let params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(
            model_path
                .to_str()
                .ok_or_else(|| TranscribeError::ModelNotLoaded("non-utf8 path".into()))?,
            params,
        )
        .map_err(|e| TranscribeError::ModelNotLoaded(e.to_string()))?;
        Ok(Self {
            ctx: Arc::new(Mutex::new(ctx)),
            language: language.to_string(),
        })
    }
}

#[async_trait]
impl Transcriber for LocalWhisper {
    async fn transcribe(
        &self,
        pcm_16k_mono: &[f32],
        prompt: Option<&str>,
    ) -> Result<String, TranscribeError> {
        if pcm_16k_mono.len() < 16_000 / 4 {
            // <250ms of audio: not enough to bother.
            return Err(TranscribeError::AudioTooShort);
        }
        let ctx = self.ctx.clone();
        let pcm = pcm_16k_mono.to_vec();
        let language = self.language.clone();
        let initial_prompt = prompt.map(|s| s.to_string());

        let text = tokio::task::spawn_blocking(move || -> Result<String, TranscribeError> {
            let guard = ctx.lock();
            let mut state = guard
                .create_state()
                .map_err(|e| TranscribeError::Inference(e.to_string()))?;

            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            params.set_language(Some(language.as_str()));
            params.set_print_special(false);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_timestamps(false);
            params.set_suppress_blank(true);
            params.set_translate(false);
            params.set_no_context(true);
            params.set_single_segment(false);
            if let Some(ref prompt_text) = initial_prompt {
                params.set_initial_prompt(prompt_text);
            }

            state
                .full(params, &pcm)
                .map_err(|e| TranscribeError::Inference(e.to_string()))?;

            let segments = state
                .full_n_segments()
                .map_err(|e| TranscribeError::Inference(e.to_string()))?;
            let mut result = String::new();
            for i in 0..segments {
                if let Ok(segment) = state.full_get_segment_text(i) {
                    result.push_str(&segment);
                }
            }
            Ok(result.trim().to_string())
        })
        .await
        .map_err(|e| TranscribeError::Inference(format!("join: {e}")))??;

        Ok(text)
    }

    fn name(&self) -> &'static str {
        "local"
    }

    fn requires_network(&self) -> bool {
        false
    }
}
