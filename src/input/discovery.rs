use std::fs;
use std::io;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

use evdev::{
    uinput::VirtualDevice, AttributeSet, AttributeSetRef, BusType, Device, InputId, KeyCode,
    RelativeAxisCode,
};

pub const PASSTHROUGH_KEYBOARD_NAME: &str = "GamepadV Passthrough Keyboard";

/// High-level classification of an evdev input device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    Keyboard,
    Mouse,
    Other,
}

/// Classifies a device from its supported `EV_KEY` and `EV_REL` capabilities.
pub fn classify_capabilities(
    keys: Option<&AttributeSetRef<KeyCode>>,
    rel_axes: Option<&AttributeSetRef<RelativeAxisCode>>,
) -> DeviceClass {
    let is_mouse = matches!((keys, rel_axes), (Some(k), Some(r))
        if k.contains(KeyCode::BTN_LEFT)
            && r.contains(RelativeAxisCode::REL_X)
            && r.contains(RelativeAxisCode::REL_Y));

    if is_mouse {
        return DeviceClass::Mouse;
    }

    let is_keyboard = matches!(keys, Some(k)
        if k.contains(KeyCode::KEY_ESC)
            && k.contains(KeyCode::KEY_A)
            && k.contains(KeyCode::KEY_ENTER)
            && k.contains(KeyCode::KEY_SPACE));

    if is_keyboard {
        return DeviceClass::Keyboard;
    }

    DeviceClass::Other
}

/// Classifies an opened `evdev::Device`.
pub fn classify_device(device: &Device) -> DeviceClass {
    classify_capabilities(device.supported_keys(), device.supported_relative_axes())
}

/// RAII guard for `EVIOCGRAB` on an `evdev::Device`.
///
/// Guarantees `ungrab()` is invoked on drop (including early returns and
/// unwinding panics) so physical input devices are never left grabbed.
#[derive(Debug)]
pub struct GrabGuard {
    device: Option<Device>,
}

impl GrabGuard {
    /// Acquires an exclusive `EVIOCGRAB` on `device`.
    pub fn acquire(mut device: Device) -> io::Result<Self> {
        device.grab()?;
        Ok(Self {
            device: Some(device),
        })
    }

    /// Explicitly releases the grab and returns the underlying `Device`.
    pub fn release(mut self) -> io::Result<Device> {
        let mut device = self
            .device
            .take()
            .expect("GrabGuard device missing before release");
        device.ungrab()?;
        Ok(device)
    }
}

impl Deref for GrabGuard {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        self.device
            .as_ref()
            .expect("GrabGuard accessed after release")
    }
}

impl DerefMut for GrabGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.device
            .as_mut()
            .expect("GrabGuard accessed after release")
    }
}

impl Drop for GrabGuard {
    fn drop(&mut self) {
        if let Some(mut device) = self.device.take() {
            let _ = device.ungrab();
        }
    }
}

/// Builds a passthrough virtual keyboard via `/dev/uinput`.
///
/// Mirrors the provided `keys` capability set so unmapped or non-gameplay
/// keys can be forwarded cleanly when a physical keyboard is grabbed.
pub fn build_passthrough_keyboard(
    name: &str,
    keys: &AttributeSetRef<KeyCode>,
) -> io::Result<VirtualDevice> {
    let id = InputId::new(BusType::BUS_VIRTUAL, 0x1d6b, 0x0001, 0x0001);
    VirtualDevice::builder()?
        .name(name)
        .input_id(id)
        .with_keys(keys)?
        .build()
}

/// Returns a minimal standard keyboard keyset for testing or fallback passthrough uinput creation.
pub fn standard_keyboard_keys() -> AttributeSet<KeyCode> {
    let mut keys = AttributeSet::<KeyCode>::new();
    for code in [
        KeyCode::KEY_ESC,
        KeyCode::KEY_W,
        KeyCode::KEY_A,
        KeyCode::KEY_S,
        KeyCode::KEY_D,
        KeyCode::KEY_SPACE,
        KeyCode::KEY_ENTER,
        KeyCode::KEY_LEFTSHIFT,
        KeyCode::KEY_LEFTCTRL,
    ] {
        keys.insert(code);
    }
    keys
}

/// Summary of `/dev/input/event*` probing and `evdev::enumerate()`.
#[derive(Debug)]
pub struct DiscoveryReport {
    pub event_nodes_total: usize,
    pub event_nodes_readable: usize,
    pub event_nodes_permission_denied: usize,
    pub sample_error: Option<(PathBuf, io::ErrorKind, String)>,
    pub enumerated_devices: Vec<(PathBuf, String, DeviceClass)>,
}

/// Probes `/dev/input/event*` permissions and runs `evdev::enumerate()`.
pub fn probe_input_devices() -> io::Result<DiscoveryReport> {
    let mut event_paths: Vec<PathBuf> = fs::read_dir("/dev/input")?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("event"))
        })
        .collect();
    event_paths.sort();

    let mut readable = 0usize;
    let mut denied = 0usize;
    let mut sample_error = None;

    for path in &event_paths {
        match Device::open(path) {
            Ok(_) => readable += 1,
            Err(err) => {
                if err.kind() == io::ErrorKind::PermissionDenied {
                    denied += 1;
                }
                if sample_error.is_none() {
                    sample_error = Some((path.clone(), err.kind(), err.to_string()));
                }
            }
        }
    }

    let enumerated_devices = evdev::enumerate()
        .map(|(path, dev)| {
            let name = dev.name().unwrap_or("<unnamed>").to_string();
            let class = classify_device(&dev);
            (path, name, class)
        })
        .collect();

    Ok(DiscoveryReport {
        event_nodes_total: event_paths.len(),
        event_nodes_readable: readable,
        event_nodes_permission_denied: denied,
        sample_error,
        enumerated_devices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_keyboard_and_mouse_and_other() {
        let mut kbd_keys = AttributeSet::<KeyCode>::new();
        kbd_keys.insert(KeyCode::KEY_ESC);
        kbd_keys.insert(KeyCode::KEY_A);
        kbd_keys.insert(KeyCode::KEY_ENTER);
        kbd_keys.insert(KeyCode::KEY_SPACE);
        assert_eq!(
            classify_capabilities(Some(&kbd_keys), None),
            DeviceClass::Keyboard
        );

        let mut mouse_keys = AttributeSet::<KeyCode>::new();
        mouse_keys.insert(KeyCode::BTN_LEFT);
        let mut mouse_rel = AttributeSet::<RelativeAxisCode>::new();
        mouse_rel.insert(RelativeAxisCode::REL_X);
        mouse_rel.insert(RelativeAxisCode::REL_Y);
        assert_eq!(
            classify_capabilities(Some(&mouse_keys), Some(&mouse_rel)),
            DeviceClass::Mouse
        );

        let mut power_keys = AttributeSet::<KeyCode>::new();
        power_keys.insert(KeyCode::KEY_POWER);
        assert_eq!(
            classify_capabilities(Some(&power_keys), None),
            DeviceClass::Other
        );
    }

    #[test]
    fn passthrough_keyboard_uinput_creation() {
        let keys = standard_keyboard_keys();
        let mut dev = match build_passthrough_keyboard(PASSTHROUGH_KEYBOARD_NAME, &keys) {
            Ok(dev) => dev,
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied => return,
            Err(err) => panic!("unexpected error creating passthrough keyboard: {err}"),
        };
        let syspath = dev.get_syspath().expect("syspath should be readable");
        assert!(syspath.starts_with("/sys/devices/virtual/input"));
    }

    #[test]
    fn probe_input_devices_handles_permissions() {
        match probe_input_devices() {
            Ok(report) => {
                assert!(
                    report.event_nodes_readable + report.event_nodes_permission_denied
                        <= report.event_nodes_total
                );
                assert_eq!(report.enumerated_devices.len(), report.event_nodes_readable);
            }
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => panic!("unexpected error probing input devices: {err}"),
        }
    }
}
