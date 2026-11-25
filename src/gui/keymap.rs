use eframe::egui::{Key, PointerButton};
use evdev::KeyCode;

/// Maps an [`egui::Key`] to an [`evdev::KeyCode`], if a direct mapping exists.
#[must_use]
pub fn egui_key_to_evdev(key: Key) -> Option<KeyCode> {
    let code = match key {
        Key::A => KeyCode::KEY_A,
        Key::B => KeyCode::KEY_B,
        Key::C => KeyCode::KEY_C,
        Key::D => KeyCode::KEY_D,
        Key::E => KeyCode::KEY_E,
        Key::F => KeyCode::KEY_F,
        Key::G => KeyCode::KEY_G,
        Key::H => KeyCode::KEY_H,
        Key::I => KeyCode::KEY_I,
        Key::J => KeyCode::KEY_J,
        Key::K => KeyCode::KEY_K,
        Key::L => KeyCode::KEY_L,
        Key::M => KeyCode::KEY_M,
        Key::N => KeyCode::KEY_N,
        Key::O => KeyCode::KEY_O,
        Key::P => KeyCode::KEY_P,
        Key::Q => KeyCode::KEY_Q,
        Key::R => KeyCode::KEY_R,
        Key::S => KeyCode::KEY_S,
        Key::T => KeyCode::KEY_T,
        Key::U => KeyCode::KEY_U,
        Key::V => KeyCode::KEY_V,
        Key::W => KeyCode::KEY_W,
        Key::X => KeyCode::KEY_X,
        Key::Y => KeyCode::KEY_Y,
        Key::Z => KeyCode::KEY_Z,
        Key::Num0 => KeyCode::KEY_0,
        Key::Num1 => KeyCode::KEY_1,
        Key::Num2 => KeyCode::KEY_2,
        Key::Num3 => KeyCode::KEY_3,
        Key::Num4 => KeyCode::KEY_4,
        Key::Num5 => KeyCode::KEY_5,
        Key::Num6 => KeyCode::KEY_6,
        Key::Num7 => KeyCode::KEY_7,
        Key::Num8 => KeyCode::KEY_8,
        Key::Num9 => KeyCode::KEY_9,
        Key::Space => KeyCode::KEY_SPACE,
        Key::Enter => KeyCode::KEY_ENTER,
        Key::Tab => KeyCode::KEY_TAB,
        Key::Backspace => KeyCode::KEY_BACKSPACE,
        Key::Escape => KeyCode::KEY_ESC,
        Key::Insert => KeyCode::KEY_INSERT,
        Key::Delete => KeyCode::KEY_DELETE,
        Key::Home => KeyCode::KEY_HOME,
        Key::End => KeyCode::KEY_END,
        Key::PageUp => KeyCode::KEY_PAGEUP,
        Key::PageDown => KeyCode::KEY_PAGEDOWN,
        Key::ArrowUp => KeyCode::KEY_UP,
        Key::ArrowDown => KeyCode::KEY_DOWN,
        Key::ArrowLeft => KeyCode::KEY_LEFT,
        Key::ArrowRight => KeyCode::KEY_RIGHT,
        Key::F1 => KeyCode::KEY_F1,
        Key::F2 => KeyCode::KEY_F2,
        Key::F3 => KeyCode::KEY_F3,
        Key::F4 => KeyCode::KEY_F4,
        Key::F5 => KeyCode::KEY_F5,
        Key::F6 => KeyCode::KEY_F6,
        Key::F7 => KeyCode::KEY_F7,
        Key::F8 => KeyCode::KEY_F8,
        Key::F9 => KeyCode::KEY_F9,
        Key::F10 => KeyCode::KEY_F10,
        Key::F11 => KeyCode::KEY_F11,
        Key::F12 => KeyCode::KEY_F12,
        Key::Minus => KeyCode::KEY_MINUS,
        Key::Equals | Key::Plus => KeyCode::KEY_EQUAL,
        Key::OpenBracket => KeyCode::KEY_LEFTBRACE,
        Key::CloseBracket => KeyCode::KEY_RIGHTBRACE,
        Key::Backslash => KeyCode::KEY_BACKSLASH,
        Key::Semicolon => KeyCode::KEY_SEMICOLON,
        Key::Quote => KeyCode::KEY_APOSTROPHE,
        Key::Comma => KeyCode::KEY_COMMA,
        Key::Period => KeyCode::KEY_DOT,
        Key::Slash => KeyCode::KEY_SLASH,
        Key::Backtick => KeyCode::KEY_GRAVE,
        _ => return None,
    };
    Some(code)
}

/// Maps an [`egui::PointerButton`] to an [`evdev::KeyCode`].
#[must_use]
pub fn pointer_button_to_evdev(button: PointerButton) -> Option<KeyCode> {
    let code = match button {
        PointerButton::Primary => KeyCode::BTN_LEFT,
        PointerButton::Secondary => KeyCode::BTN_RIGHT,
        PointerButton::Middle => KeyCode::BTN_MIDDLE,
        PointerButton::Extra1 => KeyCode::BTN_SIDE,
        PointerButton::Extra2 => KeyCode::BTN_EXTRA,
    };
    Some(code)
}

/// Returns a short, human-friendly label for an [`evdev::KeyCode`].
#[must_use]
pub fn format_keycode(code: KeyCode) -> String {
    match code {
        KeyCode::BTN_LEFT => "L-Click".to_string(),
        KeyCode::BTN_RIGHT => "R-Click".to_string(),
        KeyCode::BTN_MIDDLE => "M-Click".to_string(),
        KeyCode::BTN_SIDE => "Mouse4".to_string(),
        KeyCode::BTN_EXTRA => "Mouse5".to_string(),
        KeyCode::KEY_LEFTSHIFT => "L-Shift".to_string(),
        KeyCode::KEY_RIGHTSHIFT => "R-Shift".to_string(),
        KeyCode::KEY_LEFTCTRL => "L-Ctrl".to_string(),
        KeyCode::KEY_RIGHTCTRL => "R-Ctrl".to_string(),
        KeyCode::KEY_LEFTALT => "L-Alt".to_string(),
        KeyCode::KEY_RIGHTALT => "R-Alt".to_string(),
        KeyCode::KEY_LEFTMETA => "L-Super".to_string(),
        KeyCode::KEY_SPACE => "Space".to_string(),
        KeyCode::KEY_ENTER => "Enter".to_string(),
        KeyCode::KEY_TAB => "Tab".to_string(),
        KeyCode::KEY_BACKSPACE => "Bksp".to_string(),
        KeyCode::KEY_ESC => "Esc".to_string(),
        KeyCode::KEY_UP => "Up".to_string(),
        KeyCode::KEY_DOWN => "Down".to_string(),
        KeyCode::KEY_LEFT => "Left".to_string(),
        KeyCode::KEY_RIGHT => "Right".to_string(),
        KeyCode::KEY_MINUS => "-".to_string(),
        KeyCode::KEY_EQUAL => "=".to_string(),
        _ => {
            let raw = format!("{code:?}");
            raw.strip_prefix("KEY_")
                .or_else(|| raw.strip_prefix("BTN_"))
                .unwrap_or(&raw)
                .to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_letters_and_digits() {
        assert_eq!(egui_key_to_evdev(Key::A), Some(KeyCode::KEY_A));
        assert_eq!(egui_key_to_evdev(Key::W), Some(KeyCode::KEY_W));
        assert_eq!(egui_key_to_evdev(Key::Z), Some(KeyCode::KEY_Z));
        assert_eq!(egui_key_to_evdev(Key::Num0), Some(KeyCode::KEY_0));
        assert_eq!(egui_key_to_evdev(Key::Num1), Some(KeyCode::KEY_1));
        assert_eq!(egui_key_to_evdev(Key::Num9), Some(KeyCode::KEY_9));
    }

    #[test]
    fn maps_navigation_punctuation_and_function_keys() {
        assert_eq!(egui_key_to_evdev(Key::Space), Some(KeyCode::KEY_SPACE));
        assert_eq!(egui_key_to_evdev(Key::Enter), Some(KeyCode::KEY_ENTER));
        assert_eq!(egui_key_to_evdev(Key::Tab), Some(KeyCode::KEY_TAB));
        assert_eq!(
            egui_key_to_evdev(Key::Backspace),
            Some(KeyCode::KEY_BACKSPACE)
        );
        assert_eq!(egui_key_to_evdev(Key::Escape), Some(KeyCode::KEY_ESC));
        assert_eq!(egui_key_to_evdev(Key::ArrowUp), Some(KeyCode::KEY_UP));
        assert_eq!(egui_key_to_evdev(Key::ArrowDown), Some(KeyCode::KEY_DOWN));
        assert_eq!(egui_key_to_evdev(Key::ArrowLeft), Some(KeyCode::KEY_LEFT));
        assert_eq!(egui_key_to_evdev(Key::ArrowRight), Some(KeyCode::KEY_RIGHT));
        assert_eq!(egui_key_to_evdev(Key::F1), Some(KeyCode::KEY_F1));
        assert_eq!(egui_key_to_evdev(Key::F8), Some(KeyCode::KEY_F8));
        assert_eq!(egui_key_to_evdev(Key::F12), Some(KeyCode::KEY_F12));
        assert_eq!(egui_key_to_evdev(Key::Minus), Some(KeyCode::KEY_MINUS));
        assert_eq!(egui_key_to_evdev(Key::Equals), Some(KeyCode::KEY_EQUAL));
    }

    #[test]
    fn maps_pointer_buttons() {
        assert_eq!(
            pointer_button_to_evdev(PointerButton::Primary),
            Some(KeyCode::BTN_LEFT)
        );
        assert_eq!(
            pointer_button_to_evdev(PointerButton::Secondary),
            Some(KeyCode::BTN_RIGHT)
        );
        assert_eq!(
            pointer_button_to_evdev(PointerButton::Middle),
            Some(KeyCode::BTN_MIDDLE)
        );
        assert_eq!(
            pointer_button_to_evdev(PointerButton::Extra1),
            Some(KeyCode::BTN_SIDE)
        );
        assert_eq!(
            pointer_button_to_evdev(PointerButton::Extra2),
            Some(KeyCode::BTN_EXTRA)
        );
    }

    #[test]
    fn formats_human_friendly_labels() {
        assert_eq!(format_keycode(KeyCode::BTN_LEFT), "L-Click");
        assert_eq!(format_keycode(KeyCode::BTN_RIGHT), "R-Click");
        assert_eq!(format_keycode(KeyCode::KEY_LEFTSHIFT), "L-Shift");
        assert_eq!(format_keycode(KeyCode::KEY_SPACE), "Space");
        assert_eq!(format_keycode(KeyCode::KEY_E), "E");
    }
}
