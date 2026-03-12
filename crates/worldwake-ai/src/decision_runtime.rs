use crate::{GoalKey, GoalPriorityClass, HypotheticalEntityId, PlannedPlan};
use std::collections::BTreeMap;
use worldwake_core::{CommodityKind, EntityId, HomeostaticNeeds, Quantity, UniqueItemKind, Wound};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MaterializationBindings {
    pub hypothetical_to_authoritative: BTreeMap<HypotheticalEntityId, EntityId>,
}

impl MaterializationBindings {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind(&mut self, hyp: HypotheticalEntityId, auth: EntityId) {
        self.hypothetical_to_authoritative.insert(hyp, auth);
    }

    #[must_use]
    pub fn resolve(&self, hyp: HypotheticalEntityId) -> Option<EntityId> {
        self.hypothetical_to_authoritative.get(&hyp).copied()
    }

    pub fn clear(&mut self) {
        self.hypothetical_to_authoritative.clear();
    }
}

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
    pub materialization_bindings: MaterializationBindings,
}

#[cfg(test)]
mod tests {
    use super::{AgentDecisionRuntime, MaterializationBindings};
    use crate::HypotheticalEntityId;
    use worldwake_core::EntityId;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

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
        assert!(runtime
            .materialization_bindings
            .hypothetical_to_authoritative
            .is_empty());
    }

    #[test]
    fn agent_decision_runtime_is_not_registered_as_a_component() {
        let component_schema = include_str!("../../worldwake-core/src/component_schema.rs");

        assert!(!component_schema.contains("AgentDecisionRuntime"));
    }

    #[test]
    fn materialization_bindings_bind_and_resolve_entries() {
        let mut bindings = MaterializationBindings::new();
        let hypothetical = HypotheticalEntityId(4);
        let authoritative = entity(9);

        bindings.bind(hypothetical, authoritative);

        assert_eq!(bindings.resolve(hypothetical), Some(authoritative));
    }

    #[test]
    fn materialization_bindings_clear_removes_all_entries() {
        let mut bindings = MaterializationBindings::new();
        bindings.bind(HypotheticalEntityId(1), entity(2));
        bindings.bind(HypotheticalEntityId(3), entity(4));

        bindings.clear();

        assert_eq!(bindings.resolve(HypotheticalEntityId(1)), None);
        assert_eq!(bindings.resolve(HypotheticalEntityId(3)), None);
        assert!(bindings.hypothetical_to_authoritative.is_empty());
    }
}
