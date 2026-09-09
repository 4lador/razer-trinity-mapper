//! Iced key to domain key code conversion.

use rust_i18n::t;

use trinity_core::key::KeyCode;

/// Maps an Iced key to a domain `KeyCode`.
///
/// Bare modifier keys return `None`: they are captured through the
/// `Modifiers` state when the actual key is pressed.
pub fn iced_key_to_keycode(key: &iced::keyboard::Key) -> Option<KeyCode> {
    match key {
        iced::keyboard::Key::Character(characters) => characters_to_keycode(characters),
        iced::keyboard::Key::Named(named) => named_to_keycode(named),
        _ => None,
    }
}

fn characters_to_keycode(characters: &str) -> Option<KeyCode> {
    let mut chars = characters.chars();
    let single = chars.next()?.to_ascii_lowercase();
    if chars.next().is_some() {
        return None;
    }
    char_to_keycode(single)
}

fn char_to_keycode(character: char) -> Option<KeyCode> {
    let key = match character {
        '1' => KeyCode::KEY_1,
        '2' => KeyCode::KEY_2,
        '3' => KeyCode::KEY_3,
        '4' => KeyCode::KEY_4,
        '5' => KeyCode::KEY_5,
        '6' => KeyCode::KEY_6,
        '7' => KeyCode::KEY_7,
        '8' => KeyCode::KEY_8,
        '9' => KeyCode::KEY_9,
        '0' => KeyCode::KEY_0,
        'q' => KeyCode::KEY_Q,
        'w' => KeyCode::KEY_W,
        'e' => KeyCode::KEY_E,
        'r' => KeyCode::KEY_R,
        't' => KeyCode::KEY_T,
        'y' => KeyCode::KEY_Y,
        'u' => KeyCode::KEY_U,
        'i' => KeyCode::KEY_I,
        'o' => KeyCode::KEY_O,
        'p' => KeyCode::KEY_P,
        'a' => KeyCode::KEY_A,
        's' => KeyCode::KEY_S,
        'd' => KeyCode::KEY_D,
        'f' => KeyCode::KEY_F,
        'g' => KeyCode::KEY_G,
        'h' => KeyCode::KEY_H,
        'j' => KeyCode::KEY_J,
        'k' => KeyCode::KEY_K,
        'l' => KeyCode::KEY_L,
        'z' => KeyCode::KEY_Z,
        'x' => KeyCode::KEY_X,
        'c' => KeyCode::KEY_C,
        'v' => KeyCode::KEY_V,
        'b' => KeyCode::KEY_B,
        'n' => KeyCode::KEY_N,
        'm' => KeyCode::KEY_M,
        '-' => KeyCode::KEY_MINUS,
        '=' => KeyCode::KEY_EQUAL,
        '[' => KeyCode::KEY_LEFTBRACE,
        ']' => KeyCode::KEY_RIGHTBRACE,
        ';' => KeyCode::KEY_SEMICOLON,
        '\'' => KeyCode::KEY_APOSTROPHE,
        '`' => KeyCode::KEY_GRAVE,
        '\\' => KeyCode::KEY_BACKSLASH,
        ',' => KeyCode::KEY_COMMA,
        '.' => KeyCode::KEY_DOT,
        '/' => KeyCode::KEY_SLASH,
        ' ' => KeyCode::KEY_SPACE,
        _ => return None,
    };
    Some(key)
}

fn named_to_keycode(named: &iced::keyboard::key::Named) -> Option<KeyCode> {
    use iced::keyboard::key::Named;
    let key = match named {
        Named::Escape => KeyCode::KEY_ESC,
        Named::Enter => KeyCode::KEY_ENTER,
        Named::Tab => KeyCode::KEY_TAB,
        Named::Backspace => KeyCode::KEY_BACKSPACE,
        Named::Delete => KeyCode::KEY_DELETE,
        Named::Insert => KeyCode::KEY_INSERT,
        Named::Home => KeyCode::KEY_HOME,
        Named::End => KeyCode::KEY_END,
        Named::PageUp => KeyCode::KEY_PAGEUP,
        Named::PageDown => KeyCode::KEY_PAGEDOWN,
        Named::ArrowUp => KeyCode::KEY_UP,
        Named::ArrowDown => KeyCode::KEY_DOWN,
        Named::ArrowLeft => KeyCode::KEY_LEFT,
        Named::ArrowRight => KeyCode::KEY_RIGHT,
        Named::Space => KeyCode::KEY_SPACE,
        Named::F1 => KeyCode::KEY_F1,
        Named::F2 => KeyCode::KEY_F2,
        Named::F3 => KeyCode::KEY_F3,
        Named::F4 => KeyCode::KEY_F4,
        Named::F5 => KeyCode::KEY_F5,
        Named::F6 => KeyCode::KEY_F6,
        Named::F7 => KeyCode::KEY_F7,
        Named::F8 => KeyCode::KEY_F8,
        Named::F9 => KeyCode::KEY_F9,
        Named::F10 => KeyCode::KEY_F10,
        Named::F11 => KeyCode::KEY_F11,
        Named::F12 => KeyCode::KEY_F12,
        _ => return None,
    };
    Some(key)
}

/// Short label for grid display.
pub fn keycode_label(key: KeyCode) -> String {
    let name = key.label();
    let name = name.strip_prefix("KEY_").unwrap_or(&name);
    match key {
        KeyCode::KEY_ESC => t!("keys.esc").to_string(),
        KeyCode::KEY_ENTER => t!("keys.enter").to_string(),
        KeyCode::KEY_TAB => t!("keys.tab").to_string(),
        KeyCode::KEY_BACKSPACE => t!("keys.backspace").to_string(),
        KeyCode::KEY_DELETE => t!("keys.delete").to_string(),
        KeyCode::KEY_INSERT => t!("keys.insert").to_string(),
        KeyCode::KEY_SPACE => t!("keys.space").to_string(),
        KeyCode::KEY_PAGEUP => t!("keys.page_up").to_string(),
        KeyCode::KEY_PAGEDOWN => t!("keys.page_down").to_string(),
        KeyCode::KEY_HOME => t!("keys.home").to_string(),
        KeyCode::KEY_END => t!("keys.end").to_string(),
        KeyCode::KEY_UP => "↑".to_owned(),
        KeyCode::KEY_DOWN => "↓".to_owned(),
        KeyCode::KEY_LEFT => "←".to_owned(),
        KeyCode::KEY_RIGHT => "→".to_owned(),
        KeyCode::KEY_LEFTCTRL => t!("keys.ctrl").to_string(),
        KeyCode::KEY_LEFTSHIFT => t!("keys.shift").to_string(),
        KeyCode::KEY_LEFTALT => t!("keys.alt").to_string(),
        KeyCode::KEY_GRAVE => "`".to_owned(),
        _ => name.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from_char(character: &str) -> Option<KeyCode> {
        iced_key_to_keycode(&iced::keyboard::Key::Character(character.into()))
    }

    #[test]
    fn maps_letters_digits_and_symbols() {
        assert_eq!(from_char("a"), Some(KeyCode::KEY_A));
        assert_eq!(from_char("Q"), Some(KeyCode::KEY_Q));
        assert_eq!(from_char("1"), Some(KeyCode::KEY_1));
        assert_eq!(from_char("0"), Some(KeyCode::KEY_0));
        assert_eq!(from_char("-"), Some(KeyCode::KEY_MINUS));
        assert_eq!(from_char(" "), Some(KeyCode::KEY_SPACE));
        assert_eq!(from_char("eu"), None);
    }

    #[test]
    fn maps_named_keys() {
        use iced::keyboard::Key;
        assert_eq!(
            iced_key_to_keycode(&Key::Named(iced::keyboard::key::Named::Escape)),
            Some(KeyCode::KEY_ESC)
        );
        assert_eq!(
            iced_key_to_keycode(&Key::Named(iced::keyboard::key::Named::F12)),
            Some(KeyCode::KEY_F12)
        );
    }

    #[test]
    fn labels_are_compact() {
        rust_i18n::set_locale("fr");
        assert_eq!(keycode_label(KeyCode::KEY_A), "A");
        assert_eq!(keycode_label(KeyCode::KEY_F5), "F5");
        assert_eq!(
            keycode_label(KeyCode::KEY_ENTER),
            t!("keys.enter").to_string()
        );
    }
}
