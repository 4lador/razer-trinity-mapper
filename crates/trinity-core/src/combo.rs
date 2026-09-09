use std::fmt;

use crate::error::DomainError;
use crate::key::KeyCode;
use crate::modifier::Modifier;

/// Key and modifiers combination (e.g. `Ctrl+1`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCombination {
    modifiers: Vec<Modifier>,
    key: KeyCode,
}

impl KeyCombination {
    /// Modifiers are normalized into canonical press order.
    pub fn new(modifiers: Vec<Modifier>, key: KeyCode) -> Result<Self, DomainError> {
        let mut modifiers = modifiers;
        modifiers.sort_unstable();
        let before = modifiers.len();
        modifiers.dedup();
        if modifiers.len() != before {
            return Err(DomainError::DuplicateModifier);
        }
        Ok(Self { modifiers, key })
    }

    pub const fn plain(key: KeyCode) -> Self {
        Self {
            modifiers: Vec::new(),
            key,
        }
    }

    pub fn modifiers(&self) -> &[Modifier] {
        &self.modifiers
    }

    pub const fn key(&self) -> KeyCode {
        self.key
    }

    pub fn contains_modifier(&self, modifier: Modifier) -> bool {
        self.modifiers.contains(&modifier)
    }

    /// Injection sequence: modifiers (canonical order), then the key.
    pub fn press_sequence(&self) -> Vec<KeyCode> {
        self.modifiers
            .iter()
            .map(|modifier| modifier.key_code())
            .chain(std::iter::once(self.key))
            .collect()
    }
}

impl fmt::Display for KeyCombination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for modifier in &self.modifiers {
            write!(f, "{modifier}+")?;
        }
        write!(f, "{}", self.key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_modifiers_canonically() {
        let combo = KeyCombination::new(
            vec![Modifier::LeftShift, Modifier::RightCtrl],
            KeyCode::KEY_1,
        )
        .unwrap();
        assert_eq!(
            combo.modifiers(),
            &[Modifier::RightCtrl, Modifier::LeftShift]
        );
    }

    #[test]
    fn rejects_duplicate_modifier() {
        let err = KeyCombination::new(vec![Modifier::LeftCtrl, Modifier::LeftCtrl], KeyCode::KEY_A)
            .unwrap_err();
        assert_eq!(err, DomainError::DuplicateModifier);
    }

    #[test]
    fn press_sequence_ends_with_the_key() {
        let combo = KeyCombination::new(
            vec![Modifier::LeftShift, Modifier::LeftCtrl],
            KeyCode::KEY_1,
        )
        .unwrap();
        assert_eq!(
            combo.press_sequence(),
            vec![
                KeyCode::KEY_LEFTCTRL,
                KeyCode::KEY_LEFTSHIFT,
                KeyCode::KEY_1
            ]
        );
    }

    #[test]
    fn plain_combo_has_no_modifier() {
        let combo = KeyCombination::plain(KeyCode::KEY_F);
        assert!(combo.modifiers().is_empty());
        assert_eq!(combo.press_sequence(), vec![KeyCode::KEY_F]);
        assert_eq!(combo.to_string(), "KEY_F");
    }

    #[test]
    fn display_shows_every_modifier() {
        let combo = KeyCombination::new(vec![Modifier::LeftCtrl], KeyCode::KEY_1).unwrap();
        assert_eq!(combo.to_string(), "left-ctrl+KEY_1");
    }
}
