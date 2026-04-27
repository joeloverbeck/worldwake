use crate::{
    GoalOffer, GoalPriorityClass, RankedGoalProvenance,
    decision_trace::{CompetitionDiscount, SourceReliabilityDiscount},
    feasibility::FeasibilityHint,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldwake_core::{CommodityKind, EntityId, ExpectationId, OpportunityKey, Quantity, Tick};

pub type AgendaEntryKey = OpportunityKey;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgendaState {
    pub committed: Option<AgendaEntry>,
    pub pending: BTreeMap<AgendaEntryKey, AgendaEntry>,
    pub suspended: BTreeMap<AgendaEntryKey, AgendaEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgendaEntry {
    pub key: AgendaEntryKey,
    pub offer: GoalOffer,
    pub phase: AgendaPhase,
    pub origin: AgendaOrigin,
    pub introduced_tick: Tick,
    pub last_reconsidered_tick: Tick,
    pub revival_trigger: Option<RevivalTrigger>,
    pub kill_condition: KillCondition,
    pub priority_class: GoalPriorityClass,
    pub motive_score: u32,
    pub provenance: Option<RankedGoalProvenance>,
    pub source_reliability_discount: Option<SourceReliabilityDiscount>,
    pub competition_discount: Option<CompetitionDiscount>,
    pub feasibility: FeasibilityHint,
}

impl AgendaEntry {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn pending(
        offer: GoalOffer,
        tick: Tick,
        priority_class: GoalPriorityClass,
        motive_score: u32,
        provenance: Option<RankedGoalProvenance>,
        source_reliability_discount: Option<SourceReliabilityDiscount>,
        competition_discount: Option<CompetitionDiscount>,
        feasibility: FeasibilityHint,
    ) -> Self {
        Self {
            key: OpportunityKey {
                goal_key: offer.key,
                anchor: offer.anchor,
            },
            offer,
            phase: AgendaPhase::Pending,
            origin: AgendaOrigin::NeedDrive,
            introduced_tick: tick,
            last_reconsidered_tick: tick,
            revival_trigger: None,
            kill_condition: KillCondition::External,
            priority_class,
            motive_score,
            provenance,
            source_reliability_discount,
            competition_discount,
            feasibility,
        }
    }

    #[must_use]
    pub fn committed_from(candidate: &Self, tick: Tick) -> Self {
        let mut entry = candidate.clone();
        entry.phase = AgendaPhase::Committed;
        entry.introduced_tick = tick;
        entry.last_reconsidered_tick = tick;
        entry
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgendaPhase {
    Committed,
    Pending,
    Suspended,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgendaOrigin {
    NeedDrive,
    Obligation { artifact: EntityId },
    SocialCommitment { expectation: ExpectationId },
    Opportunity { evidence: EntityId },
    Exploration,
    Enterprise,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RevivalTrigger {
    CommodityAvailable {
        place: EntityId,
        kind: CommodityKind,
        min: Quantity,
    },
    TargetPresent {
        target: EntityId,
        place: EntityId,
    },
    RouteLearned {
        from: EntityId,
        to: EntityId,
    },
    CounterpartyAvailable {
        counterparty: EntityId,
        place: EntityId,
    },
    TickElapsed {
        at_tick: Tick,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum KillCondition {
    TickExpiry { at_tick: Tick },
    ObligationResolved { expectation: ExpectationId },
    TargetDead { target: EntityId },
    External,
}

#[cfg(test)]
mod tests {
    use super::{
        AgendaEntry, AgendaOrigin, AgendaPhase, AgendaState, KillCondition, RevivalTrigger,
    };
    use crate::{FeasibilityHint, GoalKey, GoalKind, GoalOffer, GoalPriorityClass};
    use std::collections::{BTreeMap, BTreeSet};
    use worldwake_core::{EntityId, OpportunityAnchor, OpportunityKey, Quantity, Tick};

    #[test]
    fn agenda_state_default_is_empty() {
        assert_eq!(
            AgendaState::default(),
            AgendaState {
                committed: None,
                pending: BTreeMap::new(),
                suspended: BTreeMap::new(),
            }
        );
    }

    #[test]
    fn pending_entry_uses_lifecycle_defaults() {
        let offer = GoalOffer {
            key: GoalKey::from(GoalKind::Sleep),
            anchor: OpportunityAnchor::Place(EntityId {
                slot: 1,
                generation: 0,
            }),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            acquisition_quantity: None,
        };
        let entry = AgendaEntry::pending(
            offer.clone(),
            Tick(7),
            GoalPriorityClass::Background,
            42,
            None,
            None,
            None,
            FeasibilityHint::Uncertain,
        );
        assert_eq!(
            entry.key,
            OpportunityKey {
                goal_key: offer.key,
                anchor: offer.anchor
            }
        );
        assert_eq!(entry.offer, offer);
        assert_eq!(entry.phase, AgendaPhase::Pending);
        assert_eq!(entry.origin, AgendaOrigin::NeedDrive);
        assert_eq!(entry.introduced_tick, Tick(7));
        assert_eq!(entry.last_reconsidered_tick, Tick(7));
        assert_eq!(entry.revival_trigger, None);
        assert_eq!(entry.kill_condition, KillCondition::External);
    }

    #[test]
    fn lifecycle_enums_roundtrip_through_bincode() {
        let trigger = RevivalTrigger::CommodityAvailable {
            place: EntityId {
                slot: 2,
                generation: 0,
            },
            kind: worldwake_core::CommodityKind::Bread,
            min: Quantity(3),
        };
        let bytes = bincode::serialize(&trigger).unwrap();
        let roundtrip: RevivalTrigger = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, trigger);

        let kill = KillCondition::ObligationResolved {
            expectation: worldwake_core::ExpectationId(4),
        };
        let bytes = bincode::serialize(&kill).unwrap();
        let roundtrip: KillCondition = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, kill);
    }
}
