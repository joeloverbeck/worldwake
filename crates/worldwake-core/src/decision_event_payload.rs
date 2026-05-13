use crate::{
    ActionDefId, BeliefClaimKey, BlockerKey, BlockingFact, CommodityKind, Discrepancy,
    EntityBeliefAspect, EntityId, ExpectationKindTag, FrameAssumption, FrameClearReason, GoalKey,
    HomeostaticNeedId, HypothesisKind, MaterializationTag, MismatchDetail, MotiveSourceRef,
    OpportunityKey, Permille, RecipeId, SleepRecoveryModifier, SuspensionReason, Tick,
    WakeCondition,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::num::NonZeroU32;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DecisionEventPayload {
    GoalOffered(GoalOfferedPayload),
    GoalSuppressed(GoalSuppressedPayload),
    GoalCommitted(GoalCommittedPayload),
    GoalSuspended(GoalSuspendedPayload),
    GoalAbandoned(GoalAbandonedPayload),
    SleepEpisodeStarted(SleepEpisodeStartedPayload),
    SleepEpisodeEnded(SleepEpisodeEndedPayload),
    PlanAdopted(PlanAdoptedPayload),
    PlanInvalidated(PlanInvalidatedPayload),
    ExpectationMismatch(ExpectationMismatchPayload),
    SourceExpectationFailure(SourceExpectationFailurePayload),
    RepairApplied(RepairAppliedPayload),
    ReplanTriggered(ReplanTriggeredPayload),
    BlockerRecorded(BlockerRecordedPayload),
    WasteCreated(WasteCreatedPayload),
    WashFacilityUsed(WashFacilityUsedPayload),
    SurveyRecorded(SurveyRecordedPayload),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GoalOfferedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub emitter: EmitterTag,
    pub source_evidence: EvidenceSummary,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SleepEpisodeStartedPayload {
    pub sleeper: EntityId,
    pub place: EntityId,
    pub intended_min_ticks: NonZeroU32,
    pub intended_max_ticks: NonZeroU32,
    pub target_recovery: Permille,
    pub wake_conditions: Vec<WakeCondition>,
    pub recovery_modifier: SleepRecoveryModifier,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SleepEpisodeEndedPayload {
    pub sleeper: EntityId,
    pub place: EntityId,
    pub start_tick: Tick,
    pub end_tick: Tick,
    pub end_reason: WakeReason,
    pub accumulated_recovery: Permille,
    pub final_fatigue: Permille,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WasteCreatedPayload {
    pub creator: EntityId,
    pub place: EntityId,
    pub waste_lot: EntityId,
    pub source: WasteSource,
    pub place_dirtiness_delta: Permille,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum WasteSource {
    WildernessRelief,
    OvercapacityLatrine,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WashFacilityUsedPayload {
    pub user: EntityId,
    pub basin: EntityId,
    pub water_consumed: u16,
    pub agent_dirtiness_delta: Permille,
    pub basin_dirtiness_delta: Permille,
    pub partial: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SurveyRecordedPayload {
    pub surveyor: EntityId,
    pub place: EntityId,
    pub hypothesis: HypothesisKind,
    pub found: bool,
    pub confidence: Permille,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum WakeReason {
    IntendedDuration,
    TargetRecovery,
    ProjectedNeedBreach {
        need: HomeostaticNeedId,
        projected_breach_tick: Tick,
    },
    ScheduledCommitment,
    LocalDisturbance,
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
    #[serde(default)]
    pub decisive_motive_sources: Vec<MotiveSourceRef>,
    pub rejected_alternatives: Vec<RejectedAlternativeSummary>,
    #[serde(default)]
    pub assumptions: Vec<PlanAssumptionRef>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RejectedAlternativeSummary {
    pub goal_key: GoalKey,
    pub rejection_reason: GoalRejectionReason,
    pub score_gap: i32,
    #[serde(default)]
    pub rejection_dimension: Option<RankedGoalComparisonDimensionTag>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RankedGoalComparisonDimensionTag {
    PriorityClass,
    SubstitutePreferenceOrder,
    MotiveScore,
    SourceComposite,
    Feasibility,
    GoalSpecificity,
    OpportunityStrength,
    ShareBeliefTopicOrder,
    GoalKindOrder,
    CommodityKey,
    EntityKey,
    PlaceKey,
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
    pub reason: GoalAbandonReason,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GoalAbandonReason {
    FrameCleared {
        reason: FrameClearReason,
    },
    GoalSwitched {
        new_goal: GoalKey,
        switch_kind: GoalSwitchReason,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum GoalSwitchReason {
    HigherPriorityGoal,
    SameClassMargin,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanAdoptedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub plan_step_count: u16,
    #[serde(default)]
    pub assumptions: Vec<PlanAssumptionRef>,
}

/// Frozen belief-envelope metadata captured at the moment a belief-driven
/// blocker or invalidation is written to the decision event log.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefSnapshot {
    pub confidence: Permille,
    pub status: BeliefStatusTag,
    pub acquired_tick: Tick,
}

/// Core-side historical-record mirror of `worldwake-sim::BeliefStatus`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum BeliefStatusTag {
    Certain,
    Probable,
    Stale,
    Disputed,
    Contradicted,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct BeliefRef {
    pub claim_key: BeliefClaimKey,
    pub claim_held_at_tick: Tick,
    pub status: BeliefStatusTag,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct RecordRef {
    pub record_entity: EntityId,
    pub recorded_at_tick: Tick,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ObservationRef {
    pub observed_entity: EntityId,
    pub aspect: EntityBeliefAspect,
    pub observed_tick: Tick,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PlanAssumptionRef {
    pub assumption: FrameAssumption,
    pub introduced_at_step: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanInvalidatedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub reason: PlanInvalidationReason,
    #[serde(default)]
    pub belief_snapshot: Option<BeliefSnapshot>,
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
    PursuitInvalidated {
        reason: PursuitInvalidationReasonTag,
    },
    AssumptionFailed {
        assumption: FrameAssumption,
    },
    AgentIncapacitated,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum PursuitInvalidationReasonTag {
    NoProfile,
    NoBelief,
    TargetDead,
    PlaceUnknown,
    CoLocated,
    PlaceChanged,
    ConfidenceDecayed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExpectationMismatchPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub step_index: u16,
    pub expected_materializations: Vec<MaterializationTag>,
    pub expectation_kind: Option<ExpectationKindTag>,
    pub mismatch_detail: Option<MismatchDetail>,
    #[serde(default)]
    pub decisive_beliefs: Vec<BeliefRef>,
    #[serde(default)]
    pub decisive_records: Vec<RecordRef>,
    #[serde(default)]
    pub decisive_world_observations: Vec<ObservationRef>,
    #[serde(default)]
    pub assumptions: Vec<PlanAssumptionRef>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceExpectationFailurePayload {
    pub agent: EntityId,
    pub opportunity: OpportunityKey,
    pub source: SourceKeyPayload,
    pub expectation_kind: OpportunityExpectationKindTag,
    pub phase: ExpectationFailurePhaseTag,
    pub cause: ExpectationFailureCauseTag,
    pub detected_at_tick: Tick,
    pub attribution_outcome: SourceAttributionOutcomeTag,
    #[serde(default)]
    pub decisive_beliefs: Vec<BeliefRef>,
    #[serde(default)]
    pub decisive_records: Vec<RecordRef>,
    #[serde(default)]
    pub decisive_world_observations: Vec<ObservationRef>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SourceKeyPayload {
    pub entity: EntityId,
    pub commodity: CommodityKind,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum OpportunityExpectationKindTag {
    AcquireCommodityFromConcreteSource,
    RestockCommodityFromConcreteSource,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ExpectationFailurePhaseTag {
    Observation,
    CandidateGeneration,
    Search,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ExpectationFailureCauseTag {
    SourceAbsentLocally,
    SourceDepletedLocally,
    SameGoalSearchInfeasibleWhileSiblingSucceeded,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SourceAttributionOutcomeTag {
    SourceReliabilityDecremented,
    SourceInvalidatedFrameReconsidered,
    CoalescedDuplicate,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepairAppliedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub step_index: u16,
    pub repair_kind: RepairKind,
    pub substitute_target: Option<EntityId>,
    #[serde(default)]
    pub substitute_recipe: Option<RecipeId>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RepairKind {
    RebindTarget,
    ReplaceProvider,
    InsertVerification,
    DowngradeToProgressBarrier,
    Abandon,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplanTriggeredPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub reason: ReplanReason,
    #[serde(default)]
    pub decisive_beliefs: Vec<BeliefRef>,
    #[serde(default)]
    pub decisive_records: Vec<RecordRef>,
    #[serde(default)]
    pub decisive_world_observations: Vec<ObservationRef>,
    #[serde(default)]
    pub assumptions: Vec<PlanAssumptionRef>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ReplanReason {
    PlanInvalidated {
        reason: PlanInvalidationReason,
    },
    ActionInterrupted {
        reason: ActionInterruptReasonTag,
    },
    ActionStartFailed,
    BlockingFactRecorded {
        blocking_fact: BlockingFact,
    },
    DiscrepancyRecorded {
        discrepancy: Discrepancy,
    },
    LocalRepairExhausted,
    SearchBudgetExhausted,
    GoalSwitched {
        new_goal: GoalKey,
        switch_kind: GoalSwitchReason,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ActionInterruptReasonTag {
    DangerNearby,
    Reprioritized,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockerRecordedPayload {
    pub agent: EntityId,
    pub blocker_key: BlockerKey,
    pub discrepancy: Option<Discrepancy>,
    pub blocking_fact: Option<BlockingFact>,
    pub expires_tick: Tick,
    #[serde(default)]
    pub belief_snapshot: Option<BeliefSnapshot>,
    #[serde(default)]
    pub decisive_beliefs: Vec<BeliefRef>,
    #[serde(default)]
    pub decisive_records: Vec<RecordRef>,
    #[serde(default)]
    pub decisive_world_observations: Vec<ObservationRef>,
    #[serde(default)]
    pub assumptions: Vec<PlanAssumptionRef>,
}

#[cfg(test)]
mod tests {
    use super::{
        ActionInterruptReasonTag, BeliefRef, BeliefSnapshot, BeliefStatusTag,
        BlockerRecordedPayload, DecisionEventPayload, EmitterTag, EvidenceKindTag, EvidenceSummary,
        ExpectationFailureCauseTag, ExpectationFailurePhaseTag, ExpectationMismatchPayload,
        GoalAbandonReason, GoalAbandonedPayload, GoalCommittedPayload, GoalOfferedPayload,
        GoalRejectionReason, GoalSuppressedPayload, GoalSuspendedPayload, GoalSwitchReason,
        ObservationRef, OpportunityExpectationKindTag, PlanAdoptedPayload, PlanAssumptionRef,
        PlanInvalidatedPayload, PlanInvalidationReason, PursuitInvalidationReasonTag,
        RankedGoalComparisonDimensionTag, RecordRef, RejectedAlternativeSummary,
        RepairAppliedPayload, RepairKind, ReplanReason, ReplanTriggeredPayload,
        SleepEpisodeEndedPayload, SleepEpisodeStartedPayload, SourceAttributionOutcomeTag,
        SourceExpectationFailurePayload, SourceKeyPayload, SurveyRecordedPayload, WakeReason,
        WashFacilityUsedPayload, WasteCreatedPayload, WasteSource,
    };
    use crate::{
        ActionDefId, BeliefClaimKey, BlockingFact, CommodityKind, Discrepancy, EntityBeliefAspect,
        ExpectationKindTag, FrameAssumption, FrameClearReason, HomeostaticNeedId, HypothesisKind,
        InvalidatorTag, MaterializationTag, MismatchDetail, MotiveSourceRef, ObservationPredicate,
        OpportunityAnchor, OpportunityKey, RecipeId, SuspensionReason, Tick, WakeCondition,
        test_utils::{entity_id, sample_blocker_key, sample_goal_key},
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::collections::BTreeMap;
    use std::fmt::Debug;
    use std::num::NonZeroU32;

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

    fn sample_belief_snapshot() -> BeliefSnapshot {
        BeliefSnapshot {
            confidence: crate::Permille::new(425).unwrap(),
            status: BeliefStatusTag::Stale,
            acquired_tick: Tick(17),
        }
    }

    fn sample_source_expectation_failure_payload() -> SourceExpectationFailurePayload {
        SourceExpectationFailurePayload {
            agent: entity_id(8, 0),
            opportunity: OpportunityKey {
                goal_key: sample_goal_key(),
                anchor: OpportunityAnchor::Place(entity_id(8, 1)),
            },
            source: SourceKeyPayload {
                entity: entity_id(8, 2),
                commodity: CommodityKind::Apple,
            },
            expectation_kind: OpportunityExpectationKindTag::AcquireCommodityFromConcreteSource,
            phase: ExpectationFailurePhaseTag::Observation,
            cause: ExpectationFailureCauseTag::SourceDepletedLocally,
            detected_at_tick: Tick(33),
            attribution_outcome: SourceAttributionOutcomeTag::SourceReliabilityDecremented,
            decisive_beliefs: Vec::new(),
            decisive_records: Vec::new(),
            decisive_world_observations: Vec::new(),
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
                decisive_motive_sources: Vec::new(),
                rejected_alternatives: vec![RejectedAlternativeSummary {
                    goal_key: sample_goal_key(),
                    rejection_reason: GoalRejectionReason::LowerMotive,
                    score_gap: 17,
                    rejection_dimension: Some(RankedGoalComparisonDimensionTag::MotiveScore),
                }],
                assumptions: vec![PlanAssumptionRef {
                    assumption: FrameAssumption::NoCriticalThreat,
                    introduced_at_step: 0,
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
                reason: GoalAbandonReason::GoalSwitched {
                    new_goal: sample_goal_key(),
                    switch_kind: GoalSwitchReason::SameClassMargin,
                },
            }),
            DecisionEventPayload::SleepEpisodeStarted(SleepEpisodeStartedPayload {
                sleeper: entity_id(5, 1),
                place: entity_id(5, 2),
                intended_min_ticks: NonZeroU32::new(4).unwrap(),
                intended_max_ticks: NonZeroU32::new(40).unwrap(),
                target_recovery: crate::Permille::new(750).unwrap(),
                wake_conditions: vec![
                    WakeCondition::IntendedDurationReached,
                    WakeCondition::ProjectedNeedBreach {
                        need: HomeostaticNeedId::Thirst,
                    },
                ],
                recovery_modifier: crate::SleepRecoveryModifier::new(1250),
            }),
            DecisionEventPayload::SleepEpisodeEnded(SleepEpisodeEndedPayload {
                sleeper: entity_id(5, 1),
                place: entity_id(5, 2),
                start_tick: Tick(10),
                end_tick: Tick(24),
                end_reason: WakeReason::ProjectedNeedBreach {
                    need: HomeostaticNeedId::Thirst,
                    projected_breach_tick: Tick(25),
                },
                accumulated_recovery: crate::Permille::new(225).unwrap(),
                final_fatigue: crate::Permille::new(375).unwrap(),
            }),
            DecisionEventPayload::WasteCreated(WasteCreatedPayload {
                creator: entity_id(5, 1),
                place: entity_id(5, 2),
                waste_lot: entity_id(5, 3),
                source: WasteSource::WildernessRelief,
                place_dirtiness_delta: crate::Permille::new(80).unwrap(),
            }),
            DecisionEventPayload::WashFacilityUsed(WashFacilityUsedPayload {
                user: entity_id(5, 1),
                basin: entity_id(5, 4),
                water_consumed: 1,
                agent_dirtiness_delta: crate::Permille::new(500).unwrap(),
                basin_dirtiness_delta: crate::Permille::new(25).unwrap(),
                partial: true,
            }),
            DecisionEventPayload::SurveyRecorded(SurveyRecordedPayload {
                surveyor: entity_id(5, 1),
                place: entity_id(5, 2),
                hypothesis: HypothesisKind::MayContainCommodity {
                    commodity: CommodityKind::Apple,
                },
                found: false,
                confidence: crate::Permille::new(875).unwrap(),
            }),
            DecisionEventPayload::PlanAdopted(PlanAdoptedPayload {
                agent: entity_id(6, 0),
                goal_key: sample_goal_key(),
                plan_step_count: 3,
                assumptions: vec![PlanAssumptionRef {
                    assumption: FrameAssumption::NoCriticalThreat,
                    introduced_at_step: 0,
                }],
            }),
            DecisionEventPayload::PlanInvalidated(PlanInvalidatedPayload {
                agent: entity_id(7, 0),
                goal_key: sample_goal_key(),
                reason: PlanInvalidationReason::PursuitInvalidated {
                    reason: PursuitInvalidationReasonTag::PlaceChanged,
                },
                belief_snapshot: Some(sample_belief_snapshot()),
            }),
            DecisionEventPayload::ExpectationMismatch(ExpectationMismatchPayload {
                agent: entity_id(9, 0),
                goal_key: sample_goal_key(),
                step_index: 1,
                expected_materializations: vec![MaterializationTag::SplitOffLot],
                expectation_kind: None,
                mismatch_detail: None,
                decisive_beliefs: Vec::new(),
                decisive_records: Vec::new(),
                decisive_world_observations: Vec::new(),
                assumptions: Vec::new(),
            }),
            DecisionEventPayload::ExpectationMismatch(ExpectationMismatchPayload {
                agent: entity_id(9, 1),
                goal_key: sample_goal_key(),
                step_index: 2,
                expected_materializations: vec![MaterializationTag::SplitOffLot],
                expectation_kind: Some(ExpectationKindTag::State),
                mismatch_detail: Some(MismatchDetail::GuardInvalidator(
                    InvalidatorTag::TargetMoved,
                )),
                decisive_beliefs: Vec::new(),
                decisive_records: Vec::new(),
                decisive_world_observations: Vec::new(),
                assumptions: Vec::new(),
            }),
            DecisionEventPayload::SourceExpectationFailure(
                sample_source_expectation_failure_payload(),
            ),
            DecisionEventPayload::RepairApplied(RepairAppliedPayload {
                agent: entity_id(10, 0),
                goal_key: sample_goal_key(),
                step_index: 2,
                repair_kind: RepairKind::RebindTarget,
                substitute_target: Some(entity_id(11, 0)),
                substitute_recipe: None,
            }),
            DecisionEventPayload::ReplanTriggered(ReplanTriggeredPayload {
                agent: entity_id(12, 0),
                goal_key: sample_goal_key(),
                reason: ReplanReason::PlanInvalidated {
                    reason: PlanInvalidationReason::AssumptionFailed {
                        assumption: FrameAssumption::NoCriticalThreat,
                    },
                },
                decisive_beliefs: Vec::new(),
                decisive_records: Vec::new(),
                decisive_world_observations: Vec::new(),
                assumptions: Vec::new(),
            }),
            DecisionEventPayload::BlockerRecorded(BlockerRecordedPayload {
                agent: entity_id(13, 0),
                blocker_key: sample_blocker_key(),
                discrepancy: Some(Discrepancy::BeliefContradicted),
                blocking_fact: Some(BlockingFact::TargetGone),
                expires_tick: Tick(99),
                belief_snapshot: None,
                decisive_beliefs: Vec::new(),
                decisive_records: Vec::new(),
                decisive_world_observations: Vec::new(),
                assumptions: Vec::new(),
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
        assert_copy_value_bounds::<RankedGoalComparisonDimensionTag>();
        assert_copy_value_bounds::<BeliefRef>();
        assert_copy_value_bounds::<RecordRef>();
        assert_copy_value_bounds::<ObservationRef>();
        assert_copy_value_bounds::<PlanAssumptionRef>();
        assert_copy_value_bounds::<GoalRejectionReason>();
        assert_value_bounds::<GoalSuspendedPayload>();
        assert_value_bounds::<GoalAbandonedPayload>();
        assert_value_bounds::<GoalAbandonReason>();
        assert_copy_value_bounds::<GoalSwitchReason>();
        assert_value_bounds::<SleepEpisodeStartedPayload>();
        assert_value_bounds::<SleepEpisodeEndedPayload>();
        assert_value_bounds::<WasteCreatedPayload>();
        assert_copy_value_bounds::<WasteSource>();
        assert_value_bounds::<WashFacilityUsedPayload>();
        assert_value_bounds::<SurveyRecordedPayload>();
        assert_copy_value_bounds::<WakeReason>();
        assert_value_bounds::<PlanAdoptedPayload>();
        assert_value_bounds::<BeliefSnapshot>();
        assert_copy_value_bounds::<BeliefStatusTag>();
        assert_value_bounds::<PlanInvalidatedPayload>();
        assert_value_bounds::<PlanInvalidationReason>();
        assert_copy_value_bounds::<PursuitInvalidationReasonTag>();
        assert_value_bounds::<ExpectationMismatchPayload>();
        assert_value_bounds::<SourceExpectationFailurePayload>();
        assert_value_bounds::<SourceKeyPayload>();
        assert_copy_value_bounds::<OpportunityExpectationKindTag>();
        assert_copy_value_bounds::<ExpectationFailurePhaseTag>();
        assert_copy_value_bounds::<ExpectationFailureCauseTag>();
        assert_copy_value_bounds::<SourceAttributionOutcomeTag>();
        assert_value_bounds::<RepairAppliedPayload>();
        assert_copy_value_bounds::<RepairKind>();
        assert_value_bounds::<ReplanTriggeredPayload>();
        assert_value_bounds::<ReplanReason>();
        assert_copy_value_bounds::<ActionInterruptReasonTag>();
        assert_value_bounds::<BlockerRecordedPayload>();
    }

    #[test]
    fn expectation_mismatch_payload_supports_populated_and_empty_optional_fields() {
        let empty = ExpectationMismatchPayload {
            agent: entity_id(20, 0),
            goal_key: sample_goal_key(),
            step_index: 3,
            expected_materializations: vec![MaterializationTag::SplitOffLot],
            expectation_kind: None,
            mismatch_detail: None,
            decisive_beliefs: Vec::new(),
            decisive_records: Vec::new(),
            decisive_world_observations: Vec::new(),
            assumptions: Vec::new(),
        };
        let populated = ExpectationMismatchPayload {
            agent: entity_id(21, 0),
            goal_key: sample_goal_key(),
            step_index: 4,
            expected_materializations: vec![MaterializationTag::SplitOffLot],
            expectation_kind: Some(ExpectationKindTag::Informed),
            mismatch_detail: Some(MismatchDetail::ObservationMissing {
                predicate: ObservationPredicate::EntityPerceivedAtPlace {
                    entity: entity_id(22, 0),
                    place: entity_id(23, 0),
                },
            }),
            decisive_beliefs: vec![BeliefRef {
                claim_key: BeliefClaimKey {
                    subject: entity_id(22, 0),
                    aspect: EntityBeliefAspect::Location,
                },
                claim_held_at_tick: Tick(40),
                status: BeliefStatusTag::Contradicted,
            }],
            decisive_records: vec![RecordRef {
                record_entity: entity_id(24, 0),
                recorded_at_tick: Tick(41),
            }],
            decisive_world_observations: vec![ObservationRef {
                observed_entity: entity_id(22, 0),
                aspect: EntityBeliefAspect::Location,
                observed_tick: Tick(42),
            }],
            assumptions: vec![PlanAssumptionRef {
                assumption: FrameAssumption::TargetAlive(entity_id(22, 0)),
                introduced_at_step: 1,
            }],
        };

        let encoded_empty = bincode::serialize(&empty).expect("empty payload should serialize");
        let encoded_populated =
            bincode::serialize(&populated).expect("populated payload should serialize");
        let decoded_empty: ExpectationMismatchPayload =
            bincode::deserialize(&encoded_empty).expect("empty payload should deserialize");
        let decoded_populated: ExpectationMismatchPayload =
            bincode::deserialize(&encoded_populated).expect("populated payload should deserialize");

        assert_eq!(decoded_empty, empty);
        assert_eq!(decoded_populated, populated);
    }

    #[test]
    fn source_expectation_failure_payload_roundtrips_through_bincode() {
        let payload = sample_source_expectation_failure_payload();
        let bytes = bincode::serialize(&payload).expect("source expectation failure serializes");
        let roundtrip: SourceExpectationFailurePayload =
            bincode::deserialize(&bytes).expect("source expectation failure deserializes");

        assert_eq!(roundtrip, payload);
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
    fn repair_applied_payload_substitute_recipe_roundtrips_through_bincode() {
        let payload = RepairAppliedPayload {
            agent: entity_id(10, 0),
            goal_key: sample_goal_key(),
            step_index: 2,
            repair_kind: RepairKind::RebindTarget,
            substitute_target: None,
            substitute_recipe: Some(RecipeId(7)),
        };

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: RepairAppliedPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
        assert_eq!(roundtrip.substitute_recipe, Some(RecipeId(7)));
    }

    #[test]
    fn goal_committed_payload_defaults_omitted_decisive_motive_sources() {
        let ron = r"(
            agent: (slot: 3, generation: 0),
            goal_key: (
                kind: Sleep,
                commodity: None,
                entity: None,
                place: None,
            ),
            motive_score: 420,
            rejected_alternatives: [],
            assumptions: [],
        )";

        let payload: GoalCommittedPayload = ron::from_str(ron).unwrap();

        assert!(payload.decisive_motive_sources.is_empty());
    }

    #[test]
    fn goal_committed_payload_roundtrips_populated_decisive_motive_sources() {
        let source = MotiveSourceRef {
            source: crate::MotiveSource::NeedPressure {
                need: HomeostaticNeedId::Hunger,
            },
            introduced_tick: Tick(42),
        };
        let payload = GoalCommittedPayload {
            agent: entity_id(3, 0),
            goal_key: sample_goal_key(),
            motive_score: 420,
            decisive_motive_sources: vec![source.clone()],
            rejected_alternatives: Vec::new(),
            assumptions: Vec::new(),
        };

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: GoalCommittedPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip.decisive_motive_sources, vec![source]);
        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn hygiene_decision_payloads_roundtrip_through_bincode() {
        let payloads = [
            DecisionEventPayload::WasteCreated(WasteCreatedPayload {
                creator: entity_id(30, 0),
                place: entity_id(30, 1),
                waste_lot: entity_id(30, 2),
                source: WasteSource::OvercapacityLatrine,
                place_dirtiness_delta: crate::Permille::new(80).unwrap(),
            }),
            DecisionEventPayload::WashFacilityUsed(WashFacilityUsedPayload {
                user: entity_id(31, 0),
                basin: entity_id(31, 1),
                water_consumed: 2,
                agent_dirtiness_delta: crate::Permille::new(1000).unwrap(),
                basin_dirtiness_delta: crate::Permille::new(50).unwrap(),
                partial: false,
            }),
        ];

        for payload in payloads {
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
    fn blocker_recorded_payload_roundtrips_with_belief_snapshot_some() {
        let payload = BlockerRecordedPayload {
            agent: entity_id(1, 0),
            blocker_key: sample_blocker_key(),
            discrepancy: Some(Discrepancy::BeliefStale),
            blocking_fact: Some(BlockingFact::NoKnownPath),
            expires_tick: Tick(33),
            belief_snapshot: Some(sample_belief_snapshot()),
            decisive_beliefs: Vec::new(),
            decisive_records: Vec::new(),
            decisive_world_observations: Vec::new(),
            assumptions: Vec::new(),
        };

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: BlockerRecordedPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn blocker_recorded_payload_roundtrips_with_belief_snapshot_none() {
        let payload = BlockerRecordedPayload {
            agent: entity_id(1, 0),
            blocker_key: sample_blocker_key(),
            discrepancy: Some(Discrepancy::BeliefStale),
            blocking_fact: Some(BlockingFact::NoKnownPath),
            expires_tick: Tick(33),
            belief_snapshot: None,
            decisive_beliefs: Vec::new(),
            decisive_records: Vec::new(),
            decisive_world_observations: Vec::new(),
            assumptions: Vec::new(),
        };

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: BlockerRecordedPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn plan_invalidated_payload_roundtrips_with_belief_snapshot_some() {
        let payload = PlanInvalidatedPayload {
            agent: entity_id(1, 0),
            goal_key: sample_goal_key(),
            reason: PlanInvalidationReason::BeliefUpdate {
                claim_key: BeliefClaimKey {
                    subject: entity_id(2, 0),
                    aspect: EntityBeliefAspect::Inventory(CommodityKind::Bread),
                },
            },
            belief_snapshot: Some(sample_belief_snapshot()),
        };

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: PlanInvalidatedPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
    }

    #[test]
    fn plan_invalidated_payload_roundtrips_with_belief_snapshot_none() {
        let payload = PlanInvalidatedPayload {
            agent: entity_id(1, 0),
            goal_key: sample_goal_key(),
            reason: PlanInvalidationReason::BeliefUpdate {
                claim_key: BeliefClaimKey {
                    subject: entity_id(2, 0),
                    aspect: EntityBeliefAspect::Inventory(CommodityKind::Bread),
                },
            },
            belief_snapshot: None,
        };

        let bytes = bincode::serialize(&payload).unwrap();
        let roundtrip: PlanInvalidatedPayload = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, payload);
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
            PlanInvalidationReason::PursuitInvalidated {
                reason: PursuitInvalidationReasonTag::ConfidenceDecayed,
            },
            PlanInvalidationReason::AssumptionFailed {
                assumption: FrameAssumption::TargetAlive(entity_id(9, 0)),
            },
            PlanInvalidationReason::AgentIncapacitated,
        ];

        for reason in reasons {
            let bytes = bincode::serialize(&reason).unwrap();
            let roundtrip: PlanInvalidationReason = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, reason);
        }
    }

    #[test]
    fn goal_abandon_reason_variants_roundtrip_through_bincode() {
        let reasons = [
            GoalAbandonReason::FrameCleared {
                reason: FrameClearReason::LostPlan,
            },
            GoalAbandonReason::GoalSwitched {
                new_goal: sample_goal_key(),
                switch_kind: GoalSwitchReason::HigherPriorityGoal,
            },
        ];

        for reason in reasons {
            let bytes = bincode::serialize(&reason).unwrap();
            let roundtrip: GoalAbandonReason = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, reason);
        }
    }

    #[test]
    fn replan_reason_variants_roundtrip_through_bincode() {
        let reasons = [
            ReplanReason::PlanInvalidated {
                reason: PlanInvalidationReason::BeliefUpdate {
                    claim_key: BeliefClaimKey {
                        subject: entity_id(8, 0),
                        aspect: EntityBeliefAspect::Inventory(CommodityKind::Bread),
                    },
                },
            },
            ReplanReason::ActionInterrupted {
                reason: ActionInterruptReasonTag::Reprioritized,
            },
            ReplanReason::ActionStartFailed,
            ReplanReason::BlockingFactRecorded {
                blocking_fact: BlockingFact::NoKnownPath,
            },
            ReplanReason::DiscrepancyRecorded {
                discrepancy: Discrepancy::ImproperPlanningState,
            },
            ReplanReason::LocalRepairExhausted,
            ReplanReason::SearchBudgetExhausted,
            ReplanReason::GoalSwitched {
                new_goal: sample_goal_key(),
                switch_kind: GoalSwitchReason::SameClassMargin,
            },
        ];

        for reason in reasons {
            let bytes = bincode::serialize(&reason).unwrap();
            let roundtrip: ReplanReason = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, reason);
        }
    }
}
