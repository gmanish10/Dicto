pub mod listener;
#[cfg(target_os = "macos")]
pub mod mac_tap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HotkeyEvent {
    Down,
    Up,
}

/// Parsed representation of a user-configured hotkey.
///
/// We deliberately keep this simple — a single non-modifier key, plus a set of
/// required modifier flags. Modifier-only chords (e.g. RightOption alone, Fn alone)
/// are represented by setting `key` to None and putting the modifier in `required`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedHotkey {
    pub required: ModifierSet,
    pub key: Option<rdev::Key>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModifierSet {
    pub cmd: bool,
    pub shift: bool,
    pub option_left: bool,
    pub option_right: bool,
    pub control: bool,
    pub fn_key: bool,
}

impl ModifierSet {
    pub fn is_empty(&self) -> bool {
        !self.cmd
            && !self.shift
            && !self.option_left
            && !self.option_right
            && !self.control
            && !self.fn_key
    }
}
