//! Append-only event storage and deterministic tick indexing.

use crate::{EventRecord, PendingEvent};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldwake_core::{EventId, Tick};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EventLog {
    events: Vec<EventRecord>,
    next_id: EventId,
    by_tick: BTreeMap<Tick, Vec<EventId>>,
}

impl EventLog {
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_id: EventId(0),
            by_tick: BTreeMap::new(),
        }
    }

    pub fn emit(&mut self, pending: PendingEvent) -> EventId {
        let event_id = self.next_id;
        let record = pending.into_record(event_id);
        let tick = record.tick;
        self.events.push(record);
        self.by_tick.entry(tick).or_default().push(event_id);
        self.next_id = EventId(self.next_id.0 + 1);

        event_id
    }

    #[must_use]
    pub fn get(&self, id: EventId) -> Option<&EventRecord> {
        usize::try_from(id.0)
            .ok()
            .and_then(|index| self.events.get(index))
    }

    #[must_use]
    pub fn events_at_tick(&self, tick: Tick) -> &[EventId] {
        self.by_tick.get(&tick).map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for EventLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::EventLog;
    use crate::{CauseRef, EventRecord, EventTag, PendingEvent, VisibilitySpec, WitnessData};
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::BTreeSet;
    use std::fmt::Debug;
    use worldwake_core::{EntityId, EventId, Tick};

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn pending(tick: Tick) -> PendingEvent {
        PendingEvent::new(
            tick,
            CauseRef::Bootstrap,
            Some(entity(1)),
            vec![entity(3), entity(2)],
            Some(entity(4)),
            Vec::new(),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
            BTreeSet::from([EventTag::WorldMutation]),
        )
    }

    fn assert_traits<T: Clone + Debug + Eq + Serialize + DeserializeOwned>() {}

    #[test]
    fn event_log_satisfies_required_traits() {
        assert_traits::<EventLog>();
    }

    #[test]
    fn new_log_starts_empty_with_zero_next_id() {
        let log = EventLog::new();

        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
        assert_eq!(log.events_at_tick(Tick(0)), &[]);
        assert_eq!(log.get(EventId(0)), None);
    }

    #[test]
    fn emit_assigns_gapless_ids_in_append_order() {
        let mut log = EventLog::new();

        let first_id = log.emit(pending(Tick(3)));
        let second_id = log.emit(pending(Tick(3)));
        let third_id = log.emit(pending(Tick(5)));

        assert_eq!(first_id, EventId(0));
        assert_eq!(second_id, EventId(1));
        assert_eq!(third_id, EventId(2));
        assert_eq!(log.len(), 3);
        assert_eq!(log.events_at_tick(Tick(3)), &[EventId(0), EventId(1)]);
        assert_eq!(log.events_at_tick(Tick(5)), &[EventId(2)]);
        assert_eq!(
            log.get(EventId(0)).map(|record| record.event_id),
            Some(EventId(0))
        );
        assert_eq!(
            log.get(EventId(1)).map(|record| record.event_id),
            Some(EventId(1))
        );
        assert_eq!(
            log.get(EventId(2)).map(|record| record.event_id),
            Some(EventId(2))
        );
    }

    #[test]
    fn emit_assigns_event_id_inside_stored_record() {
        let mut log = EventLog::new();
        let pending = pending(Tick(2));

        let event_id = log.emit(pending.clone());
        let stored = log.get(event_id).unwrap();

        assert_eq!(event_id, EventId(0));
        assert_eq!(stored.event_id, EventId(0));
        assert_eq!(stored.tick, pending.tick);
        assert_eq!(stored.cause, pending.cause);
        assert_eq!(stored.actor_id, pending.actor_id);
        assert_eq!(stored.target_ids, pending.target_ids);
        assert_eq!(stored.place_id, pending.place_id);
        assert_eq!(stored.state_deltas, pending.state_deltas);
        assert_eq!(stored.visibility, pending.visibility);
        assert_eq!(stored.witness_data, pending.witness_data);
        assert_eq!(stored.tags, pending.tags);
    }

    #[test]
    fn get_returns_emitted_record_and_none_for_out_of_bounds_ids() {
        let mut log = EventLog::new();
        let expected = EventRecord::new(
            EventId(0),
            Tick(8),
            CauseRef::Bootstrap,
            Some(entity(1)),
            vec![entity(3), entity(2)],
            Some(entity(4)),
            Vec::new(),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
            BTreeSet::from([EventTag::WorldMutation]),
        );
        log.emit(pending(Tick(8)));

        assert_eq!(log.get(EventId(0)), Some(&expected));
        assert_eq!(log.get(EventId(1)), None);
    }

    #[test]
    fn events_at_tick_returns_empty_slice_when_tick_has_no_events() {
        let mut log = EventLog::new();
        log.emit(pending(Tick(1)));

        assert_eq!(log.events_at_tick(Tick(9)), &[]);
    }

    #[test]
    fn event_log_roundtrips_through_bincode_with_populated_tick_index() {
        let mut log = EventLog::new();
        log.emit(pending(Tick(2)));
        log.emit(pending(Tick(2)));
        log.emit(pending(Tick(4)));

        let bytes = bincode::serialize(&log).unwrap();
        let roundtrip: EventLog = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, log);
        assert_eq!(roundtrip.len(), 3);
        assert_eq!(
            roundtrip.get(EventId(2)).map(|record| record.event_id),
            Some(EventId(2))
        );
        assert_eq!(roundtrip.events_at_tick(Tick(2)), &[EventId(0), EventId(1)]);
        assert_eq!(roundtrip.events_at_tick(Tick(4)), &[EventId(2)]);
    }
}
