//! Engine control use case: start/stop, suspend/resume, profile switching.

use trinity_core::calibration::Calibration;
use trinity_core::node::NodeId;
use trinity_core::profile::Profile;

use crate::engine::Engine;
use crate::error::AppError;
use crate::ports::{Caps, InputEventSource, OutputEventSink};

/// High-level engine operations — thin facade over `Engine` that the
/// daemon delegates to. Keeps IPC handlers free of business logic.
pub struct EngineUseCase<S: InputEventSource, O: OutputEventSink> {
    pub engine: Engine<S, O>,
    suspended: bool,
}

impl<S: InputEventSource, O: OutputEventSink> EngineUseCase<S, O> {
    pub fn new(engine: Engine<S, O>) -> Self {
        Self {
            engine,
            suspended: false,
        }
    }

    pub fn is_active(&self) -> bool {
        self.engine.is_active()
    }

    pub fn is_suspended(&self) -> bool {
        self.suspended
    }

    pub fn is_calibrating(&self) -> bool {
        self.engine.is_calibrating()
    }

    /// Starts or resumes the engine. Never tears down devices.
    pub fn enable(&mut self, nodes: &[NodeId], caps: &Caps) -> Result<(), AppError> {
        if !self.engine.is_active() {
            self.engine.start(nodes, caps)?;
        } else if self.suspended {
            self.engine.resume_translation()?;
        }
        self.suspended = false;
        Ok(())
    }

    /// Suspends translation (full passthrough). Does NOT stop the engine.
    pub fn disable(&mut self) -> Result<(), AppError> {
        if self.engine.is_active() {
            self.engine.suspend_translation()?;
            self.suspended = true;
        }
        Ok(())
    }

    pub fn set_profile(&mut self, profile: Profile) -> Result<(), AppError> {
        self.engine.set_profile(profile)
    }

    pub fn set_calibration(&mut self, calibration: Calibration) -> Result<(), AppError> {
        self.engine.set_calibration(calibration)
    }

    pub fn run_once(&mut self) -> Result<usize, AppError> {
        self.engine.run_once()
    }

    pub fn begin_calibration(&mut self) -> Result<(), AppError> {
        self.engine.begin_calibration()
    }

    pub fn cancel_calibration(&mut self) {
        self.engine.cancel_calibration();
    }

    pub fn finish_calibration(&mut self) -> Result<Calibration, AppError> {
        self.engine.finish_calibration()
    }

    pub fn calibration_progress(&self) -> Option<(Option<u8>, usize)> {
        self.engine.calibration_progress()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeSink, FakeSource};
    use crate::testkit::{full_calibration, node, profile_fixture};

    use trinity_core::key::KeyCode;
    use trinity_core::modifier::Modifier;

    fn engine_use_case() -> EngineUseCase<FakeSource, FakeSink> {
        let mut source = FakeSource::new();
        source.push_batch(vec![]);
        let mut engine = Engine::new(source, FakeSink::new());
        engine.start(&[node()], &Caps::new()).unwrap();
        engine.set_calibration(full_calibration()).unwrap();
        engine
            .set_profile(profile_fixture(&[(
                1,
                vec![Modifier::LeftCtrl],
                KeyCode::KEY_1,
            )]))
            .unwrap();
        EngineUseCase::new(engine)
    }

    #[test]
    fn enable_when_active_is_noop() {
        let mut uc = engine_use_case();
        assert!(uc.is_active());
        assert!(!uc.is_suspended());
        uc.enable(&[node()], &Caps::new()).unwrap();
        assert!(uc.is_active());
        assert!(!uc.is_suspended());
    }

    #[test]
    fn disable_suspends_without_stopping() {
        let mut uc = engine_use_case();
        uc.disable().unwrap();
        assert!(uc.is_active()); // Still active
        assert!(uc.is_suspended()); // But suspended
    }

    #[test]
    fn suspend_then_enable_resumes() {
        let mut uc = engine_use_case();
        uc.disable().unwrap();
        assert!(uc.is_suspended());
        uc.enable(&[node()], &Caps::new()).unwrap();
        assert!(!uc.is_suspended());
        assert!(uc.is_active());
    }
}
