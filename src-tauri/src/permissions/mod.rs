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
fn microphone_status() -> PermissionStatus {
    // Best-effort: try to open a stream. If the system mic is silent, we still
    // assume "granted" because cpal returned a usable device. If anything in
    // the chain fails with an OS-level access error, it'll be visible in logs;
    // the user can verify via System Settings.
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    match host.default_input_device() {
        Some(device) => match device.default_input_config() {
            Ok(_) => PermissionStatus::Granted,
            Err(_) => PermissionStatus::NotDetermined,
        },
        None => PermissionStatus::NotDetermined,
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
