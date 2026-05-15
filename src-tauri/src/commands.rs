use crate::{
    config::Settings,
    dictionary,
    history::TranscriptRow,
    keychain::{self, ApiKey},
    permissions::{self, PermissionStatus, PermissionsSnapshot},
    state::SharedState,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

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
pub fn request_microphone_permission() -> PermissionStatus {
    permissions::request_microphone()
}

#[tauri::command]
pub fn request_accessibility_permission() -> PermissionStatus {
    permissions::request_accessibility()
}

#[tauri::command]
pub fn request_input_monitoring_permission() -> PermissionStatus {
    permissions::request_input_monitoring()
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
pub fn pause_dictation(app: AppHandle, state: State<'_, SharedState>) -> Result<(), String> {
    set_paused(&app, state.inner(), true)
}

#[tauri::command]
pub fn resume_dictation(app: AppHandle, state: State<'_, SharedState>) -> Result<(), String> {
    set_paused(&app, state.inner(), false)
}

fn set_paused(app: &AppHandle, state: &SharedState, paused: bool) -> Result<(), String> {
    state.config.write().paused = paused;
    let save_result = state.save_settings().map_err(|e| e.to_string());
    crate::menubar::refresh_pause_ui(app, state);
    let _ = app.emit("settings:updated", ());
    save_result
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

/// Start the dictation runtime — recorder service + global hotkey tap.
///
/// At app launch we defer this until the user finishes onboarding (see
/// the gate in `lib.rs`'s setup block), because creating the
/// `CGEventTap` triggers macOS's Input Monitoring permission prompt.
/// The redesigned onboarding flow calls this command from the final
/// step so the prompt only fires *after* the user has been told why.
///
/// `spawn_coordinator` itself is idempotent via an `AtomicBool` flag,
/// so this command is safe to call multiple times — duplicate IPC
/// (e.g. a React remount mid-onboarding) won't spawn a second tap.
#[tauri::command]
pub fn start_runtime(app: AppHandle, state: State<'_, SharedState>) {
    crate::pipeline::spawn_coordinator(app, state.inner().clone());
}

#[tauri::command]
pub fn finish_onboarding(app: AppHandle, state: State<'_, SharedState>) -> Result<(), String> {
    {
        let mut cfg = state.config.write();
        cfg.onboarding_completed = true;
        // Clear the resume marker so a future re-onboarding (or a reinstall
        // that inherits this settings.json) starts cleanly at Welcome
        // rather than resuming onto the Permissions step.
        cfg.onboarding_step.clear();
    }
    state.save_settings().map_err(|e| e.to_string())?;
    // Chain the runtime spawn so the React side only has to round-trip
    // once: marking onboarding done implies "I've granted everything,
    // please start the dictation pipeline now."
    crate::pipeline::spawn_coordinator(app, state.inner().clone());
    Ok(())
}

// -- Bundled LLM model availability + download ----------------------------

use crate::polish::bundled_llm::manifest as bundled_llm_manifest;

/// Front-end-facing snapshot of polish-tier availability. Each on-device
/// engine reports whether it's usable on this machine so the Settings UI
/// can show meaningful status pills (Ready / Needs download / macOS 26+).
#[derive(Serialize)]
pub struct PolishAvailability {
    pub bundled_llm: BundledLlmStatus,
    pub apple_intelligence: AppleIntelligenceStatus,
}

#[derive(Serialize)]
pub struct BundledLlmStatus {
    /// True when the GGUF file is on disk.
    pub downloaded: bool,
    /// Approximate download size for the UI ("Download 940 MB model").
    pub size_mb: u32,
    /// `Some` while a download is in flight, `None` otherwise.
    pub downloading: Option<DownloadProgress>,
}

/// Snapshot of the Apple Intelligence backend's readiness. `available`
/// is true iff the resolver succeeded in registering the polisher at
/// startup — which requires macOS 26+ *and* the bundled sidecar binary
/// being present. The final "Apple Intelligence is enabled by the user"
/// check happens on the sidecar's first spawn; we don't surface it here
/// because it would require eagerly spawning the sidecar at app start.
#[derive(Serialize)]
pub struct AppleIntelligenceStatus {
    pub available: bool,
}

#[derive(Serialize, Clone, Copy)]
pub struct DownloadProgress {
    pub bytes: u64,
    /// Total bytes; 0 if server didn't send Content-Length.
    pub total: u64,
}

#[tauri::command]
pub fn check_polish_availability(
    state: State<'_, SharedState>,
    app: AppHandle,
) -> PolishAvailability {
    let downloaded = crate::model::resolve_file(&app, bundled_llm_manifest::QWEN_FILENAME).is_ok();
    let downloading = *state.polish_model_download.read();
    let apple_intelligence_available = state.polish_ctx.read().apple_ai.is_some();
    PolishAvailability {
        bundled_llm: BundledLlmStatus {
            downloaded,
            size_mb: bundled_llm_manifest::QWEN_SIZE_MB,
            downloading,
        },
        apple_intelligence: AppleIntelligenceStatus {
            available: apple_intelligence_available,
        },
    }
}

#[tauri::command]
pub async fn start_polish_model_download(
    state: State<'_, SharedState>,
    app: AppHandle,
) -> Result<(), String> {
    // Refuse if a download is already in flight.
    {
        let guard = state.polish_model_download.read();
        if guard.is_some() {
            return Err("a download is already running".into());
        }
    }
    *state.polish_model_download.write() = Some(DownloadProgress { bytes: 0, total: 0 });

    let app_for_progress = app.clone();
    let state_for_progress = state.inner().clone();
    let progress = move |bytes: u64, total: u64| {
        let p = DownloadProgress { bytes, total };
        *state_for_progress.polish_model_download.write() = Some(p);
        let _ = app_for_progress.emit("polish-model:download-progress", p);
    };

    let result = crate::model::download_file(
        &app,
        bundled_llm_manifest::QWEN_URL,
        bundled_llm_manifest::QWEN_FILENAME,
        if bundled_llm_manifest::QWEN_SHA256.is_empty() {
            None
        } else {
            Some(bundled_llm_manifest::QWEN_SHA256)
        },
        progress,
    )
    .await;

    *state.polish_model_download.write() = None;

    match result {
        Ok(_) => {
            // Populate the resolver so subsequent polish calls route through
            // the new model without an app restart.
            if let Some(p) = crate::polish::try_construct_bundled_llm(&app) {
                state.polish_ctx.write().set_bundled_llm(Some(p));
            }
            let _ = app.emit("polish-model:download-complete", ());
            Ok(())
        }
        Err(e) => {
            let msg = format!("download failed: {e}");
            let _ = app.emit("polish-model:download-failed", msg.clone());
            Err(msg)
        }
    }
}
