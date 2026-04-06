use crate::Component;
use serde::{Deserialize, Serialize};

/// Stable per-agent execution bounds used to compress planner search.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ExecutionBudget {
    pub beam_width: u8,
    pub max_prerequisite_locations: u8,
}

impl Default for ExecutionBudget {
    fn default() -> Self {
        Self {
            beam_width: 8,
            max_prerequisite_locations: 3,
        }
    }
}

impl Component for ExecutionBudget {}

#[cfg(test)]
mod tests {
    use super::ExecutionBudget;
    use crate::{ControlSource, EntityKind, Tick, Topology, World, traits::Component};
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
    fn execution_budget_default_matches_split_defaults() {
        let budget = ExecutionBudget::default();

        assert_eq!(budget.beam_width, 8);
        assert_eq!(budget.max_prerequisite_locations, 3);
    }

    #[test]
    fn execution_budget_roundtrips_through_bincode() {
        let budget = ExecutionBudget {
            beam_width: 11,
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
            beam_width: 12,
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
