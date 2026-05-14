//! macOS system notifications for events the user might miss when the
//! Dicto window is hidden.
//!
//! The rule: only emit a system notification when the user is *not*
//! currently looking at Dicto. If the main window is visible AND focused
//! we already have the in-window toast for them; doubling up adds noise.
//!
//! Notifications are best-effort — if the permission is denied or the
//! plugin errors, we log and move on rather than blocking the pipeline.

use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

/// Show a macOS notification if the Dicto window isn't currently
/// visible-and-focused. Safe to call from any pipeline step; never
/// returns an error to the caller.
pub fn notify_if_hidden(app: &AppHandle, title: &str, body: &str) {
    if window_is_in_user_view(app) {
        // User is already looking at Dicto — the in-window toast is enough.
        return;
    }
    send(app, title, body);
}

fn window_is_in_user_view(app: &AppHandle) -> bool {
    let Some(window) = app.get_webview_window("main") else {
        return false;
    };
    let visible = window.is_visible().unwrap_or(false);
    if !visible {
        return false;
    }
    // Visible isn't enough on macOS — a window can be visible-but-hidden
    // behind another app. Check focus too. If focus state is unknown,
    // err on the side of showing the notification.
    window.is_focused().unwrap_or(false)
}

fn send(app: &AppHandle, title: &str, body: &str) {
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        tracing::warn!(error = %e, "notification failed");
    }
}
