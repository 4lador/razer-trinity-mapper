use std::fmt;

use crate::key::KeyCode;

/// Modifier key family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModifierKind {
    Ctrl,
    Shift,
    Alt,
    Logo,
}

/// Physical modifier key.
///
/// Variant order defines the canonical press order
/// (Ctrl, then Shift, then Alt, then Logo; left before right).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Modifier {
    LeftCtrl,
    RightCtrl,
    LeftShift,
    RightShift,
    LeftAlt,
    RightAlt,
    LeftMeta,
    RightMeta,
}

impl Modifier {
    pub const ALL: [Modifier; 8] = [
        Modifier::LeftCtrl,
        Modifier::RightCtrl,
        Modifier::LeftShift,
        Modifier::RightShift,
        Modifier::LeftAlt,
        Modifier::RightAlt,
        Modifier::LeftMeta,
        Modifier::RightMeta,
    ];

    pub const fn kind(self) -> ModifierKind {
        match self {
            Modifier::LeftCtrl | Modifier::RightCtrl => ModifierKind::Ctrl,
            Modifier::LeftShift | Modifier::RightShift => ModifierKind::Shift,
            Modifier::LeftAlt | Modifier::RightAlt => ModifierKind::Alt,
            Modifier::LeftMeta | Modifier::RightMeta => ModifierKind::Logo,
        }
    }

    pub const fn key_code(self) -> KeyCode {
        match self {
            Modifier::LeftCtrl => KeyCode::KEY_LEFTCTRL,
            Modifier::RightCtrl => KeyCode::KEY_RIGHTCTRL,
            Modifier::LeftShift => KeyCode::KEY_LEFTSHIFT,
            Modifier::RightShift => KeyCode::KEY_RIGHTSHIFT,
            Modifier::LeftAlt => KeyCode::KEY_LEFTALT,
            Modifier::RightAlt => KeyCode::KEY_RIGHTALT,
            Modifier::LeftMeta => KeyCode::KEY_LEFTMETA,
            Modifier::RightMeta => KeyCode::KEY_RIGHTMETA,
        }
    }

    pub fn from_key_code(code: KeyCode) -> Option<Self> {
        Modifier::ALL
            .iter()
            .copied()
            .find(|modifier| modifier.key_code() == code)
    }
}

impl fmt::Display for Modifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Modifier::LeftCtrl => "left-ctrl",
            Modifier::RightCtrl => "right-ctrl",
            Modifier::LeftShift => "left-shift",
            Modifier::RightShift => "right-shift",
            Modifier::LeftAlt => "left-alt",
            Modifier::RightAlt => "right-alt",
            Modifier::LeftMeta => "left-meta",
            Modifier::RightMeta => "right-meta",
        };
        f.write_str(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_key_code_roundtrips_every_modifier() {
        for modifier in Modifier::ALL {
            assert_eq!(Modifier::from_key_code(modifier.key_code()), Some(modifier));
        }
    }

    #[test]
    fn from_key_code_rejects_non_modifier() {
        assert_eq!(Modifier::from_key_code(KeyCode::KEY_A), None);
    }

    #[test]
    fn kinds_are_mapped() {
        assert_eq!(Modifier::LeftCtrl.kind(), ModifierKind::Ctrl);
        assert_eq!(Modifier::RightCtrl.kind(), ModifierKind::Ctrl);
        assert_eq!(Modifier::RightShift.kind(), ModifierKind::Shift);
        assert_eq!(Modifier::LeftAlt.kind(), ModifierKind::Alt);
        assert_eq!(Modifier::RightMeta.kind(), ModifierKind::Logo);
    }

    #[test]
    fn canonical_order_puts_ctrl_first() {
        let mut modifiers = vec![Modifier::LeftShift, Modifier::LeftCtrl];
        modifiers.sort_unstable();
        assert_eq!(modifiers, vec![Modifier::LeftCtrl, Modifier::LeftShift]);
    }
}
