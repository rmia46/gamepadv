use std::io;

use evdev::{
    uinput::VirtualDevice, AbsInfo, AbsoluteAxisCode, AttributeSet, BusType, InputId, KeyCode,
    UinputAbsSetup,
};

pub const XBOX360_NAME: &str = "Microsoft X-Box 360 pad";
pub const XBOX360_VENDOR: u16 = 0x045e;
pub const XBOX360_PRODUCT: u16 = 0x028e;
pub const XBOX360_VERSION: u16 = 0x0110;

/// Returns the `InputId` matching a standard wired Xbox 360 controller.
pub fn xbox360_input_id() -> InputId {
    InputId::new(
        BusType::BUS_USB,
        XBOX360_VENDOR,
        XBOX360_PRODUCT,
        XBOX360_VERSION,
    )
}

/// Returns the standard Xbox 360 button set.
///
/// Note: `evdev` 0.13 `KeyCode` uses the canonical Linux names
/// (`BTN_SOUTH`, `BTN_EAST`, `BTN_NORTH`, `BTN_WEST`) rather than the
/// `BTN_A`/`BTN_B`/`BTN_X`/`BTN_Y` C preprocessor aliases.
pub fn xbox360_buttons() -> AttributeSet<KeyCode> {
    let mut keys = AttributeSet::<KeyCode>::new();
    for code in [
        KeyCode::BTN_SOUTH,
        KeyCode::BTN_EAST,
        KeyCode::BTN_NORTH,
        KeyCode::BTN_WEST,
        KeyCode::BTN_TL,
        KeyCode::BTN_TR,
        KeyCode::BTN_SELECT,
        KeyCode::BTN_START,
        KeyCode::BTN_MODE,
        KeyCode::BTN_THUMBL,
        KeyCode::BTN_THUMBR,
    ] {
        keys.insert(code);
    }
    keys
}

/// Returns the `UinputAbsSetup` descriptors for all Xbox 360 axes.
pub fn xbox360_axes() -> [UinputAbsSetup; 8] {
    let stick_info = AbsInfo::new(0, -32768, 32767, 16, 128, 0);
    let trigger_info = AbsInfo::new(0, 0, 255, 0, 0, 0);
    let dpad_info = AbsInfo::new(0, -1, 1, 0, 0, 0);

    [
        UinputAbsSetup::new(AbsoluteAxisCode::ABS_X, stick_info),
        UinputAbsSetup::new(AbsoluteAxisCode::ABS_Y, stick_info),
        UinputAbsSetup::new(AbsoluteAxisCode::ABS_RX, stick_info),
        UinputAbsSetup::new(AbsoluteAxisCode::ABS_RY, stick_info),
        UinputAbsSetup::new(AbsoluteAxisCode::ABS_Z, trigger_info),
        UinputAbsSetup::new(AbsoluteAxisCode::ABS_RZ, trigger_info),
        UinputAbsSetup::new(AbsoluteAxisCode::ABS_HAT0X, dpad_info),
        UinputAbsSetup::new(AbsoluteAxisCode::ABS_HAT0Y, dpad_info),
    ]
}

/// Builds and registers a virtual Xbox 360 controller via `/dev/uinput`.
pub fn build_xbox360_controller() -> io::Result<VirtualDevice> {
    let buttons = xbox360_buttons();
    let mut builder = VirtualDevice::builder()?
        .name(XBOX360_NAME)
        .input_id(xbox360_input_id())
        .with_keys(&buttons)?;

    for axis in xbox360_axes() {
        builder = builder.with_absolute_axis(&axis)?;
    }

    builder.build()
}

/// Emits a neutral `SYN_REPORT` packet on the virtual device.
///
/// `VirtualDevice::emit(&[])` writes 0 input events followed by `SYN_REPORT`.
pub fn emit_neutral_sync(device: &mut VirtualDevice) -> io::Result<()> {
    device.emit(&[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xbox360_identity_matches_spec() {
        let id = xbox360_input_id();
        assert_eq!(id.bus_type(), BusType::BUS_USB);
        assert_eq!(id.vendor(), 0x045e);
        assert_eq!(id.product(), 0x028e);
        assert_eq!(id.version(), 0x0110);
    }

    #[test]
    fn xbox360_axes_and_buttons_configured() {
        let buttons = xbox360_buttons();
        assert!(buttons.contains(KeyCode::BTN_SOUTH));
        assert!(buttons.contains(KeyCode::BTN_EAST));
        assert!(buttons.contains(KeyCode::BTN_NORTH));
        assert!(buttons.contains(KeyCode::BTN_WEST));
        assert!(buttons.contains(KeyCode::BTN_TL));
        assert!(buttons.contains(KeyCode::BTN_TR));
        assert!(buttons.contains(KeyCode::BTN_SELECT));
        assert!(buttons.contains(KeyCode::BTN_START));
        assert!(buttons.contains(KeyCode::BTN_MODE));
        assert!(buttons.contains(KeyCode::BTN_THUMBL));
        assert!(buttons.contains(KeyCode::BTN_THUMBR));

        let axes = xbox360_axes();
        assert_eq!(axes.len(), 8);
        assert_eq!(axes[0].absinfo().minimum(), -32768);
        assert_eq!(axes[0].absinfo().maximum(), 32767);
        assert_eq!(axes[4].absinfo().minimum(), 0);
        assert_eq!(axes[4].absinfo().maximum(), 255);
        assert_eq!(axes[6].absinfo().minimum(), -1);
        assert_eq!(axes[6].absinfo().maximum(), 1);
    }

    #[test]
    fn uinput_xbox360_creation_and_sync() {
        let mut dev = build_xbox360_controller()
            .expect("expected /dev/uinput to be writable for current user");
        let syspath = dev.get_syspath().expect("syspath should be readable");
        assert!(syspath.starts_with("/sys/devices/virtual/input"));
        emit_neutral_sync(&mut dev).expect("emitting neutral SYN_REPORT should succeed");
    }
}
