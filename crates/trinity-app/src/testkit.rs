use trinity_core::button::Button;
use trinity_core::calibration::{Calibration, PhysicalCode};
use trinity_core::combo::KeyCombination;
use trinity_core::key::KeyCode;
use trinity_core::modifier::Modifier;
use trinity_core::node::NodeId;
use trinity_core::profile::Profile;

pub fn node() -> NodeId {
    NodeId::new("usb-Razer_Razer_Naga_Trinity-event-if01").unwrap()
}

/// Complete calibration: button N maps to code 100+N on the fixture node.
pub fn full_calibration() -> Calibration {
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

/// Test profile built from (button, modifiers, key) mappings.
pub fn profile_fixture(mappings: &[(u8, Vec<Modifier>, KeyCode)]) -> Profile {
    let mut profile = Profile::new("fixture").unwrap();
    for (number, modifiers, key) in mappings {
        let combination = KeyCombination::new(modifiers.clone(), *key).unwrap();
        profile.set_mapping(Button::new(*number).unwrap(), combination);
    }
    profile
}
