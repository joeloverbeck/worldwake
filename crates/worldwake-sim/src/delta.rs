//! Typed event-log deltas over canonical world semantics.

use serde::{Deserialize, Serialize};
use worldwake_core::{
    AgentData, CommodityKind, Container, EntityId, EntityKind, FactId, ItemLot, Name, Permille,
    Quantity, ReservationRecord, UniqueItem,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ComponentKind {
    Name,
    AgentData,
    ItemLot,
    UniqueItem,
    Container,
}

impl ComponentKind {
    pub const ALL: [Self; 5] = [
        Self::Name,
        Self::AgentData,
        Self::ItemLot,
        Self::UniqueItem,
        Self::Container,
    ];
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComponentValue {
    Name(Name),
    AgentData(AgentData),
    ItemLot(ItemLot),
    UniqueItem(UniqueItem),
    Container(Container),
}

impl ComponentValue {
    #[must_use]
    pub const fn kind(&self) -> ComponentKind {
        match self {
            Self::Name(_) => ComponentKind::Name,
            Self::AgentData(_) => ComponentKind::AgentData,
            Self::ItemLot(_) => ComponentKind::ItemLot,
            Self::UniqueItem(_) => ComponentKind::UniqueItem,
            Self::Container(_) => ComponentKind::Container,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RelationKind {
    LocatedIn,
    InTransit,
    ContainedBy,
    PossessedBy,
    OwnedBy,
    MemberOf,
    LoyalTo,
    OfficeHolder,
    HostileTo,
    KnowsFact,
    BelievesFact,
}

impl RelationKind {
    pub const ALL: [Self; 11] = [
        Self::LocatedIn,
        Self::InTransit,
        Self::ContainedBy,
        Self::PossessedBy,
        Self::OwnedBy,
        Self::MemberOf,
        Self::LoyalTo,
        Self::OfficeHolder,
        Self::HostileTo,
        Self::KnowsFact,
        Self::BelievesFact,
    ];
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RelationValue {
    LocatedIn {
        entity: EntityId,
        place: EntityId,
    },
    InTransit {
        entity: EntityId,
    },
    ContainedBy {
        entity: EntityId,
        container: EntityId,
    },
    PossessedBy {
        entity: EntityId,
        holder: EntityId,
    },
    OwnedBy {
        entity: EntityId,
        owner: EntityId,
    },
    MemberOf {
        member: EntityId,
        faction: EntityId,
    },
    LoyalTo {
        subject: EntityId,
        target: EntityId,
        strength: Permille,
    },
    OfficeHolder {
        office: EntityId,
        holder: EntityId,
    },
    HostileTo {
        subject: EntityId,
        target: EntityId,
    },
    KnowsFact {
        agent: EntityId,
        fact: FactId,
    },
    BelievesFact {
        agent: EntityId,
        fact: FactId,
    },
}

impl RelationValue {
    #[must_use]
    pub const fn kind(&self) -> RelationKind {
        match self {
            Self::LocatedIn { .. } => RelationKind::LocatedIn,
            Self::InTransit { .. } => RelationKind::InTransit,
            Self::ContainedBy { .. } => RelationKind::ContainedBy,
            Self::PossessedBy { .. } => RelationKind::PossessedBy,
            Self::OwnedBy { .. } => RelationKind::OwnedBy,
            Self::MemberOf { .. } => RelationKind::MemberOf,
            Self::LoyalTo { .. } => RelationKind::LoyalTo,
            Self::OfficeHolder { .. } => RelationKind::OfficeHolder,
            Self::HostileTo { .. } => RelationKind::HostileTo,
            Self::KnowsFact { .. } => RelationKind::KnowsFact,
            Self::BelievesFact { .. } => RelationKind::BelievesFact,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EntityDelta {
    Created { entity: EntityId, kind: EntityKind },
    Archived { entity: EntityId, kind: EntityKind },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComponentDelta {
    Set {
        entity: EntityId,
        component_kind: ComponentKind,
        before: Option<ComponentValue>,
        after: ComponentValue,
    },
    Removed {
        entity: EntityId,
        component_kind: ComponentKind,
        before: ComponentValue,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RelationDelta {
    Added {
        relation_kind: RelationKind,
        relation: RelationValue,
    },
    Removed {
        relation_kind: RelationKind,
        relation: RelationValue,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum QuantityDelta {
    Changed {
        entity: EntityId,
        commodity: CommodityKind,
        before: Quantity,
        after: Quantity,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ReservationDelta {
    Created { reservation: ReservationRecord },
    Released { reservation: ReservationRecord },
}

#[cfg(test)]
mod tests {
    use super::{
        ComponentDelta, ComponentKind, ComponentValue, EntityDelta, QuantityDelta, RelationDelta,
        RelationKind, RelationValue, ReservationDelta,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::{BTreeMap, BTreeSet};
    use std::fmt::Debug;
    use worldwake_core::{
        AgentData, CommodityKind, Container, ControlSource, EntityId, EntityKind, EventId, FactId,
        ItemLot, LoadUnits, LotOperation, Name, Permille, ProvenanceEntry, Quantity, ReservationId,
        ReservationRecord, Tick, TickRange, UniqueItem, UniqueItemKind,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn reservation_record() -> ReservationRecord {
        ReservationRecord {
            id: ReservationId(7),
            entity: entity(4),
            reserver: entity(5),
            range: TickRange::new(Tick(8), Tick(12)).unwrap(),
        }
    }

    fn component_samples() -> Vec<ComponentValue> {
        vec![
            ComponentValue::Name(Name("Aster".to_string())),
            ComponentValue::AgentData(AgentData {
                control_source: ControlSource::Ai,
            }),
            ComponentValue::ItemLot(ItemLot {
                commodity: CommodityKind::Grain,
                quantity: Quantity(11),
                provenance: vec![ProvenanceEntry {
                    tick: Tick(3),
                    event_id: Some(EventId(2)),
                    operation: LotOperation::Produced,
                    related_lot: Some(entity(9)),
                    amount: Quantity(4),
                }],
            }),
            ComponentValue::UniqueItem(UniqueItem {
                kind: UniqueItemKind::Artifact,
                name: Some("Seal".to_string()),
                metadata: BTreeMap::from([("origin".to_string(), "court".to_string())]),
            }),
            ComponentValue::Container(Container {
                capacity: LoadUnits(25),
                allowed_commodities: Some(BTreeSet::from([
                    CommodityKind::Apple,
                    CommodityKind::Water,
                ])),
                allows_unique_items: true,
                allows_nested_containers: false,
            }),
        ]
    }

    fn relation_samples() -> Vec<RelationValue> {
        vec![
            RelationValue::LocatedIn {
                entity: entity(1),
                place: entity(2),
            },
            RelationValue::InTransit { entity: entity(3) },
            RelationValue::ContainedBy {
                entity: entity(4),
                container: entity(5),
            },
            RelationValue::PossessedBy {
                entity: entity(6),
                holder: entity(7),
            },
            RelationValue::OwnedBy {
                entity: entity(8),
                owner: entity(9),
            },
            RelationValue::MemberOf {
                member: entity(10),
                faction: entity(11),
            },
            RelationValue::LoyalTo {
                subject: entity(12),
                target: entity(13),
                strength: Permille::new(650).unwrap(),
            },
            RelationValue::OfficeHolder {
                office: entity(14),
                holder: entity(15),
            },
            RelationValue::HostileTo {
                subject: entity(16),
                target: entity(17),
            },
            RelationValue::KnowsFact {
                agent: entity(18),
                fact: FactId(19),
            },
            RelationValue::BelievesFact {
                agent: entity(20),
                fact: FactId(21),
            },
        ]
    }

    fn assert_traits<T: Clone + Debug + Eq + Serialize + DeserializeOwned>() {}
    fn assert_kind_traits<
        T: Copy + Clone + Debug + Eq + Ord + std::hash::Hash + Serialize + DeserializeOwned,
    >() {
    }

    #[test]
    fn delta_types_satisfy_required_traits() {
        assert_kind_traits::<ComponentKind>();
        assert_kind_traits::<RelationKind>();
        assert_traits::<ComponentValue>();
        assert_traits::<RelationValue>();
        assert_traits::<EntityDelta>();
        assert_traits::<ComponentDelta>();
        assert_traits::<RelationDelta>();
        assert_traits::<QuantityDelta>();
        assert_traits::<ReservationDelta>();
    }

    #[test]
    fn component_kind_variants_match_authoritative_components() {
        assert_eq!(
            ComponentKind::ALL,
            [
                ComponentKind::Name,
                ComponentKind::AgentData,
                ComponentKind::ItemLot,
                ComponentKind::UniqueItem,
                ComponentKind::Container,
            ]
        );
    }

    #[test]
    fn component_value_reports_matching_component_kind() {
        let samples = component_samples();

        assert_eq!(samples.len(), ComponentKind::ALL.len());
        for sample in samples {
            assert!(ComponentKind::ALL.contains(&sample.kind()));
        }
    }

    #[test]
    fn relation_kind_variants_match_semantic_relation_families() {
        assert_eq!(
            RelationKind::ALL,
            [
                RelationKind::LocatedIn,
                RelationKind::InTransit,
                RelationKind::ContainedBy,
                RelationKind::PossessedBy,
                RelationKind::OwnedBy,
                RelationKind::MemberOf,
                RelationKind::LoyalTo,
                RelationKind::OfficeHolder,
                RelationKind::HostileTo,
                RelationKind::KnowsFact,
                RelationKind::BelievesFact,
            ]
        );
    }

    #[test]
    fn relation_value_reports_matching_relation_kind() {
        let samples = relation_samples();

        assert_eq!(samples.len(), RelationKind::ALL.len());
        for sample in samples {
            assert!(RelationKind::ALL.contains(&sample.kind()));
        }
    }

    #[test]
    fn entity_delta_stores_entity_id_and_kind() {
        let created = EntityDelta::Created {
            entity: entity(1),
            kind: EntityKind::Agent,
        };
        let archived = EntityDelta::Archived {
            entity: entity(2),
            kind: EntityKind::Office,
        };

        assert!(matches!(
            created,
            EntityDelta::Created {
                entity: created_entity,
                kind: EntityKind::Agent
            } if created_entity == entity(1)
        ));
        assert!(matches!(
            archived,
            EntityDelta::Archived {
                entity: archived_entity,
                kind: EntityKind::Office
            } if archived_entity == entity(2)
        ));
    }

    #[test]
    fn component_delta_stores_typed_before_after_snapshots() {
        let before = ComponentValue::Name(Name("Old".to_string()));
        let after = ComponentValue::Name(Name("New".to_string()));
        let set = ComponentDelta::Set {
            entity: entity(3),
            component_kind: ComponentKind::Name,
            before: Some(before.clone()),
            after: after.clone(),
        };
        let removed = ComponentDelta::Removed {
            entity: entity(4),
            component_kind: ComponentKind::Container,
            before: component_samples().pop().unwrap(),
        };

        assert!(matches!(
            set,
            ComponentDelta::Set {
                entity: changed_entity,
                component_kind: ComponentKind::Name,
                before: Some(ComponentValue::Name(Name(ref old))),
                after: ComponentValue::Name(Name(ref new))
            } if changed_entity == entity(3) && old == "Old" && new == "New"
        ));
        assert!(matches!(
            removed,
            ComponentDelta::Removed {
                entity: removed_entity,
                component_kind: ComponentKind::Container,
                before: ComponentValue::Container(_)
            } if removed_entity == entity(4)
        ));
    }

    #[test]
    fn relation_delta_stores_typed_semantic_payloads() {
        let relation = RelationValue::LoyalTo {
            subject: entity(6),
            target: entity(7),
            strength: Permille::new(700).unwrap(),
        };
        let added = RelationDelta::Added {
            relation_kind: RelationKind::LoyalTo,
            relation: relation.clone(),
        };
        let removed = RelationDelta::Removed {
            relation_kind: RelationKind::KnowsFact,
            relation: RelationValue::KnowsFact {
                agent: entity(8),
                fact: FactId(9),
            },
        };

        assert!(matches!(
            added,
            RelationDelta::Added {
                relation_kind: RelationKind::LoyalTo,
                relation: RelationValue::LoyalTo { strength, .. }
            } if strength == Permille::new(700).unwrap()
        ));
        assert!(matches!(
            removed,
            RelationDelta::Removed {
                relation_kind: RelationKind::KnowsFact,
                relation: RelationValue::KnowsFact { agent, fact }
            } if agent == entity(8) && fact == FactId(9)
        ));
        assert_eq!(relation.kind(), RelationKind::LoyalTo);
    }

    #[test]
    fn quantity_delta_stores_before_and_after_quantities() {
        let delta = QuantityDelta::Changed {
            entity: entity(10),
            commodity: CommodityKind::Bread,
            before: Quantity(2),
            after: Quantity(5),
        };

        assert!(matches!(
            delta,
            QuantityDelta::Changed {
                entity: changed_entity,
                commodity: CommodityKind::Bread,
                before: Quantity(2),
                after: Quantity(5)
            } if changed_entity == entity(10)
        ));
    }

    #[test]
    fn reservation_delta_stores_full_reservation_record() {
        let reservation = reservation_record();
        let created = ReservationDelta::Created {
            reservation: reservation.clone(),
        };
        let released = ReservationDelta::Released {
            reservation: reservation.clone(),
        };

        assert!(matches!(
            created,
            ReservationDelta::Created { reservation: ref record } if record == &reservation
        ));
        assert!(matches!(
            released,
            ReservationDelta::Released { reservation: ref record } if record == &reservation
        ));
    }

    #[test]
    fn delta_variants_roundtrip_through_bincode() {
        let variants = [
            bincode::serialize(&EntityDelta::Created {
                entity: entity(1),
                kind: EntityKind::Agent,
            })
            .unwrap(),
            bincode::serialize(&ComponentDelta::Set {
                entity: entity(2),
                component_kind: ComponentKind::AgentData,
                before: None,
                after: ComponentValue::AgentData(AgentData {
                    control_source: ControlSource::Human,
                }),
            })
            .unwrap(),
            bincode::serialize(&RelationDelta::Added {
                relation_kind: RelationKind::OfficeHolder,
                relation: RelationValue::OfficeHolder {
                    office: entity(3),
                    holder: entity(4),
                },
            })
            .unwrap(),
            bincode::serialize(&QuantityDelta::Changed {
                entity: entity(5),
                commodity: CommodityKind::Coin,
                before: Quantity(10),
                after: Quantity(12),
            })
            .unwrap(),
            bincode::serialize(&ReservationDelta::Released {
                reservation: reservation_record(),
            })
            .unwrap(),
        ];

        let entity_roundtrip: EntityDelta = bincode::deserialize(&variants[0]).unwrap();
        let component_roundtrip: ComponentDelta = bincode::deserialize(&variants[1]).unwrap();
        let relation_roundtrip: RelationDelta = bincode::deserialize(&variants[2]).unwrap();
        let quantity_roundtrip: QuantityDelta = bincode::deserialize(&variants[3]).unwrap();
        let reservation_roundtrip: ReservationDelta = bincode::deserialize(&variants[4]).unwrap();

        assert!(matches!(entity_roundtrip, EntityDelta::Created { .. }));
        assert!(matches!(component_roundtrip, ComponentDelta::Set { .. }));
        assert!(matches!(relation_roundtrip, RelationDelta::Added { .. }));
        assert!(matches!(quantity_roundtrip, QuantityDelta::Changed { .. }));
        assert!(matches!(
            reservation_roundtrip,
            ReservationDelta::Released { .. }
        ));
    }
}
