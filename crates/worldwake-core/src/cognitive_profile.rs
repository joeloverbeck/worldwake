use crate::{Component, Permille};
use serde::{Deserialize, Serialize};

/// Stable per-agent cognitive reasoning parameters used by the AI layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct CognitiveProfile {
    pub max_candidates_to_plan: u8,
    pub max_plan_depth: u8,
    pub snapshot_travel_horizon: u8,
    pub max_node_expansions: u16,
    pub switch_margin: Permille,
    pub planning_switch_margin: Permille,
    pub transient_block_ticks: u32,
    pub unknown_block_ticks: u32,
    pub structural_block_ticks: u32,
    pub initial_cooldown_ticks: u32,
    pub max_cooldown_ticks: u32,
    pub max_snapshot_entities_per_place: u16,
}

impl Default for CognitiveProfile {
    fn default() -> Self {
        Self {
            max_candidates_to_plan: 2,
            max_plan_depth: 8,
            snapshot_travel_horizon: 6,
            max_node_expansions: 224,
            switch_margin: Permille::new_unchecked(100),
            planning_switch_margin: Permille::new_unchecked(150),
            transient_block_ticks: 20,
            unknown_block_ticks: 5,
            structural_block_ticks: 200,
            initial_cooldown_ticks: 4,
            max_cooldown_ticks: 64,
            max_snapshot_entities_per_place: 50,
        }
    }
}

impl Component for CognitiveProfile {}

#[cfg(test)]
mod tests {
    use super::CognitiveProfile;
    use crate::{ControlSource, EntityKind, Tick, Topology, World, traits::Component};
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn cognitive_profile_component_bounds() {
        assert_component_bounds::<CognitiveProfile>();
        assert_value_bounds::<CognitiveProfile>();
    }

    #[test]
    fn cognitive_profile_default_matches_split_defaults() {
        let profile = CognitiveProfile::default();

        assert_eq!(profile.max_candidates_to_plan, 2);
        assert_eq!(profile.max_plan_depth, 8);
        assert_eq!(profile.snapshot_travel_horizon, 6);
        assert_eq!(profile.max_node_expansions, 224);
        assert_eq!(profile.switch_margin, crate::Permille::new(100).unwrap());
        assert_eq!(
            profile.planning_switch_margin,
            crate::Permille::new(150).unwrap()
        );
        assert_eq!(profile.transient_block_ticks, 20);
        assert_eq!(profile.unknown_block_ticks, 5);
        assert_eq!(profile.structural_block_ticks, 200);
        assert_eq!(profile.initial_cooldown_ticks, 4);
        assert_eq!(profile.max_cooldown_ticks, 64);
        assert_eq!(profile.max_snapshot_entities_per_place, 50);
    }

    #[test]
    fn cognitive_profile_roundtrips_through_bincode() {
        let profile = CognitiveProfile {
            max_candidates_to_plan: 3,
            max_plan_depth: 10,
            snapshot_travel_horizon: 9,
            max_node_expansions: 512,
            switch_margin: crate::Permille::new(175).unwrap(),
            planning_switch_margin: crate::Permille::new(225).unwrap(),
            transient_block_ticks: 12,
            unknown_block_ticks: 9,
            structural_block_ticks: 320,
            initial_cooldown_ticks: 6,
            max_cooldown_ticks: 72,
            max_snapshot_entities_per_place: 75,
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: CognitiveProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn cognitive_profile_registers_for_agents() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world
            .create_agent("Planner", ControlSource::Ai, Tick(1))
            .unwrap();
        let profile = CognitiveProfile {
            max_plan_depth: 12,
            ..CognitiveProfile::default()
        };

        assert_eq!(
            world.remove_component_cognitive_profile(agent).unwrap(),
            Some(CognitiveProfile::default())
        );
        world
            .insert_component_cognitive_profile(agent, profile)
            .unwrap();

        assert_eq!(world.get_component_cognitive_profile(agent), Some(&profile));
        assert_eq!(
            world.entities_with_cognitive_profile().collect::<Vec<_>>(),
            vec![agent]
        );
        assert_eq!(
            world.query_cognitive_profile().collect::<Vec<_>>(),
            vec![(agent, &profile)]
        );
        assert_eq!(world.count_with_cognitive_profile(), 1);
        assert_eq!(world.entity_kind(agent), Some(EntityKind::Agent));
    }
}
