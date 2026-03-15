//! Immutable append-only event payloads.

use crate::WoundId;
use crate::{
    CauseRef, EventTag, MismatchKind, ObservedEntitySnapshot, StateDelta, VisibilitySpec,
    WitnessData,
};
use crate::{EntityId, EventId, Tick};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub trait EventView {
    fn tick(&self) -> Tick;
    fn cause(&self) -> CauseRef;
    fn actor_id(&self) -> Option<EntityId>;
    fn target_ids(&self) -> &[EntityId];
    fn evidence(&self) -> &[EvidenceRef];
    fn place_id(&self) -> Option<EntityId>;
    fn state_deltas(&self) -> &[StateDelta];
    fn observed_entities(&self) -> &BTreeMap<EntityId, ObservedEntitySnapshot>;
    fn visibility(&self) -> VisibilitySpec;
    fn witness_data(&self) -> &WitnessData;
    fn tags(&self) -> &BTreeSet<EventTag>;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum EvidenceRef {
    Wound {
        entity: EntityId,
        wound_id: WoundId,
    },
    Mismatch {
        observer: EntityId,
        subject: EntityId,
        kind: MismatchKind,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PendingEvent {
    payload: EventPayload,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EventPayload {
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
    payload: EventPayload,
}

impl EventView for PendingEvent {
    fn tick(&self) -> Tick {
        self.payload.tick
    }

    fn cause(&self) -> CauseRef {
        self.payload.cause
    }

    fn actor_id(&self) -> Option<EntityId> {
        self.payload.actor_id
    }

    fn target_ids(&self) -> &[EntityId] {
        &self.payload.target_ids
    }

    fn evidence(&self) -> &[EvidenceRef] {
        &self.payload.evidence
    }

    fn place_id(&self) -> Option<EntityId> {
        self.payload.place_id
    }

    fn state_deltas(&self) -> &[StateDelta] {
        &self.payload.state_deltas
    }

    fn observed_entities(&self) -> &BTreeMap<EntityId, ObservedEntitySnapshot> {
        &self.payload.observed_entities
    }

    fn visibility(&self) -> VisibilitySpec {
        self.payload.visibility
    }

    fn witness_data(&self) -> &WitnessData {
        &self.payload.witness_data
    }

    fn tags(&self) -> &BTreeSet<EventTag> {
        &self.payload.tags
    }
}

impl EventView for EventRecord {
    fn tick(&self) -> Tick {
        self.payload.tick
    }

    fn cause(&self) -> CauseRef {
        self.payload.cause
    }

    fn actor_id(&self) -> Option<EntityId> {
        self.payload.actor_id
    }

    fn target_ids(&self) -> &[EntityId] {
        &self.payload.target_ids
    }

    fn evidence(&self) -> &[EvidenceRef] {
        &self.payload.evidence
    }

    fn place_id(&self) -> Option<EntityId> {
        self.payload.place_id
    }

    fn state_deltas(&self) -> &[StateDelta] {
        &self.payload.state_deltas
    }

    fn observed_entities(&self) -> &BTreeMap<EntityId, ObservedEntitySnapshot> {
        &self.payload.observed_entities
    }

    fn visibility(&self) -> VisibilitySpec {
        self.payload.visibility
    }

    fn witness_data(&self) -> &WitnessData {
        &self.payload.witness_data
    }

    fn tags(&self) -> &BTreeSet<EventTag> {
        &self.payload.tags
    }
}

impl PendingEvent {
    #[must_use]
    pub fn from_payload(mut payload: EventPayload) -> Self {
        payload.target_ids.sort();
        payload.target_ids.dedup();
        payload.evidence.sort();
        payload.evidence.dedup();

        Self { payload }
    }

    #[must_use]
    pub fn into_record(self, event_id: EventId) -> EventRecord {
        EventRecord {
            event_id,
            payload: self.payload,
        }
    }
}

impl EventRecord {
    #[must_use]
    pub fn from_payload(event_id: EventId, payload: EventPayload) -> Self {
        PendingEvent::from_payload(payload).into_record(event_id)
    }
}

#[cfg(test)]
mod tests {
    use super::{EventPayload, EventRecord, EvidenceRef, PendingEvent};
    use crate::MismatchKind;
    use crate::{
        CauseRef, ComponentDelta, ComponentKind, ComponentValue, EventTag, QuantityDelta,
        RelationDelta, RelationKind, RelationValue, ReservationDelta, StateDelta, VisibilitySpec,
        WitnessData,
    };
    use crate::{
        CommodityKind, EntityId, EntityKind, EventId, Name, ObservedEntitySnapshot, Quantity,
        ReservationId, ReservationRecord, Tick, TickRange, WoundId,
    };
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
    fn event_payload_satisfies_required_traits() {
        assert_traits::<EventPayload>();
    }

    #[test]
    fn pending_event_satisfies_required_traits() {
        assert_traits::<PendingEvent>();
    }

    #[test]
    fn pending_event_constructs_with_all_required_fields() {
        let pending = PendingEvent::from_payload(EventPayload {
            tick: Tick(9),
            cause: CauseRef::Event(EventId(1)),
            actor_id: Some(entity(2)),
            target_ids: vec![entity(5), entity(3), entity(5), entity(4)],
            evidence: Vec::new(),
            place_id: Some(entity(6)),
            state_deltas: vec![
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
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData {
                direct_witnesses: BTreeSet::from([entity(2)]),
                potential_witnesses: BTreeSet::from([entity(2), entity(10)]),
            },
            tags: BTreeSet::from([EventTag::WorldMutation, EventTag::System]),
        });

        assert_eq!(pending.payload.tick, Tick(9));
        assert_eq!(pending.payload.cause, CauseRef::Event(EventId(1)));
        assert_eq!(pending.payload.actor_id, Some(entity(2)));
        assert_eq!(
            pending.payload.target_ids,
            vec![entity(3), entity(4), entity(5)]
        );
        assert!(pending.payload.evidence.is_empty());
        assert_eq!(pending.payload.place_id, Some(entity(6)));
        assert_eq!(pending.payload.state_deltas.len(), 2);
        assert!(pending.payload.observed_entities.is_empty());
        assert_eq!(
            pending.payload.tags.iter().copied().collect::<Vec<_>>(),
            vec![EventTag::WorldMutation, EventTag::System]
        );
    }

    #[test]
    fn event_record_constructs_with_all_required_fields() {
        let record = PendingEvent::from_payload(EventPayload {
            tick: Tick(9),
            cause: CauseRef::Event(EventId(1)),
            actor_id: Some(entity(2)),
            target_ids: vec![entity(5), entity(3), entity(5), entity(4)],
            evidence: Vec::new(),
            place_id: Some(entity(6)),
            state_deltas: vec![
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
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData {
                direct_witnesses: BTreeSet::from([entity(2)]),
                potential_witnesses: BTreeSet::from([entity(2), entity(10)]),
            },
            tags: BTreeSet::from([EventTag::WorldMutation, EventTag::System]),
        })
        .into_record(EventId(4));

        assert_eq!(record.event_id, EventId(4));
        assert_eq!(record.payload.tick, Tick(9));
        assert_eq!(record.payload.cause, CauseRef::Event(EventId(1)));
        assert_eq!(record.payload.actor_id, Some(entity(2)));
        assert_eq!(record.payload.target_ids, vec![entity(3), entity(4), entity(5)]);
        assert!(record.payload.evidence.is_empty());
        assert_eq!(record.payload.place_id, Some(entity(6)));
        assert_eq!(record.payload.state_deltas.len(), 2);
        assert!(record.payload.observed_entities.is_empty());
        assert_eq!(
            record.payload.tags.iter().copied().collect::<Vec<_>>(),
            vec![EventTag::WorldMutation, EventTag::System]
        );
    }

    #[test]
    fn event_record_allows_empty_deltas_and_targets() {
        let record = PendingEvent::from_payload(EventPayload {
            tick: Tick(0),
            cause: CauseRef::Bootstrap,
            actor_id: None,
            target_ids: Vec::new(),
            evidence: Vec::new(),
            place_id: None,
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::Hidden,
            witness_data: WitnessData::default(),
            tags: BTreeSet::new(),
        })
        .into_record(EventId(0));

        assert!(record.payload.target_ids.is_empty());
        assert!(record.payload.evidence.is_empty());
        assert!(record.payload.state_deltas.is_empty());
        assert!(record.payload.observed_entities.is_empty());
        assert!(record.payload.tags.is_empty());
    }

    #[test]
    fn pending_event_roundtrips_through_bincode_with_ordered_deltas() {
        let pending = PendingEvent::from_payload(EventPayload {
            tick: Tick(18),
            cause: CauseRef::SystemTick(Tick(18)),
            actor_id: Some(entity(1)),
            target_ids: vec![entity(4), entity(2), entity(4), entity(3)],
            evidence: vec![
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
            ],
            place_id: Some(entity(6)),
            state_deltas: vec![
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
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::AdjacentPlaces { max_hops: 2 },
            witness_data: WitnessData {
                direct_witnesses: BTreeSet::from([entity(1), entity(2)]),
                potential_witnesses: BTreeSet::from([entity(1), entity(2), entity(3)]),
            },
            tags: BTreeSet::from([EventTag::ActionCommitted, EventTag::Travel]),
        });

        let bytes = bincode::serialize(&pending).unwrap();
        let roundtrip: PendingEvent = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, pending);
        assert_eq!(roundtrip.payload.target_ids, vec![entity(2), entity(3), entity(4)]);
        assert_eq!(
            roundtrip.payload.evidence,
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
            roundtrip.payload.state_deltas[0],
            StateDelta::Component(ComponentDelta::Set { .. })
        ));
        assert!(matches!(
            roundtrip.payload.state_deltas[1],
            StateDelta::Relation(RelationDelta::Added { .. })
        ));
        assert!(matches!(
            roundtrip.payload.state_deltas[2],
            StateDelta::Reservation(ReservationDelta::Created { .. })
        ));
    }

    #[test]
    fn pending_event_orders_and_deduplicates_mismatch_evidence() {
        let pending = PendingEvent::from_payload(EventPayload {
            tick: Tick(21),
            cause: CauseRef::Bootstrap,
            actor_id: Some(entity(1)),
            target_ids: vec![entity(2)],
            evidence: vec![
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
            ],
            place_id: Some(entity(7)),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::ParticipantsOnly,
            witness_data: WitnessData::default(),
            tags: BTreeSet::from([EventTag::Discovery]),
        });

        assert_eq!(
            pending.payload.evidence,
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
    fn pending_event_from_payload_orders_and_deduplicates_refs() {
        let pending = PendingEvent::from_payload(EventPayload {
            tick: Tick(21),
            cause: CauseRef::Bootstrap,
            actor_id: Some(entity(1)),
            target_ids: vec![entity(2)],
            evidence: vec![
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
            ],
            place_id: Some(entity(7)),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::ParticipantsOnly,
            witness_data: WitnessData::default(),
            tags: BTreeSet::from([EventTag::Discovery]),
        });

        assert_eq!(
            pending.payload.evidence,
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
    fn event_record_from_payload_orders_and_deduplicates_refs() {
        let record = EventRecord::from_payload(
            EventId(18),
            EventPayload {
                tick: Tick(21),
                cause: CauseRef::Bootstrap,
                actor_id: Some(entity(1)),
                target_ids: vec![entity(4), entity(2), entity(4), entity(3)],
                evidence: vec![
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
                ],
                place_id: Some(entity(7)),
                state_deltas: Vec::new(),
                observed_entities: BTreeMap::new(),
                visibility: VisibilitySpec::ParticipantsOnly,
                witness_data: WitnessData::default(),
                tags: BTreeSet::from([EventTag::Discovery]),
            },
        );

        assert_eq!(record.payload.target_ids, vec![entity(2), entity(3), entity(4)]);
        assert_eq!(
            record.payload.evidence,
            vec![
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
            ]
        );
    }

    #[test]
    fn pending_event_roundtrips_with_observed_entities() {
        let pending = PendingEvent::from_payload(EventPayload {
            tick: Tick(7),
            cause: CauseRef::Bootstrap,
            actor_id: Some(entity(1)),
            target_ids: vec![entity(2)],
            evidence: Vec::new(),
            place_id: Some(entity(3)),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::from([(
                entity(2),
                ObservedEntitySnapshot {
                    last_known_place: Some(entity(3)),
                    last_known_inventory: BTreeMap::from([(CommodityKind::Bread, Quantity(2))]),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                },
            )]),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData::default(),
            tags: BTreeSet::from([EventTag::WorldMutation]),
        });

        let bytes = bincode::serialize(&pending).unwrap();
        let roundtrip: PendingEvent = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, pending);
        assert_eq!(
            roundtrip
                .payload
                .observed_entities
                .get(&entity(2))
                .unwrap()
                .last_known_inventory,
            BTreeMap::from([(CommodityKind::Bread, Quantity(2))])
        );
    }

    #[test]
    fn event_record_roundtrips_through_bincode_with_ordered_deltas() {
        let record = EventRecord::from_payload(EventId(12), EventPayload {
            tick: Tick(18),
            cause: CauseRef::SystemTick(Tick(18)),
            actor_id: Some(entity(1)),
            target_ids: vec![entity(4), entity(2), entity(4), entity(3)],
            evidence: vec![EvidenceRef::Wound {
                entity: entity(1),
                wound_id: WoundId(4),
            }],
            place_id: Some(entity(6)),
            state_deltas: vec![
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
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::AdjacentPlaces { max_hops: 2 },
            witness_data: WitnessData {
                direct_witnesses: BTreeSet::from([entity(1), entity(2)]),
                potential_witnesses: BTreeSet::from([entity(1), entity(2), entity(3)]),
            },
            tags: BTreeSet::from([EventTag::ActionCommitted, EventTag::Travel]),
        });

        let bytes = bincode::serialize(&record).unwrap();
        let roundtrip: EventRecord = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, record);
        assert_eq!(roundtrip.payload.target_ids, vec![entity(2), entity(3), entity(4)]);
        assert_eq!(roundtrip.payload.evidence, record.payload.evidence);
        assert!(matches!(
            roundtrip.payload.state_deltas[0],
            StateDelta::Component(ComponentDelta::Set { .. })
        ));
        assert!(matches!(
            roundtrip.payload.state_deltas[1],
            StateDelta::Relation(RelationDelta::Added { .. })
        ));
        assert!(matches!(
            roundtrip.payload.state_deltas[2],
            StateDelta::Reservation(ReservationDelta::Created { .. })
        ));
    }

    #[test]
    fn event_record_roundtrips_with_mismatch_evidence() {
        let record = EventRecord::from_payload(EventId(14), EventPayload {
            tick: Tick(22),
            cause: CauseRef::SystemTick(Tick(22)),
            actor_id: Some(entity(3)),
            target_ids: vec![entity(8)],
            evidence: vec![EvidenceRef::Mismatch {
                observer: entity(3),
                subject: entity(8),
                kind: MismatchKind::PlaceChanged {
                    believed_place: entity(5),
                    observed_place: entity(7),
                },
            }],
            place_id: Some(entity(7)),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::ParticipantsOnly,
            witness_data: WitnessData {
                direct_witnesses: BTreeSet::from([entity(3)]),
                potential_witnesses: BTreeSet::from([entity(3)]),
            },
            tags: BTreeSet::from([EventTag::Discovery, EventTag::WorldMutation]),
        });

        let bytes = bincode::serialize(&record).unwrap();
        let roundtrip: EventRecord = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, record);
        assert_eq!(
            roundtrip.payload.evidence,
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
