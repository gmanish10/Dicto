pub mod audio;
pub mod chime;
pub mod commands;
pub mod config;
pub mod dictionary;
pub mod history;
pub mod hotkey;
pub mod inject;
pub mod keychain;
pub mod menubar;
pub mod model;
pub mod notify;
pub mod permissions;
pub mod pipeline;
pub mod polish;
pub mod state;
pub mod transcribe;

use std::sync::Arc;
use tauri::Manager;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,dicto_lib=debug")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        // NOTE: we deliberately don't use tauri_plugin_global_shortcut.
        // The custom CGEventTap in `hotkey/mac_tap.rs` handles
        // modifier-only chords (Fn, Right Option) that the plugin's
        // Carbon-based listener can't observe.
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let handle = app.handle().clone();

            let app_state = Arc::new(state::AppState::initialize(&handle)?);
            app.manage(app_state.clone());

            menubar::install(&handle, app_state.clone())?;
            // Defer the recorder thread + CGEventTap until onboarding
            // is finished. The CGEventTap is what triggers the macOS
            // Input Monitoring TCC prompt; running it at startup means
            // a freshly-installed Dicto fires a permission dialog
            // before the user has seen any UI explaining why. Gating
            // here keeps first-launch silent until the user clicks
            // "Grant" inside the redesigned onboarding flow, which
            // chains `start_runtime` from `finish_onboarding`.
            if app_state.config.read().onboarding_completed {
                pipeline::spawn_coordinator(handle.clone(), app_state.clone());
            } else {
                tracing::info!(
                    "onboarding not yet complete — runtime spawn deferred to start_runtime"
                );
            }

            // If the bundled LLM model is already on disk from a previous
            // session, populate the polish resolver now so Auto can route
            // to it immediately. Cheap — just an exists() check.
            if let Some(p) = polish::try_construct_bundled_llm(&handle) {
                app_state.polish_ctx.write().set_bundled_llm(Some(p));
                tracing::info!("bundled LLM model detected; resolver will route to it");
            }

            // Apple Intelligence: register the sidecar polisher on
            // macOS 26+ when the bundled binary is present. The sidecar
            // itself does the final "Foundation Models enabled?" check
            // when first spawned.
            if let Some(p) = polish::try_construct_apple_intelligence(&handle) {
                app_state.polish_ctx.write().set_apple_ai(Some(p));
                tracing::info!(
                    "apple-polish sidecar detected; resolver will route to Apple Intelligence"
                );
            }

            // Always show the main window on launch — easier to find than hunting
            // for the tray icon. Subsequent shows happen via tray clicks.
            if let Some(window) = handle.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_settings,
            commands::check_permissions,
            commands::request_microphone_permission,
            commands::open_system_settings,
            commands::list_microphones,
            commands::list_history,
            commands::clear_history,
            commands::list_dictionary_words,
            commands::add_dictionary_word,
            commands::delete_dictionary_word,
            commands::list_replacements,
            commands::upsert_replacement,
            commands::delete_replacement,
            commands::get_api_key_status,
            commands::set_api_key,
            commands::delete_api_key,
            commands::set_hotkey,
            commands::pause_dictation,
            commands::resume_dictation,
            commands::recheck_for_updates,
            commands::install_pending_update,
            commands::check_polish_availability,
            commands::start_polish_model_download,
            commands::reinject_transcript,
            commands::record_correction,
            commands::open_main_window,
            commands::finish_onboarding,
            commands::start_runtime,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                // Keep app alive when last window closes; user must quit from tray.
                api.prevent_exit();
            }
        });
}
