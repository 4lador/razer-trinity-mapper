use std::collections::HashMap;

use trinity_core::key::KeyCode;
use trinity_core::node::NodeId;
use trinity_core::translation::TranslationTable;

use crate::event::{Action, EV_KEY, InputEvent, KEY_PRESS, KEY_RELEASE};

/// Stateful event translator.
///
/// Forwards everything as-is, except calibrated and mapped codes, which are
/// translated to key combinations. Injected codes are reference-counted:
/// two buttons holding the same key or modifier inject it once and release
/// it only when the last holder releases.
pub struct Translator {
    table: TranslationTable,
    held: HashMap<(NodeId, u16), Vec<KeyCode>>,
    down: HashMap<KeyCode, u32>,
}

impl Translator {
    pub fn new(table: TranslationTable) -> Self {
        Self {
            table,
            held: HashMap::new(),
            down: HashMap::new(),
        }
    }

    pub fn table(&self) -> &TranslationTable {
        &self.table
    }

    /// Replaces the table and releases everything held.
    pub fn set_table(&mut self, table: TranslationTable) -> Vec<Action> {
        let cleanup = self.release_all();
        self.table = table;
        cleanup
    }

    pub fn translate(&mut self, event: &InputEvent) -> Vec<Action> {
        if event.event_type != EV_KEY {
            return vec![forwarded(event)];
        }
        let Self { table, held, down } = self;
        let Some((_, combination)) = table.lookup(&event.node, event.code) else {
            return vec![forwarded(event)];
        };
        match event.value {
            KEY_PRESS => {
                let sequence = combination.press_sequence();
                let mut actions = Vec::with_capacity(sequence.len());
                let mut pressed = Vec::with_capacity(sequence.len());
                for key in sequence {
                    let count = down.entry(key).or_insert(0);
                    *count += 1;
                    if *count == 1 {
                        actions.push(Action::Key { key, pressed: true });
                    }
                    pressed.push(key);
                }
                held.insert((event.node.clone(), event.code), pressed);
                actions
            }
            KEY_RELEASE => {
                let Some(pressed) = held.remove(&(event.node.clone(), event.code)) else {
                    return Vec::new();
                };
                let mut actions = Vec::with_capacity(pressed.len());
                for key in pressed.into_iter().rev() {
                    if let Some(count) = down.get_mut(&key) {
                        *count -= 1;
                        if *count == 0 {
                            down.remove(&key);
                            actions.push(Action::Key {
                                key,
                                pressed: false,
                            });
                        }
                    }
                }
                actions
            }
            _ => Vec::new(),
        }
    }

    /// Releases all injected keys still held.
    pub fn release_all(&mut self) -> Vec<Action> {
        let mut keys: Vec<KeyCode> = self.down.keys().copied().collect();
        keys.sort_unstable();
        let actions = keys
            .into_iter()
            .map(|key| Action::Key {
                key,
                pressed: false,
            })
            .collect();
        self.down.clear();
        self.held.clear();
        actions
    }

    /// Number of source codes whose combination is still held.
    pub fn held_count(&self) -> usize {
        self.held.len()
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
    use crate::event::{EV_REL, KEY_REPEAT};
    use crate::testkit::{full_calibration, node, profile_fixture};

    use trinity_core::key::KeyCode;
    use trinity_core::modifier::Modifier;

    fn translator(mappings: &[(u8, Vec<Modifier>, KeyCode)]) -> Translator {
        Translator::new(
            TranslationTable::build(&full_calibration(), &profile_fixture(mappings)).unwrap(),
        )
    }

    fn ctrl(number: u8, key: KeyCode) -> (u8, Vec<Modifier>, KeyCode) {
        (number, vec![Modifier::LeftCtrl], key)
    }

    #[test]
    fn forwards_non_key_events() {
        let mut translator = translator(&[]);
        let event = InputEvent {
            node: node(),
            event_type: EV_REL,
            code: 0,
            value: 7,
        };
        assert_eq!(
            translator.translate(&event),
            vec![Action::Forward {
                event_type: EV_REL,
                code: 0,
                value: 7
            }]
        );
    }

    #[test]
    fn forwards_unmapped_keys() {
        let mut translator = translator(&[ctrl(1, KeyCode::KEY_1)]);
        let event = InputEvent::key(node(), 42, KEY_PRESS);
        assert_eq!(
            translator.translate(&event),
            vec![Action::Forward {
                event_type: EV_KEY,
                code: 42,
                value: KEY_PRESS
            }]
        );
    }

    #[test]
    fn press_emits_modifiers_then_key() {
        let mut translator = translator(&[ctrl(1, KeyCode::KEY_1)]);
        let event = InputEvent::key(node(), 101, KEY_PRESS);
        assert_eq!(
            translator.translate(&event),
            vec![
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
        assert_eq!(translator.held_count(), 1);
    }

    #[test]
    fn release_emits_key_then_modifiers_in_reverse() {
        let mut translator = translator(&[ctrl(1, KeyCode::KEY_1)]);
        translator.translate(&InputEvent::key(node(), 101, KEY_PRESS));
        assert_eq!(
            translator.translate(&InputEvent::key(node(), 101, KEY_RELEASE)),
            vec![
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
        assert_eq!(translator.held_count(), 0);
    }

    #[test]
    fn auto_repeat_is_swallowed() {
        let mut translator = translator(&[ctrl(1, KeyCode::KEY_1)]);
        translator.translate(&InputEvent::key(node(), 101, KEY_PRESS));
        assert_eq!(
            translator.translate(&InputEvent::key(node(), 101, KEY_REPEAT)),
            Vec::<Action>::new()
        );
    }

    #[test]
    fn release_without_press_is_ignored() {
        let mut translator = translator(&[ctrl(1, KeyCode::KEY_1)]);
        assert_eq!(
            translator.translate(&InputEvent::key(node(), 101, KEY_RELEASE)),
            Vec::<Action>::new()
        );
    }

    #[test]
    fn shared_modifier_is_injected_once_and_released_by_last_holder() {
        let mut translator = translator(&[ctrl(1, KeyCode::KEY_A), ctrl(2, KeyCode::KEY_B)]);
        translator.translate(&InputEvent::key(node(), 101, KEY_PRESS));
        assert_eq!(
            translator.translate(&InputEvent::key(node(), 102, KEY_PRESS)),
            vec![Action::Key {
                key: KeyCode::KEY_B,
                pressed: true
            }]
        );
        assert_eq!(
            translator.translate(&InputEvent::key(node(), 101, KEY_RELEASE)),
            vec![Action::Key {
                key: KeyCode::KEY_A,
                pressed: false
            }]
        );
        assert_eq!(
            translator.translate(&InputEvent::key(node(), 102, KEY_RELEASE)),
            vec![
                Action::Key {
                    key: KeyCode::KEY_B,
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
    fn shared_plain_key_is_refcounted() {
        let mappings = vec![
            (1u8, Vec::new(), KeyCode::KEY_A),
            (2u8, Vec::new(), KeyCode::KEY_A),
        ];
        let mut translator = translator(&mappings);
        assert_eq!(
            translator.translate(&InputEvent::key(node(), 101, KEY_PRESS)),
            vec![Action::Key {
                key: KeyCode::KEY_A,
                pressed: true
            }]
        );
        assert_eq!(
            translator.translate(&InputEvent::key(node(), 102, KEY_PRESS)),
            Vec::<Action>::new()
        );
        assert_eq!(
            translator.translate(&InputEvent::key(node(), 101, KEY_RELEASE)),
            Vec::<Action>::new()
        );
        assert_eq!(
            translator.translate(&InputEvent::key(node(), 102, KEY_RELEASE)),
            vec![Action::Key {
                key: KeyCode::KEY_A,
                pressed: false
            }]
        );
    }

    #[test]
    fn set_table_releases_every_held_key() {
        let mut translator = translator(&[ctrl(1, KeyCode::KEY_1)]);
        translator.translate(&InputEvent::key(node(), 101, KEY_PRESS));
        let cleanup = translator.set_table(TranslationTable::default());
        assert_eq!(
            cleanup,
            vec![
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
        assert_eq!(translator.held_count(), 0);
        assert!(translator.table().is_empty());
        assert_eq!(
            translator.translate(&InputEvent::key(node(), 101, KEY_PRESS)),
            vec![Action::Forward {
                event_type: EV_KEY,
                code: 101,
                value: KEY_PRESS
            }]
        );
    }

    #[test]
    fn canonical_modifier_order_is_applied() {
        let mappings = vec![(
            1u8,
            vec![Modifier::LeftShift, Modifier::LeftCtrl, Modifier::LeftAlt],
            KeyCode::KEY_X,
        )];
        let mut translator = translator(&mappings);
        assert_eq!(
            translator.translate(&InputEvent::key(node(), 101, KEY_PRESS)),
            vec![
                Action::Key {
                    key: KeyCode::KEY_LEFTCTRL,
                    pressed: true
                },
                Action::Key {
                    key: KeyCode::KEY_LEFTSHIFT,
                    pressed: true
                },
                Action::Key {
                    key: KeyCode::KEY_LEFTALT,
                    pressed: true
                },
                Action::Key {
                    key: KeyCode::KEY_X,
                    pressed: true
                },
            ]
        );
    }
}
