//! Expectation, last-seen, and search substrate types shared across crates.

use crate::{CommodityKind, EntityId, EvidenceKind, Quantity, Tick};
use serde::{Deserialize, Serialize};
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

/// A record of when and where an entity was last seen, with provenance.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LastSeenRecord {
    pub subject: EntityId,
    pub place: EntityId,
    pub observed_tick: Tick,
    pub source: EntityId,
    pub provenance: LastSeenProvenance,
}

/// Source provenance for a last-seen record.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LastSeenProvenance {
    DirectObservation,
    Hearsay {
        original_observer: EntityId,
        chain_depth: u8,
    },
}

/// What a searcher is looking for.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SearchTarget {
    MissingEntity {
        entity: EntityId,
        last_seen_place: Option<EntityId>,
    },
    RouteSearch {
        from: EntityId,
        to: EntityId,
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
        LastSeenProvenance, LastSeenRecord, SearchCondition, SearchResult, SearchTarget,
    };
    use crate::{CommodityKind, EntityId, EvidenceKind, Quantity, Tick};
    use serde::{de::DeserializeOwned, Serialize};
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
        assert_copy_value_bounds::<LastSeenProvenance>();
        assert_copy_value_bounds::<LastSeenRecord>();
        assert_copy_value_bounds::<SearchCondition>();
        assert_copy_value_bounds::<SearchTarget>();
        assert_value_bounds::<SearchResult>();
        assert_display_bounds::<ExpectationId>();
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
    fn last_seen_record_roundtrips_through_bincode() {
        let record = LastSeenRecord {
            subject: entity(6),
            place: entity(7),
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
        assert_roundtrip(&SearchTarget::MissingEntity {
            entity: entity(10),
            last_seen_place: Some(entity(11)),
        });
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
}
