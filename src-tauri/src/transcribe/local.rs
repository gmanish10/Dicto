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

/// Whisper.cpp emits two flavors of non-speech text the user never wants
/// in their dictation output:
///
/// 1. **No-speech markers** like `[BLANK_AUDIO]` / `[NO_SPEECH]` / `(silence)`
///    that fire when the audio segment didn't contain real speech.
/// 2. **Sound annotations** like `[exhales]`, `[chuckles]`, `(wind howling)`,
///    `(exhaling)` that the model emits when it identifies a non-speech
///    sound. Whisper's training data includes captions with these
///    annotations, so it occasionally reproduces them on real input.
///
/// This strips both. The bracket rule (`[...]`) is aggressive — anything
/// in square brackets goes — because dictation never legitimately
/// includes square brackets. The paren rule (`(...)`) is narrower: it
/// strips only when the content is lowercase letters / spaces, which is
/// the pattern Whisper uses for sound captions. A spoken parenthetical
/// like `(I mean strict mode)` is preserved because of the capital `I`.
///
/// Returns `""` if the entire transcript was annotation.
pub(crate) fn strip_no_speech_markers(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let cleaned = strip_bracketed(trimmed);
    let cleaned = strip_lowercase_parens(&cleaned);
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Remove every `[…]` group from the string. Whisper uses square brackets
/// only for annotations (blank-audio, sound tags, language tags), and
/// spoken dictation never contains literal square brackets. So we strip
/// unconditionally.
fn strip_bracketed(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            // Scan forward for the matching `]`. Cap at 80 bytes so a
            // stray `[` without a matching close doesn't eat the rest of
            // the transcript.
            let mut j = i + 1;
            let mut close = None;
            while j < bytes.len() && j - i < 80 {
                if bytes[j] == b']' {
                    close = Some(j);
                    break;
                }
                j += 1;
            }
            if let Some(end) = close {
                // Drop everything from `[` through `]` inclusive.
                out.push(' ');
                i = end + 1;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Remove `(…)` groups that look like Whisper's sound-annotation captions
/// (`(exhaling)`, `(wind howling)`, `(music playing)`). Other parens —
/// including legitimate spoken parentheticals like `(the answer)` — are
/// preserved.
///
/// The rule: every word inside the parens must appear in
/// `SOUND_ANNOTATION_WORDS`. That keeps us conservative — we'd rather
/// leave a rare false negative (a niche annotation getting through) than
/// strip dictation the user actually intended.
fn strip_lowercase_parens(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'(' {
            let mut j = i + 1;
            let mut close = None;
            while j < bytes.len() && j - i < 60 {
                if bytes[j] == b')' {
                    close = Some(j);
                    break;
                }
                j += 1;
            }
            if let Some(end) = close {
                let inner = std::str::from_utf8(&bytes[i + 1..end]).unwrap_or("");
                if is_sound_annotation(inner) {
                    out.push(' ');
                    i = end + 1;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// True when every word in `inner` is a known sound-annotation token.
/// Empty or whitespace-only content also counts (so `()` gets stripped).
fn is_sound_annotation(inner: &str) -> bool {
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return true;
    }
    trimmed
        .split_whitespace()
        .all(|word| SOUND_ANNOTATION_WORDS.contains(&word.to_ascii_lowercase().as_str()))
}

/// Whisper's caption vocabulary for non-speech sounds. Built from the
/// patterns we've seen leak through plus the common set in the model's
/// training-data subtitles. Lowercase, ASCII.
const SOUND_ANNOTATION_WORDS: &[&str] = &[
    // Breathing / vocal
    "breath",
    "breathes",
    "breathing",
    "exhale",
    "exhales",
    "exhaling",
    "inhale",
    "inhales",
    "inhaling",
    "sigh",
    "sighs",
    "sighing",
    "gasp",
    "gasps",
    "gasping",
    "groan",
    "groans",
    "groaning",
    "grunt",
    "grunts",
    "grunting",
    "yawn",
    "yawns",
    "yawning",
    "panting",
    "snores",
    "snoring",
    // Laughter / amusement
    "laugh",
    "laughs",
    "laughing",
    "laughter",
    "chuckle",
    "chuckles",
    "chuckling",
    "giggle",
    "giggles",
    "giggling",
    "snicker",
    "snickers",
    // Coughing / throat
    "cough",
    "coughs",
    "coughing",
    "clears",
    "throat",
    "sneeze",
    "sneezes",
    "sneezing",
    "sniffles",
    "sniffling",
    // Crying
    "cries",
    "crying",
    "sobs",
    "sobbing",
    "whimpers",
    "whimpering",
    // Speech-quality descriptors that show up alone
    "mumbles",
    "mumbling",
    "whispers",
    "whispering",
    "shouts",
    "shouting",
    "stutters",
    "stuttering",
    // Environment
    "music",
    "playing",
    "plays",
    "silence",
    "silent",
    "pause",
    "static",
    "noise",
    "background",
    "wind",
    "howling",
    "blowing",
    "rain",
    "raining",
    "thunder",
    "thundering",
    "footsteps",
    "knocking",
    "knocks",
    "ringing",
    "rings",
    "buzzing",
    "buzzes",
    "beeping",
    "beeps",
    "clicking",
    "clicks",
    "tapping",
    "taps",
    "phone",
    "doorbell",
    "alarm",
    "siren",
    "engine",
    "running",
    "indistinct",
    "chatter",
    "speaking",
    "foreign",
    "language",
    "applause",
    "applauding",
    "cheering",
    "cheers",
    "clapping",
    // Common filler words inside multi-word annotations
    "and",
    "in",
    "on",
    "the",
];

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

    #[test]
    fn sound_annotations_stripped() {
        // Bracketed sound tags whisper picks up from its caption training data.
        assert_eq!(
            strip_no_speech_markers("[exhales] We need to discuss this."),
            "We need to discuss this."
        );
        assert_eq!(
            strip_no_speech_markers("She said hello [chuckles] then sat down."),
            "She said hello then sat down."
        );
        // Parenthetical lowercase annotations.
        assert_eq!(
            strip_no_speech_markers("(exhaling) Right, so the plan is simple."),
            "Right, so the plan is simple."
        );
        assert_eq!(
            strip_no_speech_markers("Then suddenly (wind howling) the door opened."),
            "Then suddenly the door opened."
        );
    }

    #[test]
    fn legit_parens_preserved() {
        // Mixed-case content in parens is preserved — it's almost
        // certainly the user dictating a parenthetical, not a whisper
        // annotation.
        assert_eq!(
            strip_no_speech_markers("Run the script (I mean strict mode) before merging."),
            "Run the script (I mean strict mode) before merging."
        );
        assert_eq!(
            strip_no_speech_markers("The result was 42 (the answer)."),
            "The result was 42 (the answer)."
        );
    }

    #[test]
    fn unmatched_bracket_does_not_eat_input() {
        // A stray `[` without a close shouldn't swallow the rest of the
        // transcript — the scan caps at 80 chars and bails out.
        let s = "Hello world. Then I said [oops the rest of this should still appear in the output and not be lost at all even though there is no closing bracket anywhere.";
        let out = strip_no_speech_markers(s);
        assert!(out.contains("not be lost"), "got: {out:?}");
    }
}
