use crate::{
    GoalOffer, PlanTerminalKind, PlannedStep, PlannerOpKind,
    htn::{BeliefPredicate, PayloadTemplate},
};
use serde::{Deserialize, Serialize};
use worldwake_core::{
    CommodityKind, EntityId, EventId, IntentionAbandonCondition, IntentionResumeCondition, Tick,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct PartialPlanSegmentId {
    pub created_tick: Tick,
    pub local_counter: u16,
}

impl PartialPlanSegmentId {
    #[must_use]
    pub const fn new(created_tick: Tick, local_counter: u16) -> Self {
        Self {
            created_tick,
            local_counter,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PartialPlanSegment {
    pub id: PartialPlanSegmentId,
    pub goal: GoalOffer,
    pub completed_prefix: Vec<PlannedStep>,
    pub remaining_skeleton: Option<Vec<PlannedSkeletonStep>>,
    pub terminal_barrier: PlanTerminalKind,
    pub barrier_fact: BarrierFact,
    pub resume_conditions: Vec<IntentionResumeCondition>,
    pub abandon_conditions: Vec<IntentionAbandonCondition>,
    pub created_tick: Tick,
    pub last_resume_attempt_tick: Option<Tick>,
    pub resume_attempt_count: u8,
    pub causal_links: Vec<EventId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlannedSkeletonStep {
    pub op: PlannerOpKind,
    pub target_template: PayloadTemplate,
    pub expected_pre: Vec<BeliefPredicate>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BarrierFact {
    MissingBelief(BeliefPredicate),
    ContestedReservation(EntityId),
    DepletedResource {
        commodity: CommodityKind,
        place: EntityId,
    },
    NoAuthorityForAction(EntityId),
    BudgetExhausted {
        remaining_stages: u16,
    },
}

#[cfg(test)]
mod tests {
    use super::{BarrierFact, PartialPlanSegment, PartialPlanSegmentId, PlannedSkeletonStep};
    use crate::{
        GoalOffer, PlanTerminalKind, PlannedStep, PlannerOpKind, PlanningEntityRef,
        htn::{BeliefPredicate, CommodityTemplate, PayloadTemplate},
    };
    use std::collections::BTreeSet;
    use worldwake_core::{
        ActionDefId, CommodityKind, CommodityPurpose, EntityId, EventId, GoalKey, GoalKind,
        IntentionAbandonCondition, IntentionResumeCondition, OpportunityAnchor, Permille, Tick,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn goal_offer() -> GoalOffer {
        let kind = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: worldwake_core::AcquisitionQuantity::single(),
        };
        GoalOffer {
            key: GoalKey::from(kind),
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: Some(worldwake_core::AcquisitionQuantity::single()),
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

    fn skeleton_step() -> PlannedSkeletonStep {
        PlannedSkeletonStep {
            op: PlannerOpKind::AskWitness,
            target_template: PayloadTemplate::FromContext,
            expected_pre: vec![BeliefPredicate::SellerKnown {
                commodity: CommodityTemplate::Fixed(CommodityKind::Bread),
            }],
        }
    }

    fn segment(barrier_fact: BarrierFact) -> PartialPlanSegment {
        PartialPlanSegment {
            id: PartialPlanSegmentId::new(Tick(7), 2),
            goal: goal_offer(),
            completed_prefix: vec![planned_step()],
            remaining_skeleton: Some(vec![skeleton_step()]),
            terminal_barrier: PlanTerminalKind::ResourceBarrier {
                commodity: CommodityKind::Bread,
                place: entity(20),
            },
            barrier_fact,
            resume_conditions: vec![IntentionResumeCondition::TickElapsed(4)],
            abandon_conditions: vec![IntentionAbandonCondition::PatienceExhausted],
            created_tick: Tick(7),
            last_resume_attempt_tick: Some(Tick(9)),
            resume_attempt_count: 1,
            causal_links: vec![EventId(42)],
        }
    }

    fn roundtrip<T>(value: &T) -> T
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        let bytes = bincode::serialize(value).unwrap();
        bincode::deserialize(&bytes).unwrap()
    }

    #[test]
    fn partial_plan_segment_roundtrips_through_bincode_with_all_barrier_facts() {
        let cases = [
            BarrierFact::MissingBelief(BeliefPredicate::SellerKnown {
                commodity: CommodityTemplate::Fixed(CommodityKind::Bread),
            }),
            BarrierFact::ContestedReservation(entity(30)),
            BarrierFact::DepletedResource {
                commodity: CommodityKind::Bread,
                place: entity(31),
            },
            BarrierFact::NoAuthorityForAction(entity(32)),
            BarrierFact::BudgetExhausted {
                remaining_stages: 2,
            },
        ];

        for barrier_fact in cases {
            let original = segment(barrier_fact);
            let decoded: PartialPlanSegment = roundtrip(&original);
            assert_eq!(decoded, original);
        }
    }

    #[test]
    fn partial_plan_segment_id_preserves_tick_and_counter_identity() {
        let first = PartialPlanSegmentId::new(Tick(11), 1);
        let second = PartialPlanSegmentId::new(Tick(11), 2);
        let later_tick = PartialPlanSegmentId::new(Tick(12), 0);

        assert_ne!(first, second);
        assert_ne!(first, later_tick);
        assert!(first < second);
        assert!(second < later_tick);
    }
}
