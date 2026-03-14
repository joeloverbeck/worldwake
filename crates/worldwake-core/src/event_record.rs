//! Immutable append-only event payloads.

use crate::WoundId;
use crate::{CauseRef, EventTag, MismatchKind, ObservedEntitySnapshot, StateDelta, VisibilitySpec, WitnessData};
use crate::{EntityId, EventId, Tick};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum EvidenceRef {
    Wound { entity: EntityId, wound_id: WoundId },
    Mismatch {
        observer: EntityId,
        subject: EntityId,
        kind: MismatchKind,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PendingEvent {
    pub tick: Tick,
    pub cause: CauseRef,
    pub actor_id: Option<EntityId>,
    pub target_ids: Vec<EntityId>,
    pub evidence: Vec<EvidenceRef>,
    pub place_id: Option<EntityId>,
    pub state_deltas: Vec<StateDelta>,
    pub observed_entities: BTreeMap<EntityId, ObservedEntitySnapshot>,
    pub visibility: VisibilitySpec,
    pub witness_data: WitnessData,
    pub tags: BTreeSet<EventTag>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EventRecord {
    pub event_id: EventId,
    pub tick: Tick,
    pub cause: CauseRef,
    pub actor_id: Option<EntityId>,
    pub target_ids: Vec<EntityId>,
    pub evidence: Vec<EvidenceRef>,
    pub place_id: Option<EntityId>,
    pub state_deltas: Vec<StateDelta>,
    pub observed_entities: BTreeMap<EntityId, ObservedEntitySnapshot>,
    pub visibility: VisibilitySpec,
    pub witness_data: WitnessData,
    pub tags: BTreeSet<EventTag>,
}

impl PendingEvent {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tick: Tick,
        cause: CauseRef,
        actor_id: Option<EntityId>,
        mut target_ids: Vec<EntityId>,
        place_id: Option<EntityId>,
        state_deltas: Vec<StateDelta>,
        visibility: VisibilitySpec,
        witness_data: WitnessData,
        tags: BTreeSet<EventTag>,
    ) -> Self {
        target_ids.sort();
        target_ids.dedup();

        Self {
            tick,
            cause,
            actor_id,
            target_ids,
            evidence: Vec::new(),
            place_id,
            state_deltas,
            observed_entities: BTreeMap::new(),
            visibility,
            witness_data,
            tags,
        }
    }

    #[must_use]
    pub fn with_evidence(mut self, mut evidence: Vec<EvidenceRef>) -> Self {
        evidence.sort();
        evidence.dedup();
        self.evidence = evidence;
        self
    }

    #[must_use]
    pub fn with_observed_entities(
        mut self,
        observed_entities: BTreeMap<EntityId, ObservedEntitySnapshot>,
    ) -> Self {
        self.observed_entities = observed_entities;
        self
    }

    #[must_use]
    pub fn into_record(self, event_id: EventId) -> EventRecord {
        EventRecord {
            event_id,
            tick: self.tick,
            cause: self.cause,
            actor_id: self.actor_id,
            target_ids: self.target_ids,
            evidence: self.evidence,
            place_id: self.place_id,
            state_deltas: self.state_deltas,
            observed_entities: self.observed_entities,
            visibility: self.visibility,
            witness_data: self.witness_data,
            tags: self.tags,
        }
    }
}

impl EventRecord {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_id: EventId,
        tick: Tick,
        cause: CauseRef,
        actor_id: Option<EntityId>,
        target_ids: Vec<EntityId>,
        place_id: Option<EntityId>,
        state_deltas: Vec<StateDelta>,
        visibility: VisibilitySpec,
        witness_data: WitnessData,
        tags: BTreeSet<EventTag>,
    ) -> Self {
        PendingEvent::new(
            tick,
            cause,
            actor_id,
            target_ids,
            place_id,
            state_deltas,
            visibility,
            witness_data,
            tags,
        )
        .into_record(event_id)
    }
}

#[cfg(test)]
mod tests {
    use super::{EventRecord, EvidenceRef, PendingEvent};
    use crate::{
        CauseRef, ComponentDelta, ComponentKind, ComponentValue, EventTag, QuantityDelta,
        RelationDelta, RelationKind, RelationValue, ReservationDelta, StateDelta, VisibilitySpec,
        WitnessData,
    };
    use crate::{
        CommodityKind, EntityId, EntityKind, EventId, Name, ObservedEntitySnapshot, Quantity,
        ReservationId, ReservationRecord, Tick, TickRange, WoundId,
    };
    use crate::MismatchKind;
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::{BTreeMap, BTreeSet};
    use std::fmt::Debug;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn reservation_record() -> ReservationRecord {
        ReservationRecord {
            id: ReservationId(3),
            entity: entity(8),
            reserver: entity(9),
            range: TickRange::new(Tick(12), Tick(15)).unwrap(),
        }
    }

    fn assert_traits<T: Clone + Debug + Eq + Serialize + DeserializeOwned>() {}

    #[test]
    fn event_record_satisfies_required_traits() {
        assert_traits::<EventRecord>();
    }

    #[test]
    fn pending_event_satisfies_required_traits() {
        assert_traits::<PendingEvent>();
    }

    #[test]
    fn pending_event_constructs_with_all_required_fields() {
        let pending = PendingEvent::new(
            Tick(9),
            CauseRef::Event(EventId(1)),
            Some(entity(2)),
            vec![entity(5), entity(3), entity(5), entity(4)],
            Some(entity(6)),
            vec![
                StateDelta::Entity(crate::EntityDelta::Created {
                    entity: entity(7),
                    kind: EntityKind::Agent,
                }),
                StateDelta::Quantity(QuantityDelta::Changed {
                    entity: entity(7),
                    commodity: CommodityKind::Bread,
                    before: Quantity(1),
                    after: Quantity(2),
                }),
            ],
            VisibilitySpec::SamePlace,
            WitnessData {
                direct_witnesses: BTreeSet::from([entity(2)]),
                potential_witnesses: BTreeSet::from([entity(2), entity(10)]),
            },
            BTreeSet::from([EventTag::WorldMutation, EventTag::System]),
        );

        assert_eq!(pending.tick, Tick(9));
        assert_eq!(pending.cause, CauseRef::Event(EventId(1)));
        assert_eq!(pending.actor_id, Some(entity(2)));
        assert_eq!(pending.target_ids, vec![entity(3), entity(4), entity(5)]);
        assert!(pending.evidence.is_empty());
        assert_eq!(pending.place_id, Some(entity(6)));
        assert_eq!(pending.state_deltas.len(), 2);
        assert!(pending.observed_entities.is_empty());
        assert_eq!(
            pending.tags.iter().copied().collect::<Vec<_>>(),
            vec![EventTag::WorldMutation, EventTag::System]
        );
    }

    #[test]
    fn event_record_constructs_with_all_required_fields() {
        let record = PendingEvent::new(
            Tick(9),
            CauseRef::Event(EventId(1)),
            Some(entity(2)),
            vec![entity(5), entity(3), entity(5), entity(4)],
            Some(entity(6)),
            vec![
                StateDelta::Entity(crate::EntityDelta::Created {
                    entity: entity(7),
                    kind: EntityKind::Agent,
                }),
                StateDelta::Quantity(QuantityDelta::Changed {
                    entity: entity(7),
                    commodity: CommodityKind::Bread,
                    before: Quantity(1),
                    after: Quantity(2),
                }),
            ],
            VisibilitySpec::SamePlace,
            WitnessData {
                direct_witnesses: BTreeSet::from([entity(2)]),
                potential_witnesses: BTreeSet::from([entity(2), entity(10)]),
            },
            BTreeSet::from([EventTag::WorldMutation, EventTag::System]),
        )
        .into_record(EventId(4));

        assert_eq!(record.event_id, EventId(4));
        assert_eq!(record.tick, Tick(9));
        assert_eq!(record.cause, CauseRef::Event(EventId(1)));
        assert_eq!(record.actor_id, Some(entity(2)));
        assert_eq!(record.target_ids, vec![entity(3), entity(4), entity(5)]);
        assert!(record.evidence.is_empty());
        assert_eq!(record.place_id, Some(entity(6)));
        assert_eq!(record.state_deltas.len(), 2);
        assert!(record.observed_entities.is_empty());
        assert_eq!(
            record.tags.iter().copied().collect::<Vec<_>>(),
            vec![EventTag::WorldMutation, EventTag::System]
        );
    }

    #[test]
    fn event_record_allows_empty_deltas_and_targets() {
        let record = PendingEvent::new(
            Tick(0),
            CauseRef::Bootstrap,
            None,
            Vec::new(),
            None,
            Vec::new(),
            VisibilitySpec::Hidden,
            WitnessData::default(),
            BTreeSet::new(),
        )
        .into_record(EventId(0));

        assert!(record.target_ids.is_empty());
        assert!(record.evidence.is_empty());
        assert!(record.state_deltas.is_empty());
        assert!(record.observed_entities.is_empty());
        assert!(record.tags.is_empty());
    }

    #[test]
    fn pending_event_roundtrips_through_bincode_with_ordered_deltas() {
        let pending = PendingEvent::new(
            Tick(18),
            CauseRef::SystemTick(Tick(18)),
            Some(entity(1)),
            vec![entity(4), entity(2), entity(4), entity(3)],
            Some(entity(6)),
            vec![
                StateDelta::Component(ComponentDelta::Set {
                    entity: entity(1),
                    component_kind: ComponentKind::Name,
                    before: Some(ComponentValue::Name(Name("Old".to_string()))),
                    after: ComponentValue::Name(Name("New".to_string())),
                }),
                StateDelta::Relation(RelationDelta::Added {
                    relation_kind: RelationKind::HostileTo,
                    relation: RelationValue::HostileTo {
                        subject: entity(1),
                        target: entity(22),
                    },
                }),
                StateDelta::Reservation(ReservationDelta::Created {
                    reservation: reservation_record(),
                }),
            ],
            VisibilitySpec::AdjacentPlaces { max_hops: 2 },
            WitnessData {
                direct_witnesses: BTreeSet::from([entity(1), entity(2)]),
                potential_witnesses: BTreeSet::from([entity(1), entity(2), entity(3)]),
            },
            BTreeSet::from([EventTag::ActionCommitted, EventTag::Travel]),
        )
        .with_evidence(vec![
            EvidenceRef::Wound {
                entity: entity(1),
                wound_id: WoundId(2),
            },
            EvidenceRef::Wound {
                entity: entity(1),
                wound_id: WoundId(1),
            },
            EvidenceRef::Wound {
                entity: entity(1),
                wound_id: WoundId(2),
            },
        ]);

        let bytes = bincode::serialize(&pending).unwrap();
        let roundtrip: PendingEvent = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, pending);
        assert_eq!(roundtrip.target_ids, vec![entity(2), entity(3), entity(4)]);
        assert_eq!(
            roundtrip.evidence,
            vec![
                EvidenceRef::Wound {
                    entity: entity(1),
                    wound_id: WoundId(1),
                },
                EvidenceRef::Wound {
                    entity: entity(1),
                    wound_id: WoundId(2),
                },
            ]
        );
        assert!(matches!(
            roundtrip.state_deltas[0],
            StateDelta::Component(ComponentDelta::Set { .. })
        ));
        assert!(matches!(
            roundtrip.state_deltas[1],
            StateDelta::Relation(RelationDelta::Added { .. })
        ));
        assert!(matches!(
            roundtrip.state_deltas[2],
            StateDelta::Reservation(ReservationDelta::Created { .. })
        ));
    }

    #[test]
    fn pending_event_orders_and_deduplicates_mismatch_evidence() {
        let pending = PendingEvent::new(
            Tick(21),
            CauseRef::Bootstrap,
            Some(entity(1)),
            vec![entity(2)],
            Some(entity(7)),
            Vec::new(),
            VisibilitySpec::ParticipantsOnly,
            WitnessData::default(),
            BTreeSet::from([EventTag::Discovery]),
        )
        .with_evidence(vec![
            EvidenceRef::Mismatch {
                observer: entity(3),
                subject: entity(4),
                kind: MismatchKind::InventoryDiscrepancy {
                    commodity: CommodityKind::Bread,
                    believed: Quantity(5),
                    observed: Quantity(2),
                },
            },
            EvidenceRef::Wound {
                entity: entity(9),
                wound_id: WoundId(2),
            },
            EvidenceRef::Mismatch {
                observer: entity(3),
                subject: entity(4),
                kind: MismatchKind::InventoryDiscrepancy {
                    commodity: CommodityKind::Bread,
                    believed: Quantity(5),
                    observed: Quantity(2),
                },
            },
            EvidenceRef::Mismatch {
                observer: entity(3),
                subject: entity(4),
                kind: MismatchKind::AliveStatusChanged,
            },
        ]);

        assert_eq!(
            pending.evidence,
            vec![
                EvidenceRef::Wound {
                    entity: entity(9),
                    wound_id: WoundId(2),
                },
                EvidenceRef::Mismatch {
                    observer: entity(3),
                    subject: entity(4),
                    kind: MismatchKind::AliveStatusChanged,
                },
                EvidenceRef::Mismatch {
                    observer: entity(3),
                    subject: entity(4),
                    kind: MismatchKind::InventoryDiscrepancy {
                        commodity: CommodityKind::Bread,
                        believed: Quantity(5),
                        observed: Quantity(2),
                    },
                },
            ]
        );
    }

    #[test]
    fn pending_event_roundtrips_with_observed_entities() {
        let pending = PendingEvent::new(
            Tick(7),
            CauseRef::Bootstrap,
            Some(entity(1)),
            vec![entity(2)],
            Some(entity(3)),
            Vec::new(),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
            BTreeSet::from([EventTag::WorldMutation]),
        )
        .with_observed_entities(BTreeMap::from([(
            entity(2),
            ObservedEntitySnapshot {
                last_known_place: Some(entity(3)),
                last_known_inventory: BTreeMap::from([(CommodityKind::Bread, Quantity(2))]),
                workstation_tag: None,
                resource_source: None,
                alive: true,
                wounds: Vec::new(),
            },
        )]));

        let bytes = bincode::serialize(&pending).unwrap();
        let roundtrip: PendingEvent = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, pending);
        assert_eq!(
            roundtrip.observed_entities.get(&entity(2)).unwrap().last_known_inventory,
            BTreeMap::from([(CommodityKind::Bread, Quantity(2))])
        );
    }

    #[test]
    fn event_record_roundtrips_through_bincode_with_ordered_deltas() {
        let record = PendingEvent::new(
            Tick(18),
            CauseRef::SystemTick(Tick(18)),
            Some(entity(1)),
            vec![entity(4), entity(2), entity(4), entity(3)],
            Some(entity(6)),
            vec![
                StateDelta::Component(ComponentDelta::Set {
                    entity: entity(1),
                    component_kind: ComponentKind::Name,
                    before: Some(ComponentValue::Name(Name("Old".to_string()))),
                    after: ComponentValue::Name(Name("New".to_string())),
                }),
                StateDelta::Relation(RelationDelta::Added {
                    relation_kind: RelationKind::HostileTo,
                    relation: RelationValue::HostileTo {
                        subject: entity(1),
                        target: entity(22),
                    },
                }),
                StateDelta::Reservation(ReservationDelta::Created {
                    reservation: reservation_record(),
                }),
            ],
            VisibilitySpec::AdjacentPlaces { max_hops: 2 },
            WitnessData {
                direct_witnesses: BTreeSet::from([entity(1), entity(2)]),
                potential_witnesses: BTreeSet::from([entity(1), entity(2), entity(3)]),
            },
            BTreeSet::from([EventTag::ActionCommitted, EventTag::Travel]),
        )
        .with_evidence(vec![EvidenceRef::Wound {
            entity: entity(1),
            wound_id: WoundId(4),
        }])
        .into_record(EventId(12));

        let bytes = bincode::serialize(&record).unwrap();
        let roundtrip: EventRecord = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, record);
        assert_eq!(roundtrip.target_ids, vec![entity(2), entity(3), entity(4)]);
        assert_eq!(roundtrip.evidence, record.evidence);
        assert!(matches!(
            roundtrip.state_deltas[0],
            StateDelta::Component(ComponentDelta::Set { .. })
        ));
        assert!(matches!(
            roundtrip.state_deltas[1],
            StateDelta::Relation(RelationDelta::Added { .. })
        ));
        assert!(matches!(
            roundtrip.state_deltas[2],
            StateDelta::Reservation(ReservationDelta::Created { .. })
        ));
    }

    #[test]
    fn event_record_roundtrips_with_mismatch_evidence() {
        let record = PendingEvent::new(
            Tick(22),
            CauseRef::SystemTick(Tick(22)),
            Some(entity(3)),
            vec![entity(8)],
            Some(entity(7)),
            Vec::new(),
            VisibilitySpec::ParticipantsOnly,
            WitnessData {
                direct_witnesses: BTreeSet::from([entity(3)]),
                potential_witnesses: BTreeSet::from([entity(3)]),
            },
            BTreeSet::from([EventTag::Discovery, EventTag::WorldMutation]),
        )
        .with_evidence(vec![EvidenceRef::Mismatch {
            observer: entity(3),
            subject: entity(8),
            kind: MismatchKind::PlaceChanged {
                believed_place: entity(5),
                observed_place: entity(7),
            },
        }])
        .into_record(EventId(14));

        let bytes = bincode::serialize(&record).unwrap();
        let roundtrip: EventRecord = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, record);
        assert_eq!(
            roundtrip.evidence,
            vec![EvidenceRef::Mismatch {
                observer: entity(3),
                subject: entity(8),
                kind: MismatchKind::PlaceChanged {
                    believed_place: entity(5),
                    observed_place: entity(7),
                },
            }]
        );
    }
}
