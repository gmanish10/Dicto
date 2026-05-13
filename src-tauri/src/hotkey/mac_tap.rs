//! Minimal macOS CGEventTap-based hotkey listener.
//!
//! Replaces `rdev` for our needs. We don't translate keycodes to characters or
//! call into Carbon's UCKeyTranslate, which is where `rdev` panics on certain
//! macOS / keyboard combinations. We only need:
//!   - modifier flag transitions (Fn, Cmd, Shift, Option, Control)
//!   - a few specific main keys mapped from raw keycodes
//!
//! The event tap callback runs on a Cocoa thread inside CFRunLoopRun. It's
//! `extern "C"`, so we must never let a panic escape it; everything is wrapped
//! in `catch_unwind`.
#![allow(non_upper_case_globals)]

use super::{HotkeyEvent, ParsedHotkey};
use crossbeam_channel::Sender;
use parking_lot::RwLock;
use std::ffi::c_void;
use std::sync::Arc;

// Raw keycodes for the small set of "main" keys we support.
// Values match Apple's HIToolbox/Events.h.
const KC_SPACE: i64 = 49;
const KC_TAB: i64 = 48;
const KC_RETURN: i64 = 36;
const KC_ESCAPE: i64 = 53;
const KC_A: i64 = 0;
const KC_S: i64 = 1;
const KC_D: i64 = 2;
const KC_F: i64 = 3;
const KC_H: i64 = 4;
const KC_G: i64 = 5;
const KC_Z: i64 = 6;
const KC_X: i64 = 7;
const KC_C: i64 = 8;
const KC_V: i64 = 9;
const KC_B: i64 = 11;
const KC_Q: i64 = 12;
const KC_W: i64 = 13;
const KC_E: i64 = 14;
const KC_R: i64 = 15;
const KC_Y: i64 = 16;
const KC_T: i64 = 17;
const KC_O: i64 = 31;
const KC_U: i64 = 32;
const KC_I: i64 = 34;
const KC_P: i64 = 35;
const KC_L: i64 = 37;
const KC_J: i64 = 38;
const KC_K: i64 = 40;
const KC_N: i64 = 45;
const KC_M: i64 = 46;

fn keycode_for(target: &rdev::Key) -> Option<i64> {
    use rdev::Key::*;
    Some(match target {
        Space => KC_SPACE,
        Tab => KC_TAB,
        Return => KC_RETURN,
        Escape => KC_ESCAPE,
        KeyA => KC_A,
        KeyB => KC_B,
        KeyC => KC_C,
        KeyD => KC_D,
        KeyE => KC_E,
        KeyF => KC_F,
        KeyG => KC_G,
        KeyH => KC_H,
        KeyI => KC_I,
        KeyJ => KC_J,
        KeyK => KC_K,
        KeyL => KC_L,
        KeyM => KC_M,
        KeyN => KC_N,
        KeyO => KC_O,
        KeyP => KC_P,
        KeyQ => KC_Q,
        KeyR => KC_R,
        KeyS => KC_S,
        KeyT => KC_T,
        KeyU => KC_U,
        KeyV => KC_V,
        KeyW => KC_W,
        KeyX => KC_X,
        KeyY => KC_Y,
        KeyZ => KC_Z,
        _ => return None,
    })
}

mod flag_bits {
    pub const SHIFT: u64 = 0x00020000;
    pub const CONTROL: u64 = 0x00040000;
    pub const OPTION: u64 = 0x00080000;
    pub const COMMAND: u64 = 0x00100000;
    pub const FN: u64 = 0x00800000;
    pub const OPTION_LEFT: u64 = 0x20;
    pub const OPTION_RIGHT: u64 = 0x40;
}

#[derive(Default, Clone, Copy)]
struct ModState {
    cmd: bool,
    shift: bool,
    control: bool,
    option_left: bool,
    option_right: bool,
    fn_key: bool,
    main_key_down: bool,
    fired: bool,
}

struct CallbackContext {
    state: parking_lot::Mutex<ModState>,
    hotkey: Arc<RwLock<Option<ParsedHotkey>>>,
    paused: Arc<RwLock<bool>>,
    tx: Sender<HotkeyEvent>,
}

fn chord_satisfied(state: &ModState, hotkey: &ParsedHotkey) -> bool {
    let req = &hotkey.required;
    if req.cmd && !state.cmd {
        return false;
    }
    if req.shift && !state.shift {
        return false;
    }
    if req.control && !state.control {
        return false;
    }
    if req.fn_key && !state.fn_key {
        return false;
    }
    if req.option_left && req.option_right {
        if !state.option_left && !state.option_right {
            return false;
        }
    } else {
        if req.option_left && !state.option_left {
            return false;
        }
        if req.option_right && !state.option_right {
            return false;
        }
    }
    if hotkey.key.is_some() && !state.main_key_down {
        return false;
    }
    true
}

fn apply_flags(state: &mut ModState, flags: u64) {
    state.cmd = (flags & flag_bits::COMMAND) != 0;
    state.shift = (flags & flag_bits::SHIFT) != 0;
    state.control = (flags & flag_bits::CONTROL) != 0;
    state.fn_key = (flags & flag_bits::FN) != 0;
    let any_option = (flags & flag_bits::OPTION) != 0;
    let left_bit = (flags & flag_bits::OPTION_LEFT) != 0;
    let right_bit = (flags & flag_bits::OPTION_RIGHT) != 0;
    if any_option {
        if left_bit || right_bit {
            state.option_left = left_bit;
            state.option_right = right_bit;
        } else {
            state.option_left = true;
            state.option_right = true;
        }
    } else {
        state.option_left = false;
        state.option_right = false;
    }
}

const CG_EVENT_TYPE_KEY_DOWN: u32 = 10;
const CG_EVENT_TYPE_KEY_UP: u32 = 11;

extern "C" fn raw_callback(
    _proxy: *mut c_void,
    event_type: u32,
    cg_event: *mut c_void,
    user_info: *mut c_void,
) -> *mut c_void {
    // Defensive: absorb any panic so we never propagate UB through extern "C".
    let _ = std::panic::catch_unwind(|| unsafe {
        if user_info.is_null() || cg_event.is_null() {
            return;
        }
        let ctx = &*(user_info as *const CallbackContext);

        let flags = CGEventGetFlags(cg_event);
        let keycode = CGEventGetIntegerValueField(cg_event, KEYBOARD_KEYCODE_FIELD);

        let mut state_guard = ctx.state.lock();
        let was_fired = state_guard.fired;
        let prev_state = *state_guard;
        apply_flags(&mut state_guard, flags);
        if state_guard.fn_key != prev_state.fn_key
            || state_guard.cmd != prev_state.cmd
            || state_guard.option_left != prev_state.option_left
            || state_guard.option_right != prev_state.option_right
            || state_guard.shift != prev_state.shift
            || state_guard.control != prev_state.control
        {
            tracing::debug!(
                fn_key = state_guard.fn_key,
                cmd = state_guard.cmd,
                shift = state_guard.shift,
                control = state_guard.control,
                option_left = state_guard.option_left,
                option_right = state_guard.option_right,
                event_type,
                "modifier state changed"
            );
        }

        let hotkey_snapshot = ctx.hotkey.read().clone();
        if let Some(ref hotkey) = hotkey_snapshot {
            if let Some(target_key) = hotkey.key {
                if let Some(target_code) = keycode_for(&target_key) {
                    if keycode == target_code {
                        if event_type == CG_EVENT_TYPE_KEY_DOWN {
                            state_guard.main_key_down = true;
                        } else if event_type == CG_EVENT_TYPE_KEY_UP {
                            state_guard.main_key_down = false;
                        }
                    }
                }
            }
        }

        if *ctx.paused.read() {
            state_guard.fired = match hotkey_snapshot {
                Some(ref h) => chord_satisfied(&state_guard, h),
                None => false,
            };
            return;
        }

        let satisfied = match hotkey_snapshot {
            Some(ref h) => chord_satisfied(&state_guard, h),
            None => false,
        };
        if satisfied && !was_fired {
            state_guard.fired = true;
            tracing::info!("hotkey chord engaged → KeyDown");
            let _ = ctx.tx.send(HotkeyEvent::Down);
        } else if !satisfied && was_fired {
            state_guard.fired = false;
            tracing::info!("hotkey chord released → KeyUp");
            let _ = ctx.tx.send(HotkeyEvent::Up);
        }
        let _ = keycode;
    });
    cg_event
}

const KEYBOARD_KEYCODE_FIELD: u32 = 9; // kCGKeyboardEventKeycode
const CG_EVENT_MASK_KEY_DOWN: u64 = 1 << 10;
const CG_EVENT_MASK_KEY_UP: u64 = 1 << 11;
const CG_EVENT_MASK_FLAGS_CHANGED: u64 = 1 << 12;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        event_mask: u64,
        callback: extern "C" fn(*mut c_void, u32, *mut c_void, *mut c_void) -> *mut c_void,
        user_info: *mut c_void,
    ) -> *mut c_void;
    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
    fn CGEventGetFlags(event: *mut c_void) -> u64;
    fn CGEventGetIntegerValueField(event: *mut c_void, field: u32) -> i64;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFMachPortCreateRunLoopSource(
        allocator: *mut c_void,
        port: *mut c_void,
        order: isize,
    ) -> *mut c_void;
    fn CFRunLoopGetCurrent() -> *mut c_void;
    fn CFRunLoopAddSource(rl: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFRunLoopRun();
    static kCFRunLoopCommonModes: *const c_void;
}

/// Spawn the OS thread that owns the CGEventTap. Blocks the thread on
/// CFRunLoopRun for the life of the process.
pub fn spawn(
    tx: Sender<HotkeyEvent>,
    hotkey: Arc<RwLock<Option<ParsedHotkey>>>,
    paused: Arc<RwLock<bool>>,
) {
    std::thread::Builder::new()
        .name("dicto-hotkey".into())
        .spawn(move || {
            tracing::info!("dicto-hotkey thread starting");
            let ctx = Box::new(CallbackContext {
                state: parking_lot::Mutex::new(ModState::default()),
                hotkey,
                paused,
                tx,
            });
            let user_info = Box::into_raw(ctx) as *mut c_void;
            unsafe {
                let mask = CG_EVENT_MASK_KEY_DOWN
                    | CG_EVENT_MASK_KEY_UP
                    | CG_EVENT_MASK_FLAGS_CHANGED;
                tracing::info!(mask = format!("0x{mask:x}"), "creating CGEventTap");
                let tap = CGEventTapCreate(
                    0, // kCGHIDEventTap
                    0, // kCGHeadInsertEventTap
                    1, // kCGEventTapOptionListenOnly
                    mask,
                    raw_callback,
                    user_info,
                );
                if tap.is_null() {
                    tracing::error!(
                        "CGEventTapCreate returned NULL — likely missing Input Monitoring permission. \
                         Grant it under System Settings → Privacy & Security → Input Monitoring, then quit and relaunch Dicto."
                    );
                    let _ = Box::from_raw(user_info as *mut CallbackContext);
                    return;
                }
                tracing::info!("CGEventTapCreate succeeded");
                let source = CFMachPortCreateRunLoopSource(std::ptr::null_mut(), tap, 0);
                if source.is_null() {
                    tracing::error!("CFMachPortCreateRunLoopSource failed");
                    let _ = Box::from_raw(user_info as *mut CallbackContext);
                    return;
                }
                let current_loop = CFRunLoopGetCurrent();
                CFRunLoopAddSource(current_loop, source, kCFRunLoopCommonModes);
                CGEventTapEnable(tap, true);
                tracing::info!("CGEventTap installed, entering run loop");
                CFRunLoopRun();
                let _ = Box::from_raw(user_info as *mut CallbackContext);
            }
        })
        .expect("failed to spawn dicto-hotkey thread");
}
