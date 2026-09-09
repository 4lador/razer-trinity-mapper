use std::collections::HashMap;

use crate::button::Button;
use crate::calibration::{Calibration, PhysicalCode};
use crate::combo::KeyCombination;
use crate::error::DomainError;
use crate::node::NodeId;
use crate::profile::Profile;

/// Runtime translation table: physical code to mapping.
///
/// Only contains buttons actually mapped by the profile; every other
/// event is forwarded as-is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TranslationTable {
    entries: HashMap<PhysicalCode, (Button, KeyCombination)>,
}

impl TranslationTable {
    pub fn build(calibration: &Calibration, profile: &Profile) -> Result<Self, DomainError> {
        if !calibration.is_complete() {
            return Err(DomainError::IncompleteCalibration {
                missing: calibration
                    .missing_buttons()
                    .iter()
                    .map(|button| button.number())
                    .collect(),
            });
        }
        let entries = calibration
            .buttons()
            .filter_map(|(button, code)| {
                profile
                    .mapping(button)
                    .map(|combo| (code.clone(), (button, combo.clone())))
            })
            .collect();
        Ok(Self { entries })
    }

    pub fn lookup(&self, node: &NodeId, code: u16) -> Option<(&Button, &KeyCombination)> {
        self.entries
            .get(&PhysicalCode::new(node.clone(), code))
            .map(|(button, combo)| (button, combo))
    }

    pub fn buttons(&self) -> impl Iterator<Item = (Button, &KeyCombination)> {
        self.entries
            .values()
            .map(|(button, combo)| (*button, combo))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::KeyCode;
    use crate::modifier::Modifier;

    fn node() -> NodeId {
        NodeId::new("usb-Razer_Razer_Naga_Trinity-event-if01").unwrap()
    }

    fn full_calibration() -> Calibration {
        let mut calibration = Calibration::new();
        for button in Button::ALL {
            calibration
                .record(
                    button,
                    PhysicalCode::new(node(), 100 + u16::from(button.number())),
                )
                .unwrap();
        }
        calibration
    }

    #[test]
    fn build_requires_complete_calibration() {
        let mut calibration = full_calibration();
        calibration
            .record(Button::new(6).unwrap(), PhysicalCode::new(node(), 999))
            .unwrap();
        let mut partial = Calibration::new();
        for button in Button::ALL.iter().filter(|b| b.number() != 6) {
            partial
                .record(*button, calibration.get(*button).unwrap().clone())
                .unwrap();
        }
        let profile = Profile::new("test").unwrap();
        let err = TranslationTable::build(&partial, &profile).unwrap_err();
        assert_eq!(err, DomainError::IncompleteCalibration { missing: vec![6] });
    }

    #[test]
    fn only_mapped_buttons_are_translated() {
        let calibration = full_calibration();
        let mut profile = Profile::new("test").unwrap();
        profile.set_mapping(
            Button::new(3).unwrap(),
            KeyCombination::new(vec![Modifier::LeftCtrl], KeyCode::KEY_1).unwrap(),
        );
        let table = TranslationTable::build(&calibration, &profile).unwrap();

        assert!(table.lookup(&node(), 103).is_some());
        assert!(table.lookup(&node(), 107).is_none());
        assert!(table.lookup(&node(), 42).is_none());
        assert_eq!(table.len(), 1);
    }
}
