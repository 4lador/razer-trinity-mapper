use std::collections::{HashSet, VecDeque};

use trinity_core::button::Button;
use trinity_core::calibration::{Calibration, PhysicalCode};
use trinity_core::error::DomainError;
use trinity_core::profile::Profile;

use crate::error::AppError;
use crate::event::{BTN_LEFT, BTN_MIDDLE, BTN_RIGHT, InputEvent};
use crate::ports::{CalibrationRepository, ProfileRepository};

/// Guided calibration session: buttons are captured in order 1 to 12;
/// every non-ignored key press captures the current button.
pub struct CalibrationSession {
    captured: Calibration,
    queue: VecDeque<Button>,
    ignored: HashSet<u16>,
}

impl CalibrationSession {
    pub fn new() -> Self {
        Self::with_extra_ignored(&[])
    }

    /// Additional codes to ignore (besides mouse buttons).
    pub fn with_extra_ignored(extra: &[u16]) -> Self {
        let ignored = [BTN_LEFT, BTN_RIGHT, BTN_MIDDLE]
            .into_iter()
            .chain(extra.iter().copied())
            .collect();
        Self {
            captured: Calibration::new(),
            queue: Button::ALL.iter().copied().collect(),
            ignored,
        }
    }

    pub fn current_button(&self) -> Option<Button> {
        self.queue.front().copied()
    }

    pub fn captured_count(&self) -> usize {
        Button::ALL.len() - self.queue.len()
    }

    pub fn total_buttons(&self) -> usize {
        Button::ALL.len()
    }

    pub fn is_complete(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn remaining_buttons(&self) -> &[Button] {
        self.queue.as_slices().0
    }

    /// Feeds a single event; returns the captured button, if any.
    pub fn feed(&mut self, event: &InputEvent) -> Result<Option<Button>, AppError> {
        Ok(self
            .feed_batch(std::slice::from_ref(event))?
            .pop()
            .map(|(button, _)| button))
    }

    /// Processes a batch of events atomically.
    ///
    /// A batch maps to a single human gesture: **at most one capture per
    /// batch** (the first non-conflicting code fills the expected button,
    /// remaining codes are swallowed). The Trinity firmware may emit several
    /// codes for a single press — including a code already owned by another
    /// button, tolerated within the batch. The same code reappearing in a
    /// **later** batch is still an error (true button-to-button collision).
    pub fn feed_batch(&mut self, events: &[InputEvent]) -> Result<Vec<(Button, u16)>, AppError> {
        let mut captured = Vec::new();
        let mut seen_codes = HashSet::new();
        let mut conflict: Option<AppError> = None;
        for event in events {
            let Some(button) = self.current_button() else {
                break;
            };
            if !event.is_key_press() || self.ignored.contains(&event.code) || !captured.is_empty() {
                continue;
            }
            if !seen_codes.insert(event.code) {
                continue;
            }
            let physical = PhysicalCode::new(event.node.clone(), event.code);
            match self.captured.record(button, physical) {
                Ok(()) => {
                    self.queue.pop_front();
                    captured.push((button, event.code));
                }
                Err(err) => conflict = Some(AppError::Domain(err)),
            }
        }
        if captured.is_empty() {
            if let Some(err) = conflict {
                return Err(err);
            }
        }
        Ok(captured)
    }

    /// Partial calibration being built.
    pub fn captured(&self) -> &Calibration {
        &self.captured
    }

    /// Ends the session; the calibration must be complete.
    pub fn finish(self) -> Result<Calibration, AppError> {
        if self.is_complete() {
            Ok(self.captured)
        } else {
            Err(AppError::Domain(DomainError::IncompleteCalibration {
                missing: self.queue.iter().map(|button| button.number()).collect(),
            }))
        }
    }
}

impl Default for CalibrationSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Profile use cases.
pub struct ProfileService<R> {
    repository: R,
}

impl<R: ProfileRepository> ProfileService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn list(&self) -> Result<Vec<String>, AppError> {
        let mut names = self.repository.list()?;
        names.sort_unstable();
        Ok(names)
    }

    pub fn load(&self, name: &str) -> Result<Profile, AppError> {
        self.repository.load(name)
    }

    pub fn save(&mut self, profile: &Profile) -> Result<(), AppError> {
        self.repository.save(profile)
    }

    pub fn delete(&mut self, name: &str) -> Result<(), AppError> {
        self.repository.delete(name)
    }
}

/// Calibration use cases.
pub struct CalibrationService<R> {
    repository: R,
}

impl<R: CalibrationRepository> CalibrationService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn load(&self) -> Result<Option<Calibration>, AppError> {
        self.repository.load()
    }

    /// Only a complete calibration can be persisted.
    pub fn save(&mut self, calibration: &Calibration) -> Result<(), AppError> {
        if !calibration.is_complete() {
            return Err(AppError::Domain(DomainError::IncompleteCalibration {
                missing: calibration
                    .missing_buttons()
                    .iter()
                    .map(|button| button.number())
                    .collect(),
            }));
        }
        self.repository.save(calibration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EV_REL, KEY_PRESS, KEY_RELEASE};
    use crate::testing::{MemoryCalibrationRepository, MemoryProfileRepository};

    use crate::testkit::{full_calibration, node};

    fn press(code: u16) -> InputEvent {
        InputEvent::key(node(), code, KEY_PRESS)
    }

    #[test]
    fn happy_path_captures_all_twelve_buttons_in_order() {
        let mut session = CalibrationSession::new();
        assert_eq!(session.current_button().map(|b| b.number()), Some(1));
        assert_eq!(session.captured_count(), 0);

        for number in 1..=12u8 {
            let outcome = session.feed(&press(200 + u16::from(number))).unwrap();
            assert_eq!(outcome.map(|b| b.number()), Some(number));
        }

        assert!(session.is_complete());
        let calibration = session.finish().unwrap();
        assert!(calibration.is_complete());
        assert_eq!(
            calibration.get(Button::new(12).unwrap()).map(|c| c.code()),
            Some(212)
        );
    }

    #[test]
    fn ignores_mouse_buttons_releases_and_non_key_events() {
        let mut session = CalibrationSession::new();
        for event in [
            InputEvent::key(node(), BTN_LEFT, KEY_PRESS),
            InputEvent::key(node(), BTN_RIGHT, KEY_PRESS),
            InputEvent::key(node(), 30, KEY_RELEASE),
            InputEvent {
                node: node(),
                event_type: EV_REL,
                code: 0,
                value: 1,
            },
        ] {
            assert_eq!(session.feed(&event).unwrap(), None);
        }
        assert_eq!(session.captured_count(), 0);
    }

    #[test]
    fn code_conflict_between_buttons_is_an_error() {
        let mut session = CalibrationSession::new();
        session.feed(&press(42)).unwrap();
        let err = session.feed(&press(42)).unwrap_err();
        assert!(matches!(
            err,
            AppError::Domain(DomainError::PhysicalCodeConflict { .. })
        ));
    }

    #[test]
    fn finish_rejects_incomplete_sessions() {
        let mut session = CalibrationSession::new();
        session.feed(&press(200)).unwrap();
        let err = session.finish().unwrap_err();
        assert!(matches!(
            err,
            AppError::Domain(DomainError::IncompleteCalibration { .. })
        ));
    }

    #[test]
    fn extra_ignored_codes_are_skipped() {
        let mut session = CalibrationSession::with_extra_ignored(&[56]);
        assert_eq!(session.feed(&press(56)).unwrap(), None);
        assert_eq!(
            session.feed(&press(200)).unwrap().map(|b| b.number()),
            Some(1)
        );
    }

    #[test]
    fn profile_service_roundtrips_through_repository() {
        let mut service = ProfileService::new(MemoryProfileRepository::new());
        let mut profile = Profile::new("mmo").unwrap();
        profile.set_mapping(
            Button::new(3).unwrap(),
            trinity_core::KeyCombination::plain(trinity_core::KeyCode::KEY_F),
        );
        service.save(&profile).unwrap();

        assert_eq!(service.list().unwrap(), vec!["mmo".to_owned()]);
        assert_eq!(service.load("mmo").unwrap(), profile);

        service.delete("mmo").unwrap();
        assert!(service.list().unwrap().is_empty());
        assert!(matches!(
            service.load("mmo").unwrap_err(),
            AppError::NotFound(_)
        ));
    }

    #[test]
    fn calibration_service_refuses_partial_saves() {
        let mut service = CalibrationService::new(MemoryCalibrationRepository::new());

        let mut partial = Calibration::new();
        partial
            .record(Button::new(1).unwrap(), PhysicalCode::new(node(), 201))
            .unwrap();
        assert!(matches!(
            service.save(&partial).unwrap_err(),
            AppError::Domain(DomainError::IncompleteCalibration { .. })
        ));

        let complete = full_calibration();
        service.save(&complete).unwrap();
        assert_eq!(service.load().unwrap(), Some(complete));
    }
}
