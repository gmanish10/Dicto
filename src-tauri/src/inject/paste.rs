use super::{InjectError, Injector};
use crossbeam_channel::{unbounded, Sender};
use once_cell::sync::Lazy;
use std::time::{Duration, Instant};

/// Inject text into the focused app by:
/// 1. Snapshot the current clipboard.
/// 2. Write our text to the clipboard.
/// 3. Synthesize Cmd+V via raw CGEvent (thread-safe; no TSM involvement).
/// 4. Restore the clipboard after a short delay unless the user changed it.
pub struct ClipboardPasteInjector;

/// Write `text` to the system clipboard without synthesizing a paste.
/// Used by the dictation pipeline when the user has `auto_paste` off —
/// the polished result still ends up on the clipboard so they can paste
/// manually, but Cmd+V isn't fired into whatever app is frontmost.
pub fn copy_to_clipboard(text: &str) -> Result<(), InjectError> {
    if text.is_empty() {
        return Ok(());
    }
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| InjectError::Clipboard(e.to_string()))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| InjectError::Clipboard(e.to_string()))?;
    Ok(())
}

/// One queued clipboard-restore job: after `not_before` has elapsed,
/// if the clipboard still contains `our_text`, replace it with `prior_text`.
struct RestoreJob {
    not_before: Instant,
    our_text: String,
    prior_text: String,
}

/// Shared sender to the single restore worker thread. Spawned lazily on
/// first paste, never joins. Replaces the previous thread-per-paste
/// pattern which created thread churn under frequent dictation.
static RESTORE_TX: Lazy<Sender<RestoreJob>> = Lazy::new(|| {
    let (tx, rx) = unbounded::<RestoreJob>();
    std::thread::Builder::new()
        .name("dicto-clipboard-restore".into())
        .spawn(move || {
            while let Ok(job) = rx.recv() {
                let now = Instant::now();
                if job.not_before > now {
                    std::thread::sleep(job.not_before - now);
                }
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    if cb.get_text().ok().as_deref() == Some(job.our_text.as_str()) {
                        let _ = cb.set_text(job.prior_text);
                    }
                }
            }
        })
        .expect("failed to spawn clipboard-restore worker");
    tx
});

/// Queue a clipboard restore on the shared worker. Cheap: just sends
/// onto an unbounded channel.
fn queue_clipboard_restore(our_text: String, prior_text: String) {
    let job = RestoreJob {
        not_before: Instant::now() + Duration::from_millis(200),
        our_text,
        prior_text,
    };
    let _ = RESTORE_TX.send(job);
}

impl Injector for ClipboardPasteInjector {
    fn inject(&self, text: &str) -> Result<(), InjectError> {
        if text.is_empty() {
            return Ok(());
        }

        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| InjectError::Clipboard(e.to_string()))?;
        let prior = clipboard.get_text().ok();
        clipboard
            .set_text(text.to_string())
            .map_err(|e| InjectError::Clipboard(e.to_string()))?;

        #[cfg(target_os = "macos")]
        {
            if is_secure_input_active() {
                // Leave the clipboard set; the user can paste manually.
                return Err(InjectError::SecureInputActive);
            }
            post_cmd_v_macos().map_err(InjectError::Event)?;
        }

        // Restore the previous clipboard contents on a short delay, but only if
        // the user didn't manually replace it during that window.
        // A single long-lived worker thread (see `RESTORE_TX`) serializes
        // every restore — under frequent dictation this is much cheaper
        // than spawning one OS thread per utterance.
        if let Some(prior_text) = prior {
            queue_clipboard_restore(text.to_string(), prior_text);
        }

        Ok(())
    }
}

// --- macOS Cmd+V via CGEvent ----------------------------------------------

#[cfg(target_os = "macos")]
const KC_V: u16 = 9;

#[cfg(target_os = "macos")]
const CG_EVENT_FLAG_COMMAND: u64 = 0x00100000;

#[cfg(target_os = "macos")]
const CG_HID_EVENT_TAP: u32 = 0; // kCGHIDEventTap

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceCreate(state_id: u32) -> *mut std::ffi::c_void;
    fn CGEventCreateKeyboardEvent(
        source: *mut std::ffi::c_void,
        virtual_key: u16,
        key_down: bool,
    ) -> *mut std::ffi::c_void;
    fn CGEventSetFlags(event: *mut std::ffi::c_void, flags: u64);
    fn CGEventPost(tap: u32, event: *mut std::ffi::c_void);
    fn CFRelease(cf: *mut std::ffi::c_void);
}

#[cfg(target_os = "macos")]
fn post_cmd_v_macos() -> Result<(), String> {
    unsafe {
        // kCGEventSourceStateHIDSystemState = 1
        let source = CGEventSourceCreate(1);
        if source.is_null() {
            return Err("CGEventSourceCreate failed".into());
        }
        let key_down = CGEventCreateKeyboardEvent(source, KC_V, true);
        let key_up = CGEventCreateKeyboardEvent(source, KC_V, false);
        if key_down.is_null() || key_up.is_null() {
            if !key_down.is_null() {
                CFRelease(key_down);
            }
            if !key_up.is_null() {
                CFRelease(key_up);
            }
            CFRelease(source);
            return Err("CGEventCreateKeyboardEvent failed".into());
        }
        CGEventSetFlags(key_down, CG_EVENT_FLAG_COMMAND);
        CGEventSetFlags(key_up, CG_EVENT_FLAG_COMMAND);
        CGEventPost(CG_HID_EVENT_TAP, key_down);
        CGEventPost(CG_HID_EVENT_TAP, key_up);
        CFRelease(key_down);
        CFRelease(key_up);
        CFRelease(source);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn IsSecureEventInputEnabled() -> bool;
}

#[cfg(target_os = "macos")]
fn is_secure_input_active() -> bool {
    unsafe { IsSecureEventInputEnabled() }
}
