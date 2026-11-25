use std::collections::HashSet;
use evdev::KeyCode;

use crate::engine::mouse_stick::{quantize_stick_axes, MouseStickEngine, STICK_MAX_F64};
use crate::profile::{GamepadAction, Profile};

/// Complete snapshot of the virtual Xbox 360 gamepad state for a single tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GamepadState {
    pub left_x: i16,
    pub left_y: i16,
    pub right_x: i16,
    pub right_y: i16,
    pub lt: u8,
    pub rt: u8,
    pub hat_x: i8,
    pub hat_y: i8,
    pub south: bool,
    pub east: bool,
    pub north: bool,
    pub west: bool,
    pub lb: bool,
    pub rb: bool,
    pub select: bool,
    pub start: bool,
    pub mode: bool,
    pub thumbl: bool,
    pub thumbr: bool,
}

/// Stateful keyboard/mouse-to-gamepad mapper combining a [`Profile`] and [`MouseStickEngine`].
#[derive(Debug, Clone)]
pub struct InputMapper {
    profile: Profile,
    pressed: HashSet<KeyCode>,
    mouse_stick: MouseStickEngine,
}

impl InputMapper {
    /// Creates a new [`InputMapper`] with the given [`Profile`] and [`MouseStickEngine`].
    #[must_use]
    pub fn new(profile: Profile, mouse_stick: MouseStickEngine) -> Self {
        Self {
            profile,
            pressed: HashSet::new(),
            mouse_stick,
        }
    }

    /// Returns a reference to the active [`Profile`].
    #[must_use]
    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    /// Returns a reference to the underlying [`MouseStickEngine`].
    #[must_use]
    pub fn mouse_stick(&self) -> &MouseStickEngine {
        &self.mouse_stick
    }

    /// Handles a key or mouse-button press/release event.
    ///
    /// Returns `true` if `code` is bound in the active profile, or `false` if unmapped
    /// so the caller can pass it through untouched.
    pub fn handle_key(&mut self, code: KeyCode, pressed: bool) -> bool {
        if !self.profile.bindings.contains_key(&code) {
            return false;
        }
        if pressed {
            self.pressed.insert(code);
        } else {
            self.pressed.remove(&code);
        }
        true
    }

    /// Accumulates raw `REL_X` / `REL_Y` mouse deltas for the right stick DSP.
    #[inline]
    pub fn feed_mouse_delta(&mut self, dx: f64, dy: f64) {
        self.mouse_stick.feed_delta(dx, dy);
    }

    /// Advances the mapper by `dt_secs` and produces the current [`GamepadState`].
    pub fn tick(&mut self, dt_secs: f64) -> GamepadState {
        let mut state = GamepadState::default();

        let mut stick_up = false;
        let mut stick_down = false;
        let mut stick_left = false;
        let mut stick_right = false;
        let mut dpad_up = false;
        let mut dpad_down = false;
        let mut dpad_left = false;
        let mut dpad_right = false;
        let mut walk_active = false;
        let mut lt_active = false;
        let mut rt_active = false;

        for code in &self.pressed {
            let Some(&action) = self.profile.bindings.get(code) else {
                continue;
            };
            match action {
                GamepadAction::South => state.south = true,
                GamepadAction::East => state.east = true,
                GamepadAction::North => state.north = true,
                GamepadAction::West => state.west = true,
                GamepadAction::LeftBumper => state.lb = true,
                GamepadAction::RightBumper => state.rb = true,
                GamepadAction::LeftTrigger => lt_active = true,
                GamepadAction::RightTrigger => rt_active = true,
                GamepadAction::Select => state.select = true,
                GamepadAction::Start => state.start = true,
                GamepadAction::Mode => state.mode = true,
                GamepadAction::LeftThumb => state.thumbl = true,
                GamepadAction::RightThumb => state.thumbr = true,
                GamepadAction::DpadUp => dpad_up = true,
                GamepadAction::DpadDown => dpad_down = true,
                GamepadAction::DpadLeft => dpad_left = true,
                GamepadAction::DpadRight => dpad_right = true,
                GamepadAction::LeftStickUp => stick_up = true,
                GamepadAction::LeftStickDown => stick_down = true,
                GamepadAction::LeftStickLeft => stick_left = true,
                GamepadAction::LeftStickRight => stick_right = true,
                GamepadAction::WalkModifier => walk_active = true,
            }
        }

        let raw_lx: f64 = match (stick_left, stick_right) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            _ => 0.0,
        };
        let raw_ly: f64 = match (stick_up, stick_down) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            _ => 0.0,
        };

        let mag = raw_lx.hypot(raw_ly);
        if mag > 0.0 {
            let scale = if walk_active {
                self.profile.walk_scale.clamp(0.0, 1.0)
            } else {
                1.0
            };
            let fx = (raw_lx / mag) * scale * STICK_MAX_F64;
            let fy = (raw_ly / mag) * scale * STICK_MAX_F64;
            let (lx, ly) = quantize_stick_axes(fx, fy);
            state.left_x = lx;
            state.left_y = ly;
        }

        state.hat_x = match (dpad_left, dpad_right) {
            (true, false) => -1,
            (false, true) => 1,
            _ => 0,
        };
        state.hat_y = match (dpad_up, dpad_down) {
            (true, false) => -1,
            (false, true) => 1,
            _ => 0,
        };

        state.lt = if lt_active { 255 } else { 0 };
        state.rt = if rt_active { 255 } else { 0 };

        let (rx, ry) = self.mouse_stick.tick(dt_secs);
        state.right_x = rx;
        state.right_y = ry;

        state
    }

    /// Clears all pressed keys and resets the right-stick DSP engine.
    pub fn reset(&mut self) {
        self.pressed.clear();
        self.mouse_stick.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::mouse_stick::MouseStickConfig;
    use crate::profile::zelda_default;

    fn make_mapper() -> InputMapper {
        InputMapper::new(zelda_default(), MouseStickEngine::new(MouseStickConfig::default()))
    }

    #[test]
    fn unmapped_key_returns_false_and_does_not_affect_state() {
        let mut mapper = make_mapper();
        assert!(!mapper.handle_key(KeyCode::KEY_F12, true));
        assert!(!mapper.handle_key(KeyCode::KEY_F12, false));
        assert_eq!(mapper.tick(0.004), GamepadState::default());
    }

    #[test]
    fn opposing_left_stick_and_dpad_keys_cancel_to_zero() {
        let mut mapper = make_mapper();
        assert!(mapper.handle_key(KeyCode::KEY_W, true));
        assert!(mapper.handle_key(KeyCode::KEY_S, true));
        assert!(mapper.handle_key(KeyCode::KEY_A, true));
        assert!(mapper.handle_key(KeyCode::KEY_D, true));
        assert!(mapper.handle_key(KeyCode::KEY_1, true));
        assert!(mapper.handle_key(KeyCode::KEY_2, true));
        assert!(mapper.handle_key(KeyCode::KEY_3, true));
        assert!(mapper.handle_key(KeyCode::KEY_4, true));

        let state = mapper.tick(0.004);
        assert_eq!(state.left_x, 0);
        assert_eq!(state.left_y, 0);
        assert_eq!(state.hat_x, 0);
        assert_eq!(state.hat_y, 0);
    }

    #[test]
    fn diagonal_left_stick_is_circularly_normalized() {
        let mut mapper = make_mapper();
        assert!(mapper.handle_key(KeyCode::KEY_W, true));
        assert!(mapper.handle_key(KeyCode::KEY_D, true));

        let state = mapper.tick(0.004);
        let hypot = (state.left_x as f64).hypot(state.left_y as f64);
        assert!(
            hypot <= STICK_MAX_F64,
            "diagonal left stick hypot ({hypot}) exceeded {STICK_MAX_F64}"
        );
        assert_eq!(state.left_x, -state.left_y);
        assert_eq!(state.left_x, 23169);
        assert_eq!(state.left_y, -23169);
    }

    #[test]
    fn walk_modifier_scales_left_stick_by_profile_walk_scale() {
        let mut mapper = make_mapper();
        assert!(mapper.handle_key(KeyCode::KEY_D, true));
        assert!(mapper.handle_key(KeyCode::KEY_LEFTALT, true));

        let cardinal = mapper.tick(0.004);
        assert_eq!(cardinal.left_x, 16384); // (32767.0 * 0.5).round()
        assert_eq!(cardinal.left_y, 0);

        // Release ALT, press CTRL + W for diagonal walk
        assert!(mapper.handle_key(KeyCode::KEY_LEFTALT, false));
        assert!(mapper.handle_key(KeyCode::KEY_LEFTCTRL, true));
        assert!(mapper.handle_key(KeyCode::KEY_W, true));

        let diagonal = mapper.tick(0.004);
        let expected = (STICK_MAX_F64 * 0.5 * std::f64::consts::FRAC_1_SQRT_2).round() as i16;
        assert_eq!(diagonal.left_x, expected);
        assert_eq!(diagonal.left_y, -expected);
    }

    #[test]
    fn buttons_triggers_and_dpad_map_correctly() {
        let mut mapper = make_mapper();
        assert!(mapper.handle_key(KeyCode::BTN_LEFT, true));
        assert!(mapper.handle_key(KeyCode::BTN_RIGHT, true));
        assert!(mapper.handle_key(KeyCode::KEY_Q, true));
        assert!(mapper.handle_key(KeyCode::KEY_E, true));
        assert!(mapper.handle_key(KeyCode::KEY_SPACE, true));
        assert!(mapper.handle_key(KeyCode::KEY_LEFTSHIFT, true));
        assert!(mapper.handle_key(KeyCode::KEY_F, true));
        assert!(mapper.handle_key(KeyCode::KEY_R, true));
        assert!(mapper.handle_key(KeyCode::KEY_TAB, true));
        assert!(mapper.handle_key(KeyCode::KEY_ENTER, true));
        assert!(mapper.handle_key(KeyCode::KEY_C, true));
        assert!(mapper.handle_key(KeyCode::KEY_V, true));
        assert!(mapper.handle_key(KeyCode::KEY_1, true));
        assert!(mapper.handle_key(KeyCode::KEY_4, true));

        let state = mapper.tick(0.004);
        assert!(state.west);
        assert_eq!(state.lt, 255);
        assert_eq!(state.rt, 255);
        assert!(state.south);
        assert!(state.north);
        assert!(state.east);
        assert!(state.lb);
        assert!(state.rb);
        assert!(state.select);
        assert!(state.start);
        assert!(state.thumbl);
        assert!(state.thumbr);
        assert_eq!(state.hat_x, 1);
        assert_eq!(state.hat_y, -1);
    }

    #[test]
    fn reset_clears_pressed_keys_and_mouse_velocity() {
        let mut mapper = make_mapper();
        assert!(mapper.handle_key(KeyCode::KEY_W, true));
        assert!(mapper.handle_key(KeyCode::KEY_E, true));
        assert!(mapper.handle_key(KeyCode::KEY_Q, true));
        mapper.feed_mouse_delta(60.0, -40.0);

        let active = mapper.tick(0.004);
        assert_ne!(active.left_y, 0);
        assert!(active.south);
        assert_eq!(active.rt, 255);
        assert_ne!((active.right_x, active.right_y), (0, 0));

        mapper.feed_mouse_delta(100.0, 100.0);
        mapper.reset();

        assert_eq!(mapper.tick(0.004), GamepadState::default());
    }
}
