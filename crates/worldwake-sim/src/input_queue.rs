use crate::{InputEvent, InputKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldwake_core::Tick;

static EMPTY_INPUT_EVENTS: [InputEvent; 0] = [];

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct InputQueue {
    next_sequence_no: u64,
    events_by_tick: BTreeMap<Tick, Vec<InputEvent>>,
}

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

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events_by_tick.is_empty()
    }

    #[must_use]
    pub fn next_sequence_no(&self) -> u64 {
        self.next_sequence_no
    }
}

#[cfg(test)]
mod tests {
    use super::InputQueue;
    use crate::{ActionDefId, ActionInstanceId, InputKind};
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
        }
    }

    #[test]
    fn new_queue_starts_empty_with_zero_next_sequence() {
        let queue = InputQueue::new();

        assert!(queue.is_empty());
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
        assert_eq!(queue.next_sequence_no(), 3);
        assert_eq!(
            queue.peek_tick(Tick(5)),
            &[first.clone(), second.clone()]
        );
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
}
