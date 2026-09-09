use std::collections::BTreeMap;

use crate::button::Button;
use crate::error::DomainError;
use crate::node::NodeId;

/// Physical code emitted by the firmware on a given node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PhysicalCode {
    node: NodeId,
    code: u16,
}

impl PhysicalCode {
    pub fn new(node: NodeId, code: u16) -> Self {
        Self { node, code }
    }

    pub fn node(&self) -> &NodeId {
        &self.node
    }

    pub const fn code(&self) -> u16 {
        self.code
    }
}

/// Button to physical code table, produced by guided calibration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Calibration {
    entries: BTreeMap<Button, PhysicalCode>,
}

impl Calibration {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the physical code of a button. Replacing the code of the
    /// same button is allowed; sharing a code between buttons is not.
    pub fn record(&mut self, button: Button, code: PhysicalCode) -> Result<(), DomainError> {
        if let Some((&owner, _)) = self.entries.iter().find(|(_, existing)| **existing == code) {
            if owner != button {
                return Err(DomainError::PhysicalCodeConflict {
                    node: code.node.to_string(),
                    code: code.code,
                    first: owner.number(),
                    second: button.number(),
                });
            }
        }
        self.entries.insert(button, code);
        Ok(())
    }

    pub fn get(&self, button: Button) -> Option<&PhysicalCode> {
        self.entries.get(&button)
    }

    pub fn is_complete(&self) -> bool {
        self.entries.len() == Button::ALL.len()
    }

    pub fn missing_buttons(&self) -> Vec<Button> {
        Button::ALL
            .iter()
            .copied()
            .filter(|button| !self.entries.contains_key(button))
            .collect()
    }

    pub fn buttons(&self) -> impl Iterator<Item = (Button, &PhysicalCode)> {
        self.entries.iter().map(|(button, code)| (*button, code))
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

    fn node() -> NodeId {
        NodeId::new("usb-Razer_Razer_Naga_Trinity-event-if01").unwrap()
    }

    fn code(value: u16) -> PhysicalCode {
        PhysicalCode::new(node(), value)
    }

    #[test]
    fn record_and_get() {
        let mut calibration = Calibration::new();
        let button = Button::new(4).unwrap();
        calibration.record(button, code(104)).unwrap();
        assert_eq!(calibration.get(button), Some(&code(104)));
        assert_eq!(calibration.len(), 1);
    }

    #[test]
    fn code_shared_between_buttons_is_rejected() {
        let mut calibration = Calibration::new();
        calibration
            .record(Button::new(1).unwrap(), code(42))
            .unwrap();
        let err = calibration
            .record(Button::new(2).unwrap(), code(42))
            .unwrap_err();
        assert_eq!(
            err,
            DomainError::PhysicalCodeConflict {
                node: node().to_string(),
                code: 42,
                first: 1,
                second: 2,
            }
        );
    }

    #[test]
    fn same_button_can_be_recaptured() {
        let mut calibration = Calibration::new();
        let button = Button::new(1).unwrap();
        calibration.record(button, code(42)).unwrap();
        calibration.record(button, code(43)).unwrap();
        assert_eq!(calibration.get(button), Some(&code(43)));
        assert_eq!(calibration.len(), 1);
    }

    #[test]
    fn completeness_tracks_all_twelve_buttons() {
        let mut calibration = Calibration::new();
        assert!(!calibration.is_complete());
        for button in Button::ALL {
            calibration
                .record(button, code(200 + u16::from(button.number())))
                .unwrap();
        }
        assert!(calibration.is_complete());
        assert!(calibration.missing_buttons().is_empty());
    }

    #[test]
    fn missing_buttons_are_reported() {
        let mut calibration = Calibration::new();
        calibration
            .record(Button::new(1).unwrap(), code(201))
            .unwrap();
        calibration
            .record(Button::new(3).unwrap(), code(203))
            .unwrap();
        let missing: Vec<u8> = calibration
            .missing_buttons()
            .iter()
            .map(|b| b.number())
            .collect();
        assert_eq!(missing, vec![2, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    }
}
