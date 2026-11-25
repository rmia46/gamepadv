use std::io;

use evdev::{
    uinput::VirtualDevice, AbsInfo, AbsoluteAxisCode, AbsoluteAxisEvent, AttributeSet, BusType,
    InputEvent, InputId, KeyCode, KeyEvent, UinputAbsSetup,
};

use crate::engine::mapper::GamepadState;

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
/// `evdev` 0.13 `KeyCode` uses the canonical Linux names
/// (`BTN_SOUTH`, `BTN_EAST`, `BTN_NORTH`, `BTN_WEST`).
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

/// Computes the minimal set of `InputEvent`s needed to transition from `prev` to `next`.
pub fn diff_state_events(prev: &GamepadState, next: &GamepadState) -> Vec<InputEvent> {
    let mut events = Vec::new();

    let mut push_abs = |code: AbsoluteAxisCode, old: i32, new: i32| {
        if old != new {
            events.push(*AbsoluteAxisEvent::new(code, new));
        }
    };

    push_abs(AbsoluteAxisCode::ABS_X, prev.left_x as i32, next.left_x as i32);
    push_abs(AbsoluteAxisCode::ABS_Y, prev.left_y as i32, next.left_y as i32);
    push_abs(AbsoluteAxisCode::ABS_RX, prev.right_x as i32, next.right_x as i32);
    push_abs(AbsoluteAxisCode::ABS_RY, prev.right_y as i32, next.right_y as i32);
    push_abs(AbsoluteAxisCode::ABS_Z, prev.lt as i32, next.lt as i32);
    push_abs(AbsoluteAxisCode::ABS_RZ, prev.rt as i32, next.rt as i32);
    push_abs(AbsoluteAxisCode::ABS_HAT0X, prev.hat_x as i32, next.hat_x as i32);
    push_abs(AbsoluteAxisCode::ABS_HAT0Y, prev.hat_y as i32, next.hat_y as i32);

    let mut push_btn = |code: KeyCode, old: bool, new: bool| {
        if old != new {
            events.push(*KeyEvent::new(code, i32::from(new)));
        }
    };

    push_btn(KeyCode::BTN_SOUTH, prev.south, next.south);
    push_btn(KeyCode::BTN_EAST, prev.east, next.east);
    push_btn(KeyCode::BTN_NORTH, prev.north, next.north);
    push_btn(KeyCode::BTN_WEST, prev.west, next.west);
    push_btn(KeyCode::BTN_TL, prev.lb, next.lb);
    push_btn(KeyCode::BTN_TR, prev.rb, next.rb);
    push_btn(KeyCode::BTN_SELECT, prev.select, next.select);
    push_btn(KeyCode::BTN_START, prev.start, next.start);
    push_btn(KeyCode::BTN_MODE, prev.mode, next.mode);
    push_btn(KeyCode::BTN_THUMBL, prev.thumbl, next.thumbl);
    push_btn(KeyCode::BTN_THUMBR, prev.thumbr, next.thumbr);

    events
}

/// Returns `InputEvent`s that zero every axis and release every button.
fn all_neutral_events() -> [InputEvent; 19] {
    [
        *AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_X, 0),
        *AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_Y, 0),
        *AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_RX, 0),
        *AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_RY, 0),
        *AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_Z, 0),
        *AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_RZ, 0),
        *AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_HAT0X, 0),
        *AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_HAT0Y, 0),
        *KeyEvent::new(KeyCode::BTN_SOUTH, 0),
        *KeyEvent::new(KeyCode::BTN_EAST, 0),
        *KeyEvent::new(KeyCode::BTN_NORTH, 0),
        *KeyEvent::new(KeyCode::BTN_WEST, 0),
        *KeyEvent::new(KeyCode::BTN_TL, 0),
        *KeyEvent::new(KeyCode::BTN_TR, 0),
        *KeyEvent::new(KeyCode::BTN_SELECT, 0),
        *KeyEvent::new(KeyCode::BTN_START, 0),
        *KeyEvent::new(KeyCode::BTN_MODE, 0),
        *KeyEvent::new(KeyCode::BTN_THUMBL, 0),
        *KeyEvent::new(KeyCode::BTN_THUMBR, 0),
    ]
}

/// Virtual Xbox 360 controller backed by `/dev/uinput`.
pub struct VirtualXbox360 {
    device: VirtualDevice,
    prev_state: GamepadState,
}

impl VirtualXbox360 {
    /// Creates and registers a new virtual Xbox 360 controller.
    pub fn new() -> io::Result<Self> {
        let device = build_xbox360_controller()?;
        Ok(Self {
            device,
            prev_state: GamepadState::default(),
        })
    }

    /// Returns a shared reference to the underlying `VirtualDevice`.
    pub fn device(&self) -> &VirtualDevice {
        &self.device
    }

    /// Returns a mutable reference to the underlying `VirtualDevice`.
    pub fn device_mut(&mut self) -> &mut VirtualDevice {
        &mut self.device
    }

    /// Diffs `state` against the previously emitted state and emits only changed
    /// events followed by `SYN_REPORT` (via `VirtualDevice::emit`).
    pub fn sync_state(&mut self, state: &GamepadState) -> io::Result<()> {
        let events = diff_state_events(&self.prev_state, state);
        if events.is_empty() {
            return Ok(());
        }
        self.device.emit(&events)?;
        self.prev_state = *state;
        Ok(())
    }

    /// Resets all axes and buttons to 0 and emits `SYN_REPORT`.
    pub fn reset_neutral(&mut self) -> io::Result<()> {
        let events = all_neutral_events();
        self.device.emit(&events)?;
        self.prev_state = GamepadState::default();
        Ok(())
    }
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
        for btn in [
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
            assert!(buttons.contains(btn));
        }

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
    fn diff_state_emits_only_changed_events() {
        let prev = GamepadState::default();
        assert!(diff_state_events(&prev, &prev).is_empty());

        let mut next = prev;
        next.left_x = 16384;
        next.rt = 255;
        next.south = true;

        let events = diff_state_events(&prev, &next);
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn virtual_xbox360_uinput_sync_and_reset() {
        let mut pad = match VirtualXbox360::new() {
            Ok(pad) => pad,
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied => return,
            Err(err) => panic!("unexpected error creating VirtualXbox360: {err}"),
        };

        let syspath = pad
            .device_mut()
            .get_syspath()
            .expect("syspath should be readable");
        assert!(syspath.starts_with("/sys/devices/virtual/input"));

        let state = GamepadState {
            left_x: -12000,
            south: true,
            ..Default::default()
        };
        pad.sync_state(&state).expect("sync_state should succeed");
        pad.sync_state(&state).expect("no-op sync_state should succeed");
        pad.reset_neutral().expect("reset_neutral should succeed");
    }
}
