//! Active keyboard layout: character to physical code.
//!
//! evdev codes are positional (QWERTY-referenced) while the GUI captures
//! characters produced by the user's layout (AZERTY, ...). This module
//! loads the system keymap via xkbcommon and provides the mapping table,
//! silently falling back to an empty table (the GUI then uses the QWERTY
//! table from `keys`).

use std::collections::HashMap;

use trinity_core::key::KeyCode;
use xkbcommon::xkb;

/// Offset between xkb keycodes and evdev codes.
const XKB_OFFSET: u32 = 8;

/// Levels explored per key: 0 = unshifted, 1 = shifted.
const LEVELS: u32 = 2;

#[derive(Debug, Clone, Default)]
pub struct KeyboardLayout {
    chars: HashMap<char, u16>,
    labels: HashMap<u16, char>,
}

impl KeyboardLayout {
    /// Loads the system default layout (`XKB_*` variables set by the
    /// session under Wayland).
    pub fn load() -> Option<Self> {
        let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let keymap =
            xkb::Keymap::new_from_names(&context, "", "", "", "", None, xkb::COMPILE_NO_FLAGS)?;
        Some(Self::from_keymap(&keymap))
    }

    fn from_keymap(keymap: &xkb::Keymap) -> Self {
        let mut chars: HashMap<char, u16> = HashMap::new();
        let mut labels: HashMap<u16, char> = HashMap::new();
        let min = keymap.min_keycode().raw();
        let max = keymap.max_keycode().raw();
        for raw in min..=max {
            let Some(offset) = raw.checked_sub(XKB_OFFSET) else {
                continue;
            };
            let Ok(evdev) = u16::try_from(offset) else {
                continue;
            };
            let keycode = xkb::Keycode::new(raw);
            for level in 0..LEVELS {
                for &sym in keymap.key_get_syms_by_level(keycode, 0, level) {
                    let Some(character) = char::from_u32(xkb::keysym_to_utf32(sym)) else {
                        continue;
                    };
                    if character.is_control() || character.is_whitespace() {
                        continue;
                    }
                    // Level 0 is visited first: it stays the priority.
                    chars.entry(character).or_insert(evdev);
                    labels.entry(evdev).or_insert(character);
                    break;
                }
            }
        }
        Self { chars, labels }
    }

    /// Physical code producing this character (unshifted level first).
    pub fn code_for_char(&self, character: char) -> Option<KeyCode> {
        self.chars
            .get(&character)
            .map(|&code| KeyCode::from_code(code))
    }

    /// Character produced by this code on the layout (unshifted level).
    pub fn label_for(&self, code: KeyCode) -> Option<char> {
        self.labels.get(&code.code()).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(name: &str) -> KeyboardLayout {
        let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let keymap =
            xkb::Keymap::new_from_names(&context, "", "", name, "", None, xkb::COMPILE_NO_FLAGS)
                .unwrap();
        KeyboardLayout::from_keymap(&keymap)
    }

    #[test]
    fn fr_letters_map_to_physical_positions() {
        let fr = layout("fr");
        assert_eq!(fr.code_for_char('a'), Some(KeyCode::KEY_Q));
        assert_eq!(fr.code_for_char('q'), Some(KeyCode::KEY_A));
        assert_eq!(fr.code_for_char('z'), Some(KeyCode::KEY_W));
        assert_eq!(fr.code_for_char('w'), Some(KeyCode::KEY_Z));
        assert_eq!(fr.code_for_char('m'), Some(KeyCode::KEY_SEMICOLON));
        assert_eq!(fr.code_for_char(','), Some(KeyCode::KEY_M));
    }

    #[test]
    fn fr_digit_row_maps_unshifted_characters() {
        let fr = layout("fr");
        assert_eq!(fr.code_for_char('&'), Some(KeyCode::KEY_1));
        assert_eq!(fr.code_for_char('\u{e9}'), Some(KeyCode::KEY_2));
        assert_eq!(fr.code_for_char('\u{e7}'), Some(KeyCode::KEY_9));
        assert_eq!(fr.code_for_char('\u{e0}'), Some(KeyCode::KEY_0));
        assert_eq!(fr.code_for_char('²'), Some(KeyCode::KEY_GRAVE));
    }

    #[test]
    fn shifted_characters_map_to_their_physical_key() {
        let fr = layout("fr");
        assert_eq!(fr.code_for_char('1'), Some(KeyCode::KEY_1));
        assert_eq!(fr.code_for_char('A'), Some(KeyCode::KEY_Q));
    }

    #[test]
    fn us_layout_is_identity_for_letters() {
        let us = layout("us");
        assert_eq!(us.code_for_char('a'), Some(KeyCode::KEY_A));
        assert_eq!(us.code_for_char('q'), Some(KeyCode::KEY_Q));
    }

    #[test]
    fn labels_show_layout_characters() {
        let fr = layout("fr");
        assert_eq!(fr.label_for(KeyCode::KEY_Q), Some('a'));
        assert_eq!(fr.label_for(KeyCode::KEY_GRAVE), Some('²'));
        let us = layout("us");
        assert_eq!(us.label_for(KeyCode::KEY_Q), Some('q'));
    }
}
