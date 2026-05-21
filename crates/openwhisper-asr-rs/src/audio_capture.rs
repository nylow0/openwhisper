use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CaptureDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[cfg(target_os = "windows")]
pub fn list_input_devices() -> Vec<CaptureDeviceInfo> {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();

    let Ok(devices) = host.input_devices() else {
        return vec![];
    };

    devices
        .enumerate()
        .map(|(idx, device)| {
            let name = device
                .name()
                .unwrap_or_else(|_| format!("Unknown input device #{idx}"));
            let is_default = !default_name.is_empty() && name == default_name;
            CaptureDeviceInfo {
                id: format!("input-{idx}"),
                name,
                is_default,
            }
        })
        .collect()
}

#[cfg(not(target_os = "windows"))]
pub fn list_input_devices() -> Vec<CaptureDeviceInfo> {
    vec![CaptureDeviceInfo {
        id: "default".to_string(),
        name: "Default Microphone (stub non-windows)".to_string(),
        is_default: true,
    }]
}

#[cfg(test)]
mod tests {
    use super::list_input_devices;

    #[test]
    fn list_devices_does_not_panic() {
        let _ = list_input_devices();
    }
}
