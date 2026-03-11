//! # worldwake-ai
//!
//! GOAP planner, utility scoring, and decision architecture.
//! Depends on `worldwake-core`, `worldwake-sim`, and `worldwake-systems`.

pub mod goal_model;

pub use goal_model::{GoalPriorityClass, GroundedGoal};
pub use worldwake_core::{CommodityPurpose, GoalKey, GoalKind};

#[cfg(test)]
mod tests {
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
    }
}
