use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SttProvider {
    #[default]
    Local,
    Groq,
    OpenAi,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PolishProvider {
    /// Picks the best free option available on this Mac at runtime.
    /// Resolver order: AppleIntelligence → BundledLlm → LocalLite (Enhanced).
    #[default]
    Auto,
    /// No polishing; raw whisper output is injected as-is.
    None,
    /// Lightweight on-device cleanup (heuristics: fillers, repeats, capitalization).
    LocalLite,
    /// On-device LLM polish via Apple Intelligence Foundation Models framework.
    /// Requires macOS 26+ on Apple Silicon with Apple Intelligence enabled.
    AppleIntelligence,
    /// On-device LLM polish via a small Qwen model. Downloaded on first use (~940 MB).
    BundledLlm,
    /// Cloud LLM polish via Anthropic Claude Haiku. Needs an API key.
    Claude,
    /// Cloud LLM polish via Groq Llama. Needs an API key.
    GroqLlama,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HotkeyBinding {
    /// String form like "RightOption", "Fn", "Cmd+Shift+Space", parseable by hotkey::listener.
    pub chord: String,
}

impl Default for HotkeyBinding {
    fn default() -> Self {
        Self {
            chord: "Fn".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Settings {
    pub hotkey: HotkeyBinding,
    pub stt_provider: SttProvider,
    pub polish_provider: PolishProvider,
    pub language: String,
    pub model_name: String,
    pub microphone_name: Option<String>,
    pub play_start_chime: bool,
    pub play_stop_chime: bool,
    pub auto_paste: bool,
    pub max_recording_seconds: u32,
    pub onboarding_completed: bool,
    /// Persisted step the user was on when they last left the onboarding
    /// flow. Empty string before onboarding starts; one of the step IDs
    /// from `Onboarding.tsx` ("welcome" / "permissions" / "models" /
    /// "try-it" / "discover" / "done") otherwise. Lets us resume after
    /// the macOS-forced quit-and-relaunch that follows an Accessibility
    /// or Input Monitoring grant — without this, the user lands back on
    /// Welcome and has to walk through the flow again.
    pub onboarding_step: String,
    pub paused: bool,
    /// Show the floating "Recording" pill above all apps while the
    /// hotkey is held. Defaults on — visible feedback that Dicto is
    /// listening is the right default for a hold-to-talk app.
    pub show_recording_overlay: bool,
}

impl Settings {
    pub fn with_defaults() -> Self {
        Self {
            hotkey: HotkeyBinding::default(),
            stt_provider: SttProvider::default(),
            polish_provider: PolishProvider::default(),
            language: "en".to_string(),
            model_name: "ggml-small.en".to_string(),
            microphone_name: None,
            play_start_chime: true,
            play_stop_chime: false,
            auto_paste: true,
            max_recording_seconds: 300,
            onboarding_completed: false,
            paused: false,
            show_recording_overlay: true,
            onboarding_step: String::new(),
        }
    }
}

/// Plain-language label for a polish provider, suitable for user-facing
/// toasts and inline help. Mirrors `src/lib/polishLabels.ts` on the frontend
/// but used in Rust-side messages (e.g., the silent-downgrade toast).
pub fn provider_display_name(p: PolishProvider) -> &'static str {
    match p {
        PolishProvider::Auto => "Best available",
        PolishProvider::None => "No cleanup",
        PolishProvider::LocalLite => "Basic cleanup",
        PolishProvider::AppleIntelligence => "Apple Intelligence",
        PolishProvider::BundledLlm => "On-device LLM",
        PolishProvider::Claude => "Claude Haiku",
        PolishProvider::GroqLlama => "Groq Llama",
    }
}

pub fn settings_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("settings.json")
}

/// Load settings from disk with **field-by-field defaulting**.
///
/// If a single field's JSON is missing or malformed (e.g. an enum value
/// we no longer recognize because the schema evolved), only that field
/// falls back to its default — every *other* field on disk is preserved.
///
/// This prevents the worst-case experience of a user losing their hotkey,
/// mic preference, and chime config just because we renamed a polish
/// provider's enum variant in a future release.
pub fn load(app_data_dir: &Path) -> Settings {
    let path = settings_path(app_data_dir);
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Settings::with_defaults(),
    };
    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!(?err, "settings.json is not valid JSON; using all defaults");
            return Settings::with_defaults();
        }
    };
    merge_into_defaults(value)
}

/// Walk the user's JSON object, parsing each known field individually.
/// On per-field failure, log + keep the in-memory default for that field.
fn merge_into_defaults(value: serde_json::Value) -> Settings {
    let mut settings = Settings::with_defaults();
    let map = match value {
        serde_json::Value::Object(m) => m,
        _ => {
            tracing::warn!("settings.json root isn't a JSON object; using all defaults");
            return settings;
        }
    };

    fn pick<T: serde::de::DeserializeOwned>(
        map: &serde_json::Map<String, serde_json::Value>,
        key: &str,
        slot: &mut T,
    ) {
        let Some(raw) = map.get(key) else {
            return;
        };
        match serde_json::from_value::<T>(raw.clone()) {
            Ok(parsed) => *slot = parsed,
            Err(err) => tracing::warn!(
                field = key,
                ?err,
                "settings field couldn't be parsed; keeping default"
            ),
        }
    }

    pick(&map, "hotkey", &mut settings.hotkey);
    pick(&map, "stt_provider", &mut settings.stt_provider);
    pick(&map, "polish_provider", &mut settings.polish_provider);
    pick(&map, "language", &mut settings.language);
    pick(&map, "model_name", &mut settings.model_name);
    pick(&map, "microphone_name", &mut settings.microphone_name);
    pick(&map, "play_start_chime", &mut settings.play_start_chime);
    pick(&map, "play_stop_chime", &mut settings.play_stop_chime);
    pick(&map, "auto_paste", &mut settings.auto_paste);
    pick(
        &map,
        "max_recording_seconds",
        &mut settings.max_recording_seconds,
    );
    pick(
        &map,
        "onboarding_completed",
        &mut settings.onboarding_completed,
    );
    pick(&map, "paused", &mut settings.paused);
    pick(
        &map,
        "show_recording_overlay",
        &mut settings.show_recording_overlay,
    );
    pick(&map, "onboarding_step", &mut settings.onboarding_step);

    settings
}

pub fn save(app_data_dir: &Path, settings: &Settings) -> anyhow::Result<()> {
    std::fs::create_dir_all(app_data_dir)?;
    let path = settings_path(app_data_dir);
    let json = serde_json::to_string_pretty(settings)?;
    std::fs::write(path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unknown_polish_provider_doesnt_wipe_hotkey() {
        // Simulate a v0.1.x user whose settings.json has a now-renamed
        // polish provider value. We must preserve every other field.
        let raw = json!({
            "hotkey": { "chord": "Cmd+Shift+H" },
            "polish_provider": "this_provider_was_renamed",
            "microphone_name": "External Mic",
            "play_start_chime": false,
        });
        let s = merge_into_defaults(raw);
        assert_eq!(s.hotkey.chord, "Cmd+Shift+H");
        assert_eq!(s.microphone_name.as_deref(), Some("External Mic"));
        assert!(!s.play_start_chime);
        // polish_provider fell back to its default
        assert_eq!(s.polish_provider, PolishProvider::default());
    }

    #[test]
    fn malformed_field_doesnt_wipe_others() {
        // hotkey is the wrong shape (a string instead of an object).
        let raw = json!({
            "hotkey": "not an object",
            "stt_provider": "groq",
            "play_stop_chime": true,
        });
        let s = merge_into_defaults(raw);
        // hotkey fell back to default; the rest survived.
        assert_eq!(s.hotkey.chord, "Fn");
        assert_eq!(s.stt_provider, SttProvider::Groq);
        assert!(s.play_stop_chime);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let raw = json!({});
        let s = merge_into_defaults(raw);
        assert_eq!(s, Settings::with_defaults());
    }

    #[test]
    fn fully_populated_round_trips() {
        let mut original = Settings::with_defaults();
        original.hotkey.chord = "RightOption".into();
        original.polish_provider = PolishProvider::Claude;
        original.play_stop_chime = true;
        original.onboarding_completed = true;

        let json_value = serde_json::to_value(&original).unwrap();
        let reloaded = merge_into_defaults(json_value);

        assert_eq!(reloaded.hotkey.chord, "RightOption");
        assert_eq!(reloaded.polish_provider, PolishProvider::Claude);
        assert!(reloaded.play_stop_chime);
        assert!(reloaded.onboarding_completed);
    }

    #[test]
    fn non_object_root_yields_full_defaults() {
        let raw = json!([1, 2, 3]);
        let s = merge_into_defaults(raw);
        assert_eq!(s, Settings::with_defaults());
    }
}
