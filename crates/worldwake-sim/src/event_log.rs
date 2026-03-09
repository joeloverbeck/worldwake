//! Append-only event storage and deterministic tick indexing.

use crate::{EventRecord, EventTag, PendingEvent};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldwake_core::{EntityId, EventId, Tick};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EventLog {
    events: Vec<EventRecord>,
    next_id: EventId,
    by_tick: BTreeMap<Tick, Vec<EventId>>,
    by_actor: BTreeMap<EntityId, Vec<EventId>>,
    by_place: BTreeMap<EntityId, Vec<EventId>>,
    by_tag: BTreeMap<EventTag, Vec<EventId>>,
}

impl EventLog {
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_id: EventId(0),
            by_tick: BTreeMap::new(),
            by_actor: BTreeMap::new(),
            by_place: BTreeMap::new(),
            by_tag: BTreeMap::new(),
        }
    }

    pub fn emit(&mut self, pending: PendingEvent) -> EventId {
        let event_id = self.next_id;
        let record = pending.into_record(event_id);
        let tick = record.tick;
        let actor_id = record.actor_id;
        let place_id = record.place_id;
        let tags: Vec<_> = record.tags.iter().copied().collect();
        self.events.push(record);
        self.by_tick.entry(tick).or_default().push(event_id);
        if let Some(actor_id) = actor_id {
            self.by_actor.entry(actor_id).or_default().push(event_id);
        }
        if let Some(place_id) = place_id {
            self.by_place.entry(place_id).or_default().push(event_id);
        }
        for tag in tags {
            self.by_tag.entry(tag).or_default().push(event_id);
        }
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
    pub fn events_by_actor(&self, actor: EntityId) -> &[EventId] {
        self.by_actor.get(&actor).map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn events_by_place(&self, place: EntityId) -> &[EventId] {
        self.by_place.get(&place).map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn events_by_tag(&self, tag: EventTag) -> &[EventId] {
        self.by_tag.get(&tag).map_or(&[], Vec::as_slice)
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

    fn pending_with_metadata(
        tick: Tick,
        actor_id: Option<EntityId>,
        place_id: Option<EntityId>,
        tags: BTreeSet<EventTag>,
    ) -> PendingEvent {
        PendingEvent::new(
            tick,
            CauseRef::Bootstrap,
            actor_id,
            vec![entity(3), entity(2)],
            place_id,
            Vec::new(),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
            tags,
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

    #[test]
    fn events_by_actor_returns_emission_order_and_skips_none_actor_events() {
        let mut log = EventLog::new();

        let first = log.emit(pending_with_metadata(
            Tick(1),
            Some(entity(7)),
            Some(entity(20)),
            BTreeSet::from([EventTag::WorldMutation]),
        ));
        log.emit(pending_with_metadata(
            Tick(2),
            None,
            Some(entity(20)),
            BTreeSet::from([EventTag::System]),
        ));
        let third = log.emit(pending_with_metadata(
            Tick(3),
            Some(entity(7)),
            Some(entity(21)),
            BTreeSet::from([EventTag::Travel]),
        ));

        assert_eq!(log.events_by_actor(entity(7)), &[first, third]);
        assert_eq!(log.events_by_actor(entity(999)), &[]);
    }

    #[test]
    fn events_by_place_returns_emission_order_and_skips_none_place_events() {
        let mut log = EventLog::new();

        let first = log.emit(pending_with_metadata(
            Tick(1),
            Some(entity(7)),
            Some(entity(30)),
            BTreeSet::from([EventTag::WorldMutation]),
        ));
        log.emit(pending_with_metadata(
            Tick(2),
            Some(entity(8)),
            None,
            BTreeSet::from([EventTag::System]),
        ));
        let third = log.emit(pending_with_metadata(
            Tick(3),
            Some(entity(9)),
            Some(entity(30)),
            BTreeSet::from([EventTag::Travel]),
        ));

        assert_eq!(log.events_by_place(entity(30)), &[first, third]);
        assert_eq!(log.events_by_place(entity(999)), &[]);
    }

    #[test]
    fn events_by_tag_indexes_each_tag_and_returns_empty_slice_when_missing() {
        let mut log = EventLog::new();

        let first = log.emit(pending_with_metadata(
            Tick(1),
            Some(entity(7)),
            Some(entity(30)),
            BTreeSet::from([EventTag::System, EventTag::WorldMutation]),
        ));
        let second = log.emit(pending_with_metadata(
            Tick(2),
            Some(entity(8)),
            Some(entity(31)),
            BTreeSet::from([EventTag::Travel]),
        ));
        let third = log.emit(pending_with_metadata(
            Tick(3),
            None,
            None,
            BTreeSet::from([EventTag::System, EventTag::Travel]),
        ));

        assert_eq!(log.events_by_tag(EventTag::WorldMutation), &[first]);
        assert_eq!(log.events_by_tag(EventTag::System), &[first, third]);
        assert_eq!(log.events_by_tag(EventTag::Travel), &[second, third]);
        assert_eq!(log.events_by_tag(EventTag::Combat), &[]);
    }

    #[test]
    fn event_log_roundtrips_through_bincode_with_all_secondary_indices() {
        let mut log = EventLog::new();
        log.emit(pending_with_metadata(
            Tick(2),
            Some(entity(7)),
            Some(entity(30)),
            BTreeSet::from([EventTag::System, EventTag::WorldMutation]),
        ));
        log.emit(pending_with_metadata(
            Tick(2),
            Some(entity(7)),
            None,
            BTreeSet::from([EventTag::Travel]),
        ));
        log.emit(pending_with_metadata(
            Tick(4),
            None,
            Some(entity(30)),
            BTreeSet::from([EventTag::System]),
        ));

        let bytes = bincode::serialize(&log).unwrap();
        let roundtrip: EventLog = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, log);
        assert_eq!(
            roundtrip.events_by_actor(entity(7)),
            &[EventId(0), EventId(1)]
        );
        assert_eq!(
            roundtrip.events_by_place(entity(30)),
            &[EventId(0), EventId(2)]
        );
        assert_eq!(
            roundtrip.events_by_tag(EventTag::System),
            &[EventId(0), EventId(2)]
        );
        assert_eq!(roundtrip.events_by_tag(EventTag::Travel), &[EventId(1)]);
        assert_eq!(roundtrip.events_by_tag(EventTag::Combat), &[]);
    }
}
