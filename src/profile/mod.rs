pub mod action_default;
pub mod presets;

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use evdev::KeyCode;

use crate::engine::key_stick::KeyStickConfig;
use crate::engine::mouse_stick::MouseStickConfig;

pub use action_default::default_3d_action;
pub use presets::builtin_presets;

/// Input source mode for an analog stick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum StickMode {
    Mouse,
    Keys,
    Disabled,
}

/// Logical Xbox 360 gamepad actions and virtual input modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
    RightStickUp,
    RightStickDown,
    RightStickLeft,
    RightStickRight,
    WalkModifier,
    CenterMouseStick,
}

/// Key/mouse-to-gamepad mapping profile.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub name: String,
    pub toggle_key: KeyCode,
    pub bindings: HashMap<KeyCode, GamepadAction>,
    pub walk_scale: f64,
    pub left_stick_mode: StickMode,
    pub right_stick_mode: StickMode,
    pub key_stick: KeyStickConfig,
}

impl Profile {
    /// Creates a new [`Profile`] with the given name, toggle key, and bindings,
    /// using the default `walk_scale` of `0.5`, `left_stick_mode: StickMode::Keys`,
    /// `right_stick_mode: StickMode::Mouse`, and default [`KeyStickConfig`].
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
            left_stick_mode: StickMode::Keys,
            right_stick_mode: StickMode::Mouse,
            key_stick: KeyStickConfig::default(),
        }
    }

    /// Returns the first [`KeyCode`] bound to `action`, if any.
    #[must_use]
    pub fn binding_for_action(&self, action: GamepadAction) -> Option<KeyCode> {
        self.bindings
            .iter()
            .filter_map(|(&key, &act)| (act == action).then_some(key))
            .min_by_key(|key| key.code())
    }

    /// Returns all [`KeyCode`]s bound to `action`, sorted deterministically.
    #[must_use]
    pub fn bindings_for_action(&self, action: GamepadAction) -> Vec<KeyCode> {
        let mut keys: Vec<KeyCode> = self
            .bindings
            .iter()
            .filter_map(|(&key, &act)| (act == action).then_some(key))
            .collect();
        keys.sort_by_key(|key| key.code());
        keys
    }

    /// Replaces any existing bindings for `action` with a single binding to `key`.
    pub fn set_binding(&mut self, action: GamepadAction, key: KeyCode) {
        self.bindings.retain(|_, &mut act| act != action);
        self.bindings.insert(key, action);
    }

    /// Adds `key` as a binding for `action` without removing other keys bound to `action`.
    pub fn add_binding(&mut self, action: GamepadAction, key: KeyCode) {
        self.bindings.insert(key, action);
    }

    /// Removes a specific `key` binding if present.
    pub fn remove_binding(&mut self, key: KeyCode) -> bool {
        self.bindings.remove(&key).is_some()
    }

    /// Clears all keys bound to `action`.
    pub fn clear_action_bindings(&mut self, action: GamepadAction) {
        self.bindings.retain(|_, &mut act| act != action);
    }

    /// Returns `true` if `name` matches one of the immutable built-in presets.
    #[must_use]
    pub fn is_builtin_preset_name(name: &str) -> bool {
        builtin_presets()
            .iter()
            .any(|(_, p, _)| p.name == name)
    }
}

/// Serializable configuration snapshot combining [`Profile`] and [`MouseStickConfig`].
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProfileConfig {
    pub name: String,
    pub toggle_key: u16,
    pub walk_scale: f64,
    pub left_stick_mode: StickMode,
    pub right_stick_mode: StickMode,
    pub bindings: Vec<(u16, GamepadAction)>,
    pub mouse: MouseStickConfig,
    #[serde(default)]
    pub key_stick: KeyStickConfig,
}

impl ProfileConfig {
    /// Constructs a [`ProfileConfig`] from a [`Profile`] and [`MouseStickConfig`].
    #[must_use]
    pub fn from_parts(profile: &Profile, mouse: &MouseStickConfig) -> Self {
        let mut bindings: Vec<(u16, GamepadAction)> = profile
            .bindings
            .iter()
            .map(|(key, &action)| (key.code(), action))
            .collect();
        bindings.sort_by_key(|(code, _)| *code);

        Self {
            name: profile.name.clone(),
            toggle_key: profile.toggle_key.code(),
            walk_scale: profile.walk_scale,
            left_stick_mode: profile.left_stick_mode,
            right_stick_mode: profile.right_stick_mode,
            bindings,
            mouse: *mouse,
            key_stick: profile.key_stick,
        }
    }

    /// Converts this [`ProfileConfig`] into a [`Profile`] and [`MouseStickConfig`].
    #[must_use]
    pub fn into_parts(self) -> (Profile, MouseStickConfig) {
        let bindings = self
            .bindings
            .into_iter()
            .map(|(code, action)| (KeyCode::new(code), action))
            .collect();

        let profile = Profile {
            name: self.name,
            toggle_key: KeyCode::new(self.toggle_key),
            bindings,
            walk_scale: self.walk_scale,
            left_stick_mode: self.left_stick_mode,
            right_stick_mode: self.right_stick_mode,
            key_stick: self.key_stick,
        };
        (profile, self.mouse)
    }

    /// Serializes this configuration as pretty-printed JSON and writes it to `path`.
    pub fn save_to_file(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        std::fs::write(path, json)
    }

    /// Reads and deserializes a [`ProfileConfig`] from a JSON file at `path`.
    pub fn load_from_file(path: &Path) -> io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    /// Returns the default configuration file path
    /// (`$XDG_CONFIG_HOME/gamepadv/profile.json` or `$HOME/.config/gamepadv/profile.json`).
    #[must_use]
    pub fn default_config_path() -> Option<PathBuf> {
        Self::config_root_dir().map(|dir| dir.join("profile.json"))
    }

    /// Returns the directory for named saved profiles (`<config_root>/profiles/`).
    #[must_use]
    pub fn profiles_dir() -> Option<PathBuf> {
        Self::config_root_dir().map(|dir| dir.join("profiles"))
    }

    /// Returns a sanitized `.json` path inside `profiles_dir()` for `name`.
    #[must_use]
    pub fn profile_path_for_name(name: &str) -> Option<PathBuf> {
        let slug: String = name
            .trim()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        if slug.is_empty() {
            return None;
        }
        Self::profiles_dir().map(|dir| dir.join(format!("{slug}.json")))
    }

    /// Lists saved `.json` profiles in `profiles_dir()`, sorted by name.
    #[must_use]
    pub fn list_saved_profiles() -> Vec<(String, PathBuf)> {
        let Some(dir) = Self::profiles_dir() else {
            return Vec::new();
        };
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    out.push((stem.to_string(), path));
                }
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    fn config_root_dir() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .filter(|v| !v.is_empty())
                    .map(|home| PathBuf::from(home).join(".config"))
            })?;
        Some(base.join("gamepadv"))
    }
}

/// Returns a short human-readable label for common keyboard keys and mouse buttons.
#[must_use]
pub fn key_display_name(code: KeyCode) -> String {
    match code {
        KeyCode::KEY_SPACE => "Space".to_string(),
        KeyCode::BTN_LEFT => "L-Click".to_string(),
        KeyCode::BTN_RIGHT => "R-Click".to_string(),
        KeyCode::BTN_MIDDLE => "M-Click".to_string(),
        KeyCode::BTN_SIDE => "Mouse 4".to_string(),
        KeyCode::BTN_EXTRA => "Mouse 5".to_string(),
        KeyCode::KEY_LEFTSHIFT => "L-Shift".to_string(),
        KeyCode::KEY_RIGHTSHIFT => "R-Shift".to_string(),
        KeyCode::KEY_LEFTALT => "L-Alt".to_string(),
        KeyCode::KEY_RIGHTALT => "R-Alt".to_string(),
        KeyCode::KEY_LEFTCTRL => "L-Ctrl".to_string(),
        KeyCode::KEY_RIGHTCTRL => "R-Ctrl".to_string(),
        KeyCode::KEY_TAB => "Tab".to_string(),
        KeyCode::KEY_ENTER => "Enter".to_string(),
        KeyCode::KEY_ESC => "Esc".to_string(),
        KeyCode::KEY_BACKSPACE => "Backspace".to_string(),
        KeyCode::KEY_UP => "Up".to_string(),
        KeyCode::KEY_DOWN => "Down".to_string(),
        KeyCode::KEY_LEFT => "Left".to_string(),
        KeyCode::KEY_RIGHT => "Right".to_string(),
        KeyCode::KEY_A => "A".to_string(),
        KeyCode::KEY_B => "B".to_string(),
        KeyCode::KEY_C => "C".to_string(),
        KeyCode::KEY_D => "D".to_string(),
        KeyCode::KEY_E => "E".to_string(),
        KeyCode::KEY_F => "F".to_string(),
        KeyCode::KEY_G => "G".to_string(),
        KeyCode::KEY_H => "H".to_string(),
        KeyCode::KEY_I => "I".to_string(),
        KeyCode::KEY_J => "J".to_string(),
        KeyCode::KEY_K => "K".to_string(),
        KeyCode::KEY_L => "L".to_string(),
        KeyCode::KEY_M => "M".to_string(),
        KeyCode::KEY_N => "N".to_string(),
        KeyCode::KEY_O => "O".to_string(),
        KeyCode::KEY_P => "P".to_string(),
        KeyCode::KEY_Q => "Q".to_string(),
        KeyCode::KEY_R => "R".to_string(),
        KeyCode::KEY_S => "S".to_string(),
        KeyCode::KEY_T => "T".to_string(),
        KeyCode::KEY_U => "U".to_string(),
        KeyCode::KEY_V => "V".to_string(),
        KeyCode::KEY_W => "W".to_string(),
        KeyCode::KEY_X => "X".to_string(),
        KeyCode::KEY_Y => "Y".to_string(),
        KeyCode::KEY_Z => "Z".to_string(),
        KeyCode::KEY_0 => "0".to_string(),
        KeyCode::KEY_1 => "1".to_string(),
        KeyCode::KEY_2 => "2".to_string(),
        KeyCode::KEY_3 => "3".to_string(),
        KeyCode::KEY_4 => "4".to_string(),
        KeyCode::KEY_5 => "5".to_string(),
        KeyCode::KEY_6 => "6".to_string(),
        KeyCode::KEY_7 => "7".to_string(),
        KeyCode::KEY_8 => "8".to_string(),
        KeyCode::KEY_9 => "9".to_string(),
        KeyCode::KEY_F1 => "F1".to_string(),
        KeyCode::KEY_F2 => "F2".to_string(),
        KeyCode::KEY_F3 => "F3".to_string(),
        KeyCode::KEY_F4 => "F4".to_string(),
        KeyCode::KEY_F5 => "F5".to_string(),
        KeyCode::KEY_F6 => "F6".to_string(),
        KeyCode::KEY_F7 => "F7".to_string(),
        KeyCode::KEY_F8 => "F8".to_string(),
        KeyCode::KEY_F9 => "F9".to_string(),
        KeyCode::KEY_F10 => "F10".to_string(),
        KeyCode::KEY_F11 => "F11".to_string(),
        KeyCode::KEY_F12 => "F12".to_string(),
        _ => format!("{code:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_config_json_round_trip() {
        let mut profile = default_3d_action();
        profile.left_stick_mode = StickMode::Mouse;
        profile.right_stick_mode = StickMode::Keys;
        profile.set_binding(GamepadAction::RightStickUp, KeyCode::KEY_I);

        let mouse = MouseStickConfig {
            sensitivity: 0.0012,
            invert_y: true,
            ..MouseStickConfig::default()
        };

        let config = ProfileConfig::from_parts(&profile, &mouse);
        let json = serde_json::to_string_pretty(&config).expect("serialize ProfileConfig");
        let parsed: ProfileConfig = serde_json::from_str(&json).expect("deserialize ProfileConfig");
        assert_eq!(parsed, config);

        let (round_profile, round_mouse) = parsed.into_parts();
        assert_eq!(round_profile, profile);
        assert_eq!(round_mouse, mouse);
    }

    #[test]
    fn profile_config_save_and_load_file() {
        let profile = default_3d_action();
        let mouse = MouseStickConfig::default();
        let config = ProfileConfig::from_parts(&profile, &mouse);

        let dir = std::env::temp_dir().join(format!("gamepadv_test_{}", std::process::id()));
        let path = dir.join("nested").join("profile.json");

        config.save_to_file(&path).expect("save_to_file");
        let loaded = ProfileConfig::load_from_file(&path).expect("load_from_file");
        assert_eq!(loaded, config);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn set_binding_replaces_prior_binding_for_action() {
        let mut profile = default_3d_action();
        assert_eq!(
            profile.binding_for_action(GamepadAction::South),
            Some(KeyCode::KEY_E)
        );

        profile.set_binding(GamepadAction::South, KeyCode::KEY_G);

        assert!(!profile.bindings.contains_key(&KeyCode::KEY_E));
        assert_eq!(
            profile.binding_for_action(GamepadAction::South),
            Some(KeyCode::KEY_G)
        );
        assert_eq!(
            profile.bindings.get(&KeyCode::KEY_G),
            Some(&GamepadAction::South)
        );
    }

    #[test]
    fn key_display_name_formats_common_and_fallback_keys() {
        assert_eq!(key_display_name(KeyCode::KEY_SPACE), "Space");
        assert_eq!(key_display_name(KeyCode::BTN_LEFT), "L-Click");
        assert_eq!(key_display_name(KeyCode::BTN_RIGHT), "R-Click");
        assert_eq!(key_display_name(KeyCode::BTN_MIDDLE), "M-Click");
        assert_eq!(key_display_name(KeyCode::KEY_LEFTSHIFT), "L-Shift");
        assert_eq!(key_display_name(KeyCode::KEY_LEFTALT), "L-Alt");
        assert_eq!(key_display_name(KeyCode::KEY_LEFTCTRL), "L-Ctrl");
        assert_eq!(key_display_name(KeyCode::KEY_A), "A");
        assert_eq!(key_display_name(KeyCode::KEY_0), "0");
        assert_eq!(key_display_name(KeyCode::KEY_F8), "F8");
        assert_eq!(
            key_display_name(KeyCode::KEY_CAPSLOCK),
            format!("{:?}", KeyCode::KEY_CAPSLOCK)
        );
    }

    #[test]
    fn multi_button_add_and_clear_bindings_work() {
        let mut profile = default_3d_action();
        profile.add_binding(GamepadAction::South, KeyCode::KEY_G);
        let keys = profile.bindings_for_action(GamepadAction::South);
        assert_eq!(keys, vec![KeyCode::KEY_E, KeyCode::KEY_G]);

        profile.clear_action_bindings(GamepadAction::South);
        assert!(profile.bindings_for_action(GamepadAction::South).is_empty());
    }

    #[test]
    fn builtin_preset_names_are_detected() {
        assert!(Profile::is_builtin_preset_name("3D Action (Default)"));
        assert!(Profile::is_builtin_preset_name("FPS / Shooter"));
        assert!(!Profile::is_builtin_preset_name("3D Action (Default) (Custom)"));
        assert!(!Profile::is_builtin_preset_name("My Custom Profile"));
    }
}
