use crate::{InputEvent, InputKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use worldwake_core::Tick;

static EMPTY_INPUT_EVENTS: [InputEvent; 0] = [];

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct InputQueue {
    next_sequence_no: u64,
    events_by_tick: BTreeMap<Tick, Vec<InputEvent>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputQueueError {
    NonMonotonicSequence {
        previous_tick: Tick,
        previous_sequence_no: u64,
        attempted_tick: Tick,
        attempted_sequence_no: u64,
    },
}

impl fmt::Display for InputQueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonMonotonicSequence {
                previous_tick,
                previous_sequence_no,
                attempted_tick,
                attempted_sequence_no,
            } => write!(
                f,
                "recorded input sequence must increase strictly: previous=({previous_tick}, {previous_sequence_no}), attempted=({attempted_tick}, {attempted_sequence_no})"
            ),
        }
    }
}

impl std::error::Error for InputQueueError {}

impl InputQueue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue(&mut self, tick: Tick, kind: InputKind) -> &InputEvent {
        let sequence_no = self.next_sequence_no;
        self.next_sequence_no = self
            .next_sequence_no
            .checked_add(1)
            .expect("input sequence number overflowed");

        let event = InputEvent {
            scheduled_tick: tick,
            sequence_no,
            kind,
        };

        let events = self.events_by_tick.entry(tick).or_default();
        events.push(event);
        events
            .last()
            .expect("queue bucket must contain the event that was just pushed")
    }

    pub fn drain_tick(&mut self, tick: Tick) -> Vec<InputEvent> {
        self.events_by_tick.remove(&tick).unwrap_or_default()
    }

    #[must_use]
    pub fn peek_tick(&self, tick: Tick) -> &[InputEvent] {
        self.events_by_tick
            .get(&tick)
            .map_or(&EMPTY_INPUT_EVENTS, Vec::as_slice)
    }

    pub fn iter(&self) -> impl Iterator<Item = &InputEvent> {
        self.events_by_tick
            .values()
            .flat_map(|events| events.iter())
    }

    pub fn iter_in_sequence_order(&self) -> impl Iterator<Item = &InputEvent> {
        let mut inputs = self.iter().collect::<Vec<_>>();
        inputs.sort_unstable_by_key(|input| input.sequence_no);
        inputs.into_iter()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events_by_tick.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.events_by_tick.values().map(Vec::len).sum()
    }

    #[must_use]
    pub fn next_sequence_no(&self) -> u64 {
        self.next_sequence_no
    }

    pub fn replace_with_recorded(&mut self, inputs: &[InputEvent]) -> Result<(), InputQueueError> {
        self.events_by_tick.clear();
        self.next_sequence_no = 0;

        let mut previous: Option<&InputEvent> = None;
        for input in inputs {
            if let Some(previous_input) = previous {
                if input.sequence_no <= previous_input.sequence_no {
                    self.events_by_tick.clear();
                    self.next_sequence_no = 0;
                    return Err(InputQueueError::NonMonotonicSequence {
                        previous_tick: previous_input.scheduled_tick,
                        previous_sequence_no: previous_input.sequence_no,
                        attempted_tick: input.scheduled_tick,
                        attempted_sequence_no: input.sequence_no,
                    });
                }
            }

            self.events_by_tick
                .entry(input.scheduled_tick)
                .or_default()
                .push(input.clone());
            self.next_sequence_no = input
                .sequence_no
                .checked_add(1)
                .expect("recorded input sequence number overflowed");
            previous = Some(input);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{InputQueue, InputQueueError};
    use crate::{ActionDefId, ActionInstanceId, ActionRequestMode, InputKind};
    use worldwake_core::{EntityId, Tick};

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn request_action(actor_slot: u32, def_id: u32, target_slots: &[u32]) -> InputKind {
        InputKind::RequestAction {
            actor: entity(actor_slot),
            def_id: ActionDefId(def_id),
            targets: target_slots.iter().map(|slot| entity(*slot)).collect(),
            payload_override: None,
            mode: ActionRequestMode::Strict,
        }
    }

    #[test]
    fn new_queue_starts_empty_with_zero_next_sequence() {
        let queue = InputQueue::new();

        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.next_sequence_no(), 0);
        assert!(queue.peek_tick(Tick(5)).is_empty());
    }

    #[test]
    fn enqueue_assigns_global_sequence_numbers_across_ticks() {
        let mut queue = InputQueue::new();

        let first = queue.enqueue(Tick(5), request_action(1, 0, &[2])).clone();
        let second = queue
            .enqueue(
                Tick(5),
                InputKind::CancelAction {
                    actor: entity(3),
                    action_instance_id: ActionInstanceId(9),
                },
            )
            .clone();
        let third = queue
            .enqueue(
                Tick(3),
                InputKind::SwitchControl {
                    from: None,
                    to: Some(entity(4)),
                },
            )
            .clone();

        assert_eq!(first.sequence_no, 0);
        assert_eq!(second.sequence_no, 1);
        assert_eq!(third.sequence_no, 2);
        assert_eq!(queue.len(), 3);
        assert_eq!(queue.next_sequence_no(), 3);
        assert_eq!(queue.peek_tick(Tick(5)), &[first.clone(), second.clone()]);
        assert_eq!(queue.peek_tick(Tick(3)), &[third]);
    }

    #[test]
    fn drain_tick_returns_only_requested_tick_in_enqueue_order() {
        let mut queue = InputQueue::new();
        let tick_three = queue
            .enqueue(
                Tick(3),
                InputKind::SwitchControl {
                    from: None,
                    to: Some(entity(1)),
                },
            )
            .clone();
        let tick_five_first = queue.enqueue(Tick(5), request_action(2, 7, &[3])).clone();
        let tick_five_second = queue
            .enqueue(
                Tick(5),
                InputKind::CancelAction {
                    actor: entity(4),
                    action_instance_id: ActionInstanceId(11),
                },
            )
            .clone();

        let drained = queue.drain_tick(Tick(5));

        assert_eq!(drained, vec![tick_five_first, tick_five_second]);
        assert_eq!(queue.peek_tick(Tick(3)), &[tick_three]);
        assert!(queue.peek_tick(Tick(5)).is_empty());
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.next_sequence_no(), 3);
    }

    #[test]
    fn drain_tick_removes_events_and_second_drain_is_empty() {
        let mut queue = InputQueue::new();
        queue.enqueue(Tick(8), request_action(1, 4, &[2, 3]));

        let first = queue.drain_tick(Tick(8));
        let second = queue.drain_tick(Tick(8));

        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.next_sequence_no(), 1);
    }

    #[test]
    fn bincode_roundtrip_preserves_events_and_future_sequence_allocation() {
        let mut queue = InputQueue::new();
        let original_first = queue.enqueue(Tick(2), request_action(1, 0, &[9])).clone();
        let original_second = queue
            .enqueue(
                Tick(4),
                InputKind::SwitchControl {
                    from: Some(entity(3)),
                    to: Some(entity(4)),
                },
            )
            .clone();

        let bytes = bincode::serialize(&queue).unwrap();
        let mut restored: InputQueue = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.peek_tick(Tick(2)), &[original_first]);
        assert_eq!(restored.peek_tick(Tick(4)), &[original_second]);
        assert_eq!(restored.next_sequence_no(), 2);

        let next = restored
            .enqueue(
                Tick(2),
                InputKind::CancelAction {
                    actor: entity(5),
                    action_instance_id: ActionInstanceId(12),
                },
            )
            .clone();

        assert_eq!(next.sequence_no, 2);
        assert_eq!(restored.next_sequence_no(), 3);
    }

    #[test]
    fn iter_returns_inputs_in_tick_bucket_then_insertion_order() {
        let mut queue = InputQueue::new();
        let late_tick = queue.enqueue(Tick(5), request_action(1, 0, &[2])).clone();
        let early_tick_first = queue
            .enqueue(
                Tick(3),
                InputKind::SwitchControl {
                    from: None,
                    to: Some(entity(4)),
                },
            )
            .clone();
        let early_tick_second = queue
            .enqueue(
                Tick(3),
                InputKind::CancelAction {
                    actor: entity(3),
                    action_instance_id: ActionInstanceId(9),
                },
            )
            .clone();

        let iterated = queue.iter().cloned().collect::<Vec<_>>();

        assert_eq!(
            iterated,
            vec![early_tick_first, early_tick_second, late_tick]
        );
    }

    #[test]
    fn replace_with_recorded_rebuilds_queue_and_preserves_next_sequence_offset() {
        let mut queue = InputQueue::new();
        queue.enqueue(Tick(99), request_action(9, 9, &[9]));

        let recorded = vec![
            crate::InputEvent {
                scheduled_tick: Tick(4),
                sequence_no: 7,
                kind: request_action(1, 0, &[2]),
            },
            crate::InputEvent {
                scheduled_tick: Tick(2),
                sequence_no: 8,
                kind: InputKind::SwitchControl {
                    from: None,
                    to: Some(entity(4)),
                },
            },
            crate::InputEvent {
                scheduled_tick: Tick(4),
                sequence_no: 9,
                kind: InputKind::CancelAction {
                    actor: entity(3),
                    action_instance_id: ActionInstanceId(11),
                },
            },
        ];

        queue.replace_with_recorded(&recorded).unwrap();

        assert_eq!(queue.next_sequence_no(), 10);
        assert_eq!(queue.peek_tick(Tick(2)), &[recorded[1].clone()]);
        assert_eq!(
            queue.peek_tick(Tick(4)),
            &[recorded[0].clone(), recorded[2].clone()]
        );
    }

    #[test]
    fn replace_with_recorded_rejects_non_monotonic_sequence_numbers() {
        let mut queue = InputQueue::new();
        let recorded = vec![
            crate::InputEvent {
                scheduled_tick: Tick(4),
                sequence_no: 7,
                kind: request_action(1, 0, &[2]),
            },
            crate::InputEvent {
                scheduled_tick: Tick(2),
                sequence_no: 7,
                kind: InputKind::SwitchControl {
                    from: None,
                    to: Some(entity(4)),
                },
            },
        ];

        let error = queue.replace_with_recorded(&recorded).unwrap_err();

        assert_eq!(
            error,
            InputQueueError::NonMonotonicSequence {
                previous_tick: Tick(4),
                previous_sequence_no: 7,
                attempted_tick: Tick(2),
                attempted_sequence_no: 7,
            }
        );
        assert!(queue.is_empty());
        assert_eq!(queue.next_sequence_no(), 0);
    }

    #[test]
    fn iter_in_sequence_order_uses_global_input_sequence_not_tick_bucket_order() {
        let mut queue = InputQueue::new();
        let late_tick = queue.enqueue(Tick(5), request_action(1, 0, &[2])).clone();
        let early_tick_first = queue
            .enqueue(
                Tick(3),
                InputKind::SwitchControl {
                    from: None,
                    to: Some(entity(4)),
                },
            )
            .clone();
        let early_tick_second = queue
            .enqueue(
                Tick(3),
                InputKind::CancelAction {
                    actor: entity(3),
                    action_instance_id: ActionInstanceId(9),
                },
            )
            .clone();

        let iterated = queue.iter_in_sequence_order().cloned().collect::<Vec<_>>();

        assert_eq!(
            iterated,
            vec![late_tick, early_tick_first, early_tick_second]
        );
    }
}
