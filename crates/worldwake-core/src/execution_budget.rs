use crate::{Component, ReasoningProfile};
use serde::{Deserialize, Serialize};

/// Stable per-agent execution bounds used to compress planner search.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ExecutionBudget {
    pub max_node_expansions: u16,
    pub beam_width: u8,
    pub snapshot_travel_horizon: u8,
    pub max_prerequisite_locations: u8,
}

impl Default for ExecutionBudget {
    fn default() -> Self {
        Self::from_reasoning_profile(&ReasoningProfile::default())
    }
}

impl ExecutionBudget {
    #[must_use]
    pub fn from_reasoning_profile(reasoning: &ReasoningProfile) -> Self {
        Self {
            max_node_expansions: reasoning.max_node_expansions,
            beam_width: reasoning.beam_width,
            snapshot_travel_horizon: reasoning.snapshot_travel_horizon,
            max_prerequisite_locations: reasoning.max_prerequisite_locations,
        }
    }
}

impl Component for ExecutionBudget {}

#[cfg(test)]
mod tests {
    use super::ExecutionBudget;
    use crate::{ControlSource, EntityKind, ReasoningProfile, Tick, Topology, World, traits::Component};
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn execution_budget_component_bounds() {
        assert_component_bounds::<ExecutionBudget>();
        assert_value_bounds::<ExecutionBudget>();
    }

    #[test]
    fn execution_budget_default_matches_reasoning_profile_engine_fields() {
        let reasoning = ReasoningProfile::default();
        let budget = ExecutionBudget::default();

        assert_eq!(budget.max_node_expansions, reasoning.max_node_expansions);
        assert_eq!(budget.beam_width, reasoning.beam_width);
        assert_eq!(budget.snapshot_travel_horizon, reasoning.snapshot_travel_horizon);
        assert_eq!(
            budget.max_prerequisite_locations,
            reasoning.max_prerequisite_locations
        );
    }

    #[test]
    fn execution_budget_roundtrips_through_bincode() {
        let budget = ExecutionBudget {
            max_node_expansions: 512,
            beam_width: 11,
            snapshot_travel_horizon: 5,
            max_prerequisite_locations: 4,
        };

        let bytes = bincode::serialize(&budget).unwrap();
        let roundtrip: ExecutionBudget = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, budget);
    }

    #[test]
    fn execution_budget_registers_for_agents() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world
            .create_agent("Planner", ControlSource::Ai, Tick(1))
            .unwrap();
        let budget = ExecutionBudget {
            max_node_expansions: 400,
            ..ExecutionBudget::default()
        };

        assert_eq!(
            world.remove_component_execution_budget(agent).unwrap(),
            Some(ExecutionBudget::default())
        );
        world
            .insert_component_execution_budget(agent, budget)
            .unwrap();

        assert_eq!(world.get_component_execution_budget(agent), Some(&budget));
        assert_eq!(
            world.entities_with_execution_budget().collect::<Vec<_>>(),
            vec![agent]
        );
        assert_eq!(
            world.query_execution_budget().collect::<Vec<_>>(),
            vec![(agent, &budget)]
        );
        assert_eq!(world.count_with_execution_budget(), 1);
        assert_eq!(world.entity_kind(agent), Some(EntityKind::Agent));
    }
}
