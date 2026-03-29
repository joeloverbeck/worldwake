//! # worldwake-ai
//!
//! GOAP planner, utility scoring, and decision architecture.
//! Depends on `worldwake-core`, `worldwake-sim`, and `worldwake-systems`.

pub mod agent_tick;
pub mod budget;
pub mod candidate_generation;
pub mod decision_runtime;
pub mod decision_trace;
pub mod dirty_set;
mod enterprise;
pub mod exhaustion;
pub mod failure_handling;
pub mod feasibility;
mod frame_switch_policy;
pub mod goal_explanation;
pub mod goal_model;
pub mod goal_policy;
mod goal_switching;
mod institutional_queries;
pub mod interrupts;
pub mod knowledge_path;
pub mod plan_revalidation;
pub mod plan_selection;
pub mod planner_duration_contract;
pub mod planner_ops;
pub mod planning_snapshot;
pub mod planning_state;
pub mod pressure;
pub mod ranking;
pub mod search;
mod shared_collections;
mod theft;

pub use agent_tick::{AgentTickDriver, FrameDebugSnapshot, FrameSwitchMarginSource};
pub use budget::PlanningBudget;
pub use candidate_generation::generate_candidates;
pub use decision_runtime::{
    classify_frame_plan_relation, frame_runtime_snapshot, frame_travel_destination,
    has_active_frame_travel, has_frame, AgentDecisionRuntime, ExhaustionEntry,
    ExhaustionRetryState, FramePlanRelation, FrameRuntimeSnapshot, MaterializationBindings,
};
pub use decision_trace::{
    ActionStartFailureSummary, AgentDecisionTrace, AskWitnessOmissionDetail, BindingRejection,
    CandidateEvidenceContributor, CandidateEvidenceExclusion, CandidateEvidenceExclusionReason,
    CandidateEvidenceKind, CandidateEvidenceTrace, CandidateLegalityTrace, CandidateTrace,
    CompetitionDiscount, DecisionOutcome, DecisionTraceSink, DesireFullyBlocked,
    ExecutionFailureReason, ExecutionTrace, GoalHistoryEntry, GoalSwitchSummary, GoalTraceStatus,
    InterruptTrace, PayloadOverrideFailureReason, PlanAttemptTrace, PlanSearchOutcome,
    PlanSearchTrace, PlannedStepSummary, PlanningPipelineTrace, PoliticalCandidateOmission,
    PoliticalCandidateOmissionReason, PoliticalGoalFamily, PrerequisiteExclusionReason,
    PrerequisiteExclusionTrace, PrerequisiteGuidanceTrace, RankedGoalSummary,
    RootCandidateFilterReason, RootCandidateOutcome, RootCandidatePayloadStatus,
    RootCandidateSkipReason, RootCandidateTrace, RootOperatorOmissionDetail,
    RootOperatorOmissionReason, RootOperatorOmissionTrace, SelectedPlanReplacementKind,
    SelectedPlanReplacementTrace, SelectedPlanSearchProvenance, SelectedPlanSource,
    SelectedPlanTrace, SelectionTrace, SocialCandidateOmission, TravelPruningTrace,
    TravelSuccessorTrace,
};
pub use dirty_set::DirtySet;
pub use exhaustion::{ExhaustionBaseline, ExhaustionInvalidationCondition};
pub use failure_handling::{clear_resolved_blockers, handle_plan_failure, PlanFailureContext};
pub use feasibility::{feasibility_hint, FeasibilityHint};
pub use goal_model::{
    GoalKindPlannerExt, GoalKindTag, GoalPriorityClass, GroundedGoal, RankedDriveGoalProvenance,
    RankedDriveKind, RankedDriveMotiveInput, RankedGoal, RankedGoalProvenance,
    RankedGoalProvenanceFamily, RankedPriorityAdjustment,
};
pub use goal_policy::{
    evaluate_suppression, goal_family_policy, DecisionContext, FreeInterruptRole, GoalFamilyPolicy,
    GoalPolicyOutcome,
};
pub use goal_switching::GoalSwitchKind;
pub use interrupts::{evaluate_interrupt, InterruptDecision, InterruptTrigger};
pub use plan_revalidation::revalidate_next_step;
pub use plan_selection::select_best_plan;
pub use planner_duration_contract::PlannerDurationDependency;
pub use planner_ops::{
    apply_hypothetical_transition, authoritative_target, authoritative_targets,
    build_semantics_table, resolve_planning_target_with, resolve_planning_targets_with,
    ExpectedMaterialization, PlanTerminalKind, PlannedPlan, PlannedStep, PlannerOpKind,
    PlannerOpSemantics, PlannerTransitionKind,
};
pub use planning_snapshot::{
    build_planning_snapshot, build_planning_snapshot_with_blocked_facility_uses, PlanningSnapshot,
};
pub use planning_state::{
    HypotheticalEntityId, HypotheticalEntityMeta, PlanningEntityRef, PlanningState,
};
pub use pressure::{
    assess_danger, classify_band, derive_danger_pressure, derive_pain_pressure, DangerAssessment,
};
pub use ranking::{
    build_decision_context, rank_candidates, RankedGoalComparison, RankedGoalComparisonDimension,
    RankingOutcome,
};
pub use search::{search_plan, PlanSearchResult};
pub use worldwake_core::{CommodityPurpose, GoalKey, GoalKind, OpportunityAnchor, OpportunityKey};

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
