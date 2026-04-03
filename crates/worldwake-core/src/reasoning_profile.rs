//! Per-agent authoritative reasoning style parameters.

use crate::{Component, Permille};
use serde::{Deserialize, Serialize};

/// Stable per-agent reasoning limits and retry timing used by the AI layer.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReasoningProfile {
    pub max_candidates_to_plan: u8,
    pub max_plan_depth: u8,
    pub snapshot_travel_horizon: u8,
    pub max_prerequisite_locations: u8,
    pub max_node_expansions: u16,
    pub beam_width: u8,
    pub switch_margin: Permille,
    pub transient_block_ticks: u32,
    pub unknown_block_ticks: u32,
    pub structural_block_ticks: u32,
    pub initial_cooldown_ticks: u32,
    pub max_cooldown_ticks: u32,
}

impl Default for ReasoningProfile {
    fn default() -> Self {
        Self {
            max_candidates_to_plan: 2,
            max_plan_depth: 8,
            snapshot_travel_horizon: 6,
            max_prerequisite_locations: 3,
            max_node_expansions: 224,
            beam_width: 8,
            switch_margin: Permille::new_unchecked(100),
            transient_block_ticks: 20,
            unknown_block_ticks: 5,
            structural_block_ticks: 200,
            initial_cooldown_ticks: 4,
            max_cooldown_ticks: 64,
        }
    }
}

impl Component for ReasoningProfile {}

#[cfg(test)]
mod tests {
    use super::ReasoningProfile;
    use crate::{traits::Component, ControlSource, EntityKind, Tick, Topology, World};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn reasoning_profile_component_bounds() {
        assert_component_bounds::<ReasoningProfile>();
        assert_value_bounds::<ReasoningProfile>();
    }

    #[test]
    fn reasoning_profile_default_matches_planning_budget() {
        let profile = ReasoningProfile::default();

        assert_eq!(profile.max_candidates_to_plan, 2);
        assert_eq!(profile.max_plan_depth, 8);
        assert_eq!(profile.snapshot_travel_horizon, 6);
        assert_eq!(profile.max_prerequisite_locations, 3);
        assert_eq!(profile.max_node_expansions, 224);
        assert_eq!(profile.beam_width, 8);
        assert_eq!(profile.switch_margin, crate::Permille::new(100).unwrap());
        assert_eq!(profile.transient_block_ticks, 20);
        assert_eq!(profile.unknown_block_ticks, 5);
        assert_eq!(profile.structural_block_ticks, 200);
        assert_eq!(profile.initial_cooldown_ticks, 4);
        assert_eq!(profile.max_cooldown_ticks, 64);
    }

    #[test]
    fn reasoning_profile_roundtrips_through_bincode() {
        let profile = ReasoningProfile {
            max_candidates_to_plan: 3,
            max_plan_depth: 10,
            snapshot_travel_horizon: 5,
            max_prerequisite_locations: 4,
            max_node_expansions: 512,
            beam_width: 11,
            switch_margin: crate::Permille::new(175).unwrap(),
            transient_block_ticks: 12,
            unknown_block_ticks: 9,
            structural_block_ticks: 320,
            initial_cooldown_ticks: 6,
            max_cooldown_ticks: 72,
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: ReasoningProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn reasoning_profile_registers_for_agents() {
        let mut world = World::new(Topology::new()).unwrap();
        let agent = world
            .create_agent("Planner", ControlSource::Ai, Tick(1))
            .unwrap();
        let profile = ReasoningProfile {
            max_node_expansions: 400,
            ..ReasoningProfile::default()
        };

        world
            .insert_component_reasoning_profile(agent, profile.clone())
            .unwrap();

        assert_eq!(world.get_component_reasoning_profile(agent), Some(&profile));
        assert_eq!(world.entities_with_reasoning_profile().collect::<Vec<_>>(), vec![agent]);
        assert_eq!(
            world.query_reasoning_profile().collect::<Vec<_>>(),
            vec![(agent, &profile)]
        );
        assert_eq!(world.count_with_reasoning_profile(), 1);
        assert_eq!(world.entity_kind(agent), Some(EntityKind::Agent));
    }
}
