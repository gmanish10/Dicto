//! Audible feedback for the start and end of a recording.
//!
//! macOS ships a small library of system sounds at `/System/Library/Sounds`.
//! We use two of them — Tink (start) and Pop (stop) — because they're
//! short, distinct, recognizable, and require no asset bundling. They
//! also exist on every macOS install from 10.5 onward.
//!
//! Playback is fire-and-forget: we spawn `afplay` as a detached child
//! process. The CLI is part of base macOS, plays the sound on its own
//! audio thread, and exits when done. We deliberately do not wait on
//! the child — the chime is feedback, not a sync barrier, and any
//! latency between Down/Up and the pipeline state transition would feel
//! laggy. If the spawn fails (binary missing, sandbox restriction), we
//! log and move on — the dictation pipeline itself is unaffected.
//!
//! Wired into [`crate::pipeline`] at the Down/Up edges so each chime
//! reflects a real user-driven transition, not an internal state flip.

use std::process::{Command, Stdio};

/// Short, high "tink" — plays when recording starts.
const START_SOUND: &str = "/System/Library/Sounds/Tink.aiff";

/// Soft "pop" — plays when the user releases the hotkey and audio is
/// being handed off to transcription.
const STOP_SOUND: &str = "/System/Library/Sounds/Pop.aiff";

/// Play the start chime if the user has it enabled. Non-blocking.
pub fn play_start(enabled: bool) {
    if enabled {
        play(START_SOUND);
    }
}

/// Play the stop chime if the user has it enabled. Non-blocking.
pub fn play_stop(enabled: bool) {
    if enabled {
        play(STOP_SOUND);
    }
}

fn play(path: &str) {
    // Detach stdin/stdout/stderr so the child holds no inherited
    // descriptors. We never call .wait(); the child exits on its own
    // after the ~100 ms sound finishes.
    let spawn_result = Command::new("/usr/bin/afplay")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    if let Err(e) = spawn_result {
        tracing::warn!(error = %e, path, "chime: afplay spawn failed");
    }
}
