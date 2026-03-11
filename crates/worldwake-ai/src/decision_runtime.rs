use crate::{GoalKey, GoalPriorityClass, PlannedPlan};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AgentDecisionRuntime {
    pub current_goal: Option<GoalKey>,
    pub current_plan: Option<PlannedPlan>,
    pub dirty: bool,
    pub last_priority_class: Option<GoalPriorityClass>,
}

#[cfg(test)]
mod tests {
    use super::AgentDecisionRuntime;

    #[test]
    fn agent_decision_runtime_defaults_to_empty_clean_state() {
        let runtime = AgentDecisionRuntime::default();

        assert_eq!(runtime.current_goal, None);
        assert_eq!(runtime.current_plan, None);
        assert!(!runtime.dirty);
        assert_eq!(runtime.last_priority_class, None);
    }

    #[test]
    fn agent_decision_runtime_is_not_registered_as_a_component() {
        let component_schema = include_str!("../../worldwake-core/src/component_schema.rs");

        assert!(!component_schema.contains("AgentDecisionRuntime"));
    }
}
