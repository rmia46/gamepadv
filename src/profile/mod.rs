pub mod zelda_default;

use std::collections::HashMap;
use evdev::KeyCode;

pub use zelda_default::zelda_default;

/// Logical Xbox 360 gamepad actions and virtual input modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadAction {
    South,
    East,
    North,
    West,
    LeftBumper,
    RightBumper,
    LeftTrigger,
    RightTrigger,
    Select,
    Start,
    Mode,
    LeftThumb,
    RightThumb,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    LeftStickUp,
    LeftStickDown,
    LeftStickLeft,
    LeftStickRight,
    WalkModifier,
}

/// Key/mouse-to-gamepad mapping profile.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub name: String,
    pub toggle_key: KeyCode,
    pub bindings: HashMap<KeyCode, GamepadAction>,
    pub walk_scale: f64,
}

impl Profile {
    /// Creates a new [`Profile`] with the given name, toggle key, and bindings,
    /// using the default `walk_scale` of `0.5`.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        toggle_key: KeyCode,
        bindings: HashMap<KeyCode, GamepadAction>,
    ) -> Self {
        Self {
            name: name.into(),
            toggle_key,
            bindings,
            walk_scale: 0.5,
        }
    }
}
