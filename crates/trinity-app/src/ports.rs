use std::collections::BTreeSet;

use trinity_core::calibration::Calibration;
use trinity_core::key::KeyCode;
use trinity_core::node::NodeId;
use trinity_core::profile::Profile;

use crate::error::AppError;
use crate::event::{Action, InputEvent};

/// Event source: input node grab and blocking batch reads.
pub trait InputEventSource {
    fn grab(&mut self, nodes: &[NodeId]) -> Result<(), AppError>;
    fn release(&mut self) -> Result<(), AppError>;
    fn read_events(&mut self) -> Result<Vec<InputEvent>, AppError>;
    fn grabbed(&self) -> &[NodeId];
}

/// Output sink: virtual uinput mirror device.
pub trait OutputEventSink {
    fn create(&mut self, name: &str, caps: &Caps) -> Result<(), AppError>;
    fn emit(&mut self, actions: &[Action]) -> Result<(), AppError>;
    fn destroy(&mut self) -> Result<(), AppError>;
}

/// Capabilities of the mirror device to create.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Caps {
    keys: BTreeSet<u16>,
    rels: BTreeSet<u16>,
    abss: BTreeSet<u16>,
    mscs: BTreeSet<u16>,
}

impl Caps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_key(&mut self, key: KeyCode) -> &mut Self {
        self.keys.insert(key.code());
        self
    }

    pub fn add_key_code(&mut self, code: u16) -> &mut Self {
        self.keys.insert(code);
        self
    }

    pub fn add_rel(&mut self, code: u16) -> &mut Self {
        self.rels.insert(code);
        self
    }

    pub fn add_abs(&mut self, code: u16) -> &mut Self {
        self.abss.insert(code);
        self
    }

    pub fn add_msc(&mut self, code: u16) -> &mut Self {
        self.mscs.insert(code);
        self
    }

    pub fn keys(&self) -> &BTreeSet<u16> {
        &self.keys
    }

    pub fn rels(&self) -> &BTreeSet<u16> {
        &self.rels
    }

    pub fn abss(&self) -> &BTreeSet<u16> {
        &self.abss
    }

    pub fn mscs(&self) -> &BTreeSet<u16> {
        &self.mscs
    }

    pub fn merge(&mut self, other: &Caps) {
        self.keys.extend(other.keys.iter().copied());
        self.rels.extend(other.rels.iter().copied());
        self.abss.extend(other.abss.iter().copied());
        self.mscs.extend(other.mscs.iter().copied());
    }
}

/// Profile persistence.
pub trait ProfileRepository {
    fn list(&self) -> Result<Vec<String>, AppError>;
    fn load(&self, name: &str) -> Result<Profile, AppError>;
    fn save(&mut self, profile: &Profile) -> Result<(), AppError>;
    fn delete(&mut self, name: &str) -> Result<(), AppError>;
}

/// Calibration persistence.
pub trait CalibrationRepository {
    fn load(&self) -> Result<Option<Calibration>, AppError>;
    fn save(&mut self, calibration: &Calibration) -> Result<(), AppError>;
}

/// Locates the Trinity input nodes.
pub trait DeviceLocator {
    fn locate(&self) -> Result<Vec<NodeId>, AppError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_accumulate_and_merge() {
        let mut caps = Caps::new();
        caps.add_key(KeyCode::KEY_A)
            .add_key(KeyCode::KEY_B)
            .add_rel(1);
        assert_eq!(caps.keys().len(), 2);
        assert_eq!(caps.rels().len(), 1);

        let mut other = Caps::new();
        other.add_key_code(999).add_abs(3);
        caps.merge(&other);
        assert!(caps.keys().contains(&999));
        assert!(caps.abss().contains(&3));
        assert_eq!(caps.keys().len(), 3);
    }
}
