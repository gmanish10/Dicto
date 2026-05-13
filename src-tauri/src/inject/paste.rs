use super::{InjectError, Injector};

/// Inject text into the focused app by:
/// 1. Snapshot the current clipboard.
/// 2. Write our text to the clipboard.
/// 3. Synthesize Cmd+V via raw CGEvent (thread-safe; no TSM involvement).
/// 4. Restore the clipboard after a short delay unless the user changed it.
pub struct ClipboardPasteInjector;

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
        if let Some(prior_text) = prior {
            let our_text = text.to_string();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(200));
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    if cb.get_text().ok().as_deref() == Some(our_text.as_str()) {
                        let _ = cb.set_text(prior_text);
                    }
                }
            });
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
