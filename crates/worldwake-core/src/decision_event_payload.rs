use crate::{
    ActionDefId, BeliefClaimKey, BlockerKey, BlockingFact, Discrepancy, EntityId, GoalKey,
    MaterializationTag, SuspensionReason, Tick,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DecisionEventPayload {
    GoalOffered(GoalOfferedPayload),
    GoalSuppressed(GoalSuppressedPayload),
    GoalCommitted(GoalCommittedPayload),
    GoalSuspended(GoalSuspendedPayload),
    GoalAbandoned(GoalAbandonedPayload),
    PlanAdopted(PlanAdoptedPayload),
    PlanInvalidated(PlanInvalidatedPayload),
    ExpectationMismatch(ExpectationMismatchPayload),
    RepairApplied(RepairAppliedPayload),
    ReplanTriggered(ReplanTriggeredPayload),
    BlockerRecorded(BlockerRecordedPayload),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GoalOfferedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub emitter: EmitterTag,
    pub source_evidence: EvidenceSummary,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum EmitterTag {
    HomeostaticNeeds,
    Production,
    Enterprise,
    Disposal,
    Bounty,
    ArtifactPosting,
    Combat,
    Crime,
    Social,
    Patrol,
    Political,
    RecordedViolation,
    Search,
    Escort,
    Exploration,
    ProactiveExploration,
    ExpectationViolation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceSummary {
    pub evidence_kind_counts: BTreeMap<EvidenceKindTag, u16>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum EvidenceKindTag {
    HomeostaticPressure,
    SelfKnowledge,
    PerceptionObservation,
    InstitutionalRecord,
    RecordedViolation,
    ExpectationRecord,
    ExplorationPressure,
    PatrolRoute,
    EnterpriseState,
    LearnedOpportunity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GoalSuppressedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub reason: GoalRejectionReason,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GoalCommittedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub motive_score: u32,
    pub rejected_alternatives: Vec<RejectedAlternativeSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RejectedAlternativeSummary {
    pub goal_key: GoalKey,
    pub rejection_reason: GoalRejectionReason,
    pub score_gap: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum GoalRejectionReason {
    LowerMotive,
    FeasibilityProbeFailed,
    SuppressedByBlocker,
    SuppressedByDiscrepancy,
    SuppressedByStressPolicy,
    SuppressedByContentionPreempt,
    ArbitrationLost,
    SwitchMarginInsufficient,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GoalSuspendedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub reason: SuspensionReason,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GoalAbandonedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub reason: GoalRejectionReason,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanAdoptedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub plan_step_count: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanInvalidatedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub reason: PlanInvalidationReason,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum PlanInvalidationReason {
    BeliefUpdate {
        claim_key: BeliefClaimKey,
    },
    TargetGone {
        target: EntityId,
    },
    ExpectationMismatch {
        step_index: u16,
    },
    ContentionLost {
        place: EntityId,
        action: ActionDefId,
    },
    DiscrepancyRecorded {
        discrepancy: Discrepancy,
    },
    PreemptedByHigherGoal {
        new_goal: GoalKey,
    },
    AgentIncapacitated,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExpectationMismatchPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub step_index: u16,
    pub expected_materializations: Vec<MaterializationTag>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepairAppliedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub step_index: u16,
    pub repair_kind: RepairKind,
    pub substitute_target: Option<EntityId>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RepairKind {
    AlternateTarget,
    AlternateRoute,
    AlternateMerchant,
    AlternateRecipe,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplanTriggeredPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub reason: ReplanReason,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ReplanReason {
    PlanInvalidated,
    LocalRepairExhausted,
    SearchBudgetExhausted,
    GoalSwitched,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockerRecordedPayload {
    pub agent: EntityId,
    pub blocker_key: BlockerKey,
    pub discrepancy: Option<Discrepancy>,
    pub blocking_fact: Option<BlockingFact>,
    pub expires_tick: Tick,
}

#[cfg(test)]
mod tests {
    use super::{
        BlockerRecordedPayload, DecisionEventPayload, EmitterTag, EvidenceKindTag, EvidenceSummary,
        ExpectationMismatchPayload, GoalAbandonedPayload, GoalCommittedPayload, GoalOfferedPayload,
        GoalRejectionReason, GoalSuppressedPayload, GoalSuspendedPayload, PlanAdoptedPayload,
        PlanInvalidatedPayload, PlanInvalidationReason, RejectedAlternativeSummary,
        RepairAppliedPayload, RepairKind, ReplanReason, ReplanTriggeredPayload,
    };
    use crate::{
        ActionDefId, BeliefClaimKey, BlockingFact, CommodityKind, Discrepancy, EntityBeliefAspect,
        MaterializationTag, SuspensionReason, Tick,
        test_utils::{entity_id, sample_blocker_key, sample_goal_key},
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::collections::BTreeMap;
    use std::fmt::Debug;

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn assert_copy_value_bounds<
        T: Copy + Clone + Eq + Ord + Debug + Serialize + DeserializeOwned,
    >() {
    }

    fn sample_goal_offered_payload() -> GoalOfferedPayload {
        GoalOfferedPayload {
            agent: entity_id(1, 0),
            goal_key: sample_goal_key(),
            emitter: EmitterTag::HomeostaticNeeds,
            source_evidence: EvidenceSummary {
                evidence_kind_counts: BTreeMap::from([
                    (EvidenceKindTag::HomeostaticPressure, 2),
                    (EvidenceKindTag::PerceptionObservation, 1),
                ]),
            },
        }
    }

    fn sample_decision_payloads() -> Vec<DecisionEventPayload> {
        vec![
            DecisionEventPayload::GoalOffered(sample_goal_offered_payload()),
            DecisionEventPayload::GoalSuppressed(GoalSuppressedPayload {
                agent: entity_id(2, 0),
                goal_key: sample_goal_key(),
                reason: GoalRejectionReason::SuppressedByBlocker,
            }),
            DecisionEventPayload::GoalCommitted(GoalCommittedPayload {
                agent: entity_id(3, 0),
                goal_key: sample_goal_key(),
                motive_score: 420,
                rejected_alternatives: vec![RejectedAlternativeSummary {
                    goal_key: sample_goal_key(),
                    rejection_reason: GoalRejectionReason::LowerMotive,
                    score_gap: 17,
                }],
            }),
            DecisionEventPayload::GoalSuspended(GoalSuspendedPayload {
                agent: entity_id(4, 0),
                goal_key: sample_goal_key(),
                reason: SuspensionReason::RouteBlocked,
            }),
            DecisionEventPayload::GoalAbandoned(GoalAbandonedPayload {
                agent: entity_id(5, 0),
                goal_key: sample_goal_key(),
                reason: GoalRejectionReason::ArbitrationLost,
            }),
            DecisionEventPayload::PlanAdopted(PlanAdoptedPayload {
                agent: entity_id(6, 0),
                goal_key: sample_goal_key(),
                plan_step_count: 3,
            }),
            DecisionEventPayload::PlanInvalidated(PlanInvalidatedPayload {
                agent: entity_id(7, 0),
                goal_key: sample_goal_key(),
                reason: PlanInvalidationReason::BeliefUpdate {
                    claim_key: BeliefClaimKey {
                        subject: entity_id(8, 0),
                        aspect: EntityBeliefAspect::Inventory(CommodityKind::Bread),
                    },
                },
            }),
            DecisionEventPayload::ExpectationMismatch(ExpectationMismatchPayload {
                agent: entity_id(9, 0),
                goal_key: sample_goal_key(),
                step_index: 1,
                expected_materializations: vec![MaterializationTag::SplitOffLot],
            }),
            DecisionEventPayload::RepairApplied(RepairAppliedPayload {
                agent: entity_id(10, 0),
                goal_key: sample_goal_key(),
                step_index: 2,
                repair_kind: RepairKind::AlternateTarget,
                substitute_target: Some(entity_id(11, 0)),
            }),
            DecisionEventPayload::ReplanTriggered(ReplanTriggeredPayload {
                agent: entity_id(12, 0),
                goal_key: sample_goal_key(),
                reason: ReplanReason::PlanInvalidated,
            }),
            DecisionEventPayload::BlockerRecorded(BlockerRecordedPayload {
                agent: entity_id(13, 0),
                blocker_key: sample_blocker_key(),
                discrepancy: Some(Discrepancy::BeliefContradicted),
                blocking_fact: Some(BlockingFact::TargetGone),
                expires_tick: Tick(99),
            }),
        ]
    }

    #[test]
    fn decision_event_payload_types_satisfy_required_bounds() {
        assert_value_bounds::<DecisionEventPayload>();
        assert_value_bounds::<GoalOfferedPayload>();
        assert_copy_value_bounds::<EmitterTag>();
        assert_value_bounds::<EvidenceSummary>();
        assert_copy_value_bounds::<EvidenceKindTag>();
        assert_value_bounds::<GoalSuppressedPayload>();
        assert_value_bounds::<GoalCommittedPayload>();
        assert_value_bounds::<RejectedAlternativeSummary>();
        assert_copy_value_bounds::<GoalRejectionReason>();
        assert_value_bounds::<GoalSuspendedPayload>();
        assert_value_bounds::<GoalAbandonedPayload>();
        assert_value_bounds::<PlanAdoptedPayload>();
        assert_value_bounds::<PlanInvalidatedPayload>();
        assert_copy_value_bounds::<PlanInvalidationReason>();
        assert_value_bounds::<ExpectationMismatchPayload>();
        assert_value_bounds::<RepairAppliedPayload>();
        assert_copy_value_bounds::<RepairKind>();
        assert_value_bounds::<ReplanTriggeredPayload>();
        assert_copy_value_bounds::<ReplanReason>();
        assert_value_bounds::<BlockerRecordedPayload>();
    }

    #[test]
    fn decision_event_payload_variants_roundtrip_through_bincode() {
        for payload in sample_decision_payloads() {
            let bytes = bincode::serialize(&payload).unwrap();
            let roundtrip: DecisionEventPayload = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, payload);
        }
    }

    #[test]
    fn evidence_summary_roundtrips_through_bincode() {
        let summary = sample_goal_offered_payload().source_evidence;

        let bytes = bincode::serialize(&summary).unwrap();
        let roundtrip: EvidenceSummary = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, summary);
    }

    #[test]
    fn plan_invalidation_reason_variants_roundtrip_through_bincode() {
        let reasons = [
            PlanInvalidationReason::TargetGone {
                target: entity_id(1, 0),
            },
            PlanInvalidationReason::ExpectationMismatch { step_index: 3 },
            PlanInvalidationReason::ContentionLost {
                place: entity_id(2, 0),
                action: ActionDefId(7),
            },
            PlanInvalidationReason::DiscrepancyRecorded {
                discrepancy: Discrepancy::RouteUnknown,
            },
            PlanInvalidationReason::PreemptedByHigherGoal {
                new_goal: sample_goal_key(),
            },
            PlanInvalidationReason::AgentIncapacitated,
        ];

        for reason in reasons {
            let bytes = bincode::serialize(&reason).unwrap();
            let roundtrip: PlanInvalidationReason = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, reason);
        }
    }
}
