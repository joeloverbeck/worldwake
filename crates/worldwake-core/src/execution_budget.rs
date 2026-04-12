use crate::Component;
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

/// Stable per-agent execution bounds used to compress planner search.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct ExecutionBudget {
    beam_width: u8,
    max_prerequisite_locations: u8,
    /// Number of consecutive preferred-operator expansions before alternating
    /// to the regular queue. Higher values focus search more aggressively on
    /// landmark-derived actions. 0 = no boosting (dual queue alternates 1:1).
    preferred_operator_boost: u8,
}

impl ExecutionBudget {
    pub fn try_new(
        beam_width: u8,
        max_prerequisite_locations: u8,
        preferred_operator_boost: u8,
    ) -> Result<Self, &'static str> {
        if beam_width == 0 {
            return Err("ExecutionBudget.beam_width must be at least 1");
        }
        if max_prerequisite_locations == 0 {
            return Err("ExecutionBudget.max_prerequisite_locations must be at least 1");
        }
        Ok(Self {
            beam_width,
            max_prerequisite_locations,
            preferred_operator_boost,
        })
    }

    pub fn new(
        beam_width: u8,
        max_prerequisite_locations: u8,
        preferred_operator_boost: u8,
    ) -> Self {
        Self::try_new(
            beam_width,
            max_prerequisite_locations,
            preferred_operator_boost,
        )
        .expect("ExecutionBudget invariants violated")
    }

    pub const fn beam_width(&self) -> u8 {
        self.beam_width
    }

    pub const fn max_prerequisite_locations(&self) -> u8 {
        self.max_prerequisite_locations
    }

    pub const fn preferred_operator_boost(&self) -> u8 {
        self.preferred_operator_boost
    }
}

impl Default for ExecutionBudget {
    fn default() -> Self {
        Self::new(8, 3, 2)
    }
}

impl<'de> Deserialize<'de> for ExecutionBudget {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct ExecutionBudgetRepr {
            beam_width: u8,
            max_prerequisite_locations: u8,
            preferred_operator_boost: u8,
        }

        let repr = ExecutionBudgetRepr::deserialize(deserializer)?;
        Self::try_new(
            repr.beam_width,
            repr.max_prerequisite_locations,
            repr.preferred_operator_boost,
        )
        .map_err(D::Error::custom)
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

        assert_eq!(budget.beam_width(), 8);
        assert_eq!(budget.max_prerequisite_locations(), 3);
        assert_eq!(budget.preferred_operator_boost(), 2);
    }

    #[test]
    fn execution_budget_try_new_rejects_zero_beam_width() {
        assert_eq!(
            ExecutionBudget::try_new(0, 3, 2),
            Err("ExecutionBudget.beam_width must be at least 1")
        );
    }

    #[test]
    fn execution_budget_try_new_rejects_zero_prerequisite_locations() {
        assert_eq!(
            ExecutionBudget::try_new(8, 0, 2),
            Err("ExecutionBudget.max_prerequisite_locations must be at least 1")
        );
    }

    #[test]
    fn execution_budget_try_new_allows_zero_preferred_operator_boost() {
        let budget = ExecutionBudget::try_new(8, 3, 0).unwrap();

        assert_eq!(budget.preferred_operator_boost(), 0);
    }

    #[test]
    fn execution_budget_roundtrips_through_bincode() {
        let budget = ExecutionBudget::new(11, 4, 4);

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
        let budget = ExecutionBudget::new(
            12,
            ExecutionBudget::default().max_prerequisite_locations(),
            ExecutionBudget::default().preferred_operator_boost(),
        );

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

    #[test]
    fn execution_budget_deserialization_rejects_zero_values() {
        #[derive(Serialize)]
        struct ExecutionBudgetRepr {
            beam_width: u8,
            max_prerequisite_locations: u8,
            preferred_operator_boost: u8,
        }

        let bytes = bincode::serialize(&ExecutionBudgetRepr {
            beam_width: 0,
            max_prerequisite_locations: 3,
            preferred_operator_boost: 2,
        })
        .unwrap();
        let error = bincode::deserialize::<ExecutionBudget>(&bytes).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("ExecutionBudget.beam_width must be at least 1"),
            "unexpected error: {error}"
        );
    }
}
