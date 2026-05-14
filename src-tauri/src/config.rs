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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    pub paused: bool,
    pub launch_at_login: bool,
    pub telemetry_opted_in: bool,
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
            launch_at_login: false,
            telemetry_opted_in: false,
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

pub fn load(app_data_dir: &Path) -> Settings {
    let path = settings_path(app_data_dir);
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|err| {
            tracing::warn!(?err, "settings file unreadable, using defaults");
            Settings::with_defaults()
        }),
        Err(_) => Settings::with_defaults(),
    }
}

pub fn save(app_data_dir: &Path, settings: &Settings) -> anyhow::Result<()> {
    std::fs::create_dir_all(app_data_dir)?;
    let path = settings_path(app_data_dir);
    let json = serde_json::to_string_pretty(settings)?;
    std::fs::write(path, json)?;
    Ok(())
}
