use std::collections::HashMap;
use evdev::KeyCode;

use super::{GamepadAction, Profile};

/// Returns the default 3D Action / Zelda profile.
#[must_use]
pub fn zelda_default() -> Profile {
    let mut bindings = HashMap::new();

    // Left stick (WASD)
    bindings.insert(KeyCode::KEY_W, GamepadAction::LeftStickUp);
    bindings.insert(KeyCode::KEY_S, GamepadAction::LeftStickDown);
    bindings.insert(KeyCode::KEY_A, GamepadAction::LeftStickLeft);
    bindings.insert(KeyCode::KEY_D, GamepadAction::LeftStickRight);

    // Walk modifier
    bindings.insert(KeyCode::KEY_LEFTALT, GamepadAction::WalkModifier);
    bindings.insert(KeyCode::KEY_LEFTCTRL, GamepadAction::WalkModifier);

    // Mouse buttons & combat/action face buttons
    bindings.insert(KeyCode::BTN_LEFT, GamepadAction::West);
    bindings.insert(KeyCode::BTN_RIGHT, GamepadAction::LeftTrigger);
    bindings.insert(KeyCode::KEY_SPACE, GamepadAction::North);
    bindings.insert(KeyCode::KEY_LEFTSHIFT, GamepadAction::East);
    bindings.insert(KeyCode::KEY_E, GamepadAction::South);
    bindings.insert(KeyCode::KEY_Q, GamepadAction::RightTrigger);
    bindings.insert(KeyCode::KEY_R, GamepadAction::RightBumper);
    bindings.insert(KeyCode::KEY_F, GamepadAction::LeftBumper);

    // D-pad (1..4)
    bindings.insert(KeyCode::KEY_1, GamepadAction::DpadUp);
    bindings.insert(KeyCode::KEY_2, GamepadAction::DpadDown);
    bindings.insert(KeyCode::KEY_3, GamepadAction::DpadLeft);
    bindings.insert(KeyCode::KEY_4, GamepadAction::DpadRight);

    // Menu & stick clicks
    bindings.insert(KeyCode::KEY_TAB, GamepadAction::Select);
    bindings.insert(KeyCode::KEY_ENTER, GamepadAction::Start);
    bindings.insert(KeyCode::KEY_C, GamepadAction::LeftThumb);
    bindings.insert(KeyCode::KEY_V, GamepadAction::RightThumb);

    Profile {
        name: "zelda_default".to_string(),
        toggle_key: KeyCode::KEY_F8,
        bindings,
        walk_scale: 0.5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zelda_default_has_expected_toggle_and_walk_scale() {
        let profile = zelda_default();
        assert_eq!(profile.name, "zelda_default");
        assert_eq!(profile.toggle_key, KeyCode::KEY_F8);
        assert!((profile.walk_scale - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn zelda_default_contains_all_expected_bindings() {
        let profile = zelda_default();
        let expected = [
            (KeyCode::KEY_W, GamepadAction::LeftStickUp),
            (KeyCode::KEY_S, GamepadAction::LeftStickDown),
            (KeyCode::KEY_A, GamepadAction::LeftStickLeft),
            (KeyCode::KEY_D, GamepadAction::LeftStickRight),
            (KeyCode::KEY_LEFTALT, GamepadAction::WalkModifier),
            (KeyCode::KEY_LEFTCTRL, GamepadAction::WalkModifier),
            (KeyCode::BTN_LEFT, GamepadAction::West),
            (KeyCode::BTN_RIGHT, GamepadAction::LeftTrigger),
            (KeyCode::KEY_SPACE, GamepadAction::North),
            (KeyCode::KEY_LEFTSHIFT, GamepadAction::East),
            (KeyCode::KEY_E, GamepadAction::South),
            (KeyCode::KEY_Q, GamepadAction::RightTrigger),
            (KeyCode::KEY_R, GamepadAction::RightBumper),
            (KeyCode::KEY_F, GamepadAction::LeftBumper),
            (KeyCode::KEY_1, GamepadAction::DpadUp),
            (KeyCode::KEY_2, GamepadAction::DpadDown),
            (KeyCode::KEY_3, GamepadAction::DpadLeft),
            (KeyCode::KEY_4, GamepadAction::DpadRight),
            (KeyCode::KEY_TAB, GamepadAction::Select),
            (KeyCode::KEY_ENTER, GamepadAction::Start),
            (KeyCode::KEY_C, GamepadAction::LeftThumb),
            (KeyCode::KEY_V, GamepadAction::RightThumb),
        ];

        assert_eq!(profile.bindings.len(), expected.len());
        for (key, action) in expected {
            assert_eq!(
                profile.bindings.get(&key),
                Some(&action),
                "missing or mismatched binding for {key:?}"
            );
        }
    }
}
