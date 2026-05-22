//! Expectation, last-seen, and search substrate types shared across crates.

use crate::{
    CommodityKind, Component, EntityId, EntityKind, EvidenceKind, ExpectationKindTag, Quantity,
    Tick,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Unique identifier for an expectation record.
#[derive(
    Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize,
)]
pub struct ExpectationId(pub u64);

impl fmt::Display for ExpectationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "exp{}", self.0)
    }
}

/// Why an expectation exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExpectationBasis {
    DutyAssignment {
        office: EntityId,
    },
    DeliveryCommitment {
        commodity: CommodityKind,
        quantity: Quantity,
    },
    RoutineReturn,
    EscortObligation {
        charge: EntityId,
    },
    SocialPromise,
    /// A plan step expects completion by `deadline_tick`. The rich
    /// `PlanExpectation` lives on the runtime `PlannedStep`; persisted
    /// expectation records cross-reference that runtime state by index and tag.
    PlanStepCompletion {
        step_index: u16,
        kind_tag: ExpectationKindTag,
    },
}

/// Lifecycle state of an expectation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExpectationState {
    Active,
    Overdue,
    Resolved { outcome: ExpectationOutcome },
    Expired,
}

/// Resolved state for an expectation record.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExpectationOutcome {
    Fulfilled,
    FoundSafe { at_place: EntityId },
    FoundWounded { at_place: EntityId },
    FoundDead { at_place: EntityId },
    NotFound,
    ReturnedLate,
}

/// A time-bounded expectation that a subject will be at a place.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExpectationRecord {
    pub id: ExpectationId,
    pub owner: EntityId,
    pub subject: EntityId,
    pub expected_place: EntityId,
    pub deadline_tick: Tick,
    pub grace_ticks: u64,
    pub basis: ExpectationBasis,
    pub state: ExpectationState,
    pub created_tick: Tick,
}

/// Per-agent store of active expectations about other entities.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExpectationStore {
    pub records: BTreeMap<ExpectationId, ExpectationRecord>,
    pub(crate) next_expectation_id: ExpectationId,
}

impl Default for ExpectationStore {
    fn default() -> Self {
        Self {
            records: BTreeMap::new(),
            next_expectation_id: ExpectationId(0),
        }
    }
}

impl ExpectationStore {
    pub fn next_expectation_id(&self) -> ExpectationId {
        self.next_expectation_id
    }

    fn next_fresh_expectation_id(&self) -> ExpectationId {
        let highest_live_id = self
            .records
            .keys()
            .map(|id| id.0)
            .max()
            .map_or(0, |max_id| max_id.saturating_add(1));
        ExpectationId(self.next_expectation_id.0.max(highest_live_id))
    }

    pub fn allocate_record(
        &mut self,
        build: impl FnOnce(ExpectationId) -> ExpectationRecord,
    ) -> ExpectationId {
        let id = self.next_fresh_expectation_id();
        let mut record = build(id);
        record.id = id;
        self.records.insert(id, record);
        self.next_expectation_id = ExpectationId(id.0.saturating_add(1));
        id
    }
}

impl Component for ExpectationStore {}

/// A record of when, where, and as what kind an entity was last seen, with provenance.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LastSeenRecord {
    pub subject: EntityId,
    pub place: EntityId,
    /// Entity kind observed at last-seen time; belief carrier state that may go stale.
    pub observed_kind: Option<EntityKind>,
    pub observed_tick: Tick,
    pub source: EntityId,
    pub provenance: LastSeenProvenance,
}

/// Per-agent store of last-seen records for known entities.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LastSeenMemory {
    pub records: BTreeMap<EntityId, LastSeenRecord>,
    pub capacity: u16,
}

impl Default for LastSeenMemory {
    fn default() -> Self {
        Self {
            records: BTreeMap::new(),
            capacity: 20,
        }
    }
}

impl Component for LastSeenMemory {}

/// Source provenance for a last-seen record.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LastSeenProvenance {
    DirectObservation,
    Hearsay {
        original_observer: EntityId,
        chain_depth: u8,
    },
}

/// Result of a search action at a specific place.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SearchResult {
    FoundAlive {
        entity: EntityId,
        condition: SearchCondition,
    },
    FoundDead {
        entity: EntityId,
    },
    FoundEvidence {
        evidence_kinds: Vec<EvidenceKind>,
    },
    NothingFound,
}

/// Condition of an entity located during a search.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SearchCondition {
    Healthy,
    Wounded,
    Unconscious,
}

#[cfg(test)]
mod tests {
    use super::{
        ExpectationBasis, ExpectationId, ExpectationOutcome, ExpectationRecord, ExpectationState,
        ExpectationStore, LastSeenMemory, LastSeenProvenance, LastSeenRecord, SearchCondition,
        SearchResult,
    };
    use crate::{
        CauseRef, CommodityKind, Component, ControlSource, EntityId, EventLog, EvidenceKind,
        ExpectationKindTag, Quantity, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
        build_prototype_world,
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::collections::BTreeMap;
    use std::fmt::{Debug, Display};

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn assert_copy_value_bounds<T: Copy + Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn assert_display_bounds<T: Display>() {}
    fn assert_component_bounds<T: Component>() {}

    fn assert_roundtrip<T>(value: &T)
    where
        T: Clone + Eq + Debug + Serialize + DeserializeOwned,
    {
        let bytes = bincode::serialize(value).unwrap();
        let roundtrip: T = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, *value);
    }

    #[test]
    fn expectation_types_satisfy_required_bounds() {
        assert_copy_value_bounds::<ExpectationId>();
        assert_copy_value_bounds::<ExpectationBasis>();
        assert_copy_value_bounds::<ExpectationOutcome>();
        assert_copy_value_bounds::<ExpectationState>();
        assert_copy_value_bounds::<ExpectationRecord>();
        assert_value_bounds::<ExpectationStore>();
        assert_copy_value_bounds::<LastSeenProvenance>();
        assert_copy_value_bounds::<LastSeenRecord>();
        assert_value_bounds::<LastSeenMemory>();
        assert_copy_value_bounds::<SearchCondition>();
        assert_value_bounds::<SearchResult>();
        assert_display_bounds::<ExpectationId>();
        assert_component_bounds::<ExpectationStore>();
        assert_component_bounds::<LastSeenMemory>();
    }

    #[test]
    fn expectation_identifier_display_and_roundtrip() {
        let id = ExpectationId(7);
        assert_eq!(id.to_string(), "exp7");
        assert_roundtrip(&id);
    }

    #[test]
    fn expectation_record_roundtrips_through_bincode() {
        let record = ExpectationRecord {
            id: ExpectationId(1),
            owner: entity(2),
            subject: entity(3),
            expected_place: entity(4),
            deadline_tick: Tick(50),
            grace_ticks: 12,
            basis: ExpectationBasis::DeliveryCommitment {
                commodity: CommodityKind::Bread,
                quantity: Quantity(6),
            },
            state: ExpectationState::Resolved {
                outcome: ExpectationOutcome::FoundWounded {
                    at_place: entity(5),
                },
            },
            created_tick: Tick(10),
        };

        assert_roundtrip(&record);
    }

    #[test]
    fn expectation_basis_plan_step_completion_round_trips_through_bincode() {
        let basis = ExpectationBasis::PlanStepCompletion {
            step_index: 3,
            kind_tag: ExpectationKindTag::State,
        };
        let bytes = bincode::serialize(&basis).unwrap();
        let decoded: ExpectationBasis = bincode::deserialize(&bytes).unwrap();
        assert_eq!(decoded, basis);

        let existing = ExpectationBasis::RoutineReturn;
        assert_eq!(
            bincode::deserialize::<ExpectationBasis>(&bincode::serialize(&existing).unwrap())
                .unwrap(),
            existing,
        );
    }

    #[test]
    fn last_seen_record_roundtrips_through_bincode() {
        let record = LastSeenRecord {
            subject: entity(6),
            place: entity(7),
            observed_kind: Some(crate::EntityKind::Agent),
            observed_tick: Tick(80),
            source: entity(8),
            provenance: LastSeenProvenance::Hearsay {
                original_observer: entity(9),
                chain_depth: 2,
            },
        };

        assert_roundtrip(&record);
    }

    #[test]
    fn search_types_roundtrip_through_bincode() {
        assert_roundtrip(&SearchCondition::Unconscious);
        assert_roundtrip(&SearchResult::FoundAlive {
            entity: entity(12),
            condition: SearchCondition::Wounded,
        });
        assert_roundtrip(&SearchResult::FoundDead { entity: entity(13) });
        assert_roundtrip(&SearchResult::FoundEvidence {
            evidence_kinds: vec![
                EvidenceKind::ContainerTampered {
                    container: entity(14),
                    tampered_at: Tick(81),
                },
                EvidenceKind::MovementTrace {
                    entity: entity(15),
                    departed_from: entity(16),
                    direction: entity(17),
                    observed_at: Tick(82),
                },
            ],
        });
        assert_roundtrip(&SearchResult::NothingFound);
    }

    #[test]
    fn expectation_components_default_to_empty_agent_state() {
        let expectation_store = ExpectationStore::default();
        assert!(expectation_store.records.is_empty());
        assert_eq!(expectation_store.next_expectation_id(), ExpectationId(0));

        let last_seen_memory = LastSeenMemory::default();
        assert!(last_seen_memory.records.is_empty());
        assert_eq!(last_seen_memory.capacity, 20);
    }

    #[test]
    fn create_agent_seeds_default_expectation_components() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::Hidden,
            WitnessData::default(),
        );

        let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();

        assert_eq!(
            txn.get_component_expectation_store(agent),
            Some(&ExpectationStore::default())
        );
        assert_eq!(
            txn.get_component_last_seen_memory(agent),
            Some(&LastSeenMemory::default())
        );
    }

    #[test]
    fn expectation_component_setters_roundtrip_through_world_api() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();

        let agent = {
            let mut txn = WorldTxn::new(
                &mut world,
                Tick(1),
                CauseRef::Bootstrap,
                None,
                None,
                VisibilitySpec::Hidden,
                WitnessData::default(),
            );
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let _ = txn.commit(&mut event_log);
            agent
        };

        let expectation_store = ExpectationStore {
            records: BTreeMap::from([(
                ExpectationId(1),
                ExpectationRecord {
                    id: ExpectationId(1),
                    owner: agent,
                    subject: entity(20),
                    expected_place: entity(21),
                    deadline_tick: Tick(30),
                    grace_ticks: 4,
                    basis: ExpectationBasis::RoutineReturn,
                    state: ExpectationState::Active,
                    created_tick: Tick(10),
                },
            )]),
            next_expectation_id: ExpectationId(2),
        };
        let last_seen_memory = LastSeenMemory {
            records: BTreeMap::from([(
                entity(22),
                LastSeenRecord {
                    subject: entity(22),
                    place: entity(23),
                    observed_kind: Some(crate::EntityKind::Agent),
                    observed_tick: Tick(12),
                    source: agent,
                    provenance: LastSeenProvenance::DirectObservation,
                },
            )]),
            capacity: 7,
        };

        let mut txn = WorldTxn::new(
            &mut world,
            Tick(13),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::Hidden,
            WitnessData::default(),
        );
        txn.set_component_expectation_store(agent, expectation_store.clone())
            .unwrap();
        txn.set_component_last_seen_memory(agent, last_seen_memory.clone())
            .unwrap();
        let _ = txn.commit(&mut event_log);

        assert_eq!(
            world.get_component_expectation_store(agent),
            Some(&expectation_store)
        );
        assert_eq!(
            world.get_component_last_seen_memory(agent),
            Some(&last_seen_memory)
        );
    }

    #[test]
    fn allocate_record_returns_fresh_id_and_advances_counter() {
        let owner = entity(1);
        let subject = entity(2);
        let expected_place = entity(3);
        let mut store = ExpectationStore::default();

        let id = store.allocate_record(|id| ExpectationRecord {
            id,
            owner,
            subject,
            expected_place,
            deadline_tick: Tick(30),
            grace_ticks: 4,
            basis: ExpectationBasis::RoutineReturn,
            state: ExpectationState::Active,
            created_tick: Tick(10),
        });

        assert_eq!(id, ExpectationId(0));
        assert_eq!(store.next_expectation_id(), ExpectationId(1));
        assert_eq!(store.records.get(&id).unwrap().id, id);
        assert_eq!(store.records.get(&id).unwrap().owner, owner);
    }

    #[test]
    fn allocate_record_repairs_stale_counter_against_live_records() {
        let owner = entity(1);
        let mut store = ExpectationStore {
            records: BTreeMap::from([(
                ExpectationId(7),
                ExpectationRecord {
                    id: ExpectationId(7),
                    owner,
                    subject: entity(2),
                    expected_place: entity(3),
                    deadline_tick: Tick(20),
                    grace_ticks: 2,
                    basis: ExpectationBasis::SocialPromise,
                    state: ExpectationState::Active,
                    created_tick: Tick(5),
                },
            )]),
            next_expectation_id: ExpectationId(1),
        };

        let id = store.allocate_record(|id| ExpectationRecord {
            id,
            owner,
            subject: entity(4),
            expected_place: entity(5),
            deadline_tick: Tick(40),
            grace_ticks: 3,
            basis: ExpectationBasis::RoutineReturn,
            state: ExpectationState::Overdue,
            created_tick: Tick(12),
        });

        assert_eq!(id, ExpectationId(8));
        assert_eq!(store.next_expectation_id(), ExpectationId(9));
        assert!(store.records.contains_key(&ExpectationId(7)));
        assert!(store.records.contains_key(&ExpectationId(8)));
    }

    #[test]
    fn expectation_store_roundtrips_advanced_counter_after_allocation() {
        let mut store = ExpectationStore::default();
        store.allocate_record(|id| ExpectationRecord {
            id,
            owner: entity(1),
            subject: entity(2),
            expected_place: entity(3),
            deadline_tick: Tick(30),
            grace_ticks: 4,
            basis: ExpectationBasis::RoutineReturn,
            state: ExpectationState::Active,
            created_tick: Tick(10),
        });

        assert_roundtrip(&store);
        assert_eq!(store.next_expectation_id(), ExpectationId(1));
    }
}
