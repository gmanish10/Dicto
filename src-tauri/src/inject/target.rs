//! Track the app that was frontmost when recording started, so we can
//! refocus it before pasting.
//!
//! Without this, Cmd+V goes to whichever app happens to be frontmost at
//! paste time. If the user clicked away during transcription (or the
//! polish call took a moment and they moved their cursor), the
//! transcript pastes into the wrong window — silently and invisibly.
//! Worse, we still wrote to the clipboard, so the "missing" text turns
//! up only when the user goes hunting through history.
//!
//! macOS only — other platforms get no-op stubs.

#![allow(dead_code)]

/// Process identifier of an app captured via `NSWorkspace`. Wraps `pid_t`
/// to make accidental misuse harder.
#[derive(Debug, Clone, Copy)]
pub struct TargetApp(i32);

impl TargetApp {
    pub fn pid(&self) -> i32 {
        self.0
    }
}

/// Capture the currently-frontmost app. Returns `None` if we couldn't
/// query (permissions, no frontmost app, AppKit unavailable on this
/// platform). Safe to call from any thread.
pub fn capture_frontmost() -> Option<TargetApp> {
    #[cfg(target_os = "macos")]
    {
        macos::frontmost_pid().map(TargetApp)
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

/// Activate (focus) the captured app. Best-effort — if the app has
/// quit, or focus is denied, we log and let the caller proceed without
/// the refocus. Returns true if activation was issued; false otherwise.
pub fn activate(target: TargetApp) -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::activate_pid(target.0)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = target;
        false
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use objc2::msg_send;
    use objc2::runtime::AnyClass;

    /// Returns the PID of the frontmost app, or `None` if we can't get one.
    pub(super) fn frontmost_pid() -> Option<i32> {
        unsafe {
            let workspace_cls = AnyClass::get("NSWorkspace")?;
            let workspace: *mut objc2::runtime::AnyObject =
                msg_send![workspace_cls, sharedWorkspace];
            if workspace.is_null() {
                return None;
            }
            let app: *mut objc2::runtime::AnyObject = msg_send![workspace, frontmostApplication];
            if app.is_null() {
                return None;
            }
            let pid: i32 = msg_send![app, processIdentifier];
            // pid <= 0 means "no PID" in Cocoa-land.
            if pid <= 0 {
                return None;
            }
            Some(pid)
        }
    }

    /// `NSApplicationActivateIgnoringOtherApps` — bring this app to
    /// front even when another app is currently key.
    const ACTIVATE_IGNORE_OTHERS: usize = 1 << 1;

    /// Look up the app by PID and call `activateWithOptions:`.
    pub(super) fn activate_pid(pid: i32) -> bool {
        unsafe {
            let cls = match AnyClass::get("NSRunningApplication") {
                Some(c) => c,
                None => return false,
            };
            let app: *mut objc2::runtime::AnyObject =
                msg_send![cls, runningApplicationWithProcessIdentifier: pid];
            if app.is_null() {
                return false;
            }
            let _: bool = msg_send![app, activateWithOptions: ACTIVATE_IGNORE_OTHERS];
            true
        }
    }
}
