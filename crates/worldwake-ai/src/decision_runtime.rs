use crate::{GoalKey, GoalPriorityClass, PlannedPlan};
use worldwake_core::{CommodityKind, EntityId, HomeostaticNeeds, Quantity, UniqueItemKind, Wound};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AgentDecisionRuntime {
    pub current_goal: Option<GoalKey>,
    pub current_plan: Option<PlannedPlan>,
    pub current_step_index: usize,
    pub step_in_flight: bool,
    pub dirty: bool,
    pub last_priority_class: Option<GoalPriorityClass>,
    pub last_effective_place: Option<EntityId>,
    pub last_needs: Option<HomeostaticNeeds>,
    pub last_wounds: Vec<Wound>,
    pub last_commodity_signature: Vec<(CommodityKind, Quantity)>,
    pub last_unique_item_signature: Vec<(UniqueItemKind, u32)>,
}

#[cfg(test)]
mod tests {
    use super::AgentDecisionRuntime;

    #[test]
    fn agent_decision_runtime_defaults_to_empty_clean_state() {
        let runtime = AgentDecisionRuntime::default();

        assert_eq!(runtime.current_goal, None);
        assert_eq!(runtime.current_plan, None);
        assert_eq!(runtime.current_step_index, 0);
        assert!(!runtime.step_in_flight);
        assert!(!runtime.dirty);
        assert_eq!(runtime.last_priority_class, None);
        assert_eq!(runtime.last_effective_place, None);
        assert_eq!(runtime.last_needs, None);
        assert!(runtime.last_wounds.is_empty());
        assert!(runtime.last_commodity_signature.is_empty());
        assert!(runtime.last_unique_item_signature.is_empty());
    }

    #[test]
    fn agent_decision_runtime_is_not_registered_as_a_component() {
        let component_schema = include_str!("../../worldwake-core/src/component_schema.rs");

        assert!(!component_schema.contains("AgentDecisionRuntime"));
    }
}
