use std::collections::BTreeMap;

use crate::button::Button;
use crate::combo::KeyCombination;
use crate::error::DomainError;

/// Remapping profile: a subset of the 12 mapped buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    name: String,
    mappings: BTreeMap<Button, KeyCombination>,
}

impl Profile {
    pub fn new(name: impl Into<String>) -> Result<Self, DomainError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DomainError::EmptyProfileName);
        }
        Ok(Self {
            name,
            mappings: BTreeMap::new(),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn mapping(&self, button: Button) -> Option<&KeyCombination> {
        self.mappings.get(&button)
    }

    pub fn set_mapping(&mut self, button: Button, combination: KeyCombination) {
        self.mappings.insert(button, combination);
    }

    pub fn remove_mapping(&mut self, button: Button) -> bool {
        self.mappings.remove(&button).is_some()
    }

    pub fn mappings(&self) -> impl Iterator<Item = (Button, &KeyCombination)> {
        self.mappings.iter().map(|(button, combo)| (*button, combo))
    }

    pub fn mapping_count(&self) -> usize {
        self.mappings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::KeyCode;
    use crate::modifier::Modifier;

    fn ctrl_1() -> KeyCombination {
        KeyCombination::new(vec![Modifier::LeftCtrl], KeyCode::KEY_1).unwrap()
    }

    #[test]
    fn rejects_empty_names() {
        assert_eq!(Profile::new("").unwrap_err(), DomainError::EmptyProfileName);
        assert_eq!(
            Profile::new(" \t ").unwrap_err(),
            DomainError::EmptyProfileName
        );
    }

    #[test]
    fn set_get_remove_mapping() {
        let mut profile = Profile::new("mmo").unwrap();
        let button = Button::new(3).unwrap();
        assert!(profile.mapping(button).is_none());
        profile.set_mapping(button, ctrl_1());
        assert_eq!(profile.mapping(button), Some(&ctrl_1()));
        assert_eq!(profile.mapping_count(), 1);
        assert!(profile.remove_mapping(button));
        assert!(!profile.remove_mapping(button));
        assert!(profile.is_empty());
    }

    #[test]
    fn mappings_iterate_in_button_order() {
        let mut profile = Profile::new("mmo").unwrap();
        profile.set_mapping(Button::new(7).unwrap(), ctrl_1());
        profile.set_mapping(
            Button::new(2).unwrap(),
            KeyCombination::plain(KeyCode::KEY_F),
        );
        let numbers: Vec<u8> = profile.mappings().map(|(b, _)| b.number()).collect();
        assert_eq!(numbers, vec![2, 7]);
    }
}
