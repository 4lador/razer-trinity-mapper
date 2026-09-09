use trinity_core::calibration::Calibration;
use trinity_core::error::DomainError;
use trinity_core::node::NodeId;
use trinity_core::profile::Profile;
use trinity_core::translation::TranslationTable;

use crate::error::AppError;
use crate::event::{Action, InputEvent};
use crate::ports::{Caps, InputEventSource, OutputEventSink};
use crate::session::CalibrationSession;
use crate::translator::Translator;

/// Name of the virtual device created by the engine.
pub const MIRROR_DEVICE_NAME: &str = "trinity-mapper-mirror";

/// Remapping engine: grabs the source, translates, injects into the mirror.
///
/// Started without calibration or profile, it forwards everything as-is;
/// `set_calibration` + `set_profile` arm translation, and
/// `begin_calibration` switches to guided calibration mode (captured codes
/// are swallowed, the rest is forwarded raw).
pub struct Engine<S, O> {
    source: S,
    sink: O,
    translator: Option<Translator>,
    calibration: Option<Calibration>,
    profile: Option<Profile>,
    calibrating: Option<CalibrationSession>,
}

impl<S: InputEventSource, O: OutputEventSink> Engine<S, O> {
    pub fn new(source: S, sink: O) -> Self {
        Self {
            source,
            sink,
            translator: None,
            calibration: None,
            profile: None,
            calibrating: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.translator.is_some()
    }

    pub fn is_calibrating(&self) -> bool {
        self.calibrating.is_some()
    }

    pub fn grabbed_nodes(&self) -> &[NodeId] {
        self.source.grabbed()
    }

    /// Grabs the nodes and creates the mirror; everything passes through
    /// until a profile is applied.
    pub fn start(&mut self, nodes: &[NodeId], caps: &Caps) -> Result<(), AppError> {
        if self.is_active() {
            return Err(AppError::InvalidState("engine already active".into()));
        }
        self.sink.create(MIRROR_DEVICE_NAME, caps)?;
        if let Err(err) = self.source.grab(nodes) {
            let _ = self.sink.destroy();
            return Err(err);
        }
        self.translator = Some(Translator::new(TranslationTable::default()));
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), AppError> {
        let Some(mut translator) = self.translator.take() else {
            return Err(AppError::InvalidState("engine is not active".into()));
        };
        let cleanup = translator.release_all();
        if !cleanup.is_empty() {
            self.sink.emit(&cleanup)?;
        }
        self.source.release()?;
        self.sink.destroy()?;
        self.calibrating = None;
        Ok(())
    }

    pub fn set_calibration(&mut self, calibration: Calibration) -> Result<(), AppError> {
        self.calibration = Some(calibration);
        self.rebuild_table()
    }

    pub fn set_profile(&mut self, profile: Profile) -> Result<(), AppError> {
        self.profile = Some(profile);
        self.rebuild_table()
    }

    /// Switches to full passthrough: releases held keys and clears the
    /// translation table. Grab and mirror stay in place — tearing devices
    /// down on toggle races the compositor.
    pub fn suspend_translation(&mut self) -> Result<(), AppError> {
        if let Some(translator) = self.translator.as_mut() {
            let cleanup = translator.set_table(TranslationTable::default());
            if !cleanup.is_empty() {
                self.sink.emit(&cleanup)?;
            }
        }
        Ok(())
    }

    /// Re-arms translation from the remembered calibration and profile.
    pub fn resume_translation(&mut self) -> Result<(), AppError> {
        self.rebuild_table()
    }

    fn rebuild_table(&mut self) -> Result<(), AppError> {
        let (Some(calibration), Some(profile)) = (self.calibration.as_ref(), self.profile.as_ref())
        else {
            return Ok(());
        };
        if !calibration.is_complete() {
            return Ok(());
        }
        let table = TranslationTable::build(calibration, profile)?;
        if let Some(translator) = self.translator.as_mut() {
            let cleanup = translator.set_table(table);
            if !cleanup.is_empty() {
                self.sink.emit(&cleanup)?;
            }
        }
        Ok(())
    }

    pub fn active_profile(&self) -> Option<&Profile> {
        self.profile.as_ref()
    }

    /// Switches to guided calibration: releases held keys, swallows
    /// captured codes, forwards the rest raw.
    pub fn begin_calibration(&mut self) -> Result<(), AppError> {
        if !self.is_active() {
            return Err(AppError::InvalidState("engine is not active".into()));
        }
        if let Some(translator) = self.translator.as_mut() {
            let cleanup = translator.release_all();
            if !cleanup.is_empty() {
                self.sink.emit(&cleanup)?;
            }
        }
        self.calibrating = Some(CalibrationSession::new());
        Ok(())
    }

    pub fn cancel_calibration(&mut self) {
        self.calibrating = None;
    }

    /// Progress: (expected button, captured count). `None` outside calibration.
    pub fn calibration_progress(&self) -> Option<(Option<u8>, usize)> {
        self.calibrating.as_ref().map(|session| {
            (
                session.current_button().map(|button| button.number()),
                session.captured_count(),
            )
        })
    }

    /// Calibration captures so far: (button, node, code).
    pub fn calibration_snapshot(&self) -> Vec<(u8, &NodeId, u16)> {
        self.calibrating
            .as_ref()
            .map(|session| {
                session
                    .captured()
                    .buttons()
                    .map(|(button, code)| (button.number(), code.node(), code.code()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Ends calibration: stores it and re-arms translation.
    pub fn finish_calibration(&mut self) -> Result<Calibration, AppError> {
        let Some(session) = self.calibrating.take() else {
            return Err(AppError::InvalidState("no calibration in progress".into()));
        };
        if !session.is_complete() {
            let missing = session
                .remaining_buttons()
                .iter()
                .map(|button| button.number())
                .collect();
            self.calibrating = Some(session);
            return Err(AppError::Domain(DomainError::IncompleteCalibration {
                missing,
            }));
        }
        let calibration = session.finish()?;
        self.set_calibration(calibration.clone())?;
        Ok(calibration)
    }

    /// Processes a batch of events: read, translate or capture, inject.
    pub fn run_once(&mut self) -> Result<usize, AppError> {
        if !self.is_active() {
            return Err(AppError::InvalidState("engine is not active".into()));
        }
        let events = self.source.read_events()?;
        let mut actions = Vec::with_capacity(events.len());
        if let Some(session) = self.calibrating.as_mut() {
            let captured = session.feed_batch(&events)?;
            let swallow_presses = !captured.is_empty();
            for event in &events {
                if event.is_key_press() && swallow_presses {
                    continue;
                }
                actions.push(forwarded(event));
            }
        } else if let Some(translator) = self.translator.as_mut() {
            for event in &events {
                actions.extend(translator.translate(event));
            }
        }
        if !actions.is_empty() {
            self.sink.emit(&actions)?;
        }
        Ok(events.len())
    }
}

fn forwarded(event: &InputEvent) -> Action {
    Action::Forward {
        event_type: event.event_type,
        code: event.code,
        value: event.value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{BTN_LEFT, KEY_PRESS};
    use crate::testing::{FakeSink, FakeSource};
    use crate::testkit::{full_calibration, node, profile_fixture};

    use trinity_core::button::Button;
    use trinity_core::key::KeyCode;
    use trinity_core::modifier::Modifier;

    fn ctrl_1() -> (u8, Vec<Modifier>, KeyCode) {
        (1, vec![Modifier::LeftCtrl], KeyCode::KEY_1)
    }

    fn started_engine(batches: Vec<Vec<InputEvent>>) -> Engine<FakeSource, FakeSink> {
        let mut source = FakeSource::new();
        for batch in batches {
            source.push_batch(batch);
        }
        let mut engine = Engine::new(source, FakeSink::new());
        engine.start(&[node()], &Caps::new()).unwrap();
        engine.set_calibration(full_calibration()).unwrap();
        engine.set_profile(profile_fixture(&[ctrl_1()])).unwrap();
        engine
    }

    #[test]
    fn start_grabs_nodes_and_creates_mirror() {
        let engine = started_engine(vec![]);
        assert!(engine.is_active());
        assert!(!engine.is_calibrating());
        assert_eq!(engine.grabbed_nodes(), &[node()]);
    }

    #[test]
    fn run_once_translates_and_forwards() {
        let mut engine = started_engine(vec![vec![
            InputEvent::key(node(), 101, KEY_PRESS),
            InputEvent {
                node: node(),
                event_type: crate::event::EV_REL,
                code: 0,
                value: 7,
            },
            InputEvent::key(node(), 101, crate::event::KEY_RELEASE),
        ]]);
        assert_eq!(engine.run_once().unwrap(), 3);
        let sink = &engine.sink;
        assert_eq!(
            sink.emitted(),
            vec![
                Action::Key {
                    key: KeyCode::KEY_LEFTCTRL,
                    pressed: true
                },
                Action::Key {
                    key: KeyCode::KEY_1,
                    pressed: true
                },
                Action::Forward {
                    event_type: crate::event::EV_REL,
                    code: 0,
                    value: 7
                },
                Action::Key {
                    key: KeyCode::KEY_1,
                    pressed: false
                },
                Action::Key {
                    key: KeyCode::KEY_LEFTCTRL,
                    pressed: false
                },
            ]
        );
    }

    #[test]
    fn start_without_calibration_forwards_everything() {
        let mut source = FakeSource::new();
        source.push_batch(vec![
            InputEvent::key(node(), 101, KEY_PRESS),
            InputEvent {
                node: node(),
                event_type: crate::event::EV_REL,
                code: 0,
                value: 3,
            },
        ]);
        let mut engine = Engine::new(source, FakeSink::new());
        engine.start(&[node()], &Caps::new()).unwrap();
        assert_eq!(engine.run_once().unwrap(), 2);
        let sink = &engine.sink;
        assert_eq!(
            sink.emitted(),
            vec![
                Action::Forward {
                    event_type: crate::event::EV_KEY,
                    code: 101,
                    value: KEY_PRESS
                },
                Action::Forward {
                    event_type: crate::event::EV_REL,
                    code: 0,
                    value: 3
                },
            ]
        );
    }

    #[test]
    fn profile_without_calibration_stays_passthrough() {
        let mut engine = Engine::new(FakeSource::new(), FakeSink::new());
        engine.start(&[node()], &Caps::new()).unwrap();
        engine.set_profile(profile_fixture(&[ctrl_1()])).unwrap();
        let mut source = engine.source.clone();
        source.push_batch(vec![InputEvent::key(node(), 101, KEY_PRESS)]);
        engine.source = source;
        engine.run_once().unwrap();
        let sink = &engine.sink;
        assert_eq!(
            sink.emitted(),
            vec![Action::Forward {
                event_type: crate::event::EV_KEY,
                code: 101,
                value: KEY_PRESS
            }]
        );
    }

    #[test]
    fn stop_releases_held_keys_and_destroys_mirror() {
        let mut engine = started_engine(vec![vec![InputEvent::key(node(), 101, KEY_PRESS)]]);
        engine.run_once().unwrap();
        engine.stop().unwrap();
        assert!(!engine.is_active());
        let sink = &engine.sink;
        assert_eq!(sink.destroy_count(), 1);
        let emitted = sink.emitted();
        assert_eq!(
            &emitted[emitted.len() - 2..],
            &[
                Action::Key {
                    key: KeyCode::KEY_1,
                    pressed: false
                },
                Action::Key {
                    key: KeyCode::KEY_LEFTCTRL,
                    pressed: false
                }
            ]
        );
    }

    #[test]
    fn double_start_is_rejected() {
        let mut engine = started_engine(vec![]);
        let err = engine.start(&[node()], &Caps::new()).unwrap_err();
        assert!(matches!(err, AppError::InvalidState(_)));
    }

    #[test]
    fn stop_when_inactive_is_rejected() {
        let mut engine = Engine::new(FakeSource::new(), FakeSink::new());
        let err = engine.stop().unwrap_err();
        assert!(matches!(err, AppError::InvalidState(_)));
    }

    #[test]
    fn set_profile_swaps_translation_and_releases_held() {
        let mut engine = started_engine(vec![
            vec![InputEvent::key(node(), 101, KEY_PRESS)],
            vec![InputEvent::key(node(), 101, KEY_PRESS)],
        ]);
        engine.run_once().unwrap();

        let new_profile = profile_fixture(&[(1, vec![Modifier::LeftShift], KeyCode::KEY_2)]);
        engine.set_profile(new_profile).unwrap();
        engine.run_once().unwrap();

        let sink = &engine.sink;
        assert_eq!(
            sink.emitted(),
            vec![
                Action::Key {
                    key: KeyCode::KEY_LEFTCTRL,
                    pressed: true
                },
                Action::Key {
                    key: KeyCode::KEY_1,
                    pressed: true
                },
                Action::Key {
                    key: KeyCode::KEY_1,
                    pressed: false
                },
                Action::Key {
                    key: KeyCode::KEY_LEFTCTRL,
                    pressed: false
                },
                Action::Key {
                    key: KeyCode::KEY_LEFTSHIFT,
                    pressed: true
                },
                Action::Key {
                    key: KeyCode::KEY_2,
                    pressed: true
                },
            ]
        );
    }

    #[test]
    fn calibration_flow_captures_then_rearms_translation() {
        let mut engine = started_engine(vec![]);
        engine.begin_calibration().unwrap();
        assert!(engine.is_calibrating());
        assert_eq!(engine.calibration_progress(), Some((Some(1), 0)));

        let mut batches: Vec<Vec<InputEvent>> = vec![vec![
            InputEvent::key(node(), 101, KEY_PRESS),
            InputEvent::key(node(), BTN_LEFT, KEY_PRESS),
        ]];
        for number in 2u8..=12 {
            batches.push(vec![InputEvent::key(
                node(),
                100 + u16::from(number),
                KEY_PRESS,
            )]);
        }
        for batch in batches {
            engine.source.push_batch(batch);
        }
        engine.run_once().unwrap();
        assert_eq!(engine.calibration_progress(), Some((Some(2), 1)));

        for _ in 2..=12 {
            engine.run_once().unwrap();
        }
        assert_eq!(engine.calibration_progress(), Some((None, 12)));

        let calibration = engine.finish_calibration().unwrap();
        assert!(calibration.is_complete());
        assert!(!engine.is_calibrating());

        let mut source = engine.source.clone();
        source.push_batch(vec![InputEvent::key(node(), 101, KEY_PRESS)]);
        engine.source = source;
        engine.run_once().unwrap();
        let sink = &engine.sink;
        // Presses from the captured batch (including the ignored
        // BTN_LEFT) are swallowed; only post-calibration translation emits.
        assert_eq!(
            sink.emitted().first(),
            Some(&Action::Key {
                key: KeyCode::KEY_LEFTCTRL,
                pressed: true
            })
        );
        assert!(
            !sink
                .emitted()
                .iter()
                .any(|action| matches!(action, Action::Forward { code: BTN_LEFT, .. }))
        );
    }

    #[test]
    fn finish_calibration_requires_all_buttons() {
        let mut engine = started_engine(vec![vec![InputEvent::key(node(), 101, KEY_PRESS)]]);
        engine.begin_calibration().unwrap();
        engine.run_once().unwrap();
        let err = engine.finish_calibration().unwrap_err();
        assert!(matches!(
            err,
            AppError::Domain(DomainError::IncompleteCalibration { .. })
        ));
        assert!(engine.is_calibrating());
    }

    #[test]
    fn finish_calibration_without_session_is_rejected() {
        let mut engine = started_engine(vec![]);
        let err = engine.finish_calibration().unwrap_err();
        assert!(matches!(err, AppError::InvalidState(_)));
    }

    #[test]
    fn cancel_calibration_restores_translation() {
        let mut engine = started_engine(vec![vec![InputEvent::key(node(), 101, KEY_PRESS)]]);
        engine.begin_calibration().unwrap();
        engine.cancel_calibration();
        assert!(!engine.is_calibrating());
        engine.run_once().unwrap();
        let sink = &engine.sink;
        assert_eq!(
            sink.emitted().first(),
            Some(&Action::Key {
                key: KeyCode::KEY_LEFTCTRL,
                pressed: true
            })
        );
    }

    #[test]
    fn calibration_conflict_propagates() {
        let mut engine = started_engine(vec![
            vec![InputEvent::key(node(), 42, KEY_PRESS)],
            vec![InputEvent::key(node(), 42, KEY_PRESS)],
        ]);
        engine.begin_calibration().unwrap();
        engine.run_once().unwrap();
        let err = engine.run_once().unwrap_err();
        assert!(matches!(
            err,
            AppError::Domain(DomainError::PhysicalCodeConflict { .. })
        ));
    }

    #[test]
    fn dual_node_emission_captures_a_single_button_and_swallows_both() {
        let node2 =
            NodeId::new("usb-Razer_Razer_Naga_Trinity_00000000001A-if02-event-kbd").unwrap();
        let mut engine = started_engine(vec![vec![
            InputEvent::key(node(), 30, KEY_PRESS),
            InputEvent::key(node2.clone(), 30, KEY_PRESS),
        ]]);
        engine.begin_calibration().unwrap();
        engine.run_once().unwrap();
        assert_eq!(engine.calibration_progress(), Some((Some(2), 1)));
        let sink = &engine.sink;
        assert!(sink.emitted().is_empty());
        let snapshot = engine.calibration_snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0], (1, &node(), 30));
    }

    #[test]
    fn same_node_double_press_in_one_batch_is_swallowed() {
        let mut engine = started_engine(vec![vec![
            InputEvent::key(node(), 42, KEY_PRESS),
            InputEvent::key(node(), 42, KEY_PRESS),
        ]]);
        engine.begin_calibration().unwrap();
        engine.run_once().unwrap();
        assert_eq!(engine.calibration_progress(), Some((Some(2), 1)));
    }

    fn codes_for_all_buttons() -> [u16; 12] {
        [101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112]
    }

    #[test]
    fn multi_code_press_captures_first_code_and_forwards_nothing() {
        // Buttons 1..10 captured; button 10 owns code 110.
        let mut batches: Vec<Vec<InputEvent>> = Vec::new();
        for &code in &codes_for_all_buttons()[..10] {
            batches.push(vec![InputEvent::key(node(), code, KEY_PRESS)]);
        }
        // Button 11 press: emits its own code (111), THEN button 10's code (110).
        batches.push(vec![
            InputEvent::key(node(), 111, KEY_PRESS),
            InputEvent::key(node(), 110, KEY_PRESS),
        ]);
        // Button 12.
        batches.push(vec![InputEvent::key(node(), 112, KEY_PRESS)]);
        let mut engine = started_engine(batches);
        engine.begin_calibration().unwrap();
        for _ in 0..12 {
            engine.run_once().unwrap();
        }
        assert_eq!(engine.calibration_progress(), Some((None, 12)));
        let calibration = engine.finish_calibration().unwrap();
        assert_eq!(
            calibration.get(Button::new(11).unwrap()).map(|c| c.code()),
            Some(111)
        );
        let sink = &engine.sink;
        assert!(sink.emitted().is_empty());
    }

    #[test]
    fn conflicting_leading_code_is_tolerated_within_batch() {
        let mut batches: Vec<Vec<InputEvent>> = Vec::new();
        for &code in &codes_for_all_buttons()[..10] {
            batches.push(vec![InputEvent::key(node(), code, KEY_PRESS)]);
        }
        // Button 11 press: the stray code (110, already owned by button
        // 10) arrives BEFORE the useful code (111).
        batches.push(vec![
            InputEvent::key(node(), 110, KEY_PRESS),
            InputEvent::key(node(), 111, KEY_PRESS),
        ]);
        for &code in &codes_for_all_buttons()[11..] {
            batches.push(vec![InputEvent::key(node(), code, KEY_PRESS)]);
        }
        let mut engine = started_engine(batches);
        engine.begin_calibration().unwrap();
        for _ in 0..12 {
            engine.run_once().unwrap();
        }
        assert_eq!(engine.calibration_progress(), Some((None, 12)));
        let calibration = engine.finish_calibration().unwrap();
        assert_eq!(
            calibration.get(Button::new(11).unwrap()).map(|c| c.code()),
            Some(111)
        );
    }

    #[test]
    fn begin_calibration_requires_active_engine() {
        let mut engine = Engine::new(FakeSource::new(), FakeSink::new());
        let err = engine.begin_calibration().unwrap_err();
        assert!(matches!(err, AppError::InvalidState(_)));
    }

    #[test]
    fn suspend_forwards_everything_and_releases_held_keys() {
        let mut engine = started_engine(vec![vec![InputEvent::key(node(), 101, KEY_PRESS)]]);
        engine.run_once().unwrap();
        engine.suspend_translation().unwrap();
        let mut source = engine.source.clone();
        source.push_batch(vec![InputEvent::key(node(), 101, KEY_PRESS)]);
        engine.source = source;
        engine.run_once().unwrap();
        let sink = &engine.sink;
        assert_eq!(
            sink.emitted().last(),
            Some(&Action::Forward {
                event_type: crate::event::EV_KEY,
                code: 101,
                value: KEY_PRESS
            })
        );
    }

    #[test]
    fn resume_rearms_translation() {
        let mut engine = started_engine(vec![
            vec![InputEvent::key(node(), 101, KEY_PRESS)],
            vec![InputEvent::key(node(), 101, KEY_PRESS)],
        ]);
        engine.suspend_translation().unwrap();
        engine.run_once().unwrap();
        engine.resume_translation().unwrap();
        engine.run_once().unwrap();
        let sink = &engine.sink;
        let emitted = sink.emitted();
        assert_eq!(emitted.len(), 3);
        assert_eq!(
            &emitted[emitted.len() - 2..],
            &[
                Action::Key {
                    key: KeyCode::KEY_LEFTCTRL,
                    pressed: true
                },
                Action::Key {
                    key: KeyCode::KEY_1,
                    pressed: true
                },
            ]
        );
    }
}
