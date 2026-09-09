use std::fmt;

use crate::error::DomainError;

/// Linux evdev key code (`input-event-codes.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyCode(u16);

impl KeyCode {
    pub const fn from_code(code: u16) -> Self {
        Self(code)
    }

    pub const fn code(self) -> u16 {
        self.0
    }

    pub fn from_name(name: &str) -> Result<Self, DomainError> {
        KNOWN_KEYS
            .iter()
            .find(|(known, _)| *known == name)
            .map(|&(_, code)| Self(code))
            .ok_or_else(|| DomainError::UnknownKeyName(name.to_owned()))
    }

    pub fn name(self) -> Option<&'static str> {
        KNOWN_KEYS
            .iter()
            .find(|&&(_, code)| code == self.0)
            .map(|&(name, _)| name)
    }

    /// Canonical label, falls back to `KEY_0x…` for unknown codes.
    pub fn label(self) -> String {
        match self.name() {
            Some(name) => name.to_owned(),
            None => format!("KEY_{:#06x}", self.0),
        }
    }

    pub fn known() -> &'static [(&'static str, u16)] {
        KNOWN_KEYS
    }
}

impl fmt::Display for KeyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label())
    }
}

macro_rules! define_keycodes {
    ($($name:ident = $code:expr;)*) => {
        impl KeyCode {
            $(pub const $name: KeyCode = KeyCode($code);)*
        }
        const KNOWN_KEYS: &[(&str, u16)] = &[$((stringify!($name), $code),)*];
    };
}

define_keycodes! {
    KEY_ESC = 1;
    KEY_1 = 2; KEY_2 = 3; KEY_3 = 4; KEY_4 = 5; KEY_5 = 6;
    KEY_6 = 7; KEY_7 = 8; KEY_8 = 9; KEY_9 = 10; KEY_0 = 11;
    KEY_MINUS = 12; KEY_EQUAL = 13; KEY_BACKSPACE = 14; KEY_TAB = 15;
    KEY_Q = 16; KEY_W = 17; KEY_E = 18; KEY_R = 19; KEY_T = 20;
    KEY_Y = 21; KEY_U = 22; KEY_I = 23; KEY_O = 24; KEY_P = 25;
    KEY_LEFTBRACE = 26; KEY_RIGHTBRACE = 27; KEY_ENTER = 28; KEY_LEFTCTRL = 29;
    KEY_A = 30; KEY_S = 31; KEY_D = 32; KEY_F = 33; KEY_G = 34;
    KEY_H = 35; KEY_J = 36; KEY_K = 37; KEY_L = 38;
    KEY_SEMICOLON = 39; KEY_APOSTROPHE = 40; KEY_GRAVE = 41;
    KEY_LEFTSHIFT = 42; KEY_BACKSLASH = 43;
    KEY_Z = 44; KEY_X = 45; KEY_C = 46; KEY_V = 47; KEY_B = 48; KEY_N = 49; KEY_M = 50;
    KEY_COMMA = 51; KEY_DOT = 52; KEY_SLASH = 53;
    KEY_RIGHTSHIFT = 54; KEY_KPASTERISK = 55; KEY_LEFTALT = 56; KEY_SPACE = 57; KEY_CAPSLOCK = 58;
    KEY_F1 = 59; KEY_F2 = 60; KEY_F3 = 61; KEY_F4 = 62; KEY_F5 = 63;
    KEY_F6 = 64; KEY_F7 = 65; KEY_F8 = 66; KEY_F9 = 67; KEY_F10 = 68;
    KEY_NUMLOCK = 69; KEY_SCROLLLOCK = 70;
    KEY_KP7 = 71; KEY_KP8 = 72; KEY_KP9 = 73; KEY_KPMINUS = 74;
    KEY_KP4 = 75; KEY_KP5 = 76; KEY_KP6 = 77; KEY_KPPLUS = 78;
    KEY_KP1 = 79; KEY_KP2 = 80; KEY_KP3 = 81; KEY_KP0 = 82; KEY_KPDOT = 83;
    KEY_102ND = 86; KEY_F11 = 87; KEY_F12 = 88;
    KEY_KPENTER = 96; KEY_RIGHTCTRL = 97; KEY_KPSLASH = 98; KEY_SYSRQ = 99;
    KEY_RIGHTALT = 100; KEY_LINEFEED = 101;
    KEY_HOME = 102; KEY_UP = 103; KEY_PAGEUP = 104;
    KEY_LEFT = 105; KEY_RIGHT = 106; KEY_END = 107; KEY_DOWN = 108;
    KEY_PAGEDOWN = 109; KEY_INSERT = 110; KEY_DELETE = 111;
    KEY_KPCOMMA = 121; KEY_LEFTMETA = 125; KEY_RIGHTMETA = 126; KEY_COMPOSE = 127;
    KEY_F13 = 183; KEY_F14 = 184; KEY_F15 = 185; KEY_F16 = 186;
    KEY_F17 = 187; KEY_F18 = 188; KEY_F19 = 189; KEY_F20 = 190;
    KEY_F21 = 191; KEY_F22 = 192; KEY_F23 = 193; KEY_F24 = 194;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spot_checks_reference_values() {
        assert_eq!(KeyCode::KEY_ESC.code(), 1);
        assert_eq!(KeyCode::KEY_1.code(), 2);
        assert_eq!(KeyCode::KEY_ENTER.code(), 28);
        assert_eq!(KeyCode::KEY_A.code(), 30);
        assert_eq!(KeyCode::KEY_F.code(), 33);
        assert_eq!(KeyCode::KEY_LEFTCTRL.code(), 29);
        assert_eq!(KeyCode::KEY_F12.code(), 88);
        assert_eq!(KeyCode::KEY_KPPLUS.code(), 78);
        assert_eq!(KeyCode::KEY_LEFTMETA.code(), 125);
        assert_eq!(KeyCode::KEY_F24.code(), 194);
    }

    #[test]
    fn name_lookup_roundtrip() {
        for &(name, code) in KeyCode::known() {
            let key = KeyCode::from_name(name).unwrap();
            assert_eq!(key.code(), code);
            assert_eq!(KeyCode::from_code(code).name(), Some(name));
        }
    }

    #[test]
    fn unknown_name_is_rejected() {
        assert_eq!(
            KeyCode::from_name("KEY_NOPE").unwrap_err(),
            DomainError::UnknownKeyName("KEY_NOPE".to_owned())
        );
    }

    #[test]
    fn label_falls_back_to_hex_code() {
        assert_eq!(KeyCode::from_code(0x270f).label(), "KEY_0x270f");
    }

    #[test]
    fn known_table_has_no_duplicate() {
        let mut codes: Vec<u16> = KeyCode::known().iter().map(|&(_, c)| c).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), KeyCode::known().len());
    }
}
