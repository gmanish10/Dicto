use crate::{
    config::Settings,
    dictionary,
    history::TranscriptRow,
    keychain::{self, ApiKey},
    permissions::{self, PermissionStatus, PermissionsSnapshot},
    state::SharedState,
};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn get_settings(state: State<'_, SharedState>) -> Settings {
    state.config.read().clone()
}

#[tauri::command]
pub fn set_settings(state: State<'_, SharedState>, settings: Settings) -> Result<(), String> {
    *state.config.write() = settings;
    state.save_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn check_permissions() -> PermissionsSnapshot {
    permissions::snapshot()
}

#[tauri::command]
pub async fn request_microphone_permission() -> PermissionStatus {
    permissions::request_microphone().await
}

#[tauri::command]
pub fn open_system_settings(pane: String) -> Result<(), String> {
    permissions::open_settings_pane(&pane).map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct MicrophoneInfo {
    pub name: String,
    pub is_default: bool,
}

#[tauri::command]
pub fn list_microphones() -> Result<Vec<MicrophoneInfo>, String> {
    crate::audio::recorder::list_input_devices().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_history(
    state: State<'_, SharedState>,
    limit: Option<u32>,
) -> Result<Vec<TranscriptRow>, String> {
    state
        .history
        .list_recent(limit.unwrap_or(20))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_history(state: State<'_, SharedState>) -> Result<(), String> {
    state.history.clear().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_dictionary_words(
    state: State<'_, SharedState>,
) -> Result<Vec<dictionary::CustomWord>, String> {
    state.history.list_custom_words().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_dictionary_word(
    state: State<'_, SharedState>,
    word: String,
    weight: i64,
) -> Result<(), String> {
    state
        .history
        .add_custom_word(&word, weight)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_dictionary_word(state: State<'_, SharedState>, id: i64) -> Result<(), String> {
    state
        .history
        .delete_custom_word(id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_replacements(
    state: State<'_, SharedState>,
) -> Result<Vec<dictionary::Replacement>, String> {
    state.history.list_replacements().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn upsert_replacement(
    state: State<'_, SharedState>,
    trigger: String,
    replacement: String,
    case_sensitive: bool,
) -> Result<(), String> {
    state
        .history
        .upsert_replacement(&trigger, &replacement, case_sensitive)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_replacement(state: State<'_, SharedState>, id: i64) -> Result<(), String> {
    state
        .history
        .delete_replacement(id)
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct ApiKeyStatus {
    pub key: ApiKey,
    pub configured: bool,
}

#[tauri::command]
pub fn get_api_key_status() -> Vec<ApiKeyStatus> {
    ApiKey::all()
        .into_iter()
        .map(|k| ApiKeyStatus {
            configured: keychain::exists(k),
            key: k,
        })
        .collect()
}

#[tauri::command]
pub fn set_api_key(key: ApiKey, value: String) -> Result<(), String> {
    keychain::set(key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_api_key(key: ApiKey) -> Result<(), String> {
    keychain::delete(key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_hotkey(state: State<'_, SharedState>, chord: String) -> Result<(), String> {
    state.config.write().hotkey.chord = chord;
    state.save_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pause_dictation(state: State<'_, SharedState>) -> Result<(), String> {
    state.config.write().paused = true;
    state.save_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resume_dictation(state: State<'_, SharedState>) -> Result<(), String> {
    state.config.write().paused = false;
    state.save_settings().map_err(|e| e.to_string())
}

/// Check whether an update is available. Returns the new version string
/// if one is pending, or `None` if the user is already current.
#[tauri::command]
pub async fn recheck_for_updates(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app
        .updater()
        .map_err(|e| format!("updater unavailable: {e}"))?;
    match updater
        .check()
        .await
        .map_err(|e| format!("update check failed: {e}"))?
    {
        Some(update) => Ok(Some(update.version.clone())),
        None => Ok(None),
    }
}

/// Download and apply a pending update, then restart Dicto.
///
/// Caller-side flow:
/// 1. Show a "Downloading..." indicator.
/// 2. Invoke this command. It returns only on failure; on success the
///    process is replaced (Tauri restarts the new binary).
///
/// We don't wire progress events to the frontend in v0.1.2 (keeping the
/// fix minimal); a follow-up issue will add a progress bar.
#[tauri::command]
pub async fn install_pending_update(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app
        .updater()
        .map_err(|e| format!("updater unavailable: {e}"))?;
    let update = updater
        .check()
        .await
        .map_err(|e| format!("update check failed: {e}"))?
        .ok_or_else(|| "no update available".to_string())?;
    update
        .download_and_install(|_chunk_length, _content_length| {}, || {})
        .await
        .map_err(|e| format!("update install failed: {e}"))?;
    app.restart();
}

#[tauri::command]
pub fn reinject_transcript(state: State<'_, SharedState>, id: i64) -> Result<(), String> {
    let row = state
        .history
        .get_transcript(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "transcript not found".to_string())?;
    crate::inject::paste::ClipboardPasteInjector
        .inject(&row.polished)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn record_correction(
    state: State<'_, SharedState>,
    raw: String,
    final_text: String,
) -> Result<(), String> {
    state
        .history
        .add_correction(&raw, &final_text)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn finish_onboarding(state: State<'_, SharedState>) -> Result<(), String> {
    state.config.write().onboarding_completed = true;
    state.save_settings().map_err(|e| e.to_string())
}

// Trait import for the inject command above.
use crate::inject::Injector;
