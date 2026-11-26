use std::collections::HashSet;
use evdev::KeyCode;

use crate::engine::key_stick::KeyStickSmoother;
use crate::engine::mouse_stick::{MouseStickConfig, MouseStickEngine};
use crate::profile::{GamepadAction, Profile, StickMode};

/// Complete snapshot of the virtual Xbox 360 gamepad state for a single tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
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
    left_key_smoother: KeyStickSmoother,
    right_key_smoother: KeyStickSmoother,
}

impl InputMapper {
    /// Creates a new [`InputMapper`] with the given [`Profile`] and [`MouseStickEngine`].
    #[must_use]
    pub fn new(profile: Profile, mouse_stick: MouseStickEngine) -> Self {
        Self {
            profile,
            pressed: HashSet::new(),
            mouse_stick,
            left_key_smoother: KeyStickSmoother::new(),
            right_key_smoother: KeyStickSmoother::new(),
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

    /// Replaces the active [`Profile`], pruning any pressed keys no longer bound.
    pub fn set_profile(&mut self, profile: Profile) {
        self.profile = profile;
        self.pressed
            .retain(|code| self.profile.bindings.contains_key(code));
    }

    /// Updates the [`MouseStickEngine`] configuration.
    pub fn set_mouse_config(&mut self, config: MouseStickConfig) {
        self.mouse_stick.set_config(config);
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

        let mut left_up = false;
        let mut left_down = false;
        let mut left_left = false;
        let mut left_right = false;
        let mut right_up = false;
        let mut right_down = false;
        let mut right_left = false;
        let mut right_right = false;
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
                GamepadAction::LeftStickUp => left_up = true,
                GamepadAction::LeftStickDown => left_down = true,
                GamepadAction::LeftStickLeft => left_left = true,
                GamepadAction::LeftStickRight => left_right = true,
                GamepadAction::RightStickUp => right_up = true,
                GamepadAction::RightStickDown => right_down = true,
                GamepadAction::RightStickLeft => right_left = true,
                GamepadAction::RightStickRight => right_right = true,
                GamepadAction::WalkModifier => walk_active = true,
            }
        }

        let walk_scale = self.profile.walk_scale;
        let key_stick_cfg = self.profile.key_stick;
        let (tgt_lx, tgt_ly) = compute_key_stick_target(
            left_up,
            left_down,
            left_left,
            left_right,
            walk_active,
            walk_scale,
        );
        let (tgt_rx, tgt_ry) = compute_key_stick_target(
            right_up,
            right_down,
            right_left,
            right_right,
            walk_active,
            walk_scale,
        );

        let key_left = if self.profile.left_stick_mode == StickMode::Keys {
            self.left_key_smoother
                .update(tgt_lx, tgt_ly, dt_secs, &key_stick_cfg)
        } else {
            self.left_key_smoother.reset();
            (0, 0)
        };
        let key_right = if self.profile.right_stick_mode == StickMode::Keys {
            self.right_key_smoother
                .update(tgt_rx, tgt_ry, dt_secs, &key_stick_cfg)
        } else {
            self.right_key_smoother.reset();
            (0, 0)
        };

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

        let mouse_axes = if self.profile.left_stick_mode == StickMode::Mouse
            || self.profile.right_stick_mode == StickMode::Mouse
        {
            self.mouse_stick.tick(dt_secs)
        } else {
            self.mouse_stick.reset();
            (0, 0)
        };

        let (lx, ly) = match self.profile.left_stick_mode {
            StickMode::Mouse => mouse_axes,
            StickMode::Keys => key_left,
            StickMode::Disabled => (0, 0),
        };
        state.left_x = lx;
        state.left_y = ly;

        let (rx, ry) = match self.profile.right_stick_mode {
            StickMode::Mouse => mouse_axes,
            StickMode::Keys => key_right,
            StickMode::Disabled => (0, 0),
        };
        state.right_x = rx;
        state.right_y = ry;

        state
    }

    /// Clears all pressed keys and resets both key-stick smoothers and the mouse DSP engine.
    pub fn reset(&mut self) {
        self.pressed.clear();
        self.mouse_stick.reset();
        self.left_key_smoother.reset();
        self.right_key_smoother.reset();
    }
}

#[inline]
fn compute_key_stick_target(
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    walk_active: bool,
    walk_scale: f64,
) -> (f64, f64) {
    let raw_x: f64 = match (left, right) {
        (true, false) => -1.0,
        (false, true) => 1.0,
        _ => 0.0,
    };
    let raw_y: f64 = match (up, down) {
        (true, false) => -1.0,
        (false, true) => 1.0,
        _ => 0.0,
    };

    let mag = raw_x.hypot(raw_y);
    if mag == 0.0 {
        return (0.0, 0.0);
    }

    let scale = if walk_active {
        walk_scale.clamp(0.0, 1.0)
    } else {
        1.0
    };
    ((raw_x / mag) * scale, (raw_y / mag) * scale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::mouse_stick::{MouseStickConfig, STICK_MAX_F64};
    use crate::profile::zelda_default;

    fn make_mapper() -> InputMapper {
        let mut profile = zelda_default();
        profile.key_stick.sweep_ms = 0.0;
        profile.key_stick.release_ms = 0.0;
        InputMapper::new(profile, MouseStickEngine::new(MouseStickConfig::default()))
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

    #[test]
    fn mouse_on_left_stick_and_keys_on_right_stick() {
        let mut profile = zelda_default();
        profile.left_stick_mode = StickMode::Mouse;
        profile.right_stick_mode = StickMode::Keys;
        profile.key_stick.sweep_ms = 0.0;
        profile.set_binding(GamepadAction::RightStickUp, KeyCode::KEY_I);
        profile.set_binding(GamepadAction::RightStickRight, KeyCode::KEY_L);

        let mut mapper = InputMapper::new(profile, MouseStickEngine::new(MouseStickConfig::default()));
        mapper.feed_mouse_delta(50.0, -25.0);
        assert!(mapper.handle_key(KeyCode::KEY_I, true));
        assert!(mapper.handle_key(KeyCode::KEY_L, true));
        // WASD should be ignored on left stick since left_stick_mode is Mouse
        assert!(mapper.handle_key(KeyCode::KEY_W, true));

        let state = mapper.tick(0.004);
        assert_ne!(state.left_x, 0);
        assert!(state.left_y < 0);
        assert_eq!(state.right_x, 23169);
        assert_eq!(state.right_y, -23169);
    }

    #[test]
    fn disabled_stick_modes_zero_axes_and_reset_mouse() {
        let mut mapper = make_mapper();
        mapper.feed_mouse_delta(50.0, 50.0);
        let active = mapper.tick(0.004);
        assert_ne!((active.right_x, active.right_y), (0, 0));

        let mut disabled = zelda_default();
        disabled.left_stick_mode = StickMode::Disabled;
        disabled.right_stick_mode = StickMode::Disabled;
        mapper.set_profile(disabled);

        assert!(mapper.handle_key(KeyCode::KEY_W, true));
        mapper.feed_mouse_delta(50.0, 50.0);
        let state = mapper.tick(0.004);
        assert_eq!(state.left_x, 0);
        assert_eq!(state.left_y, 0);
        assert_eq!(state.right_x, 0);
        assert_eq!(state.right_y, 0);

        // Re-enabling Mouse mode should start from a clean reset state (zeroed velocity)
        let mut reenabled = zelda_default();
        reenabled.right_stick_mode = StickMode::Mouse;
        mapper.set_profile(reenabled);
        let after_reenable = mapper.tick(0.004);
        assert_eq!((after_reenable.right_x, after_reenable.right_y), (0, 0));
    }

    #[test]
    fn set_mouse_config_updates_engine_live() {
        let mut mapper = make_mapper();
        let cfg = MouseStickConfig {
            invert_y: true,
            ..MouseStickConfig::default()
        };
        mapper.set_mouse_config(cfg);
        assert_eq!(mapper.mouse_stick().config(), cfg);

        mapper.feed_mouse_delta(0.0, 20.0);
        let state = mapper.tick(0.004);
        assert!(state.right_y < 0);
    }
}
