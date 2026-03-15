//! Typed event-log deltas over canonical world semantics.

use crate::{
    component_schema::with_component_schema_entries, AgentBeliefStore, AgentData,
    BlockedIntentMemory, CarryCapacity, CombatProfile, CombatStance, CommodityKind, Container,
    DeadAt, DemandMemory, DeprivationExposure, DriveThresholds, EntityId, EntityKind,
    ExclusiveFacilityPolicy, FacilityQueueDispositionProfile, FacilityUseQueue, FactionData,
    HomeostaticNeeds, InTransitOnEdge, ItemLot, KnownRecipes, MerchandiseProfile,
    MetabolismProfile, Name, OfficeData, PerceptionProfile, Permille, ProductionJob,
    ProductionOutputOwnershipPolicy, Quantity, ReservationRecord, ResourceSource,
    SubstitutePreferences, TellProfile, TradeDispositionProfile, TravelDispositionProfile,
    UniqueItem, UtilityProfile, WorkstationMarker, WoundList,
};
use serde::{Deserialize, Serialize};

macro_rules! define_component_kind {
    ($({ $field:ident, $component_ty:ty, $table_insert:ident, $table_get:ident, $table_get_mut:ident, $table_remove:ident, $table_has:ident, $table_iter:ident, $insert_fn:ident, $get_fn:ident, $get_mut_fn:ident, $remove_fn:ident, $has_fn:ident, $entities_fn:ident, $query_fn:ident, $count_fn:ident, $component_name:literal, $kind_check:expr, $component_variant:ident })*) => {
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
        pub enum ComponentKind {
            $($component_variant,)*
        }

        impl ComponentKind {
            pub const ALL: [Self; with_component_schema_entries!(forward_authoritative_components, count_authoritative_components)] = [
                $(Self::$component_variant,)*
            ];
        }
    };
}

macro_rules! define_component_value {
    ($({ $field:ident, $component_ty:ty, $table_insert:ident, $table_get:ident, $table_get_mut:ident, $table_remove:ident, $table_has:ident, $table_iter:ident, $insert_fn:ident, $get_fn:ident, $get_mut_fn:ident, $remove_fn:ident, $has_fn:ident, $entities_fn:ident, $query_fn:ident, $count_fn:ident, $component_name:literal, $kind_check:expr, $component_variant:ident })*) => {
        #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
        pub enum ComponentValue {
            $($component_variant($component_ty),)*
        }

        impl ComponentValue {
            #[must_use]
            pub const fn kind(&self) -> ComponentKind {
                match self {
                    $(Self::$component_variant(_) => ComponentKind::$component_variant,)*
                }
            }
        }
    };
}

macro_rules! count_authoritative_components {
    ($({ $field:ident, $component_ty:ty, $table_insert:ident, $table_get:ident, $table_get_mut:ident, $table_remove:ident, $table_has:ident, $table_iter:ident, $insert_fn:ident, $get_fn:ident, $get_mut_fn:ident, $remove_fn:ident, $has_fn:ident, $entities_fn:ident, $query_fn:ident, $count_fn:ident, $component_name:literal, $kind_check:expr, $component_variant:ident })*) => {
        <[()]>::len(&[$(count_authoritative_components!(@replace $component_variant)),*])
    };
    (@replace $component_variant:ident) => { () };
}

with_component_schema_entries!(forward_authoritative_components, define_component_kind);
with_component_schema_entries!(forward_authoritative_components, define_component_value);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RelationKind {
    LocatedIn,
    InTransit,
    ContainedBy,
    PossessedBy,
    OwnedBy,
    MemberOf,
    LoyalTo,
    SupportDeclaration,
    OfficeHolder,
    HostileTo,
}

impl RelationKind {
    pub const ALL: [Self; 10] = [
        Self::LocatedIn,
        Self::InTransit,
        Self::ContainedBy,
        Self::PossessedBy,
        Self::OwnedBy,
        Self::MemberOf,
        Self::LoyalTo,
        Self::SupportDeclaration,
        Self::OfficeHolder,
        Self::HostileTo,
    ];
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
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
    SupportDeclaration {
        supporter: EntityId,
        office: EntityId,
        candidate: EntityId,
    },
    OfficeHolder {
        office: EntityId,
        holder: EntityId,
    },
    HostileTo {
        subject: EntityId,
        target: EntityId,
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
            Self::SupportDeclaration { .. } => RelationKind::SupportDeclaration,
            Self::OfficeHolder { .. } => RelationKind::OfficeHolder,
            Self::HostileTo { .. } => RelationKind::HostileTo,
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum StateDelta {
    Entity(EntityDelta),
    Component(ComponentDelta),
    Relation(RelationDelta),
    Quantity(QuantityDelta),
    Reservation(ReservationDelta),
}

#[cfg(test)]
mod tests {
    use super::{
        ComponentDelta, ComponentKind, ComponentValue, EntityDelta, QuantityDelta, RelationDelta,
        RelationKind, RelationValue, ReservationDelta, StateDelta,
    };
    use crate::{
        test_utils::{
            sample_blocked_intent_memory, sample_demand_memory,
            sample_facility_queue_disposition_profile, sample_merchandise_profile,
            sample_substitute_preferences, sample_trade_disposition_profile,
            sample_travel_disposition_profile, sample_utility_profile,
        },
        AgentBeliefStore, AgentData, BeliefConfidencePolicy, BelievedEntityState, BodyPart,
        CarryCapacity, CombatProfile, CombatStance, CommodityKind, Container, ControlSource,
        DeadAt, DeprivationExposure, DeprivationKind, DriveThresholds, EntityId, EntityKind,
        EventId, ExclusiveFacilityPolicy, FacilityUseQueue, FactionData, HomeostaticNeeds,
        InTransitOnEdge, ItemLot, KnownRecipes, LoadUnits, LotOperation, MetabolismProfile, Name,
        OfficeData, PerceptionProfile, PerceptionSource, Permille, ProductionJob,
        ProductionOutputOwner, ProductionOutputOwnershipPolicy, ProvenanceEntry, Quantity,
        ReservationId, ReservationRecord, ResourceSource, TellProfile, Tick, TickRange,
        TravelEdgeId, UniqueItem, UniqueItemKind, WorkstationMarker, WorkstationTag, Wound,
        WoundCause, WoundList,
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
            id: ReservationId(7),
            entity: entity(4),
            reserver: entity(5),
            range: TickRange::new(Tick(8), Tick(12)).unwrap(),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn component_samples() -> Vec<ComponentValue> {
        vec![
            ComponentValue::Name(Name("Aster".to_string())),
            ComponentValue::AgentData(AgentData {
                control_source: ControlSource::Ai,
            }),
            ComponentValue::WoundList(WoundList {
                wounds: vec![Wound {
                    id: crate::WoundId(1),
                    body_part: BodyPart::Head,
                    cause: WoundCause::Deprivation(DeprivationKind::Starvation),
                    severity: Permille::new(900).unwrap(),
                    inflicted_at: Tick(5),
                    bleed_rate_per_tick: Permille::new(0).unwrap(),
                }],
            }),
            ComponentValue::CombatProfile(CombatProfile::new(
                Permille::new(1000).unwrap(),
                Permille::new(700).unwrap(),
                Permille::new(640).unwrap(),
                Permille::new(590).unwrap(),
                Permille::new(75).unwrap(),
                Permille::new(22).unwrap(),
                Permille::new(17).unwrap(),
                Permille::new(130).unwrap(),
                Permille::new(28).unwrap(),
                std::num::NonZeroU32::new(6).unwrap(),
            )),
            ComponentValue::DeadAt(DeadAt(Tick(18))),
            ComponentValue::CombatStance(CombatStance::Defending),
            ComponentValue::FacilityQueueDispositionProfile(
                sample_facility_queue_disposition_profile(),
            ),
            ComponentValue::UtilityProfile(sample_utility_profile()),
            ComponentValue::OfficeData(OfficeData {
                title: "Granary Chair".to_string(),
                jurisdiction: entity(32),
                succession_law: crate::SuccessionLaw::Support,
                eligibility_rules: Vec::new(),
                succession_period_ticks: 12,
                vacancy_since: Some(Tick(6)),
            }),
            ComponentValue::FactionData(FactionData {
                name: "River Pact".to_string(),
                purpose: crate::FactionPurpose::Political,
            }),
            ComponentValue::BlockedIntentMemory(sample_blocked_intent_memory()),
            ComponentValue::AgentBeliefStore(AgentBeliefStore {
                known_entities: BTreeMap::from([(
                    entity(18),
                    BelievedEntityState {
                        last_known_place: Some(entity(19)),
                        last_known_inventory: BTreeMap::from([
                            (CommodityKind::Apple, Quantity(2)),
                            (CommodityKind::Water, Quantity(1)),
                        ]),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        observed_tick: Tick(14),
                        source: PerceptionSource::DirectObservation,
                    },
                )]),
                social_observations: Vec::new(),
            }),
            ComponentValue::PerceptionProfile(PerceptionProfile {
                memory_capacity: 16,
                memory_retention_ticks: 64,
                observation_fidelity: Permille::new(920).unwrap(),
                confidence_policy: BeliefConfidencePolicy::default(),
            }),
            ComponentValue::TellProfile(TellProfile {
                max_tell_candidates: 4,
                max_relay_chain_len: 2,
                acceptance_fidelity: Permille::new(720).unwrap(),
            }),
            ComponentValue::DriveThresholds(DriveThresholds::default()),
            ComponentValue::HomeostaticNeeds(HomeostaticNeeds::new(
                Permille::new(100).unwrap(),
                Permille::new(200).unwrap(),
                Permille::new(300).unwrap(),
                Permille::new(400).unwrap(),
                Permille::new(500).unwrap(),
            )),
            ComponentValue::DeprivationExposure(DeprivationExposure {
                hunger_critical_ticks: 1,
                thirst_critical_ticks: 2,
                fatigue_critical_ticks: 3,
                bladder_critical_ticks: 4,
            }),
            ComponentValue::MetabolismProfile(MetabolismProfile::default()),
            ComponentValue::CarryCapacity(CarryCapacity(LoadUnits(14))),
            ComponentValue::KnownRecipes(KnownRecipes::with([
                crate::RecipeId(2),
                crate::RecipeId(7),
            ])),
            ComponentValue::DemandMemory(sample_demand_memory()),
            ComponentValue::TravelDispositionProfile(sample_travel_disposition_profile()),
            ComponentValue::TradeDispositionProfile(sample_trade_disposition_profile()),
            ComponentValue::MerchandiseProfile(sample_merchandise_profile()),
            ComponentValue::SubstitutePreferences(sample_substitute_preferences()),
            ComponentValue::ExclusiveFacilityPolicy(ExclusiveFacilityPolicy {
                grant_hold_ticks: std::num::NonZeroU32::new(3).unwrap(),
            }),
            ComponentValue::FacilityUseQueue(FacilityUseQueue::default()),
            ComponentValue::WorkstationMarker(WorkstationMarker(WorkstationTag::Forge)),
            ComponentValue::ResourceSource(ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(6),
                max_quantity: Quantity(10),
                regeneration_ticks_per_unit: Some(std::num::NonZeroU32::new(4).unwrap()),
                last_regeneration_tick: Some(Tick(12)),
            }),
            ComponentValue::ProductionOutputOwnershipPolicy(ProductionOutputOwnershipPolicy {
                output_owner: ProductionOutputOwner::ProducerOwner,
            }),
            ComponentValue::ProductionJob(ProductionJob {
                recipe_id: crate::RecipeId(3),
                worker: entity(24),
                staged_inputs_container: entity(25),
                progress_ticks: 9,
            }),
            ComponentValue::InTransitOnEdge(InTransitOnEdge {
                edge_id: TravelEdgeId(4),
                origin: entity(30),
                destination: entity(31),
                departure_tick: Tick(13),
                arrival_tick: Tick(21),
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
            RelationValue::SupportDeclaration {
                supporter: entity(14),
                office: entity(15),
                candidate: entity(16),
            },
            RelationValue::OfficeHolder {
                office: entity(17),
                holder: entity(18),
            },
            RelationValue::HostileTo {
                subject: entity(19),
                target: entity(20),
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
        assert_traits::<StateDelta>();
    }

    #[test]
    fn component_kind_variants_match_authoritative_components() {
        assert_eq!(
            ComponentKind::ALL,
            [
                ComponentKind::Name,
                ComponentKind::AgentData,
                ComponentKind::WoundList,
                ComponentKind::CombatProfile,
                ComponentKind::DeadAt,
                ComponentKind::CombatStance,
                ComponentKind::FacilityQueueDispositionProfile,
                ComponentKind::UtilityProfile,
                ComponentKind::OfficeData,
                ComponentKind::FactionData,
                ComponentKind::BlockedIntentMemory,
                ComponentKind::AgentBeliefStore,
                ComponentKind::PerceptionProfile,
                ComponentKind::TellProfile,
                ComponentKind::DriveThresholds,
                ComponentKind::HomeostaticNeeds,
                ComponentKind::DeprivationExposure,
                ComponentKind::MetabolismProfile,
                ComponentKind::CarryCapacity,
                ComponentKind::KnownRecipes,
                ComponentKind::DemandMemory,
                ComponentKind::TravelDispositionProfile,
                ComponentKind::TradeDispositionProfile,
                ComponentKind::MerchandiseProfile,
                ComponentKind::SubstitutePreferences,
                ComponentKind::ExclusiveFacilityPolicy,
                ComponentKind::FacilityUseQueue,
                ComponentKind::WorkstationMarker,
                ComponentKind::ResourceSource,
                ComponentKind::ProductionOutputOwnershipPolicy,
                ComponentKind::ProductionJob,
                ComponentKind::InTransitOnEdge,
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
                RelationKind::SupportDeclaration,
                RelationKind::OfficeHolder,
                RelationKind::HostileTo,
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
            relation_kind: RelationKind::HostileTo,
            relation: RelationValue::HostileTo {
                subject: entity(8),
                target: entity(9),
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
                relation_kind: RelationKind::HostileTo,
                relation: RelationValue::HostileTo { subject, target }
            } if subject == entity(8) && target == entity(9)
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

    #[test]
    fn state_delta_wraps_all_delta_families() {
        let reservation = reservation_record();
        let variants = [
            StateDelta::Entity(EntityDelta::Created {
                entity: entity(1),
                kind: EntityKind::Agent,
            }),
            StateDelta::Component(ComponentDelta::Set {
                entity: entity(2),
                component_kind: ComponentKind::Name,
                before: None,
                after: ComponentValue::Name(Name("Kite".to_string())),
            }),
            StateDelta::Relation(RelationDelta::Added {
                relation_kind: RelationKind::LocatedIn,
                relation: RelationValue::LocatedIn {
                    entity: entity(3),
                    place: entity(4),
                },
            }),
            StateDelta::Quantity(QuantityDelta::Changed {
                entity: entity(5),
                commodity: CommodityKind::Water,
                before: Quantity(2),
                after: Quantity(6),
            }),
            StateDelta::Reservation(ReservationDelta::Created { reservation }),
        ];

        assert!(matches!(
            variants[0],
            StateDelta::Entity(EntityDelta::Created { .. })
        ));
        assert!(matches!(
            variants[1],
            StateDelta::Component(ComponentDelta::Set { .. })
        ));
        assert!(matches!(
            variants[2],
            StateDelta::Relation(RelationDelta::Added { .. })
        ));
        assert!(matches!(
            variants[3],
            StateDelta::Quantity(QuantityDelta::Changed { .. })
        ));
        assert!(matches!(
            variants[4],
            StateDelta::Reservation(ReservationDelta::Created { .. })
        ));
    }

    #[test]
    fn state_delta_roundtrips_through_bincode() {
        let deltas = [
            StateDelta::Entity(EntityDelta::Archived {
                entity: entity(6),
                kind: EntityKind::Office,
            }),
            StateDelta::Component(ComponentDelta::Removed {
                entity: entity(7),
                component_kind: ComponentKind::Container,
                before: component_samples().pop().unwrap(),
            }),
            StateDelta::Relation(RelationDelta::Removed {
                relation_kind: RelationKind::HostileTo,
                relation: RelationValue::HostileTo {
                    subject: entity(8),
                    target: entity(11),
                },
            }),
            StateDelta::Quantity(QuantityDelta::Changed {
                entity: entity(9),
                commodity: CommodityKind::Coin,
                before: Quantity(4),
                after: Quantity(9),
            }),
            StateDelta::Reservation(ReservationDelta::Released {
                reservation: reservation_record(),
            }),
        ];

        for delta in deltas {
            let bytes = bincode::serialize(&delta).unwrap();
            let roundtrip: StateDelta = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, delta);
        }
    }
}
