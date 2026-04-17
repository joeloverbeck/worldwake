use crate::{Component, Permille};
use serde::{Deserialize, Serialize};

/// Stable per-agent cognitive reasoning parameters used by the AI layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct CognitiveProfile {
    /// Maximum number of top-scoring goal candidates the planner evaluates per decision cycle.
    pub max_candidates_to_plan: u8,
    /// Maximum action successors expanded per search node during plan search.
    #[serde(default = "default_max_candidates_per_expansion")]
    pub max_candidates_per_expansion: u16,
    /// Maximum number of sequential actions allowed in a single plan.
    pub max_plan_depth: u8,
    /// Optional per-expansion cap on travel candidates kept for successor construction.
    /// `None` preserves the uncapped historical behavior.
    pub max_travel_candidates_per_expansion: Option<u16>,
    /// How many travel hops away the planner considers when building world snapshots for search.
    pub snapshot_travel_horizon: u8,
    /// Hard cap on total nodes expanded during a single plan search before giving up.
    pub max_node_expansions: u16,
    /// Utility margin a new goal must exceed over the current goal to trigger a goal switch during execution.
    pub switch_margin: Permille,
    /// Utility margin a challenger plan must exceed over the current plan to trigger a plan switch.
    pub planning_switch_margin: Permille,
    /// Ticks before a transiently blocked goal is re-evaluated.
    pub transient_block_ticks: u32,
    /// Ticks before a goal blocked for unknown reasons is re-evaluated.
    pub unknown_block_ticks: u32,
    /// Ticks before a structurally blocked goal (no valid plan exists) is re-evaluated.
    pub structural_block_ticks: u32,
    /// Base cooldown ticks after a goal fails before the agent retries it.
    pub initial_cooldown_ticks: u32,
    /// Maximum cooldown ticks after repeated failures (exponential backoff cap).
    pub max_cooldown_ticks: u32,
    /// Maximum entities included per place in the planner's world snapshot.
    pub max_snapshot_entities_per_place: u16,
    /// Maximum depth of landmark chain extraction during tactical planning.
    /// Higher values produce more landmarks for better search guidance at
    /// increased extraction cost. 0 disables landmarks.
    pub landmark_extraction_depth: u8,
    /// Whether this agent uses the FF-style relaxed-plan heuristic for
    /// tactical search guidance.
    #[serde(default = "default_use_ff_heuristic")]
    pub use_ff_heuristic: bool,
}

impl Default for CognitiveProfile {
    fn default() -> Self {
        Self {
            max_candidates_to_plan: 2,
            max_candidates_per_expansion: default_max_candidates_per_expansion(),
            max_plan_depth: 8,
            max_travel_candidates_per_expansion: None,
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
            landmark_extraction_depth: 4,
            use_ff_heuristic: default_use_ff_heuristic(),
        }
    }
}

impl Component for CognitiveProfile {}

const fn default_max_candidates_per_expansion() -> u16 {
    200
}

const fn default_use_ff_heuristic() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::CognitiveProfile;
    use crate::{ControlSource, EntityKind, Tick, Topology, World, traits::Component};
    use ron::{
        de::from_str,
        ser::{PrettyConfig, to_string_pretty},
    };
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
        assert_eq!(profile.max_candidates_per_expansion, 200);
        assert_eq!(profile.max_plan_depth, 8);
        assert_eq!(profile.max_travel_candidates_per_expansion, None);
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
        assert_eq!(profile.landmark_extraction_depth, 4);
        assert!(profile.use_ff_heuristic);
    }

    #[test]
    fn cognitive_profile_roundtrips_through_bincode() {
        let profile = CognitiveProfile {
            max_candidates_to_plan: 3,
            max_candidates_per_expansion: 144,
            max_plan_depth: 10,
            max_travel_candidates_per_expansion: Some(5),
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
            landmark_extraction_depth: 5,
            use_ff_heuristic: false,
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: CognitiveProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn cognitive_profile_deserialization_defaults_use_ff_heuristic() {
        let serialized = to_string_pretty(
            &CognitiveProfile {
                use_ff_heuristic: false,
                ..CognitiveProfile::default()
            },
            PrettyConfig::default(),
        )
        .unwrap();
        let without_field = serialized
            .lines()
            .filter(|line| !line.contains("use_ff_heuristic"))
            .collect::<Vec<_>>()
            .join("\n");
        let profile: CognitiveProfile = from_str(&without_field).unwrap();

        assert!(profile.use_ff_heuristic);
    }

    #[test]
    fn cognitive_profile_deserialization_defaults_travel_candidate_cap_to_none() {
        let serialized = to_string_pretty(
            &CognitiveProfile {
                max_travel_candidates_per_expansion: Some(4),
                ..CognitiveProfile::default()
            },
            PrettyConfig::default(),
        )
        .unwrap();
        let without_field = serialized
            .lines()
            .filter(|line| !line.contains("max_travel_candidates_per_expansion"))
            .collect::<Vec<_>>()
            .join("\n");
        let profile: CognitiveProfile = from_str(&without_field).unwrap();

        assert_eq!(profile.max_travel_candidates_per_expansion, None);
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
