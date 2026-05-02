use crate::{Component, Permille};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PortfolioSlotWeights {
    pub survival: Permille,
    pub commitment: Permille,
    pub economic: Permille,
}

impl Default for PortfolioSlotWeights {
    fn default() -> Self {
        Self {
            survival: Permille::new_unchecked(1000),
            commitment: Permille::new_unchecked(900),
            economic: Permille::new_unchecked(700),
        }
    }
}

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
    /// Ticks before a structurally blocked goal (no valid plan exists) is re-evaluated.
    pub structural_block_ticks: u32,
    /// TTL for stale-belief discrepancies before retry.
    #[serde(default = "default_stale_belief_backoff_ticks")]
    pub stale_belief_backoff_ticks: u32,
    /// TTL for contradicted-belief discrepancies before retry.
    #[serde(default = "default_contradicted_belief_backoff_ticks")]
    pub contradicted_belief_backoff_ticks: u32,
    /// TTL for planner-state discrepancies before retry.
    #[serde(default = "default_improper_state_backoff_ticks")]
    pub improper_state_backoff_ticks: u32,
    /// TTL for missing-observation discrepancies before retry.
    #[serde(default = "default_missing_observation_backoff_ticks")]
    pub missing_observation_backoff_ticks: u32,
    /// TTL for legal-binding discrepancies before retry.
    #[serde(default = "default_no_legal_binding_backoff_ticks")]
    pub no_legal_binding_backoff_ticks: u32,
    /// TTL for counterparty-refusal discrepancies before retry.
    #[serde(default = "default_counterparty_refusal_backoff_ticks")]
    pub counterparty_refusal_backoff_ticks: u32,
    /// TTL for route-unknown discrepancies before retry.
    #[serde(default = "default_route_unknown_backoff_ticks")]
    pub route_unknown_backoff_ticks: u32,
    /// TTL for search-budget-exhaustion discrepancies before retry.
    #[serde(default = "default_search_exhaustion_backoff_ticks")]
    pub search_exhaustion_backoff_ticks: u32,
    /// TTL for partial-execution-drift discrepancies before retry.
    #[serde(default = "default_partial_drift_backoff_ticks")]
    pub partial_drift_backoff_ticks: u32,
    /// Grace window before a plan-step expectation is treated as overdue.
    #[serde(default = "default_expectation_tolerance_ticks")]
    pub expectation_tolerance_ticks: u32,
    /// Per-agent cap on the minimum confidence required for plan-step guards.
    #[serde(default = "default_guard_min_confidence_ceiling")]
    pub guard_min_confidence_ceiling: Permille,
    /// Ticks a successful repair remains ranking-relevant before expiring.
    #[serde(default = "default_repair_memory_ticks")]
    pub repair_memory_ticks: u32,
    /// Ticks a learned opportunity remains ranking-relevant before expiring.
    #[serde(default = "default_learned_opportunity_memory_ticks")]
    pub learned_opportunity_memory_ticks: u32,
    /// Maximum number of survey records retained per agent.
    #[serde(default = "default_survey_memory_capacity")]
    pub survey_memory_capacity: usize,
    /// Ticks a survey record remains ranking-relevant before expiring.
    #[serde(default = "default_survey_memory_retention_ticks")]
    pub survey_memory_retention_ticks: u64,
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
    /// Maximum number of rejected alternatives recorded in decision history events.
    #[serde(default = "default_decision_history_alternatives")]
    pub decision_history_alternatives: u8,
    /// Relative slot weights for portfolio candidate ordering.
    #[serde(default)]
    pub slot_weights: PortfolioSlotWeights,
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
            structural_block_ticks: 200,
            stale_belief_backoff_ticks: default_stale_belief_backoff_ticks(),
            contradicted_belief_backoff_ticks: default_contradicted_belief_backoff_ticks(),
            improper_state_backoff_ticks: default_improper_state_backoff_ticks(),
            missing_observation_backoff_ticks: default_missing_observation_backoff_ticks(),
            no_legal_binding_backoff_ticks: default_no_legal_binding_backoff_ticks(),
            counterparty_refusal_backoff_ticks: default_counterparty_refusal_backoff_ticks(),
            route_unknown_backoff_ticks: default_route_unknown_backoff_ticks(),
            search_exhaustion_backoff_ticks: default_search_exhaustion_backoff_ticks(),
            partial_drift_backoff_ticks: default_partial_drift_backoff_ticks(),
            expectation_tolerance_ticks: default_expectation_tolerance_ticks(),
            guard_min_confidence_ceiling: default_guard_min_confidence_ceiling(),
            repair_memory_ticks: default_repair_memory_ticks(),
            learned_opportunity_memory_ticks: default_learned_opportunity_memory_ticks(),
            survey_memory_capacity: default_survey_memory_capacity(),
            survey_memory_retention_ticks: default_survey_memory_retention_ticks(),
            initial_cooldown_ticks: 4,
            max_cooldown_ticks: 64,
            max_snapshot_entities_per_place: 50,
            landmark_extraction_depth: 4,
            use_ff_heuristic: default_use_ff_heuristic(),
            decision_history_alternatives: default_decision_history_alternatives(),
            slot_weights: PortfolioSlotWeights::default(),
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

const fn default_decision_history_alternatives() -> u8 {
    5
}

const fn default_stale_belief_backoff_ticks() -> u32 {
    30
}

const fn default_contradicted_belief_backoff_ticks() -> u32 {
    60
}

const fn default_improper_state_backoff_ticks() -> u32 {
    2
}

const fn default_missing_observation_backoff_ticks() -> u32 {
    20
}

const fn default_no_legal_binding_backoff_ticks() -> u32 {
    120
}

const fn default_counterparty_refusal_backoff_ticks() -> u32 {
    40
}

const fn default_route_unknown_backoff_ticks() -> u32 {
    200
}

const fn default_search_exhaustion_backoff_ticks() -> u32 {
    100
}

const fn default_partial_drift_backoff_ticks() -> u32 {
    4
}

const fn default_expectation_tolerance_ticks() -> u32 {
    2
}

const fn default_guard_min_confidence_ceiling() -> Permille {
    Permille::new_unchecked(1000)
}

const fn default_repair_memory_ticks() -> u32 {
    120
}

const fn default_learned_opportunity_memory_ticks() -> u32 {
    60
}

const fn default_survey_memory_capacity() -> usize {
    24
}

const fn default_survey_memory_retention_ticks() -> u64 {
    300
}

#[cfg(test)]
mod tests {
    use super::{CognitiveProfile, PortfolioSlotWeights};
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
        assert_value_bounds::<PortfolioSlotWeights>();
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
        assert_eq!(profile.structural_block_ticks, 200);
        assert_eq!(profile.stale_belief_backoff_ticks, 30);
        assert_eq!(profile.contradicted_belief_backoff_ticks, 60);
        assert_eq!(profile.improper_state_backoff_ticks, 2);
        assert_eq!(profile.missing_observation_backoff_ticks, 20);
        assert_eq!(profile.no_legal_binding_backoff_ticks, 120);
        assert_eq!(profile.counterparty_refusal_backoff_ticks, 40);
        assert_eq!(profile.route_unknown_backoff_ticks, 200);
        assert_eq!(profile.search_exhaustion_backoff_ticks, 100);
        assert_eq!(profile.partial_drift_backoff_ticks, 4);
        assert_eq!(profile.expectation_tolerance_ticks, 2);
        assert_eq!(
            profile.guard_min_confidence_ceiling,
            crate::Permille::new(1000).unwrap()
        );
        assert_eq!(profile.repair_memory_ticks, 120);
        assert_eq!(profile.learned_opportunity_memory_ticks, 60);
        assert_eq!(profile.survey_memory_capacity, 24);
        assert_eq!(profile.survey_memory_retention_ticks, 300);
        assert_eq!(profile.initial_cooldown_ticks, 4);
        assert_eq!(profile.max_cooldown_ticks, 64);
        assert_eq!(profile.max_snapshot_entities_per_place, 50);
        assert_eq!(profile.landmark_extraction_depth, 4);
        assert!(profile.use_ff_heuristic);
        assert_eq!(profile.decision_history_alternatives, 5);
        assert_eq!(profile.slot_weights, PortfolioSlotWeights::default());
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
            structural_block_ticks: 320,
            stale_belief_backoff_ticks: 31,
            contradicted_belief_backoff_ticks: 61,
            improper_state_backoff_ticks: 3,
            missing_observation_backoff_ticks: 21,
            no_legal_binding_backoff_ticks: 121,
            counterparty_refusal_backoff_ticks: 41,
            route_unknown_backoff_ticks: 201,
            search_exhaustion_backoff_ticks: 101,
            partial_drift_backoff_ticks: 5,
            expectation_tolerance_ticks: 7,
            guard_min_confidence_ceiling: crate::Permille::new(875).unwrap(),
            repair_memory_ticks: 144,
            learned_opportunity_memory_ticks: 88,
            survey_memory_capacity: 12,
            survey_memory_retention_ticks: 240,
            initial_cooldown_ticks: 6,
            max_cooldown_ticks: 72,
            max_snapshot_entities_per_place: 75,
            landmark_extraction_depth: 5,
            use_ff_heuristic: false,
            decision_history_alternatives: 8,
            slot_weights: PortfolioSlotWeights {
                survival: crate::Permille::new(950).unwrap(),
                commitment: crate::Permille::new(800).unwrap(),
                economic: crate::Permille::new(650).unwrap(),
            },
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
    fn cognitive_profile_deserialization_defaults_decision_history_alternatives() {
        let serialized = to_string_pretty(
            &CognitiveProfile {
                decision_history_alternatives: 9,
                ..CognitiveProfile::default()
            },
            PrettyConfig::default(),
        )
        .unwrap();
        let without_field = serialized
            .lines()
            .filter(|line| !line.contains("decision_history_alternatives"))
            .collect::<Vec<_>>()
            .join("\n");
        let profile: CognitiveProfile = from_str(&without_field).unwrap();

        assert_eq!(
            profile.decision_history_alternatives,
            super::default_decision_history_alternatives()
        );
    }

    #[test]
    fn cognitive_profile_deserialization_defaults_slot_weights() {
        let serialized = to_string_pretty(
            &CognitiveProfile {
                slot_weights: PortfolioSlotWeights {
                    survival: crate::Permille::new(850).unwrap(),
                    commitment: crate::Permille::new(750).unwrap(),
                    economic: crate::Permille::new(650).unwrap(),
                },
                ..CognitiveProfile::default()
            },
            PrettyConfig::default(),
        )
        .unwrap();
        let mut skipping_slot_weights = false;
        let without_field = serialized
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                if skipping_slot_weights {
                    if trimmed == ")," {
                        skipping_slot_weights = false;
                    }
                    return false;
                }
                if trimmed.starts_with("slot_weights: (") {
                    skipping_slot_weights = true;
                    return false;
                }
                true
            })
            .collect::<Vec<_>>()
            .join("\n");
        let profile: CognitiveProfile = from_str(&without_field).unwrap();

        assert_eq!(profile.slot_weights, PortfolioSlotWeights::default());
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
    fn cognitive_profile_deserialization_defaults_memory_ttls() {
        let serialized = to_string_pretty(
            &CognitiveProfile {
                repair_memory_ticks: 11,
                learned_opportunity_memory_ticks: 22,
                survey_memory_capacity: 13,
                survey_memory_retention_ticks: 44,
                ..CognitiveProfile::default()
            },
            PrettyConfig::default(),
        )
        .unwrap();
        let without_fields = serialized
            .lines()
            .filter(|line| {
                !line.contains("repair_memory_ticks")
                    && !line.contains("learned_opportunity_memory_ticks")
                    && !line.contains("survey_memory_capacity")
                    && !line.contains("survey_memory_retention_ticks")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let profile: CognitiveProfile = from_str(&without_fields).unwrap();

        assert_eq!(
            profile.repair_memory_ticks,
            CognitiveProfile::default().repair_memory_ticks
        );
        assert_eq!(
            profile.learned_opportunity_memory_ticks,
            CognitiveProfile::default().learned_opportunity_memory_ticks
        );
        assert_eq!(
            profile.survey_memory_capacity,
            CognitiveProfile::default().survey_memory_capacity
        );
        assert_eq!(
            profile.survey_memory_retention_ticks,
            CognitiveProfile::default().survey_memory_retention_ticks
        );
    }

    #[test]
    fn cognitive_profile_deserialization_defaults_expectation_tolerance_ticks() {
        let serialized = to_string_pretty(
            &CognitiveProfile {
                expectation_tolerance_ticks: 9,
                ..CognitiveProfile::default()
            },
            PrettyConfig::default(),
        )
        .unwrap();
        let without_field = serialized
            .lines()
            .filter(|line| !line.contains("expectation_tolerance_ticks"))
            .collect::<Vec<_>>()
            .join("\n");
        let profile: CognitiveProfile = from_str(&without_field).unwrap();

        assert_eq!(
            profile.expectation_tolerance_ticks,
            super::default_expectation_tolerance_ticks()
        );
    }

    #[test]
    fn cognitive_profile_deserialization_defaults_guard_min_confidence_ceiling() {
        let serialized = to_string_pretty(
            &CognitiveProfile {
                guard_min_confidence_ceiling: crate::Permille::new(700).unwrap(),
                ..CognitiveProfile::default()
            },
            PrettyConfig::default(),
        )
        .unwrap();
        let without_field = serialized
            .lines()
            .filter(|line| !line.contains("guard_min_confidence_ceiling"))
            .collect::<Vec<_>>()
            .join("\n");
        let profile: CognitiveProfile = from_str(&without_field).unwrap();

        assert_eq!(
            profile.guard_min_confidence_ceiling,
            super::default_guard_min_confidence_ceiling()
        );
    }

    #[test]
    fn cognitive_profile_deserialization_defaults_discrepancy_ttls() {
        let serialized = to_string_pretty(
            &CognitiveProfile {
                stale_belief_backoff_ticks: 1,
                contradicted_belief_backoff_ticks: 2,
                improper_state_backoff_ticks: 3,
                missing_observation_backoff_ticks: 4,
                no_legal_binding_backoff_ticks: 5,
                counterparty_refusal_backoff_ticks: 6,
                route_unknown_backoff_ticks: 7,
                search_exhaustion_backoff_ticks: 8,
                partial_drift_backoff_ticks: 9,
                ..CognitiveProfile::default()
            },
            PrettyConfig::default(),
        )
        .unwrap();
        let without_fields = serialized
            .lines()
            .filter(|line| {
                !line.contains("stale_belief_backoff_ticks")
                    && !line.contains("contradicted_belief_backoff_ticks")
                    && !line.contains("improper_state_backoff_ticks")
                    && !line.contains("missing_observation_backoff_ticks")
                    && !line.contains("no_legal_binding_backoff_ticks")
                    && !line.contains("counterparty_refusal_backoff_ticks")
                    && !line.contains("route_unknown_backoff_ticks")
                    && !line.contains("search_exhaustion_backoff_ticks")
                    && !line.contains("partial_drift_backoff_ticks")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let profile: CognitiveProfile = from_str(&without_fields).unwrap();

        assert_eq!(
            profile.stale_belief_backoff_ticks,
            CognitiveProfile::default().stale_belief_backoff_ticks
        );
        assert_eq!(
            profile.contradicted_belief_backoff_ticks,
            CognitiveProfile::default().contradicted_belief_backoff_ticks
        );
        assert_eq!(
            profile.improper_state_backoff_ticks,
            CognitiveProfile::default().improper_state_backoff_ticks
        );
        assert_eq!(
            profile.missing_observation_backoff_ticks,
            CognitiveProfile::default().missing_observation_backoff_ticks
        );
        assert_eq!(
            profile.no_legal_binding_backoff_ticks,
            CognitiveProfile::default().no_legal_binding_backoff_ticks
        );
        assert_eq!(
            profile.counterparty_refusal_backoff_ticks,
            CognitiveProfile::default().counterparty_refusal_backoff_ticks
        );
        assert_eq!(
            profile.route_unknown_backoff_ticks,
            CognitiveProfile::default().route_unknown_backoff_ticks
        );
        assert_eq!(
            profile.search_exhaustion_backoff_ticks,
            CognitiveProfile::default().search_exhaustion_backoff_ticks
        );
        assert_eq!(
            profile.partial_drift_backoff_ticks,
            CognitiveProfile::default().partial_drift_backoff_ticks
        );
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
