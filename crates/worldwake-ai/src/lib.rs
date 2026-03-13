//! # worldwake-ai
//!
//! GOAP planner, utility scoring, and decision architecture.
//! Depends on `worldwake-core`, `worldwake-sim`, and `worldwake-systems`.

pub mod agent_tick;
pub mod budget;
pub mod candidate_generation;
pub mod decision_runtime;
mod enterprise;
pub mod failure_handling;
pub mod goal_model;
mod goal_switching;
pub mod interrupts;
mod journey_switch_policy;
pub mod plan_revalidation;
pub mod plan_selection;
pub mod planner_ops;
pub mod planning_snapshot;
pub mod planning_state;
pub mod pressure;
pub mod ranking;
pub mod search;

pub use agent_tick::AgentTickDriver;
pub use budget::PlanningBudget;
pub use candidate_generation::generate_candidates;
pub use decision_runtime::{
    AgentDecisionRuntime, JourneyCommitmentState, JourneyPlanRelation, MaterializationBindings,
};
pub use failure_handling::{clear_resolved_blockers, handle_plan_failure, PlanFailureContext};
pub use goal_model::{
    GoalKindPlannerExt, GoalKindTag, GoalPriorityClass, GroundedGoal, RankedGoal,
};
pub use interrupts::{evaluate_interrupt, InterruptDecision, InterruptTrigger};
pub use plan_revalidation::revalidate_next_step;
pub use plan_selection::select_best_plan;
pub use planner_ops::{
    apply_hypothetical_transition, authoritative_target, authoritative_targets,
    build_semantics_table, resolve_planning_target_with, resolve_planning_targets_with,
    ExpectedMaterialization, PlanTerminalKind, PlannedPlan, PlannedStep, PlannerOpKind,
    PlannerOpSemantics, PlannerTransitionKind,
};
pub use planning_snapshot::{build_planning_snapshot, PlanningSnapshot};
pub use planning_state::{
    HypotheticalEntityId, HypotheticalEntityMeta, PlanningEntityRef, PlanningState,
};
pub use pressure::{classify_band, derive_danger_pressure, derive_pain_pressure};
pub use ranking::rank_candidates;
pub use search::search_plan;
pub use worldwake_core::{CommodityPurpose, GoalKey, GoalKind};

#[cfg(test)]
mod tests {
    use crate::{PlanningSnapshot, PlanningState};
    use std::any::type_name;
    use worldwake_sim::{
        ActionDefId, ActionDefRegistry, ActionPayload, Affordance, BeliefView, InputEvent,
        InputKind, OmniscientBeliefView, ReplanNeeded,
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
            "worldwake_sim::action_ids::ActionDefId"
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
            assert_type_is_available::<OmniscientBeliefView<'static>>()
                .starts_with("worldwake_sim::omniscient_belief_view::OmniscientBeliefView<"),
            "OmniscientBeliefView should be available from worldwake-sim"
        );

        let _: Option<&dyn BeliefView> = None;
        let _ = assert_type_is_available::<PlanningSnapshot>();
        let _ = assert_type_is_available::<PlanningState<'static>>();
    }
}
