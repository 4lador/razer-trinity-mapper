use trinity_app::error::AppError;
use trinity_app::event::Action;
use trinity_app::ports::{Caps, OutputEventSink};

/// uinput sink: virtual mirror device receiving the actions.
pub struct UinputSink {
    device: Option<evdev::uinput::VirtualDevice>,
}

impl UinputSink {
    pub fn new() -> Self {
        Self { device: None }
    }
}

impl Default for UinputSink {
    fn default() -> Self {
        Self::new()
    }
}

fn port_err(context: &str, err: impl std::fmt::Display) -> AppError {
    AppError::Port(format!("{context} : {err}"))
}

fn raw_event(event_type: u16, code: u16, value: i32) -> Option<evdev::InputEvent> {
    match evdev::EventType(event_type) {
        evdev::EventType::KEY => Some(evdev::KeyEvent::new(evdev::KeyCode(code), value).into()),
        evdev::EventType::RELATIVE => {
            Some(evdev::RelativeAxisEvent::new(evdev::RelativeAxisCode(code), value).into())
        }
        evdev::EventType::ABSOLUTE => {
            Some(evdev::AbsoluteAxisEvent::new(evdev::AbsoluteAxisCode(code), value).into())
        }
        _ => None,
    }
}

impl OutputEventSink for UinputSink {
    fn create(&mut self, name: &str, caps: &Caps) -> Result<(), AppError> {
        let mut builder = evdev::uinput::VirtualDevice::builder()
            .map_err(|err| port_err("/dev/uinput access", err))?
            .name(name);
        if !caps.keys().is_empty() {
            let mut keys = evdev::AttributeSet::<evdev::KeyCode>::new();
            for &code in caps.keys() {
                keys.insert(evdev::KeyCode(code));
            }
            builder = builder
                .with_keys(&keys)
                .map_err(|err| port_err("key capability declaration", err))?;
        }
        if !caps.rels().is_empty() {
            let mut rels = evdev::AttributeSet::<evdev::RelativeAxisCode>::new();
            for &code in caps.rels() {
                rels.insert(evdev::RelativeAxisCode(code));
            }
            builder = builder
                .with_relative_axes(&rels)
                .map_err(|err| port_err("relative axis declaration", err))?;
        }
        for &code in caps.abss() {
            let setup = evdev::UinputAbsSetup::new(
                evdev::AbsoluteAxisCode(code),
                evdev::AbsInfo::new(0, 0, 65535, 0, 0, 0),
            );
            builder = builder
                .with_absolute_axis(&setup)
                .map_err(|err| port_err("absolute axis declaration", err))?;
        }
        self.device = Some(
            builder
                .build()
                .map_err(|err| port_err("mirror creation", err))?,
        );
        Ok(())
    }

    fn emit(&mut self, actions: &[Action]) -> Result<(), AppError> {
        let Some(device) = self.device.as_mut() else {
            return Err(AppError::InvalidState("mirror not created".into()));
        };
        let mut events = Vec::with_capacity(actions.len());
        for action in actions {
            match action {
                Action::Forward {
                    event_type,
                    code,
                    value,
                } => {
                    if let Some(event) = raw_event(*event_type, *code, *value) {
                        events.push(event);
                    }
                }
                Action::Key { key, pressed } => events.push(
                    evdev::KeyEvent::new(evdev::KeyCode(key.code()), i32::from(*pressed)).into(),
                ),
            }
        }
        device
            .emit(&events)
            .map_err(|err| port_err("injection", err))
    }

    fn destroy(&mut self) -> Result<(), AppError> {
        self.device = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trinity_core::key::KeyCode;

    #[test]
    fn emit_without_create_is_rejected() {
        let mut sink = UinputSink::new();
        assert!(matches!(
            sink.emit(&[Action::Key {
                key: KeyCode::KEY_A,
                pressed: true
            }])
            .unwrap_err(),
            AppError::InvalidState(_)
        ));
    }

    #[test]
    #[ignore = "requires /dev/uinput access (uinput group)"]
    fn create_emit_destroy_roundtrip() {
        let mut sink = UinputSink::new();
        let mut caps = Caps::new();
        caps.add_key(KeyCode::KEY_A).add_key(KeyCode::KEY_LEFTCTRL);
        sink.create("trinity-test-mirror", &caps).unwrap();
        sink.emit(&[
            Action::Key {
                key: KeyCode::KEY_LEFTCTRL,
                pressed: true,
            },
            Action::Key {
                key: KeyCode::KEY_A,
                pressed: true,
            },
            Action::Forward {
                event_type: 2,
                code: 0,
                value: 5,
            },
            Action::Key {
                key: KeyCode::KEY_A,
                pressed: false,
            },
            Action::Key {
                key: KeyCode::KEY_LEFTCTRL,
                pressed: false,
            },
        ])
        .unwrap();
        sink.destroy().unwrap();
    }
}
