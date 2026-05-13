use super::{HotkeyEvent, ParsedHotkey};
use crossbeam_channel::Sender;
use parking_lot::RwLock;
use rdev::{listen, Event, EventType, Key};
use std::sync::Arc;

/// Parse a hotkey chord string like "RightOption", "Fn", "Cmd+Shift+Space" into a
/// `ParsedHotkey`. Returns None if the chord is empty or unparseable.
pub fn parse(chord: &str) -> Option<ParsedHotkey> {
    let mut parsed = ParsedHotkey::default();
    if chord.trim().is_empty() {
        return None;
    }
    for raw in chord.split('+') {
        let part = raw.trim();
        match part.to_ascii_lowercase().as_str() {
            "cmd" | "command" | "meta" | "super" | "win" => parsed.required.cmd = true,
            "shift" => parsed.required.shift = true,
            "option" | "alt" => {
                // Plain Option matches either side.
                parsed.required.option_left = true;
                parsed.required.option_right = true;
            }
            "leftoption" | "loption" => parsed.required.option_left = true,
            "rightoption" | "roption" => parsed.required.option_right = true,
            "ctrl" | "control" => parsed.required.control = true,
            "fn" | "function" => parsed.required.fn_key = true,
            "space" => parsed.key = Some(Key::Space),
            "tab" => parsed.key = Some(Key::Tab),
            "return" | "enter" => parsed.key = Some(Key::Return),
            "escape" | "esc" => parsed.key = Some(Key::Escape),
            // Single-letter keys (a..z, 0..9).
            other if other.len() == 1 => {
                let c = other.chars().next().unwrap();
                parsed.key = letter_to_key(c);
            }
            _ => return None,
        }
    }
    if parsed.key.is_none() && parsed.required.is_empty() {
        return None;
    }
    Some(parsed)
}

fn letter_to_key(c: char) -> Option<Key> {
    match c.to_ascii_lowercase() {
        'a' => Some(Key::KeyA),
        'b' => Some(Key::KeyB),
        'c' => Some(Key::KeyC),
        'd' => Some(Key::KeyD),
        'e' => Some(Key::KeyE),
        'f' => Some(Key::KeyF),
        'g' => Some(Key::KeyG),
        'h' => Some(Key::KeyH),
        'i' => Some(Key::KeyI),
        'j' => Some(Key::KeyJ),
        'k' => Some(Key::KeyK),
        'l' => Some(Key::KeyL),
        'm' => Some(Key::KeyM),
        'n' => Some(Key::KeyN),
        'o' => Some(Key::KeyO),
        'p' => Some(Key::KeyP),
        'q' => Some(Key::KeyQ),
        'r' => Some(Key::KeyR),
        's' => Some(Key::KeyS),
        't' => Some(Key::KeyT),
        'u' => Some(Key::KeyU),
        'v' => Some(Key::KeyV),
        'w' => Some(Key::KeyW),
        'x' => Some(Key::KeyX),
        'y' => Some(Key::KeyY),
        'z' => Some(Key::KeyZ),
        _ => None,
    }
}

/// Tracks which physical keys are currently pressed.
#[derive(Default, Debug, Clone, Copy)]
struct KeyState {
    cmd: bool,
    shift: bool,
    option_left: bool,
    option_right: bool,
    control: bool,
    fn_key: bool,
    main_key_down: bool,
    /// Whether we have already emitted a Down event for the current chord-press.
    /// Cleared when the chord becomes un-satisfied.
    fired: bool,
}

fn update_key(state: &mut KeyState, ev: &EventType) {
    match ev {
        EventType::KeyPress(k) => match k {
            Key::MetaLeft | Key::MetaRight => state.cmd = true,
            Key::ShiftLeft | Key::ShiftRight => state.shift = true,
            Key::Alt => state.option_left = true,
            Key::AltGr => state.option_right = true,
            Key::ControlLeft | Key::ControlRight => state.control = true,
            Key::Function => state.fn_key = true,
            _ => {}
        },
        EventType::KeyRelease(k) => match k {
            Key::MetaLeft | Key::MetaRight => state.cmd = false,
            Key::ShiftLeft | Key::ShiftRight => state.shift = false,
            Key::Alt => state.option_left = false,
            Key::AltGr => state.option_right = false,
            Key::ControlLeft | Key::ControlRight => state.control = false,
            Key::Function => state.fn_key = false,
            _ => {}
        },
        _ => {}
    }
}

fn chord_satisfied(state: &KeyState, hotkey: &ParsedHotkey) -> bool {
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
    // For Option, either side counts when both are required (plain "Option" alias).
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

/// Spawn the rdev listener thread. Blocks the thread on `rdev::listen` (OS-level callback loop).
///
/// The thread reads the *current* hotkey config from `current_hotkey` on every event,
/// so settings changes take effect immediately without restarting the thread.
///
/// `paused` is a flag the menubar "Pause" toggle flips.
pub fn spawn(
    tx: Sender<HotkeyEvent>,
    current_hotkey: Arc<RwLock<Option<ParsedHotkey>>>,
    paused: Arc<RwLock<bool>>,
) {
    std::thread::Builder::new()
        .name("dicto-hotkey".to_string())
        .spawn(move || {
            let mut state = KeyState::default();

            let callback = move |event: Event| {
                update_key(&mut state, &event.event_type);

                let track_main = |state: &mut KeyState, ev: &EventType, hotkey_key: Option<Key>| {
                    if let Some(target) = hotkey_key {
                        if let EventType::KeyPress(k) = ev {
                            if *k == target {
                                state.main_key_down = true;
                            }
                        } else if let EventType::KeyRelease(k) = ev {
                            if *k == target {
                                state.main_key_down = false;
                            }
                        }
                    }
                };

                let hotkey_guard = current_hotkey.read();
                let Some(hotkey) = hotkey_guard.clone() else {
                    return;
                };
                drop(hotkey_guard);
                track_main(&mut state, &event.event_type, hotkey.key);

                if *paused.read() {
                    // Still track key state; just don't fire.
                    state.fired = chord_satisfied(&state, &hotkey);
                    return;
                }

                let satisfied = chord_satisfied(&state, &hotkey);
                if satisfied && !state.fired {
                    state.fired = true;
                    let _ = tx.send(HotkeyEvent::Down);
                } else if !satisfied && state.fired {
                    state.fired = false;
                    let _ = tx.send(HotkeyEvent::Up);
                }
            };

            if let Err(err) = listen(callback) {
                tracing::error!(?err, "rdev::listen failed (Input Monitoring permission?)");
            }
        })
        .expect("failed to spawn hotkey listener thread");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_right_option() {
        let parsed = parse("RightOption").unwrap();
        assert!(parsed.required.option_right);
        assert!(!parsed.required.option_left);
        assert!(parsed.key.is_none());
    }

    #[test]
    fn parse_cmd_shift_space() {
        let parsed = parse("Cmd+Shift+Space").unwrap();
        assert!(parsed.required.cmd);
        assert!(parsed.required.shift);
        assert_eq!(parsed.key, Some(Key::Space));
    }

    #[test]
    fn parse_fn_alone() {
        let parsed = parse("Fn").unwrap();
        assert!(parsed.required.fn_key);
        assert!(parsed.key.is_none());
    }

    #[test]
    fn parse_empty_is_none() {
        assert!(parse("").is_none());
        assert!(parse("   ").is_none());
    }
}
