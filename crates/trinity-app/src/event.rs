use trinity_core::key::KeyCode;
use trinity_core::node::NodeId;

pub const EV_SYN: u16 = 0x00;
pub const EV_KEY: u16 = 0x01;
pub const EV_REL: u16 = 0x02;
pub const EV_ABS: u16 = 0x03;
pub const EV_MSC: u16 = 0x04;

pub const KEY_RELEASE: i32 = 0;
pub const KEY_PRESS: i32 = 1;
pub const KEY_REPEAT: i32 = 2;

pub const BTN_LEFT: u16 = 0x110;
pub const BTN_RIGHT: u16 = 0x111;
pub const BTN_MIDDLE: u16 = 0x112;
pub const BTN_SIDE: u16 = 0x113;
pub const BTN_EXTRA: u16 = 0x114;

/// Raw event read from a grabbed input node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputEvent {
    pub node: NodeId,
    pub event_type: u16,
    pub code: u16,
    pub value: i32,
}

impl InputEvent {
    pub fn key(node: NodeId, code: u16, value: i32) -> Self {
        Self {
            node,
            event_type: EV_KEY,
            code,
            value,
        }
    }

    pub const fn is_key(&self) -> bool {
        self.event_type == EV_KEY
    }

    pub const fn is_key_press(&self) -> bool {
        self.is_key() && self.value == KEY_PRESS
    }

    pub const fn is_key_release(&self) -> bool {
        self.is_key() && self.value == KEY_RELEASE
    }

    pub const fn is_key_repeat(&self) -> bool {
        self.is_key() && self.value == KEY_REPEAT
    }
}

/// Translator output: passthrough or key injection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Forward {
        event_type: u16,
        code: u16,
        value: i32,
    },
    Key {
        key: KeyCode,
        pressed: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node() -> NodeId {
        NodeId::new("node").unwrap()
    }

    #[test]
    fn key_helpers_classify_values() {
        let press = InputEvent::key(node(), 30, KEY_PRESS);
        let repeat = InputEvent::key(node(), 30, KEY_REPEAT);
        let release = InputEvent::key(node(), 30, KEY_RELEASE);
        let rel = InputEvent {
            node: node(),
            event_type: EV_REL,
            code: 0,
            value: 3,
        };
        assert!(press.is_key() && press.is_key_press() && !press.is_key_release());
        assert!(repeat.is_key_repeat());
        assert!(release.is_key_release());
        assert!(!rel.is_key());
    }
}
