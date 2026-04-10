use crate::{Component, Permille};
use serde::{Deserialize, Serialize};

/// Stable per-agent parameters governing need-driven exploration pressure.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ExplorationProfile {
    pub curiosity_weight: Permille,
    pub need_activation_threshold: Permille,
    pub max_consecutive_explorations: u8,
    pub visit_lookback_ticks: u32,
    pub consecutive_exploration_count: u8,
}

impl Default for ExplorationProfile {
    fn default() -> Self {
        Self {
            curiosity_weight: Permille::new_unchecked(500),
            need_activation_threshold: Permille::new_unchecked(400),
            max_consecutive_explorations: 3,
            visit_lookback_ticks: 200,
            consecutive_exploration_count: 0,
        }
    }
}

impl Component for ExplorationProfile {}

#[cfg(test)]
mod tests {
    use super::ExplorationProfile;
    use crate::{ControlSource, EntityKind, Tick, Topology, World, traits::Component};
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn exploration_profile_component_bounds() {
        assert_component_bounds::<ExplorationProfile>();
        assert_value_bounds::<ExplorationProfile>();
    }

    #[test]
    fn exploration_profile_default_matches_spec_defaults() {
        let profile = ExplorationProfile::default();

        assert_eq!(profile.curiosity_weight, crate::Permille::new(500).unwrap());
        assert_eq!(
            profile.need_activation_threshold,
            crate::Permille::new(400).unwrap()
        );
        assert_eq!(profile.max_consecutive_explorations, 3);
        assert_eq!(profile.visit_lookback_ticks, 200);
        assert_eq!(profile.consecutive_exploration_count, 0);
    }

    #[test]
    fn exploration_profile_roundtrips_through_bincode() {
        let profile = ExplorationProfile {
            curiosity_weight: crate::Permille::new(650).unwrap(),
            need_activation_threshold: crate::Permille::new(550).unwrap(),
            max_consecutive_explorations: 4,
            visit_lookback_ticks: 320,
            consecutive_exploration_count: 2,
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: ExplorationProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn exploration_profile_registers_for_agents() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world
            .create_agent("Scout", ControlSource::Ai, Tick(1))
            .unwrap();
        let profile = ExplorationProfile {
            max_consecutive_explorations: 5,
            ..ExplorationProfile::default()
        };

        assert_eq!(
            world.get_component_exploration_profile(agent),
            Some(&ExplorationProfile::default())
        );
        assert_eq!(
            world.remove_component_exploration_profile(agent).unwrap(),
            Some(ExplorationProfile::default())
        );

        world
            .insert_component_exploration_profile(agent, profile)
            .unwrap();

        assert_eq!(
            world.get_component_exploration_profile(agent),
            Some(&profile)
        );
        assert_eq!(
            world
                .entities_with_exploration_profile()
                .collect::<Vec<_>>(),
            vec![agent]
        );
        assert_eq!(
            world.query_exploration_profile().collect::<Vec<_>>(),
            vec![(agent, &profile)]
        );
        assert_eq!(world.count_with_exploration_profile(), 1);
        assert_eq!(world.entity_kind(agent), Some(EntityKind::Agent));
    }
}
