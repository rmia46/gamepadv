use std::io;

use evdev::{uinput::VirtualDevice, AttributeSet, Device, KeyCode, KeyEvent};

use crate::input::discovery::{
    build_passthrough_keyboard, classify_device, standard_keyboard_keys, DeviceClass,
    PASSTHROUGH_KEYBOARD_NAME,
};
use crate::virtual_pad::xbox360::XBOX360_NAME;

/// Manages discovered physical keyboard/mouse devices, exclusive `EVIOCGRAB`
/// state, and a virtual passthrough keyboard for unmapped keys.
pub struct InputRig {
    keyboards: Vec<Device>,
    mice: Vec<Device>,
    passthrough_kbd: VirtualDevice,
    grabbed: bool,
}

impl InputRig {
    /// Creates an `InputRig` from pre-opened keyboard and mouse handles and a
    /// passthrough virtual keyboard, configuring all physical devices as non-blocking.
    pub fn new(
        keyboards: Vec<Device>,
        mice: Vec<Device>,
        passthrough_kbd: VirtualDevice,
    ) -> io::Result<Self> {
        for dev in keyboards.iter().chain(mice.iter()) {
            dev.set_nonblocking(true)?;
        }
        Ok(Self {
            keyboards,
            mice,
            passthrough_kbd,
            grabbed: false,
        })
    }

    /// Discovers all readable physical keyboards and mice via `evdev::enumerate()`,
    /// sets them to non-blocking mode, and creates a matching passthrough keyboard.
    pub fn discover() -> io::Result<Self> {
        let mut keyboards = Vec::new();
        let mut mice = Vec::new();
        let mut passthrough_keys = AttributeSet::<KeyCode>::new();

        for (_path, dev) in evdev::enumerate() {
            let name = dev.name().unwrap_or_default();
            if name == PASSTHROUGH_KEYBOARD_NAME || name == XBOX360_NAME {
                continue;
            }

            match classify_device(&dev) {
                DeviceClass::Keyboard => {
                    if let Some(keys) = dev.supported_keys() {
                        for key in keys.iter() {
                            passthrough_keys.insert(key);
                        }
                    }
                    keyboards.push(dev);
                }
                DeviceClass::Mouse => {
                    mice.push(dev);
                }
                DeviceClass::Other => {}
            }
        }

        if passthrough_keys.iter().next().is_none() {
            passthrough_keys = standard_keyboard_keys();
        }

        let passthrough_kbd =
            build_passthrough_keyboard(PASSTHROUGH_KEYBOARD_NAME, &passthrough_keys)?;

        Self::new(keyboards, mice, passthrough_kbd)
    }

    /// Returns whether managed physical devices are currently exclusively grabbed.
    pub fn is_grabbed(&self) -> bool {
        self.grabbed
    }

    /// Returns the number of managed keyboard devices.
    pub fn keyboard_count(&self) -> usize {
        self.keyboards.len()
    }

    /// Returns the number of managed mouse devices.
    pub fn mouse_count(&self) -> usize {
        self.mice.len()
    }

    /// Returns mutable access to the managed keyboard devices.
    pub fn keyboards_mut(&mut self) -> &mut [Device] {
        &mut self.keyboards
    }

    /// Returns mutable access to the managed mouse devices.
    pub fn mice_mut(&mut self) -> &mut [Device] {
        &mut self.mice
    }

    /// Toggles `EVIOCGRAB` across all managed keyboard and mouse devices.
    ///
    /// If acquiring grabs fails partway through, any already-grabbed devices
    /// are immediately ungrabbed before returning the error. When releasing,
    /// all devices are ungrabbed even if an individual `ungrab()` fails.
    pub fn set_grab(&mut self, grab: bool) -> io::Result<()> {
        if grab == self.grabbed {
            return Ok(());
        }

        if grab {
            let total = self.keyboards.len() + self.mice.len();
            let mut grabbed_count = 0usize;

            for dev in self.keyboards.iter_mut().chain(self.mice.iter_mut()) {
                if let Err(err) = dev.grab() {
                    for rollback in self
                        .keyboards
                        .iter_mut()
                        .chain(self.mice.iter_mut())
                        .take(grabbed_count)
                    {
                        let _ = rollback.ungrab();
                    }
                    self.grabbed = false;
                    return Err(err);
                }
                grabbed_count += 1;
            }
            debug_assert_eq!(grabbed_count, total);
            self.grabbed = true;
            Ok(())
        } else {
            let mut first_err = None;
            for dev in self.keyboards.iter_mut().chain(self.mice.iter_mut()) {
                if let Err(err) = dev.ungrab() {
                    if first_err.is_none() {
                        first_err = Some(err);
                    }
                }
            }
            self.grabbed = false;
            match first_err {
                Some(err) => Err(err),
                None => Ok(()),
            }
        }
    }

    /// Forwards an unmapped key event (`value`: 0 = release, 1 = press, 2 = repeat)
    /// to the passthrough virtual keyboard followed by `SYN_REPORT`.
    pub fn forward_passthrough_key(&mut self, code: KeyCode, value: i32) -> io::Result<()> {
        let event = *KeyEvent::new(code, value);
        self.passthrough_kbd.emit(&[event])
    }
}

impl Drop for InputRig {
    fn drop(&mut self) {
        let _ = self.set_grab(false);
    }
}
