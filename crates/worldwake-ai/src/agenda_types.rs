use crate::{
    GoalOffer, GoalPriorityClass, PartialPlanSegment, RankedGoalProvenance, SourceCompositeRank,
    decision_trace::{
        CompetitionDiscount, LearnedOpportunityBonusAttribution, RepairMemoryBonusAttribution,
        SourceReliabilityDiscount,
    },
    feasibility::FeasibilityHint,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldwake_core::{
    CommodityKind, EntityId, ExpectationId, MotiveSourceRef, OpportunityKey, Quantity, SlotKind,
    Tick,
};

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
    #[serde(default)]
    pub motive_source_contributions: Vec<(MotiveSourceRef, u32)>,
    pub provenance: Option<RankedGoalProvenance>,
    pub source_reliability_discount: Option<SourceReliabilityDiscount>,
    pub competition_discount: Option<CompetitionDiscount>,
    #[serde(default)]
    pub learned_opportunity_bonus: Option<LearnedOpportunityBonusAttribution>,
    #[serde(default)]
    pub repair_memory_bonus: Option<RepairMemoryBonusAttribution>,
    pub source_composite: Option<SourceCompositeRank>,
    pub feasibility: FeasibilityHint,
    #[serde(default)]
    pub partial_plan_segment: Option<PartialPlanSegment>,
}

impl AgendaEntry {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn pending(
        offer: GoalOffer,
        tick: Tick,
        priority_class: GoalPriorityClass,
        motive_score: u32,
        motive_source_contributions: Vec<(MotiveSourceRef, u32)>,
        provenance: Option<RankedGoalProvenance>,
        source_reliability_discount: Option<SourceReliabilityDiscount>,
        competition_discount: Option<CompetitionDiscount>,
        learned_opportunity_bonus: Option<LearnedOpportunityBonusAttribution>,
        repair_memory_bonus: Option<RepairMemoryBonusAttribution>,
        source_composite: Option<SourceCompositeRank>,
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
            motive_source_contributions,
            provenance,
            source_reliability_discount,
            competition_discount,
            learned_opportunity_bonus,
            repair_memory_bonus,
            source_composite,
            feasibility,
            partial_plan_segment: None,
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
    Obligation {
        artifact: EntityId,
    },
    SocialCommitment {
        expectation: ExpectationId,
    },
    Opportunity {
        evidence: EntityId,
    },
    Exploration,
    Enterprise,
    Companion {
        primary: AgendaEntryKey,
        slot: SlotKind,
    },
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
    use crate::{
        BarrierFact, FeasibilityHint, GoalKey, GoalKind, GoalOffer, GoalPriorityClass,
        PartialPlanSegment, PartialPlanSegmentId, PlanTerminalKind, PlannedStep, PlannerOpKind,
        PlanningEntityRef,
        htn::{BeliefPredicate, CommodityTemplate, PayloadTemplate},
    };
    use std::collections::{BTreeMap, BTreeSet};
    use worldwake_core::{
        ActionDefId, CommodityKind, CommodityPurpose, EntityId, EventId, IntentionAbandonCondition,
        IntentionResumeCondition, OpportunityAnchor, OpportunityKey, Quantity, SlotKind, Tick,
    };

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
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let entry = AgendaEntry::pending(
            offer.clone(),
            Tick(7),
            GoalPriorityClass::Background,
            42,
            Vec::new(),
            None,
            None,
            None,
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
        assert_eq!(entry.partial_plan_segment, None);
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

        let origin = AgendaOrigin::Companion {
            primary: OpportunityKey {
                goal_key: GoalKey::from(GoalKind::Sleep),
                anchor: OpportunityAnchor::Place(EntityId {
                    slot: 9,
                    generation: 0,
                }),
            },
            slot: SlotKind::SocialMotive,
        };
        let bytes = bincode::serialize(&origin).unwrap();
        let roundtrip: AgendaOrigin = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, origin);
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn planned_step() -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(1),
            targets: vec![PlanningEntityRef::Authoritative(entity(10))],
            target_place: Some(entity(20)),
            payload_override: None,
            op_kind: PlannerOpKind::Travel,
            estimated_ticks: 3,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        }
    }

    fn partial_segment(goal: GoalOffer) -> PartialPlanSegment {
        PartialPlanSegment {
            id: PartialPlanSegmentId::new(Tick(7), 2),
            goal,
            completed_prefix: vec![planned_step()],
            remaining_skeleton: Some(vec![crate::PlannedSkeletonStep {
                op: PlannerOpKind::AskWitness,
                target_template: PayloadTemplate::FromContext,
                expected_pre: vec![BeliefPredicate::SellerKnown {
                    commodity: CommodityTemplate::Fixed(CommodityKind::Bread),
                }],
            }]),
            terminal_barrier: PlanTerminalKind::ResourceBarrier {
                commodity: CommodityKind::Bread,
                place: entity(20),
            },
            barrier_fact: BarrierFact::DepletedResource {
                commodity: CommodityKind::Bread,
                place: entity(20),
            },
            resume_conditions: vec![IntentionResumeCondition::TickElapsed(4)],
            abandon_conditions: vec![IntentionAbandonCondition::PatienceExhausted],
            created_tick: Tick(7),
            last_resume_attempt_tick: Some(Tick(9)),
            resume_attempt_count: 1,
            causal_links: vec![EventId(42)],
        }
    }

    #[test]
    fn agenda_state_roundtrip_preserves_suspended_partial_plan_segment() {
        let offer = GoalOffer {
            key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
                quantity: worldwake_core::AcquisitionQuantity::single(),
            }),
            anchor: OpportunityAnchor::Place(entity(20)),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: Some(worldwake_core::AcquisitionQuantity::single()),
        };
        let mut entry = AgendaEntry::pending(
            offer.clone(),
            Tick(7),
            GoalPriorityClass::High,
            900,
            Vec::new(),
            None,
            None,
            None,
            None,
            None,
            None,
            FeasibilityHint::Uncertain,
        );
        entry.phase = AgendaPhase::Suspended;
        entry.partial_plan_segment = Some(partial_segment(offer));
        let state = AgendaState {
            committed: None,
            pending: BTreeMap::new(),
            suspended: BTreeMap::from([(entry.key, entry.clone())]),
        };

        let bytes = bincode::serialize(&state).unwrap();
        let roundtrip: AgendaState = bincode::deserialize(&bytes).unwrap();

        assert_eq!(
            roundtrip
                .suspended
                .get(&entry.key)
                .unwrap()
                .partial_plan_segment,
            entry.partial_plan_segment
        );
    }
}
