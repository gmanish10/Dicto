use crate::state::{PipelineState, SharedState};
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Emitter, Manager};

/// Install the menubar tray. The tray menu shows: Open Dicto, Pause/Resume,
/// History, Settings, Check for Updates, About, Quit. Left-click opens main window.
///
/// The tray icon is intentionally static. We tried an animated pulse on
/// Recording / Transcribing earlier in v0.2.0; it didn't read as
/// animation at menubar size on macOS 26 (the user couldn't reliably
/// notice it even with sharply-distinct shapes + main-thread dispatch).
/// State feedback already comes through the tray tooltip and the audio
/// chimes, so a static icon is the right call until we have a clear UX
/// win to justify the complexity.
pub fn install(app: &AppHandle, state: SharedState) -> tauri::Result<()> {
    let menu = build_menu(app, &state)?;
    let icon_bytes = include_bytes!("../icons/tray-idle.png");
    let icon = Image::from_bytes(icon_bytes)?;

    let _tray = TrayIconBuilder::with_id("dicto-tray")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Dicto — idle")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event({
            let app = app.clone();
            move |_tray, event| {
                if let tauri::tray::TrayIconEvent::Click {
                    button: tauri::tray::MouseButton::Left,
                    button_state: tauri::tray::MouseButtonState::Up,
                    ..
                } = event
                {
                    tracing::debug!("tray icon left-clicked");
                    open_main_window(&app);
                }
            }
        })
        .on_menu_event({
            let state = state.clone();
            move |app, event| handle_menu_event(app, &state, event.id().as_ref())
        })
        .build(app)?;
    Ok(())
}

fn open_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        tracing::error!("main window not found");
        return;
    };
    if window.is_minimized().unwrap_or(false) {
        let _ = window.unminimize();
    }
    let _ = window.show();
    let _ = window.set_focus();
    // Bring Dicto to the front even if another app currently has focus.
    #[cfg(target_os = "macos")]
    macos_activate();
}

#[cfg(target_os = "macos")]
fn macos_activate() {
    use objc2::msg_send;
    use objc2::runtime::AnyClass;
    unsafe {
        if let Some(cls) = AnyClass::get("NSApplication") {
            let app_obj: *mut objc2::runtime::AnyObject = msg_send![cls, sharedApplication];
            if !app_obj.is_null() {
                let _: () = msg_send![app_obj, activateIgnoringOtherApps: true];
            }
        }
    }
}

fn build_menu(app: &AppHandle, state: &SharedState) -> tauri::Result<Menu<tauri::Wry>> {
    let paused = state.config.read().paused;
    let pause_label = if paused {
        "Resume Dicto"
    } else {
        "Pause Dicto"
    };

    let open = MenuItem::with_id(app, "open", "Open Dicto…", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", pause_label, true, None::<&str>)?;
    let history = MenuItem::with_id(app, "history", "History", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let check_updates = MenuItem::with_id(
        app,
        "check-updates",
        "Check for Updates…",
        true,
        None::<&str>,
    )?;
    let about = MenuItem::with_id(app, "about", "About Dicto", true, None::<&str>)?;
    let separator2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &open,
            &pause,
            &history,
            &settings,
            &separator,
            &check_updates,
            &about,
            &separator2,
            &quit,
        ],
    )
}

fn handle_menu_event(app: &AppHandle, state: &SharedState, id: &str) {
    match id {
        "open" | "settings" => {
            open_main_window(app);
            let _ = app.emit("nav:goto", "/settings");
        }
        "history" => {
            open_main_window(app);
            let _ = app.emit("nav:goto", "/history");
        }
        "about" => {
            open_main_window(app);
            let _ = app.emit("nav:goto", "/about");
        }
        "pause" => {
            let mut cfg = state.config.write();
            cfg.paused = !cfg.paused;
            drop(cfg);
            let _ = state.save_settings();
            // Rebuild the menu to flip the label.
            if let Ok(menu) = build_menu(app, state) {
                if let Some(tray) = app.tray_by_id("dicto-tray") {
                    let _ = tray.set_menu(Some(menu));
                }
            }
            update_tooltip(app, state);
        }
        "check-updates" => {
            let _ = app.emit("update:check-requested", ());
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}

pub fn update_state_indicator(app: &AppHandle, state: &SharedState) {
    update_tooltip(app, state);
    let pipeline = *state.pipeline_state.read();
    let _ = app.emit("pipeline:state", pipeline as i32);

    // The always-on-top "Recording" pill is driven from the same hook:
    // every set_pipeline_state caller already invokes this function, so
    // the overlay can never drift out of sync with the tooltip.
    let show_overlay = state.config.read().show_recording_overlay;
    crate::overlay::sync_for_state(app, pipeline, show_overlay);
}

fn update_tooltip(app: &AppHandle, state: &SharedState) {
    let paused = state.config.read().paused;
    let pipeline = *state.pipeline_state.read();
    let label = if paused {
        "Dicto — paused"
    } else {
        match pipeline {
            PipelineState::Idle => "Dicto — idle",
            PipelineState::Recording => "Dicto — recording…",
            PipelineState::Transcribing => "Dicto — transcribing…",
            PipelineState::UpdateAvailable => "Dicto — update available",
        }
    };
    if let Some(tray) = app.tray_by_id("dicto-tray") {
        let _ = tray.set_tooltip(Some(label));
    }
}

#[allow(dead_code)]
pub fn _ref_tray_icon(_: &TrayIcon) {}
