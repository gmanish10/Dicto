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
    if unsafe { AXIsProcessTrusted() } {
        PermissionStatus::Granted
    } else {
        PermissionStatus::NotDetermined
    }
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
