//! Always-on-top "Listening" pill that floats above all apps so the
//! user can never miss that Dicto is recording.
//!
//! The window is declared in `tauri.conf.json` as `recording-overlay` —
//! transparent, frameless, no shadow, always-on-top, doesn't take
//! focus, doesn't appear in the Dock. We start it `visible: true`
//! and **never call `window.hide()`** — see the design note below.
//!
//! ## Why the window is always visible
//!
//! On macOS, a window that is hidden (`orderOut`) when a fullscreen
//! Space is activated never registers as an auxiliary in that Space.
//! If we later `show()` it, the WindowServer puts it in the regular
//! Space — which is hidden behind the fullscreen app, so the user
//! never sees it. Keeping the window visible at the OS layer
//! sidesteps this entirely.
//!
//! "Show/hide" is a frontend concept: we emit `overlay:set-visible`
//! and the React route renders the pill or `null`. The window itself
//! is always transparent + click-through so an empty render is
//! visually identical to a hidden window.
//!
//! ## NSWindow configuration
//!
//! Done once at startup via `init`:
//! - `collectionBehavior |= CanJoinAllSpaces | FullScreenAuxiliary |
//!   Stationary | IgnoresCycle` — the window joins every Space
//!   including native-fullscreen ones, doesn't animate during Space
//!   switches, and doesn't show up in Cmd-Tab.
//! - `level = kCGScreenSaverWindowLevel (1000)` — high enough to draw
//!   above fullscreen content. Tauri's `set_always_on_top(true)`
//!   alone gives NSFloatingWindowLevel (3), which fullscreen apps
//!   paint over.
//! - Click-through via `set_ignore_cursor_events(true)`.
//! - Positioned at top-center of the primary monitor.

use tauri::{AppHandle, Emitter, LogicalPosition, Manager};

const OVERLAY_LABEL: &str = "recording-overlay";

/// Vertical offset below the top of the screen (below the menubar).
const VERTICAL_OFFSET_PX: f64 = 32.0;

/// Width of the overlay window as configured in tauri.conf.json. Used
/// to center horizontally.
const OVERLAY_WIDTH_PX: f64 = 220.0;

/// Configure the overlay window once at app startup. Idempotent — safe
/// to call repeatedly if needed. Pushes the initial "not visible" state
/// to the frontend so the empty render is in place before any
/// pipeline state change.
pub fn init(app: &AppHandle) {
    let app_for_main = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(window) = app_for_main.get_webview_window(OVERLAY_LABEL) else {
            tracing::warn!("overlay: '{OVERLAY_LABEL}' window not found at init");
            return;
        };
        let _ = window.set_ignore_cursor_events(true);
        let _ = window.set_visible_on_all_workspaces(true);
        let _ = window.set_always_on_top(true);
        position_top_center(&app_for_main, &window);
        #[cfg(target_os = "macos")]
        configure_macos_window_for_fullscreen(&window);
        // Initial state: empty pill body. The window is visible at the
        // OS layer but renders nothing.
        let _ = app_for_main.emit_to(OVERLAY_LABEL, "overlay:set-visible", false);
        tracing::info!("overlay: initialized");
    });
}

/// Update the overlay's visibility from the current pipeline state.
/// Respects the `show_recording_overlay` setting — if the user opted
/// out, this is a no-op (we emit `false` so the pill doesn't paint).
pub fn sync_for_state(app: &AppHandle, state: crate::state::PipelineState, enabled: bool) {
    let should_show = enabled && matches!(state, crate::state::PipelineState::Recording);
    // Re-position on every show in case the user changed displays.
    if should_show {
        let app_for_main = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(window) = app_for_main.get_webview_window(OVERLAY_LABEL) {
                position_top_center(&app_for_main, &window);
            }
        });
    }
    let _ = app.emit_to(OVERLAY_LABEL, "overlay:set-visible", should_show);
}

/// Position the overlay window horizontally centered on the primary
/// monitor, just below the menubar.
fn position_top_center(app: &AppHandle, window: &tauri::WebviewWindow) {
    let monitor = match app.primary_monitor() {
        Ok(Some(m)) => m,
        _ => return,
    };
    let scale = monitor.scale_factor();
    let monitor_pos = monitor.position();
    let monitor_size = monitor.size();
    let monitor_width_logical = (monitor_size.width as f64) / scale;
    let monitor_x_logical = (monitor_pos.x as f64) / scale;
    let monitor_y_logical = (monitor_pos.y as f64) / scale;
    let x = monitor_x_logical + (monitor_width_logical - OVERLAY_WIDTH_PX) / 2.0;
    let y = monitor_y_logical + VERTICAL_OFFSET_PX;
    let _ = window.set_position(LogicalPosition::new(x, y));
}

/// Apply the NSWindow-level configuration that lets the overlay paint
/// above fullscreen apps and follow into every Space.
#[cfg(target_os = "macos")]
fn configure_macos_window_for_fullscreen(window: &tauri::WebviewWindow) {
    use objc2::msg_send;

    const CAN_JOIN_ALL_SPACES: u64 = 1 << 0;
    const STATIONARY: u64 = 1 << 4;
    const IGNORES_CYCLE: u64 = 1 << 6;
    const FULL_SCREEN_AUXILIARY: u64 = 1 << 8;

    const SCREEN_SAVER_WINDOW_LEVEL: i64 = 1000;

    let ns_window_ptr = match window.ns_window() {
        Ok(p) => p as *mut objc2::runtime::AnyObject,
        Err(e) => {
            tracing::warn!(error = %e, "overlay: ns_window() failed");
            return;
        }
    };
    if ns_window_ptr.is_null() {
        tracing::warn!("overlay: NSWindow ptr is null");
        return;
    }
    unsafe {
        let current: u64 = msg_send![ns_window_ptr, collectionBehavior];
        let combined =
            current | CAN_JOIN_ALL_SPACES | STATIONARY | IGNORES_CYCLE | FULL_SCREEN_AUXILIARY;
        let _: () = msg_send![ns_window_ptr, setCollectionBehavior: combined];
        let _: () = msg_send![ns_window_ptr, setLevel: SCREEN_SAVER_WINDOW_LEVEL];
    }
}
