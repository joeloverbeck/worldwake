use crate::{Component, Tick, numerics::Permille};
use serde::{Deserialize, Serialize};

/// Per-agent parameters governing proactive diversification exploration.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct DiversificationProfile {
    pub base_curiosity: Permille,
    pub comfort_threshold: Permille,
    pub curiosity_buildup_rate: Permille,
    pub exploration_cooldown_ticks: u32,
    pub familiarity_per_visit: Permille,
    pub familiarity_recovery_per_tick: Permille,
    pub familiarity_floor: Permille,
    pub max_exploration_hops: u16,
}

impl Default for DiversificationProfile {
    fn default() -> Self {
        Self {
            base_curiosity: Permille::new_unchecked(400),
            comfort_threshold: Permille::new_unchecked(450),
            curiosity_buildup_rate: Permille::new_unchecked(5),
            exploration_cooldown_ticks: 60,
            familiarity_per_visit: Permille::new_unchecked(150),
            familiarity_recovery_per_tick: Permille::new_unchecked(2),
            familiarity_floor: Permille::new_unchecked(50),
            max_exploration_hops: 3,
        }
    }
}

impl Component for DiversificationProfile {}

/// Tick of the agent's most recent proactive exploration commitment.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct LastProactiveExplorationTick(pub Option<Tick>);

impl Component for LastProactiveExplorationTick {}

#[cfg(test)]
mod tests {
    use super::{DiversificationProfile, LastProactiveExplorationTick};
    use crate::{ControlSource, EntityKind, Tick, Topology, World, traits::Component};
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn diversification_profile_component_bounds() {
        assert_component_bounds::<DiversificationProfile>();
        assert_value_bounds::<DiversificationProfile>();
    }

    #[test]
    fn last_proactive_exploration_tick_component_bounds() {
        assert_component_bounds::<LastProactiveExplorationTick>();
        assert_value_bounds::<LastProactiveExplorationTick>();
    }

    #[test]
    fn diversification_profile_default_matches_spec_defaults() {
        let profile = DiversificationProfile::default();

        assert_eq!(profile.base_curiosity, crate::Permille::new(400).unwrap());
        assert_eq!(
            profile.comfort_threshold,
            crate::Permille::new(450).unwrap()
        );
        assert_eq!(
            profile.curiosity_buildup_rate,
            crate::Permille::new(5).unwrap()
        );
        assert_eq!(profile.exploration_cooldown_ticks, 60);
        assert_eq!(
            profile.familiarity_per_visit,
            crate::Permille::new(150).unwrap()
        );
        assert_eq!(
            profile.familiarity_recovery_per_tick,
            crate::Permille::new(2).unwrap()
        );
        assert_eq!(profile.familiarity_floor, crate::Permille::new(50).unwrap());
        assert_eq!(profile.max_exploration_hops, 3);
    }

    #[test]
    fn diversification_profile_roundtrips_through_bincode() {
        let profile = DiversificationProfile {
            base_curiosity: crate::Permille::new(620).unwrap(),
            comfort_threshold: crate::Permille::new(390).unwrap(),
            curiosity_buildup_rate: crate::Permille::new(8).unwrap(),
            exploration_cooldown_ticks: 75,
            familiarity_per_visit: crate::Permille::new(180).unwrap(),
            familiarity_recovery_per_tick: crate::Permille::new(3).unwrap(),
            familiarity_floor: crate::Permille::new(80).unwrap(),
            max_exploration_hops: 4,
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: DiversificationProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn diversification_profile_registers_for_agents_without_default_seeding() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world
            .create_agent("Scout", ControlSource::Ai, Tick(1))
            .unwrap();
        let profile = DiversificationProfile {
            max_exploration_hops: 5,
            ..DiversificationProfile::default()
        };

        assert_eq!(world.get_component_diversification_profile(agent), None);

        world
            .insert_component_diversification_profile(agent, profile)
            .unwrap();

        assert_eq!(
            world.get_component_diversification_profile(agent),
            Some(&profile)
        );
        assert_eq!(
            world
                .entities_with_diversification_profile()
                .collect::<Vec<_>>(),
            vec![agent]
        );
        assert_eq!(
            world.query_diversification_profile().collect::<Vec<_>>(),
            vec![(agent, &profile)]
        );
        assert_eq!(world.count_with_diversification_profile(), 1);
        assert_eq!(world.entity_kind(agent), Some(EntityKind::Agent));
    }

    #[test]
    fn last_proactive_exploration_tick_registers_for_agents_without_default_seeding() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world
            .create_agent("Scout", ControlSource::Ai, Tick(1))
            .unwrap();
        let tick = LastProactiveExplorationTick(Some(Tick(42)));

        assert_eq!(
            world.get_component_last_proactive_exploration_tick(agent),
            None
        );

        world
            .insert_component_last_proactive_exploration_tick(agent, tick)
            .unwrap();

        assert_eq!(
            world.get_component_last_proactive_exploration_tick(agent),
            Some(&tick)
        );
        assert_eq!(
            world
                .entities_with_last_proactive_exploration_tick()
                .collect::<Vec<_>>(),
            vec![agent]
        );
        assert_eq!(
            world
                .query_last_proactive_exploration_tick()
                .collect::<Vec<_>>(),
            vec![(agent, &tick)]
        );
        assert_eq!(world.count_with_last_proactive_exploration_tick(), 1);
        assert_eq!(world.entity_kind(agent), Some(EntityKind::Agent));
    }
}
