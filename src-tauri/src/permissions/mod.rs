use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionStatus {
    Granted,
    Denied,
    NotDetermined,
}

#[derive(Debug, Clone, Serialize)]
pub struct PermissionsSnapshot {
    pub microphone: PermissionStatus,
    pub accessibility: PermissionStatus,
    pub input_monitoring: PermissionStatus,
}

pub fn snapshot() -> PermissionsSnapshot {
    PermissionsSnapshot {
        microphone: microphone_status(),
        accessibility: accessibility_status(),
        input_monitoring: input_monitoring_status(),
    }
}

/// Trigger the system microphone-access prompt by briefly opening a cpal input
/// stream. The NSMicrophoneUsageDescription Info.plist key makes macOS show the
/// dialog the first time. Returns the resulting status.
pub async fn request_microphone() -> PermissionStatus {
    #[cfg(target_os = "macos")]
    {
        let _ = tokio::task::spawn_blocking(|| {
            use cpal::traits::{DeviceTrait, HostTrait};
            let host = cpal::default_host();
            if let Some(device) = host.default_input_device() {
                let _ = device.default_input_config();
            }
        })
        .await;
    }
    microphone_status()
}

/// Deep-link to the right Privacy pane in System Settings.
pub fn open_settings_pane(pane: &str) -> anyhow::Result<()> {
    let url = match pane {
        "microphone" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
        }
        "accessibility" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
        }
        "input_monitoring" => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
        }
        other => return Err(anyhow::anyhow!("unknown pane: {other}")),
    };
    // The user has now been shown the relevant pane — once they leave
    // it without granting, we want subsequent `accessibility_status`
    // checks to report `Denied` instead of `NotDetermined`. (See the
    // comment on `accessibility_status` for why this can't be derived
    // from the OS alone.)
    if pane == "accessibility" {
        mark_accessibility_requested();
    }
    std::process::Command::new("open").arg(url).spawn()?;
    Ok(())
}

// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
#[link(name = "AVFoundation", kind = "framework")]
extern "C" {
    static AVMediaTypeAudio: *const objc2::runtime::AnyObject;
}

#[cfg(target_os = "macos")]
fn microphone_status() -> PermissionStatus {
    // The cpal proxy we used through v0.2.0 returned "granted" whenever
    // a default input device existed, which on macOS is true even when
    // the app had never been granted mic TCC. Onboarding therefore
    // showed mic as granted before the user actually granted it, and
    // conversely sometimes appeared stuck after a fresh install when
    // cpal's device enumeration was racy. The reliable signal is the
    // TCC database itself, queried via AVCaptureDevice
    // authorizationStatusForMediaType:AVMediaTypeAudio. AVAuthorizationStatus:
    // 0 = NotDetermined, 1 = Restricted, 2 = Denied, 3 = Authorized.
    use objc2::msg_send;
    use objc2::runtime::AnyClass;

    unsafe {
        let Some(cls) = AnyClass::get("AVCaptureDevice") else {
            return PermissionStatus::NotDetermined;
        };
        let status: i64 = msg_send![cls, authorizationStatusForMediaType: AVMediaTypeAudio];
        match status {
            3 => PermissionStatus::Granted,
            1 | 2 => PermissionStatus::Denied,
            _ => PermissionStatus::NotDetermined,
        }
    }
}

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

#[cfg(target_os = "macos")]
fn accessibility_status() -> PermissionStatus {
    // macOS doesn't expose a public API to distinguish "user explicitly
    // denied" from "user has never been asked" for Accessibility —
    // `AXIsProcessTrusted()` returns false in both cases. The matching
    // call `AXIsProcessTrustedWithOptions` only differs by being able
    // to *trigger* the system prompt, not by reporting denial state.
    //
    // After the user has interacted with the Accessibility row in our
    // onboarding (which deep-links them to System Settings) we treat a
    // still-false result as `Denied` so the UI surfaces a red pill +
    // a clearer call-to-action; before they've interacted we report
    // `NotDetermined` so the first-launch yellow "not granted" pill
    // doesn't accuse them of refusing something they never saw.
    //
    // We also wait out `ACCESSIBILITY_DENIED_GRACE` after the deep-link
    // before reporting `Denied`: clicking "Allow" opens System Settings,
    // and the user needs a few seconds to actually flip the toggle.
    // Going red the instant they click would flash an alarming pill
    // during the entirely-normal grant flow.
    if unsafe { AXIsProcessTrusted() } {
        PermissionStatus::Granted
    } else if accessibility_grant_overdue() {
        PermissionStatus::Denied
    } else {
        PermissionStatus::NotDetermined
    }
}

/// Timestamp of the first time the frontend called
/// `open_system_settings("accessibility")` (the "Allow" button on the
/// onboarding / Settings permission row). `None` until then. Lets
/// `accessibility_status` distinguish "user has never seen the prompt"
/// from "user was shown the pane and still hasn't enabled access".
static ACCESSIBILITY_GRANT_REQUESTED_AT: std::sync::OnceLock<std::time::Instant> =
    std::sync::OnceLock::new();

/// Grace period after the user is deep-linked to the Accessibility pane
/// during which we keep reporting `NotDetermined` instead of `Denied`,
/// so the normal "click Allow → toggle in System Settings" flow doesn't
/// flash a red pill before they've had a chance to act.
#[cfg(target_os = "macos")]
const ACCESSIBILITY_DENIED_GRACE: std::time::Duration = std::time::Duration::from_secs(3);

pub fn mark_accessibility_requested() {
    // First call wins; later calls are no-ops (the grace period is
    // measured from the first time the user was shown the pane).
    let _ = ACCESSIBILITY_GRANT_REQUESTED_AT.set(std::time::Instant::now());
}

/// True once the user has been shown the Accessibility pane and the
/// grace period has elapsed without access being granted.
#[cfg(target_os = "macos")]
fn accessibility_grant_overdue() -> bool {
    ACCESSIBILITY_GRANT_REQUESTED_AT
        .get()
        .is_some_and(|t| t.elapsed() >= ACCESSIBILITY_DENIED_GRACE)
}

#[cfg(target_os = "macos")]
#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOHIDCheckAccess(request: u32) -> u32;
}

#[cfg(target_os = "macos")]
fn input_monitoring_status() -> PermissionStatus {
    const K_IO_HID_REQUEST_TYPE_LISTEN_EVENT: u32 = 1;
    match unsafe { IOHIDCheckAccess(K_IO_HID_REQUEST_TYPE_LISTEN_EVENT) } {
        0 => PermissionStatus::Granted,
        1 => PermissionStatus::Denied,
        _ => PermissionStatus::NotDetermined,
    }
}

// --- Non-macOS stubs ------------------------------------------------------

#[cfg(not(target_os = "macos"))]
fn microphone_status() -> PermissionStatus {
    PermissionStatus::Granted
}
#[cfg(not(target_os = "macos"))]
fn accessibility_status() -> PermissionStatus {
    PermissionStatus::Granted
}
#[cfg(not(target_os = "macos"))]
fn input_monitoring_status() -> PermissionStatus {
    PermissionStatus::Granted
}
