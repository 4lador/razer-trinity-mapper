//! Test doubles for the ports.

use std::collections::{BTreeMap, VecDeque};

use trinity_core::calibration::Calibration;
use trinity_core::node::NodeId;
use trinity_core::profile::Profile;

use crate::error::AppError;
use crate::event::{Action, InputEvent};
use crate::ports::{
    CalibrationRepository, Caps, DeviceLocator, InputEventSource, OutputEventSink,
    ProfileRepository,
};

/// Scriptable source: yields pushed batches, then empty.
#[derive(Debug, Clone, Default)]
pub struct FakeSource {
    batches: VecDeque<Vec<InputEvent>>,
    grabbed: Vec<NodeId>,
    release_count: usize,
}

impl FakeSource {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_batch(&mut self, events: Vec<InputEvent>) {
        self.batches.push_back(events);
    }

    pub fn release_count(&self) -> usize {
        self.release_count
    }
}

impl InputEventSource for FakeSource {
    fn grab(&mut self, nodes: &[NodeId]) -> Result<(), AppError> {
        self.grabbed = nodes.to_vec();
        Ok(())
    }

    fn release(&mut self) -> Result<(), AppError> {
        self.release_count += 1;
        self.grabbed.clear();
        Ok(())
    }

    fn read_events(&mut self) -> Result<Vec<InputEvent>, AppError> {
        Ok(self.batches.pop_front().unwrap_or_default())
    }

    fn grabbed(&self) -> &[NodeId] {
        &self.grabbed
    }
}

/// Recording sink: stores everything emitted to it.
#[derive(Debug, Clone, Default)]
pub struct FakeSink {
    device: Option<String>,
    caps: Option<Caps>,
    log: Vec<Action>,
    destroy_count: usize,
}

impl FakeSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn device(&self) -> Option<&str> {
        self.device.as_deref()
    }

    pub fn was_created(&self) -> bool {
        self.device.is_some()
    }

    pub fn caps(&self) -> Option<&Caps> {
        self.caps.as_ref()
    }

    pub fn emitted(&self) -> &[Action] {
        &self.log
    }

    pub fn destroy_count(&self) -> usize {
        self.destroy_count
    }
}

impl OutputEventSink for FakeSink {
    fn create(&mut self, name: &str, caps: &Caps) -> Result<(), AppError> {
        self.device = Some(name.to_owned());
        self.caps = Some(caps.clone());
        self.log.clear();
        Ok(())
    }

    fn emit(&mut self, actions: &[Action]) -> Result<(), AppError> {
        self.log.extend_from_slice(actions);
        Ok(())
    }

    fn destroy(&mut self) -> Result<(), AppError> {
        self.destroy_count += 1;
        self.device = None;
        self.caps = None;
        Ok(())
    }
}

/// In-memory profile repository.
#[derive(Debug, Clone, Default)]
pub struct MemoryProfileRepository {
    profiles: BTreeMap<String, Profile>,
}

impl MemoryProfileRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ProfileRepository for MemoryProfileRepository {
    fn list(&self) -> Result<Vec<String>, AppError> {
        Ok(self.profiles.keys().cloned().collect())
    }

    fn load(&self, name: &str) -> Result<Profile, AppError> {
        self.profiles
            .get(name)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("profile '{name}'")))
    }

    fn save(&mut self, profile: &Profile) -> Result<(), AppError> {
        self.profiles
            .insert(profile.name().to_owned(), profile.clone());
        Ok(())
    }

    fn delete(&mut self, name: &str) -> Result<(), AppError> {
        self.profiles
            .remove(name)
            .ok_or_else(|| AppError::NotFound(format!("profile '{name}'")))?;
        Ok(())
    }
}

/// In-memory calibration repository.
#[derive(Debug, Clone, Default)]
pub struct MemoryCalibrationRepository {
    calibration: Option<Calibration>,
}

impl MemoryCalibrationRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CalibrationRepository for MemoryCalibrationRepository {
    fn load(&self) -> Result<Option<Calibration>, AppError> {
        Ok(self.calibration.clone())
    }

    fn save(&mut self, calibration: &Calibration) -> Result<(), AppError> {
        self.calibration = Some(calibration.clone());
        Ok(())
    }
}

/// Static locator.
#[derive(Debug, Clone, Default)]
pub struct StaticLocator {
    pub nodes: Vec<NodeId>,
}

impl StaticLocator {
    pub fn new(nodes: Vec<NodeId>) -> Self {
        Self { nodes }
    }
}

impl DeviceLocator for StaticLocator {
    fn locate(&self) -> Result<Vec<NodeId>, AppError> {
        Ok(self.nodes.clone())
    }
}
