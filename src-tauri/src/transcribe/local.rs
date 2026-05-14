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
            Ok(strip_no_speech_markers(result.trim()))
        })
        .await
        .map_err(|e| TranscribeError::Inference(format!("join: {e}")))??;

        if text.is_empty() {
            tracing::debug!("no speech detected — dropping transcript");
        }
        Ok(text)
    }

    fn name(&self) -> &'static str {
        "local"
    }

    fn requires_network(&self) -> bool {
        false
    }
}

/// Whisper.cpp emits special markers like `[BLANK_AUDIO]` (or `[ _BLANK_AUDIO_ ]`
/// depending on internal state) when it doesn't detect speech. These aren't
/// special tokens in the usual sense — `set_print_special(false)` doesn't
/// filter them — so we strip them ourselves.
///
/// Returns the input with any known marker tokens removed and whitespace
/// collapsed. Returns `""` if the entire transcript was a marker.
pub(crate) fn strip_no_speech_markers(s: &str) -> String {
    const MARKERS: &[&str] = &[
        "[BLANK_AUDIO]",
        "[ BLANK_AUDIO ]",
        "[_BLANK_AUDIO_]",
        "[ _BLANK_AUDIO_ ]",
        "[NO_SPEECH]",
        "[ NO_SPEECH ]",
        "(silence)",
        "[silence]",
        "[ silence ]",
        "[Silence]",
        "[ Silence ]",
    ];

    let trimmed = s.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // Fast path: if the whole transcript is exactly one marker, return empty.
    for m in MARKERS {
        if trimmed.eq_ignore_ascii_case(m) {
            return String::new();
        }
    }

    // Inline removal: replace each marker (case-sensitive — whisper.cpp is
    // consistent about the casing) wherever it appears, then collapse spaces.
    let mut cleaned = trimmed.to_string();
    for m in MARKERS {
        cleaned = cleaned.replace(m, " ");
    }
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::strip_no_speech_markers;

    #[test]
    fn plain_marker_returns_empty() {
        assert_eq!(strip_no_speech_markers("[BLANK_AUDIO]"), "");
        assert_eq!(strip_no_speech_markers(" [BLANK_AUDIO] "), "");
        assert_eq!(strip_no_speech_markers("[ _BLANK_AUDIO_ ]"), "");
        assert_eq!(strip_no_speech_markers("(silence)"), "");
    }

    #[test]
    fn case_insensitive_exact_match() {
        assert_eq!(strip_no_speech_markers("[blank_audio]"), "");
        assert_eq!(strip_no_speech_markers("[ Silence ]"), "");
    }

    #[test]
    fn inline_marker_removed_preserves_other_text() {
        // Whisper sometimes pads a real transcript with a marker.
        assert_eq!(
            strip_no_speech_markers("Hello [BLANK_AUDIO] world."),
            "Hello world."
        );
        assert_eq!(
            strip_no_speech_markers("[BLANK_AUDIO] real speech here"),
            "real speech here"
        );
    }

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(strip_no_speech_markers(""), "");
        assert_eq!(strip_no_speech_markers("   "), "");
    }

    #[test]
    fn normal_text_passes_through() {
        assert_eq!(
            strip_no_speech_markers("Normal transcribed text."),
            "Normal transcribed text."
        );
        assert_eq!(
            strip_no_speech_markers("The meeting is at three pm"),
            "The meeting is at three pm"
        );
    }

    #[test]
    fn whitespace_collapsed_after_removal() {
        // Multiple markers + multiple spaces should not produce double-spaces.
        assert_eq!(
            strip_no_speech_markers("[BLANK_AUDIO]   word   [BLANK_AUDIO]"),
            "word"
        );
    }
}
