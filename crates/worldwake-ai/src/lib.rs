//! # worldwake-ai
//!
//! GOAP planner, utility scoring, and decision architecture.
//! Depends on `worldwake-core`, `worldwake-sim`, and `worldwake-systems`.

pub mod agent_tick;
pub mod candidate_generation;
pub mod decision_runtime;
pub mod decision_trace;
pub mod dirty_set;
mod enterprise;
pub mod exhaustion;
pub mod failure_handling;
pub mod feasibility;
mod frame_switch_policy;
mod goal_dispatch_decl;
mod goal_dispatch_key;
pub mod goal_explanation;
pub mod goal_model;
pub mod goal_policy;
mod goal_switching;
mod institutional_queries;
pub mod interrupts;
pub mod knowledge_path;
pub mod perf_telemetry;
pub mod plan_revalidation;
pub mod plan_selection;
pub mod planner_duration_contract;
pub mod planner_ops;
pub mod planning_snapshot;
pub mod planning_state;
pub mod pressure;
pub mod pursuit_belief;
pub mod ranking;
mod route_threat;
pub mod search;
mod shared_collections;
pub mod side_benefit;
pub mod survival_forensics;
mod theft;

pub use agent_tick::{AgentTickDriver, FrameDebugSnapshot, FrameSwitchMarginSource};
pub use candidate_generation::generate_candidates;
pub use decision_runtime::{
    AgentDecisionRuntime, ExhaustionEntry, ExhaustionRetryState, FramePlanRelation,
    FrameRuntimeSnapshot, MaterializationBindings, classify_frame_plan_relation,
    frame_runtime_snapshot, frame_travel_destination, has_active_frame_travel, has_frame,
};
pub use decision_trace::{
    ActionStartFailureSummary, AffordanceSummary, AffordanceTrace, AgentDecisionTrace,
    AskWitnessOmissionDetail, BanditCandidateOmission, BanditCandidateOmissionReason,
    BanditGoalFamily, BindingRejection, CandidateEvidenceContributor, CandidateEvidenceExclusion,
    CandidateEvidenceExclusionReason, CandidateEvidenceKind, CandidateEvidenceTrace,
    CandidateLegalityTrace, CandidateTrace, CompetitionDiscount, DecisionOutcome,
    DecisionTraceSink, DesireFullyBlocked, ExecutionFailureReason, ExecutionTrace,
    ExhaustionTraceEntry, GoalHistoryEntry, GoalSwitchSummary, GoalTraceStatus, InterruptTrace,
    PatrolRouteSnapshotTrace, PayloadOverrideFailureReason, PlanAttemptTrace, PlanSearchOutcome,
    PlanSearchTrace, PlannedStepSummary, PlanningPipelineTrace, PoliticalCandidateOmission,
    PoliticalCandidateOmissionReason, PoliticalGoalFamily, PrerequisiteExclusionReason,
    PrerequisiteExclusionTrace, PrerequisiteGuidanceTrace, PursuitDiagnostic,
    PursuitInvalidationReason, PursuitOmissionReason, RankedGoalSummary, RootCandidateFilterReason,
    RootCandidateOutcome, RootCandidatePayloadStatus, RootCandidateSkipReason, RootCandidateTrace,
    RootOperatorOmissionDetail, RootOperatorOmissionReason, RootOperatorOmissionTrace,
    SameGoalPlanningStopReason, SameGoalPlanningTrace, SelectedPlanReplacementKind,
    SelectedPlanReplacementTrace, SelectedPlanSearchProvenance, SelectedPlanSource,
    SelectedPlanTrace, SelectionTrace, SnapshotContinuationOutcome, SnapshotContinuationTrace,
    SocialCandidateOmission, TravelPruningTrace, TravelSuccessorTrace, ViolationDetectionOmission,
    ViolationDetectionOmissionReason,
};
pub use dirty_set::DirtySet;
pub use exhaustion::{ExhaustionBaseline, ExhaustionInvalidationCondition};
pub use failure_handling::{PlanFailureContext, clear_resolved_failures, handle_plan_failure};
pub use feasibility::{FeasibilityHint, feasibility_hint};
pub use goal_dispatch_decl::{FeasibilityStrategy, GoalDispatchDeclaration, InvalidationStrategy};
pub use goal_dispatch_key::GoalDispatchKey;
pub use goal_model::{
    GoalKindPlannerExt, GoalPriorityClass, GroundedGoal, RankedDriveGoalProvenance,
    RankedDriveKind, RankedDriveMotiveInput, RankedGoal, RankedGoalProvenance,
    RankedGoalProvenanceFamily, RankedPriorityAdjustment,
};
pub use goal_policy::{
    DecisionContext, FreeInterruptRole, GoalFamilyPolicy, GoalPolicyOutcome, evaluate_suppression,
};
pub use goal_switching::GoalSwitchKind;
pub use interrupts::{InterruptDecision, InterruptTrigger, evaluate_interrupt};
pub use plan_revalidation::{is_pursuit_plan_invalid, revalidate_next_step};
pub use plan_selection::{SelectionPolicy, select_best_plan};
pub use planner_duration_contract::PlannerDurationDependency;
pub use planner_ops::{
    ExpectedMaterialization, PlanTerminalKind, PlannedPlan, PlannedStep, PlannerOpKind,
    PlannerOpSemantics, PlannerTransitionKind, apply_hypothetical_transition, authoritative_target,
    authoritative_targets, build_semantics_table, resolve_planning_target_with,
    resolve_planning_targets_with,
};
pub use planning_snapshot::{
    PlanningSnapshot, build_planning_snapshot, build_planning_snapshot_with_blocked_facility_uses,
};
pub use planning_state::{
    HypotheticalEntityId, HypotheticalEntityMeta, PlanningEntityRef, PlanningState,
};
pub use pressure::{
    DangerAssessment, assess_danger, classify_band, derive_danger_pressure, derive_pain_pressure,
};
pub use pursuit_belief::{PursuitTargetBelief, pursuit_target_belief};
pub use ranking::{
    RankedGoalComparison, RankedGoalComparisonDimension, RankingOutcome, build_decision_context,
    rank_candidates,
};
pub use search::{PlanSearchResult, search_plan};
pub use side_benefit::{PlanValue, SideBenefit, build_plan_value, detect_side_benefits};
pub use survival_forensics::{
    ActionTraceSnapshot, ActiveActionSummary, BlockerSummary, CriticalWindowFrame,
    CriticalWindowReport, ExhaustionSummary, LocalSurvivalStateSummary, RankedGoalSnapshot,
    SurvivalForensicExtractor,
};
pub use worldwake_core::{
    CognitiveProfile, CommodityPurpose, ExecutionBudget, GoalKey, GoalKind, OpportunityAnchor,
    OpportunityKey,
};

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProfileFixture {
    pub max_candidates_to_plan: u8,
    pub max_plan_depth: u8,
    pub snapshot_travel_horizon: u8,
    pub max_node_expansions: u16,
    pub beam_width: u8,
    pub max_prerequisite_locations: u8,
    pub switch_margin: worldwake_core::Permille,
    pub transient_block_ticks: u32,
    pub structural_block_ticks: u32,
    pub initial_cooldown_ticks: u32,
    pub max_cooldown_ticks: u32,
}

#[cfg(test)]
impl Default for ProfileFixture {
    fn default() -> Self {
        Self {
            max_candidates_to_plan: 2,
            max_plan_depth: 8,
            snapshot_travel_horizon: 6,
            max_node_expansions: 224,
            beam_width: 8,
            max_prerequisite_locations: 3,
            switch_margin: worldwake_core::Permille::new_unchecked(100),
            transient_block_ticks: 20,
            structural_block_ticks: 200,
            initial_cooldown_ticks: 4,
            max_cooldown_ticks: 64,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{PlanningSnapshot, PlanningState};
    use std::any::type_name;
    use worldwake_core::ActionDefId;
    use worldwake_sim::{
        ActionDefRegistry, ActionPayload, Affordance, GoalBeliefView, InputEvent, InputKind,
        PerAgentBeliefView, ReplanNeeded, RuntimeBeliefView,
    };

    fn assert_type_is_available<T>() -> &'static str {
        type_name::<T>()
    }

    #[test]
    fn e13_decision_dependencies_are_available() {
        assert_eq!(
            assert_type_is_available::<ActionDefRegistry>(),
            "worldwake_sim::action_def_registry::ActionDefRegistry"
        );
        assert_eq!(
            assert_type_is_available::<Affordance>(),
            "worldwake_sim::affordance::Affordance"
        );
        assert_eq!(
            assert_type_is_available::<ActionDefId>(),
            "worldwake_core::ids::ActionDefId"
        );
        assert_eq!(
            assert_type_is_available::<ActionPayload>(),
            "worldwake_sim::action_payload::ActionPayload"
        );
        assert_eq!(
            assert_type_is_available::<InputEvent>(),
            "worldwake_sim::input_event::InputEvent"
        );
        assert_eq!(
            assert_type_is_available::<InputKind>(),
            "worldwake_sim::input_event::InputKind"
        );
        assert_eq!(
            assert_type_is_available::<ReplanNeeded>(),
            "worldwake_sim::replan_needed::ReplanNeeded"
        );
        assert!(
            assert_type_is_available::<PerAgentBeliefView<'static>>()
                .starts_with("worldwake_sim::per_agent_belief_view::PerAgentBeliefView<"),
            "PerAgentBeliefView should be available from worldwake-sim"
        );

        let _: Option<&dyn RuntimeBeliefView> = None;
        let _: Option<&dyn GoalBeliefView> = None;
        let _ = assert_type_is_available::<PlanningSnapshot>();
        let _ = assert_type_is_available::<PlanningState<'static>>();
    }
}
