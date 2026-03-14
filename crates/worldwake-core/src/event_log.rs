//! Append-only event storage, deterministic indexing, and causal traversal.

use crate::{CauseRef, EventRecord, EventTag, PendingEvent};
use crate::{EntityId, EventId, Tick};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EventLog {
    events: Vec<EventRecord>,
    next_id: EventId,
    by_tick: BTreeMap<Tick, Vec<EventId>>,
    by_actor: BTreeMap<EntityId, Vec<EventId>>,
    by_place: BTreeMap<EntityId, Vec<EventId>>,
    by_tag: BTreeMap<EventTag, Vec<EventId>>,
    by_cause: BTreeMap<EventId, Vec<EventId>>,
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
            by_cause: BTreeMap::new(),
        }
    }

    pub fn emit(&mut self, pending: PendingEvent) -> EventId {
        let event_id = self.next_id;
        let record = pending.into_record(event_id);
        let cause = record.payload.cause;
        let tick = record.payload.tick;
        let actor_id = record.payload.actor_id;
        let place_id = record.payload.place_id;
        let tags: Vec<_> = record.payload.tags.iter().copied().collect();

        if let CauseRef::Event(cause_id) = cause {
            assert!(
                cause_id < event_id,
                "event cause {cause_id:?} must precede emitted event {event_id:?}"
            );
            assert!(
                self.get(cause_id).is_some(),
                "event cause {cause_id:?} must exist before emitted event {event_id:?}"
            );
        }

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
        if let CauseRef::Event(cause_id) = cause {
            self.by_cause.entry(cause_id).or_default().push(event_id);
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
    pub fn get_effects(&self, event_id: EventId) -> &[EventId] {
        self.by_cause.get(&event_id).map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn trace_event_cause(&self, event_id: EventId) -> Vec<EventId> {
        let mut ancestors = Vec::new();
        let mut current_id = event_id;

        while let Some(record) = self.get(current_id) {
            match record.payload.cause {
                CauseRef::Event(cause_id) => {
                    debug_assert!(
                        cause_id < current_id,
                        "validated event log contains non-backward cause {current_id:?} -> {cause_id:?}"
                    );
                    ancestors.push(cause_id);
                    current_id = cause_id;
                }
                CauseRef::Bootstrap | CauseRef::SystemTick(_) | CauseRef::ExternalInput(_) => {
                    break;
                }
            }
        }

        ancestors.reverse();
        ancestors
    }

    #[must_use]
    pub fn causal_depth(&self, event_id: EventId) -> u32 {
        self.trace_event_cause(event_id)
            .len()
            .try_into()
            .expect("causal chain length exceeds u32")
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn from_records_for_test(events: Vec<EventRecord>) -> Self {
        let mut log = Self {
            events,
            next_id: EventId(0),
            by_tick: BTreeMap::new(),
            by_actor: BTreeMap::new(),
            by_place: BTreeMap::new(),
            by_tag: BTreeMap::new(),
            by_cause: BTreeMap::new(),
        };

        for record in &log.events {
            let event_id = record.event_id;
            log.by_tick
                .entry(record.payload.tick)
                .or_default()
                .push(event_id);
            if let Some(actor_id) = record.payload.actor_id {
                log.by_actor.entry(actor_id).or_default().push(event_id);
            }
            if let Some(place_id) = record.payload.place_id {
                log.by_place.entry(place_id).or_default().push(event_id);
            }
            for tag in &record.payload.tags {
                log.by_tag.entry(*tag).or_default().push(event_id);
            }
            if let CauseRef::Event(cause_id) = record.payload.cause {
                log.by_cause.entry(cause_id).or_default().push(event_id);
            }
        }

        log.next_id =
            EventId(u64::try_from(log.events.len()).expect("test event log length exceeds u64"));
        log
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
    use crate::{
        CauseRef, EventPayload, EventRecord, EventTag, PendingEvent, VisibilitySpec, WitnessData,
    };
    use crate::{EntityId, EventId, Tick};
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::{BTreeMap, BTreeSet};
    use std::fmt::Debug;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn pending(tick: Tick) -> PendingEvent {
        pending_with_cause(tick, CauseRef::Bootstrap)
    }

    fn pending_with_cause(tick: Tick, cause: CauseRef) -> PendingEvent {
        PendingEvent::from_payload(EventPayload {
            tick,
            cause,
            actor_id: Some(entity(1)),
            target_ids: vec![entity(3), entity(2)],
            evidence: Vec::new(),
            place_id: Some(entity(4)),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData::default(),
            tags: BTreeSet::from([EventTag::WorldMutation]),
        })
    }

    fn pending_with_metadata(
        tick: Tick,
        cause: CauseRef,
        actor_id: Option<EntityId>,
        place_id: Option<EntityId>,
        tags: BTreeSet<EventTag>,
    ) -> PendingEvent {
        PendingEvent::from_payload(EventPayload {
            tick,
            cause,
            actor_id,
            target_ids: vec![entity(3), entity(2)],
            evidence: Vec::new(),
            place_id,
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData::default(),
            tags,
        })
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
        assert_eq!(stored.payload.tick, pending.payload.tick);
        assert_eq!(stored.payload.cause, pending.payload.cause);
        assert_eq!(stored.payload.actor_id, pending.payload.actor_id);
        assert_eq!(stored.payload.target_ids, pending.payload.target_ids);
        assert_eq!(stored.payload.place_id, pending.payload.place_id);
        assert_eq!(stored.payload.state_deltas, pending.payload.state_deltas);
        assert_eq!(stored.payload.visibility, pending.payload.visibility);
        assert_eq!(stored.payload.witness_data, pending.payload.witness_data);
        assert_eq!(stored.payload.tags, pending.payload.tags);
    }

    #[test]
    fn get_returns_emitted_record_and_none_for_out_of_bounds_ids() {
        let mut log = EventLog::new();
        let expected = EventRecord::from_payload(
            EventId(0),
            EventPayload {
                tick: Tick(8),
                cause: CauseRef::Bootstrap,
                actor_id: Some(entity(1)),
                target_ids: vec![entity(3), entity(2)],
                evidence: Vec::new(),
                place_id: Some(entity(4)),
                state_deltas: Vec::new(),
                observed_entities: BTreeMap::new(),
                visibility: VisibilitySpec::SamePlace,
                witness_data: WitnessData::default(),
                tags: BTreeSet::from([EventTag::WorldMutation]),
            },
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
            CauseRef::Bootstrap,
            Some(entity(7)),
            Some(entity(20)),
            BTreeSet::from([EventTag::WorldMutation]),
        ));
        log.emit(pending_with_metadata(
            Tick(2),
            CauseRef::Bootstrap,
            None,
            Some(entity(20)),
            BTreeSet::from([EventTag::System]),
        ));
        let third = log.emit(pending_with_metadata(
            Tick(3),
            CauseRef::Bootstrap,
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
            CauseRef::Bootstrap,
            Some(entity(7)),
            Some(entity(30)),
            BTreeSet::from([EventTag::WorldMutation]),
        ));
        log.emit(pending_with_metadata(
            Tick(2),
            CauseRef::Bootstrap,
            Some(entity(8)),
            None,
            BTreeSet::from([EventTag::System]),
        ));
        let third = log.emit(pending_with_metadata(
            Tick(3),
            CauseRef::Bootstrap,
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
            CauseRef::Bootstrap,
            Some(entity(7)),
            Some(entity(30)),
            BTreeSet::from([EventTag::System, EventTag::WorldMutation]),
        ));
        let second = log.emit(pending_with_metadata(
            Tick(2),
            CauseRef::Bootstrap,
            Some(entity(8)),
            Some(entity(31)),
            BTreeSet::from([EventTag::Travel]),
        ));
        let third = log.emit(pending_with_metadata(
            Tick(3),
            CauseRef::Bootstrap,
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
            CauseRef::Bootstrap,
            Some(entity(7)),
            Some(entity(30)),
            BTreeSet::from([EventTag::System, EventTag::WorldMutation]),
        ));
        log.emit(pending_with_metadata(
            Tick(2),
            CauseRef::Bootstrap,
            Some(entity(7)),
            None,
            BTreeSet::from([EventTag::Travel]),
        ));
        log.emit(pending_with_metadata(
            Tick(4),
            CauseRef::Bootstrap,
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

    #[test]
    fn trace_event_cause_returns_empty_for_explicit_root_causes() {
        let mut log = EventLog::new();

        let bootstrap = log.emit(pending_with_cause(Tick(1), CauseRef::Bootstrap));
        let tick = log.emit(pending_with_cause(Tick(2), CauseRef::SystemTick(Tick(2))));
        let input = log.emit(pending_with_cause(Tick(3), CauseRef::ExternalInput(9)));

        assert_eq!(log.trace_event_cause(bootstrap), Vec::<EventId>::new());
        assert_eq!(log.trace_event_cause(tick), Vec::<EventId>::new());
        assert_eq!(log.trace_event_cause(input), Vec::<EventId>::new());
        assert_eq!(log.causal_depth(bootstrap), 0);
        assert_eq!(log.causal_depth(tick), 0);
        assert_eq!(log.causal_depth(input), 0);
    }

    #[test]
    fn trace_event_cause_and_causal_depth_follow_a_linear_event_chain() {
        let mut log = EventLog::new();

        let root = log.emit(pending_with_cause(Tick(1), CauseRef::Bootstrap));
        let middle = log.emit(pending_with_cause(Tick(2), CauseRef::Event(root)));
        let leaf = log.emit(pending_with_cause(Tick(3), CauseRef::Event(middle)));

        assert_eq!(log.trace_event_cause(leaf), vec![root, middle]);
        assert_eq!(log.trace_event_cause(middle), vec![root]);
        assert_eq!(log.causal_depth(leaf), 2);
        assert_eq!(log.causal_depth(middle), 1);
    }

    #[test]
    fn get_effects_returns_direct_effects_only_in_emission_order() {
        let mut log = EventLog::new();

        let root = log.emit(pending_with_cause(Tick(1), CauseRef::Bootstrap));
        let first_child = log.emit(pending_with_cause(Tick(2), CauseRef::Event(root)));
        let second_child = log.emit(pending_with_cause(Tick(3), CauseRef::Event(root)));
        let grandchild = log.emit(pending_with_cause(Tick(4), CauseRef::Event(first_child)));

        assert_eq!(log.get_effects(root), &[first_child, second_child]);
        assert_eq!(log.get_effects(first_child), &[grandchild]);
        assert_eq!(log.get_effects(second_child), &[]);
        assert_eq!(log.get_effects(grandchild), &[]);
    }

    #[test]
    fn event_log_roundtrips_through_bincode_with_cause_index_and_traversal() {
        let mut log = EventLog::new();
        let root = log.emit(pending_with_cause(Tick(1), CauseRef::Bootstrap));
        let child = log.emit(pending_with_cause(Tick(2), CauseRef::Event(root)));
        let sibling = log.emit(pending_with_cause(Tick(3), CauseRef::Event(root)));

        let bytes = bincode::serialize(&log).unwrap();
        let roundtrip: EventLog = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, log);
        assert_eq!(roundtrip.get_effects(root), &[child, sibling]);
        assert_eq!(roundtrip.trace_event_cause(child), vec![root]);
        assert_eq!(roundtrip.trace_event_cause(sibling), vec![root]);
    }

    #[test]
    #[should_panic(expected = "must exist before emitted event")]
    fn emit_rejects_missing_cause_event() {
        let mut log = EventLog::new();
        log.emit(pending_with_cause(Tick(1), CauseRef::Bootstrap));
        log.next_id = EventId(2);

        log.emit(pending_with_cause(Tick(2), CauseRef::Event(EventId(1))));
    }

    #[test]
    #[should_panic(expected = "must precede emitted event")]
    fn emit_rejects_self_or_future_cause_event() {
        let mut log = EventLog::new();
        let root = log.emit(pending_with_cause(Tick(1), CauseRef::Bootstrap));

        log.emit(pending_with_cause(
            Tick(2),
            CauseRef::Event(EventId(root.0 + 1)),
        ));
    }
}
