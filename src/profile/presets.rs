use std::collections::HashMap;

use evdev::KeyCode;

use crate::engine::MouseStickConfig;
use crate::profile::{default_3d_action, GamepadAction, Profile, StickMode};

/// Returns all built-in game presets as `(label, Profile, MouseStickConfig)`.
#[must_use]
pub fn builtin_presets() -> Vec<(&'static str, Profile, MouseStickConfig)> {
    vec![
        (
            "3D Action (Default)",
            default_3d_action(),
            MouseStickConfig::default(),
        ),
        fps_shooter(),
        racing_driving(),
        platformer_2d(),
    ]
}

/// FPS / Third-Person Shooter preset: LMB=RT (Shoot), RMB=LT (Aim), WASD=Left Stick, Mouse=Right Stick.
#[must_use]
pub fn fps_shooter() -> (&'static str, Profile, MouseStickConfig) {
    let mut bindings = HashMap::new();
    bindings.insert(KeyCode::KEY_W, GamepadAction::LeftStickUp);
    bindings.insert(KeyCode::KEY_S, GamepadAction::LeftStickDown);
    bindings.insert(KeyCode::KEY_A, GamepadAction::LeftStickLeft);
    bindings.insert(KeyCode::KEY_D, GamepadAction::LeftStickRight);
    bindings.insert(KeyCode::KEY_LEFTALT, GamepadAction::WalkModifier);

    bindings.insert(KeyCode::BTN_LEFT, GamepadAction::RightTrigger);
    bindings.insert(KeyCode::BTN_RIGHT, GamepadAction::LeftTrigger);
    bindings.insert(KeyCode::KEY_SPACE, GamepadAction::South);
    bindings.insert(KeyCode::KEY_R, GamepadAction::West);
    bindings.insert(KeyCode::KEY_E, GamepadAction::North);
    bindings.insert(KeyCode::KEY_C, GamepadAction::East);
    bindings.insert(KeyCode::KEY_LEFTSHIFT, GamepadAction::LeftThumb);
    bindings.insert(KeyCode::KEY_V, GamepadAction::RightThumb);
    bindings.insert(KeyCode::KEY_Q, GamepadAction::LeftBumper);
    bindings.insert(KeyCode::KEY_F, GamepadAction::RightBumper);

    bindings.insert(KeyCode::KEY_1, GamepadAction::DpadUp);
    bindings.insert(KeyCode::KEY_2, GamepadAction::DpadDown);
    bindings.insert(KeyCode::KEY_3, GamepadAction::DpadLeft);
    bindings.insert(KeyCode::KEY_4, GamepadAction::DpadRight);
    bindings.insert(KeyCode::KEY_TAB, GamepadAction::Select);
    bindings.insert(KeyCode::KEY_ENTER, GamepadAction::Start);

    let mut profile = Profile::new("FPS / Shooter", KeyCode::KEY_F8, bindings);
    profile.left_stick_mode = StickMode::Keys;
    profile.right_stick_mode = StickMode::Mouse;

    let mouse = MouseStickConfig {
        sensitivity: 0.00065,
        anti_deadzone: 0.14,
        power_gamma: 0.80,
        decay_half_life_ms: 14.0,
        ..MouseStickConfig::default()
    };

    ("FPS / Shooter", profile, mouse)
}

/// Racing / Driving preset: A/D steering, W=RT (Gas), S/Space=LT (Brake/Handbrake).
#[must_use]
pub fn racing_driving() -> (&'static str, Profile, MouseStickConfig) {
    let mut bindings = HashMap::new();
    bindings.insert(KeyCode::KEY_A, GamepadAction::LeftStickLeft);
    bindings.insert(KeyCode::KEY_D, GamepadAction::LeftStickRight);
    bindings.insert(KeyCode::KEY_W, GamepadAction::RightTrigger);
    bindings.insert(KeyCode::KEY_S, GamepadAction::LeftTrigger);
    bindings.insert(KeyCode::KEY_SPACE, GamepadAction::East);
    bindings.insert(KeyCode::KEY_LEFTSHIFT, GamepadAction::RightBumper);
    bindings.insert(KeyCode::KEY_LEFTCTRL, GamepadAction::LeftBumper);
    bindings.insert(KeyCode::KEY_E, GamepadAction::South);
    bindings.insert(KeyCode::KEY_C, GamepadAction::North);
    bindings.insert(KeyCode::KEY_R, GamepadAction::West);
    bindings.insert(KeyCode::KEY_TAB, GamepadAction::Select);
    bindings.insert(KeyCode::KEY_ENTER, GamepadAction::Start);

    let mut profile = Profile::new("Racing / Driving", KeyCode::KEY_F8, bindings);
    profile.left_stick_mode = StickMode::Keys;
    profile.right_stick_mode = StickMode::Disabled;
    profile.key_stick.sweep_ms = 110.0;
    profile.key_stick.release_ms = 45.0;

    ("Racing / Driving", profile, MouseStickConfig::default())
}

/// 2D Platformer / Metroidvania preset: WASD movement, Space=Jump (A), J/LMB=Attack (X), K/RMB=Dash (B).
#[must_use]
pub fn platformer_2d() -> (&'static str, Profile, MouseStickConfig) {
    let mut bindings = HashMap::new();
    bindings.insert(KeyCode::KEY_W, GamepadAction::LeftStickUp);
    bindings.insert(KeyCode::KEY_S, GamepadAction::LeftStickDown);
    bindings.insert(KeyCode::KEY_A, GamepadAction::LeftStickLeft);
    bindings.insert(KeyCode::KEY_D, GamepadAction::LeftStickRight);

    bindings.insert(KeyCode::KEY_SPACE, GamepadAction::South);
    bindings.insert(KeyCode::KEY_J, GamepadAction::West);
    bindings.insert(KeyCode::KEY_K, GamepadAction::East);
    bindings.insert(KeyCode::KEY_L, GamepadAction::North);
    bindings.insert(KeyCode::KEY_Q, GamepadAction::LeftBumper);
    bindings.insert(KeyCode::KEY_E, GamepadAction::RightBumper);
    bindings.insert(KeyCode::KEY_U, GamepadAction::LeftTrigger);
    bindings.insert(KeyCode::KEY_O, GamepadAction::RightTrigger);

    bindings.insert(KeyCode::KEY_UP, GamepadAction::DpadUp);
    bindings.insert(KeyCode::KEY_DOWN, GamepadAction::DpadDown);
    bindings.insert(KeyCode::KEY_LEFT, GamepadAction::DpadLeft);
    bindings.insert(KeyCode::KEY_RIGHT, GamepadAction::DpadRight);
    bindings.insert(KeyCode::KEY_TAB, GamepadAction::Select);
    bindings.insert(KeyCode::KEY_ENTER, GamepadAction::Start);

    let mut profile = Profile::new("2D Platformer", KeyCode::KEY_F8, bindings);
    profile.left_stick_mode = StickMode::Keys;
    profile.right_stick_mode = StickMode::Disabled;
    profile.key_stick.sweep_ms = 0.0;
    profile.key_stick.release_ms = 0.0;

    ("2D Platformer", profile, MouseStickConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_presets_are_unique_and_valid() {
        let presets = builtin_presets();
        assert_eq!(presets.len(), 4);
        for (label, profile, mouse) in presets {
            assert!(!label.is_empty());
            assert!(!profile.bindings.is_empty());
            assert!(mouse.hz > 0.0);
        }
    }
}
