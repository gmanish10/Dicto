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
use std::sync::{mpsc, Arc};

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
    state: Arc<parking_lot::Mutex<ModState>>,
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

/// Release-only flag reconcile for the HID poll. Clears a modifier when
/// `CGEventSourceFlagsState` says it is no longer held, but NEVER sets
/// one.
///
/// **Why release-only.** The HID flag state reports
/// `kCGEventFlagMaskSecondaryFn` for as long as a "function row" key —
/// arrow keys, F-keys, Page Up/Down, Home/End — is physically held down.
/// Trusting the poll to *engage* `fn_key` therefore let a long arrow-key
/// press silently satisfy the Fn hotkey, start the recorder, and paste a
/// garbage transcript on release. Chord engagement happens exclusively
/// through real `flagsChanged` events in the tap callback; the poll's
/// only job is to catch a *missed release*, so it must never set a bit.
fn reconcile_release(state: &mut ModState, flags: u64) {
    let mut fresh = ModState::default();
    apply_flags(&mut fresh, flags);
    state.cmd &= fresh.cmd;
    state.shift &= fresh.shift;
    state.control &= fresh.control;
    state.fn_key &= fresh.fn_key;
    state.option_left &= fresh.option_left;
    state.option_right &= fresh.option_right;
}

const CG_EVENT_TYPE_KEY_DOWN: u32 = 10;
const CG_EVENT_TYPE_KEY_UP: u32 = 11;
const CG_EVENT_TYPE_FLAGS_CHANGED: u32 = 12;

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
        // Only trust `flags` from flagsChanged events.
        //
        // **Why this gate matters.** macOS sets `kCGEventFlagMaskSecondaryFn`
        // in the event flags for every keyDown/keyUp of the "function row"
        // and its compatriots — arrow keys, F-keys, Page Up/Down, Home/End,
        // and a few more. The Fn flag isn't actually "set" by the user; it's
        // an OS-level annotation that the key being pressed lives on that
        // row. Updating our `fn_key` state from those events made arrow-key
        // presses spuriously satisfy the Fn hotkey: start chime would fire,
        // recorder would spin up, then the keyUp would tear it down — all
        // for a 50 ms arrow tap. The actual modifier keys (Fn, Cmd, Shift,
        // Control, Option) only emit flagsChanged events on press/release,
        // so we lose nothing by ignoring flags on keyDown/keyUp.
        if event_type == CG_EVENT_TYPE_FLAGS_CHANGED {
            apply_flags(&mut state_guard, flags);
        }
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
    /// Query the current modifier-flag state directly from the input
    /// source. Lets us reconcile our event-driven `ModState` against
    /// reality on a timer — important because if the user briefly taps
    /// Fn and then doesn't touch the keyboard, the matching release
    /// `flagsChanged` event never fires through our tap and the state
    /// stays "engaged" indefinitely.
    fn CGEventSourceFlagsState(state_id: i32) -> u64;
}

/// `kCGEventSourceStateHIDSystemState` — query the raw HID layer rather
/// than a per-session view. We want the truth from hardware, not whatever
/// the focused session believes.
const CG_EVENT_SOURCE_STATE_HID: i32 = 1;

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
///
/// Also spawns a companion thread that polls `CGEventSourceFlagsState`
/// every `POLL_INTERVAL_MS` to detect "stuck-modifier" scenarios — see
/// `spawn_modifier_poll` for the full failure mode.
pub fn spawn(
    tx: Sender<HotkeyEvent>,
    hotkey: Arc<RwLock<Option<ParsedHotkey>>>,
    paused: Arc<RwLock<bool>>,
) -> Result<(), String> {
    let shared_state = Arc::new(parking_lot::Mutex::new(ModState::default()));
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);

    let state_for_tap = shared_state;
    std::thread::Builder::new()
        .name("dicto-hotkey".into())
        .spawn(move || {
            tracing::info!("dicto-hotkey thread starting");
            let poll_state = state_for_tap.clone();
            let poll_hotkey = hotkey.clone();
            let poll_paused = paused.clone();
            let poll_tx = tx.clone();
            let ctx = Box::new(CallbackContext {
                state: state_for_tap,
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
                    let msg = "CGEventTapCreate returned NULL — likely missing Input Monitoring permission. Grant it under System Settings → Privacy & Security → Input Monitoring, then retry.";
                    tracing::error!("{msg}");
                    let _ = ready_tx.send(Err(msg.to_string()));
                    let _ = Box::from_raw(user_info as *mut CallbackContext);
                    return;
                }
                tracing::info!("CGEventTapCreate succeeded");
                let source = CFMachPortCreateRunLoopSource(std::ptr::null_mut(), tap, 0);
                if source.is_null() {
                    let msg = "CFMachPortCreateRunLoopSource failed";
                    tracing::error!("{msg}");
                    let _ = ready_tx.send(Err(msg.to_string()));
                    let _ = Box::from_raw(user_info as *mut CallbackContext);
                    return;
                }
                let current_loop = CFRunLoopGetCurrent();
                CFRunLoopAddSource(current_loop, source, kCFRunLoopCommonModes);
                CGEventTapEnable(tap, true);
                tracing::info!("CGEventTap installed, entering run loop");
                spawn_modifier_poll(poll_state, poll_hotkey, poll_paused, poll_tx);
                let _ = ready_tx.send(Ok(()));
                CFRunLoopRun();
                let _ = Box::from_raw(user_info as *mut CallbackContext);
            }
        })
        .map_err(|e| format!("failed to spawn dicto-hotkey thread: {e}"))?;

    ready_rx
        .recv()
        .unwrap_or_else(|_| Err("dicto-hotkey thread exited during startup".to_string()))
}

/// How often the modifier-poll thread reconciles our event-driven state
/// against the actual OS state. 200 ms keeps stuck-modifier recoveries
/// near-instant while costing ~5 wakeups/second — negligible.
const POLL_INTERVAL_MS: u64 = 200;

/// Companion thread to the CGEventTap that periodically queries the
/// real OS modifier-flag state and reconciles our event-driven
/// `ModState`.
///
/// **Why this exists.** Our tap only fires on key/flagsChanged events.
/// If the user briefly taps Fn (engaging the chord) and then stops
/// touching the keyboard — e.g., watches a video — the matching
/// flagsChanged-release event sometimes arrives well after the actual
/// physical release (the Globe key behavior on Apple Silicon, focus
/// transitions, and Tahoe's stricter input handling have all been
/// observed swallowing it). Without polling, we'd believe the chord is
/// held for arbitrary durations and record audio the user never
/// intended — and worse, paste it into the first thing they typed.
///
/// The poll calls `CGEventSourceFlagsState(HID)`, which returns the
/// authoritative current modifier state from the hardware layer.
/// `main_key_down` is intentionally NOT touched here — regular key
/// releases reliably generate `keyUp` events, and the poll has no view
/// into per-key state.
fn spawn_modifier_poll(
    state: Arc<parking_lot::Mutex<ModState>>,
    hotkey: Arc<RwLock<Option<ParsedHotkey>>>,
    paused: Arc<RwLock<bool>>,
    tx: Sender<HotkeyEvent>,
) {
    std::thread::Builder::new()
        .name("dicto-mod-poll".into())
        .spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS));

            let flags = unsafe { CGEventSourceFlagsState(CG_EVENT_SOURCE_STATE_HID) };

            let mut state_guard = state.lock();
            let was_fired = state_guard.fired;
            let prev = *state_guard;
            // Release-only: the poll catches a missed modifier *release*.
            // It must never engage a modifier — see `reconcile_release`.
            reconcile_release(&mut state_guard, flags);

            // No-op fast path: state didn't drift.
            if state_guard.fn_key == prev.fn_key
                && state_guard.cmd == prev.cmd
                && state_guard.shift == prev.shift
                && state_guard.control == prev.control
                && state_guard.option_left == prev.option_left
                && state_guard.option_right == prev.option_right
            {
                continue;
            }

            tracing::debug!(
                fn_key = state_guard.fn_key,
                cmd = state_guard.cmd,
                shift = state_guard.shift,
                control = state_guard.control,
                option_left = state_guard.option_left,
                option_right = state_guard.option_right,
                "modifier state reconciled from HID poll"
            );

            if *paused.read() {
                state_guard.fired = match hotkey.read().as_ref() {
                    Some(h) => chord_satisfied(&state_guard, h),
                    None => false,
                };
                continue;
            }

            let satisfied = match hotkey.read().as_ref() {
                Some(h) => chord_satisfied(&state_guard, h),
                None => false,
            };

            if !satisfied && was_fired {
                state_guard.fired = false;
                tracing::warn!(
                    "hotkey chord auto-released via HID poll — missed flagsChanged event upstream"
                );
                let _ = tx.send(HotkeyEvent::Up);
            }
        })
        .expect("failed to spawn dicto-mod-poll thread");
}
