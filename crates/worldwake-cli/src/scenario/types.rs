//! Scenario definition types for RON-based world initialization.
//!
//! These are pure data types — no logic, just the schema for scenario files.
//! All location references use string names, resolved to `EntityId` during spawning.

use std::num::NonZeroU32;

use serde::Deserialize;
use worldwake_core::{
    ArtifactPostingProfile, CarryCapacity, CognitiveProfile, CombatProfile,
    CommodityValuationProfile, CommunicationProfile, ContentionDispositionProfile, ControlSource,
    DisposalProfile, DriveThresholds, EpistemicDispositionProfile, ExecutionBudget,
    ExpectationStore, HomeostaticNeeds, IntentionDispositionProfile, JusticeDispositionProfile,
    LastSeenMemory, MetabolismProfile, ObligationSatiationProfile, PatrolProfile,
    PerceptionProfile, Permille, PlaceVisibilityProfile, PreferenceProfile, PursuitProfile,
    Quantity, SubstitutePreferences, TellProfile, TheftDispositionProfile, TradeDispositionProfile,
    UtilityProfile, ViolationDispositionProfile, WorkstationTag, items::CommodityKind,
    topology::PlaceTag,
};

/// Top-level scenario definition. Describes an entire world to initialize.
#[derive(Clone, Debug, Deserialize)]
pub struct ScenarioDef {
    pub seed: u64,
    pub places: Vec<PlaceDef>,
    #[serde(default)]
    pub edges: Vec<EdgeDef>,
    #[serde(default)]
    pub agents: Vec<AgentDef>,
    #[serde(default)]
    pub items: Vec<ItemDef>,
    #[serde(default)]
    pub facilities: Vec<FacilityDef>,
    #[serde(default)]
    pub resource_sources: Vec<ResourceSourceDef>,
    /// Ticks between checkpoint snapshots for event log compaction.
    /// Default: 50. Set to 0 to disable compaction.
    #[serde(default = "default_compaction_interval")]
    pub compaction_interval: u32,
}

fn default_compaction_interval() -> u32 {
    50
}

/// A place in the world graph.
#[derive(Clone, Debug, Deserialize)]
pub struct PlaceDef {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<PlaceTag>,
    #[serde(default)]
    pub visibility_profile: Option<PlaceVisibilityProfile>,
}

/// A travel edge connecting two places.
#[derive(Clone, Debug, Deserialize)]
pub struct EdgeDef {
    pub from: String,
    pub to: String,
    pub travel_ticks: u32,
    #[serde(default = "default_true")]
    pub bidirectional: bool,
}

/// An agent to spawn in the world.
#[derive(Clone, Debug, Deserialize)]
pub struct AgentDef {
    pub name: String,
    pub location: String,
    pub control: ControlSource,
    #[serde(default)]
    pub needs: Option<HomeostaticNeeds>,
    #[serde(default)]
    pub combat_profile: Option<CombatProfile>,
    #[serde(default)]
    pub utility_profile: Option<UtilityProfile>,
    #[serde(default)]
    pub artifact_posting_profile: Option<ArtifactPostingProfile>,
    #[serde(default)]
    pub merchandise_profile: Option<MerchandiseProfileDef>,
    #[serde(default)]
    pub trade_disposition: Option<TradeDispositionProfile>,
    #[serde(default)]
    pub perception_profile: Option<PerceptionProfile>,
    #[serde(default)]
    pub tell_profile: Option<TellProfile>,
    #[serde(default)]
    pub cognitive_profile: Option<CognitiveProfile>,
    #[serde(default)]
    pub execution_budget: Option<ExecutionBudget>,
    #[serde(default)]
    pub epistemic_disposition: Option<EpistemicDispositionProfile>,
    #[serde(default)]
    pub intention_disposition: Option<IntentionDispositionProfile>,
    #[serde(default)]
    pub communication_profile: Option<CommunicationProfile>,
    #[serde(default)]
    pub preference_profile: Option<PreferenceProfile>,
    #[serde(default)]
    pub expectation_store: Option<ExpectationStore>,
    #[serde(default)]
    pub last_seen_memory: Option<LastSeenMemory>,
    #[serde(default)]
    pub obligation_satiation_profile: Option<ObligationSatiationProfile>,
    #[serde(default)]
    pub drive_thresholds: Option<DriveThresholds>,
    #[serde(default)]
    pub metabolism_profile: Option<MetabolismProfile>,
    #[serde(default)]
    pub disposal_profile: Option<DisposalProfile>,
    #[serde(default)]
    pub exploration_profile: Option<ExplorationProfileDef>,
    #[serde(default)]
    pub carry_capacity: Option<CarryCapacity>,
    #[serde(default)]
    pub theft_disposition: Option<TheftDispositionProfile>,
    #[serde(default)]
    pub justice_disposition: Option<JusticeDispositionProfile>,
    #[serde(default)]
    pub violation_disposition: Option<ViolationDispositionProfile>,
    #[serde(default)]
    pub patrol_profile: Option<PatrolProfile>,
    #[serde(default)]
    pub patrol_route: Option<PatrolRouteDef>,
    #[serde(default)]
    pub pursuit_profile: Option<PursuitProfile>,
    #[serde(default)]
    pub contention_disposition: Option<ContentionDispositionProfile>,
    #[serde(default)]
    pub commodity_valuation: Option<CommodityValuationProfile>,
    #[serde(default)]
    pub substitute_preferences: Option<SubstitutePreferences>,
    #[serde(default)]
    pub known_recipes: Option<Vec<String>>,
}

/// Scenario-specific merchandise profile using string names instead of `EntityId`.
///
/// `MerchandiseProfile` in core contains `home_facility: Option<EntityId>`, which
/// cannot appear in a RON file before entities are spawned. This def uses a
/// facility/entity name string, resolved to `EntityId` during spawning.
#[derive(Clone, Debug, Deserialize)]
pub struct MerchandiseProfileDef {
    pub sale_kinds: Vec<CommodityKind>,
    #[serde(default)]
    pub home_facility: Option<String>,
}

/// Scenario-specific patrol route using string place names instead of `EntityId`.
///
/// `PatrolRoute` in core contains `assigned_places: Vec<EntityId>`, which
/// cannot appear in a RON file before entities are spawned. This def uses
/// place name strings, resolved to `EntityId` during spawning.
#[derive(Clone, Debug, Deserialize)]
pub struct PatrolRouteDef {
    pub assigned_places: Vec<String>,
}

/// Scenario-facing exploration disposition.
///
/// `ExplorationProfile` in core also contains the runtime-only
/// `consecutive_exploration_count`, which must always start at `0` during
/// scenario bootstrap and therefore is not directly authorable in RON.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExplorationProfileDef {
    pub curiosity_weight: Permille,
    pub need_activation_threshold: Permille,
    pub max_consecutive_explorations: u8,
    pub visit_lookback_ticks: u32,
}

/// An item lot to place in the world.
#[derive(Clone, Debug, Deserialize)]
pub struct ItemDef {
    pub commodity: CommodityKind,
    pub quantity: Quantity,
    pub location: String,
    #[serde(default)]
    pub container: bool,
}

/// A workstation facility at a place.
#[derive(Clone, Debug, Deserialize)]
pub struct FacilityDef {
    #[serde(default)]
    pub name: Option<String>,
    pub workstation: WorkstationTag,
    pub location: String,
}

/// A resource source at a place.
#[derive(Clone, Debug, Deserialize)]
pub struct ResourceSourceDef {
    pub commodity: CommodityKind,
    pub location: String,
    #[serde(default)]
    pub facility: Option<String>,
    pub regeneration_ticks_per_unit: Option<NonZeroU32>,
    pub capacity: Quantity,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deserialize with RON extensions that the scenario loader will use.
    fn from_ron_str<'de, T: serde::Deserialize<'de>>(s: &'de str) -> T {
        let options = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::UNWRAP_NEWTYPES)
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME);
        options.from_str(s).expect("RON deserialization failed")
    }

    #[test]
    fn test_scenario_def_deserialize_minimal() {
        let ron_str = r#"(
            seed: 42,
            places: [
                (name: "Village", tags: [Village]),
            ],
            agents: [
                (name: "Alice", location: "Village", control: Human),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        assert_eq!(def.seed, 42);
        assert_eq!(def.places.len(), 1);
        assert_eq!(def.places[0].name, "Village");
        assert_eq!(def.places[0].tags, vec![PlaceTag::Village]);
        assert_eq!(def.places[0].visibility_profile, None);
        assert!(def.edges.is_empty());
        assert_eq!(def.agents.len(), 1);
        assert_eq!(def.agents[0].name, "Alice");
        assert_eq!(def.agents[0].location, "Village");
        assert_eq!(def.agents[0].control, ControlSource::Human);
        assert!(def.items.is_empty());
        assert!(def.facilities.is_empty());
        assert!(def.resource_sources.is_empty());
    }

    #[test]
    fn test_scenario_def_deserialize_full() {
        let ron_str = r#"(
            seed: 123,
            places: [
                (name: "Town", tags: [Village, Store]),
                (name: "Forest", tags: [Forest]),
            ],
            edges: [
                (from: "Town", to: "Forest", travel_ticks: 3, bidirectional: true),
            ],
            agents: [
                (
                    name: "Bob",
                    location: "Town",
                    control: Ai,
                    needs: (
                        hunger: 100,
                        thirst: 200,
                        fatigue: 50,
                        bladder: 0,
                        dirtiness: 0,
                    ),
                    combat_profile: (
                        wound_capacity: 800,
                        incapacitation_threshold: 700,
                        attack_skill: 500,
                        guard_skill: 400,
                        defend_bonus: 100,
                        natural_clot_resistance: 300,
                        natural_recovery_rate: 50,
                        unarmed_wound_severity: 200,
                        unarmed_bleed_rate: 100,
                        unarmed_attack_ticks: 3,
                        defend_stance_ticks: 10,
                    ),
                    utility_profile: (
                        hunger_weight: 500,
                        thirst_weight: 500,
                        fatigue_weight: 500,
                        bladder_weight: 500,
                        dirtiness_weight: 500,
                        pain_weight: 500,
                        danger_weight: 500,
                        enterprise_weight: 500,
                        social_weight: 200,
                        activity_awareness_weight: 200,
                        side_benefit_weight: 100,
                        bounty_posting_weight: 0,
                        notice_posting_weight: 0,
                        courage: 500,
                        care_weight: 200,
                    ),
                    merchandise_profile: (
                        sale_kinds: [Apple, Bread],
                        home_facility: "Town",
                    ),
                    trade_disposition: (
                        negotiation_round_ticks: 2,
                        initial_offer_bias: 600,
                        concession_rate: 100,
                        rejection_escalation_rate: 200,
                        demand_memory_retention_ticks: 50,
                        market_presence_ticks: 30,
                    ),
                    perception_profile: (
                        entity_memory_capacity: 6,
                        entity_claim_capacity: 9,
                        memory_retention_ticks: 24,
                        infrastructure_retention_ticks: 240,
                        observation_fidelity: 900,
                        confidence_policy: (
                            direct_observation_base: 980,
                            report_base: 820,
                            rumor_base: 610,
                            inference_base: 430,
                            report_chain_penalty: 45,
                            rumor_chain_penalty: 120,
                            staleness_penalty_per_tick: 4,
                        ),
                        institutional_memory_capacity: 14,
                        consultation_speed_factor: 600,
                        contradiction_tolerance: 250,
                    ),
                    drive_thresholds: (
                        hunger: (low: 150, medium: 300, high: 600, critical: 850),
                        thirst: (low: 160, medium: 320, high: 610, critical: 860),
                        fatigue: (low: 170, medium: 340, high: 620, critical: 870),
                        bladder: (low: 180, medium: 360, high: 630, critical: 880),
                        dirtiness: (low: 190, medium: 380, high: 640, critical: 890),
                        pain: (low: 120, medium: 240, high: 520, critical: 800),
                        danger: (low: 80, medium: 220, high: 480, critical: 760),
                    ),
                    exploration_profile: (
                        curiosity_weight: 275,
                        need_activation_threshold: 350,
                        max_consecutive_explorations: 5,
                        visit_lookback_ticks: 17,
                    ),
                    obligation_satiation_profile: (
                        satiation_threshold: 4,
                        window_ticks: 96,
                        decay_per_execution: 150,
                        satiation_floor: 125,
                    ),
                    theft_disposition: (
                        steal_duration_ticks: 6,
                        theft_motive_weight: 620,
                        witness_risk_penalty: 180,
                    ),
                    patrol_route: (
                        assigned_places: ["Town", "Forest"],
                    ),
                ),
            ],
            items: [
                (commodity: Apple, quantity: 10, location: "Town", container: false),
                (commodity: Sword, quantity: 1, location: "Bob"),
            ],
            facilities: [
                (workstation: Forge, location: "Town"),
            ],
            resource_sources: [
                (commodity: Apple, location: "Forest", regeneration_ticks_per_unit: Some(5), capacity: 20),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        assert_eq!(def.seed, 123);
        assert_eq!(def.places.len(), 2);
        assert_eq!(def.edges.len(), 1);
        assert_eq!(def.edges[0].from, "Town");
        assert_eq!(def.edges[0].to, "Forest");
        assert_eq!(def.edges[0].travel_ticks, 3);
        assert!(def.edges[0].bidirectional);
        assert_eq!(def.agents.len(), 1);

        let bob = &def.agents[0];
        assert_eq!(bob.name, "Bob");
        assert_eq!(bob.control, ControlSource::Ai);
        assert!(bob.needs.is_some());
        assert!(bob.combat_profile.is_some());
        assert!(bob.utility_profile.is_some());
        assert_eq!(
            bob.utility_profile
                .as_ref()
                .unwrap()
                .activity_awareness_weight
                .value(),
            200
        );
        assert_eq!(
            bob.utility_profile
                .as_ref()
                .unwrap()
                .bounty_posting_weight
                .value(),
            0
        );
        assert_eq!(
            bob.utility_profile
                .as_ref()
                .unwrap()
                .side_benefit_weight
                .value(),
            100
        );
        assert!(bob.merchandise_profile.is_some());
        let merch = bob.merchandise_profile.as_ref().unwrap();
        assert_eq!(
            merch.sale_kinds,
            vec![CommodityKind::Apple, CommodityKind::Bread]
        );
        assert_eq!(merch.home_facility, Some("Town".to_string()));
        assert!(bob.trade_disposition.is_some());
        assert!(bob.perception_profile.is_some());
        let perception = bob.perception_profile.unwrap();
        assert_eq!(perception.entity_memory_capacity, 6);
        assert_eq!(perception.entity_claim_capacity, 9);
        assert!(bob.drive_thresholds.is_some());
        assert_eq!(bob.drive_thresholds.unwrap().hunger.low().value(), 150);
        assert_eq!(
            bob.exploration_profile,
            Some(ExplorationProfileDef {
                curiosity_weight: Permille::new(275).unwrap(),
                need_activation_threshold: Permille::new(350).unwrap(),
                max_consecutive_explorations: 5,
                visit_lookback_ticks: 17,
            })
        );
        assert_eq!(
            bob.obligation_satiation_profile,
            Some(ObligationSatiationProfile {
                satiation_threshold: 4,
                window_ticks: 96,
                decay_per_execution: Permille::new(150).unwrap(),
                satiation_floor: Permille::new(125).unwrap(),
            })
        );
        assert!(bob.theft_disposition.is_some());
        assert_eq!(
            bob.theft_disposition
                .as_ref()
                .unwrap()
                .theft_motive_weight
                .value(),
            620
        );
        assert!(bob.patrol_route.is_some());
        assert_eq!(
            bob.patrol_route.as_ref().unwrap().assigned_places,
            vec!["Town".to_string(), "Forest".to_string()]
        );

        assert_eq!(def.items.len(), 2);
        assert!(!def.items[0].container);
        assert_eq!(def.facilities.len(), 1);
        assert_eq!(def.facilities[0].workstation, WorkstationTag::Forge);
        assert_eq!(def.resource_sources.len(), 1);
        assert_eq!(def.resource_sources[0].capacity, Quantity(20));
    }

    #[test]
    fn test_place_def_deserializes_visibility_profile() {
        let ron_str = r#"(
            seed: 7,
            places: [
                (
                    name: "Forest",
                    tags: [Forest],
                    visibility_profile: (
                        base_concealment: 400,
                    ),
                ),
            ],
            agents: [
                (name: "Scout", location: "Forest", control: Ai),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        assert_eq!(
            def.places[0].visibility_profile,
            Some(PlaceVisibilityProfile {
                base_concealment: Permille::new(400).unwrap(),
            })
        );
    }

    #[test]
    fn test_exploration_profile_def_rejects_runtime_counter_field() {
        let ron_str = r#"(
            seed: 7,
            places: [
                (name: "Village", tags: [Village]),
            ],
            agents: [
                (
                    name: "Scout",
                    location: "Village",
                    control: Ai,
                    exploration_profile: (
                        curiosity_weight: 275,
                        need_activation_threshold: 350,
                        max_consecutive_explorations: 5,
                        visit_lookback_ticks: 17,
                        consecutive_exploration_count: 1,
                    ),
                ),
            ],
        )"#;

        let options = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::UNWRAP_NEWTYPES)
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME);
        let error = options
            .from_str::<ScenarioDef>(ron_str)
            .expect_err("runtime-only exploration counter should not deserialize");

        assert!(
            error
                .to_string()
                .contains("Unexpected field named `consecutive_exploration_count`"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn test_place_def_visibility_profile_defaults_to_none() {
        let ron_str = r#"(
            seed: 8,
            places: [
                (name: "Square", tags: [Village]),
            ],
            agents: [
                (name: "Watcher", location: "Square", control: Ai),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        assert_eq!(def.places[0].visibility_profile, None);
    }

    #[test]
    fn test_agent_def_default_optional_fields() {
        let ron_str = r#"(
            seed: 1,
            places: [(name: "Nowhere", tags: [])],
            agents: [
                (name: "Minimal", location: "Nowhere", control: None),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        let agent = &def.agents[0];
        assert_eq!(agent.name, "Minimal");
        assert_eq!(agent.location, "Nowhere");
        assert_eq!(agent.control, ControlSource::None);
        assert!(agent.needs.is_none());
        assert!(agent.combat_profile.is_none());
        assert!(agent.utility_profile.is_none());
        assert!(agent.artifact_posting_profile.is_none());
        assert!(agent.merchandise_profile.is_none());
        assert!(agent.trade_disposition.is_none());
        assert!(agent.perception_profile.is_none());
        assert!(agent.tell_profile.is_none());
        assert!(agent.cognitive_profile.is_none());
        assert!(agent.execution_budget.is_none());
        assert!(agent.epistemic_disposition.is_none());
        assert!(agent.intention_disposition.is_none());
        assert!(agent.communication_profile.is_none());
        assert!(agent.preference_profile.is_none());
        assert!(agent.expectation_store.is_none());
        assert!(agent.last_seen_memory.is_none());
        assert!(agent.obligation_satiation_profile.is_none());
        assert!(agent.drive_thresholds.is_none());
        assert!(agent.metabolism_profile.is_none());
        assert!(agent.carry_capacity.is_none());
        assert!(agent.theft_disposition.is_none());
        assert!(agent.justice_disposition.is_none());
        assert!(agent.violation_disposition.is_none());
        assert!(agent.patrol_profile.is_none());
        assert!(agent.patrol_route.is_none());
        assert!(agent.pursuit_profile.is_none());
        assert!(agent.contention_disposition.is_none());
        assert!(agent.commodity_valuation.is_none());
        assert!(agent.substitute_preferences.is_none());
    }

    #[test]
    fn test_scenario_def_cognitive_profile_missing_new_field_uses_default() {
        let ron_str = r#"(
            seed: 1,
            places: [(name: "Nowhere", tags: [])],
            agents: [
                (
                    name: "Planner",
                    location: "Nowhere",
                    control: Ai,
                    cognitive_profile: (
                        max_candidates_to_plan: 4,
                        max_plan_depth: 10,
                        snapshot_travel_horizon: 6,
                        max_node_expansions: 300,
                        switch_margin: 100,
                        planning_switch_margin: 150,
                        transient_block_ticks: 20,
                        unknown_block_ticks: 5,
                        structural_block_ticks: 200,
                        initial_cooldown_ticks: 4,
                        max_cooldown_ticks: 64,
                        max_snapshot_entities_per_place: 50,
                        speculative_acquisition: true,
                        landmark_extraction_depth: 3,
                    ),
                ),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        let cognitive = def.agents[0]
            .cognitive_profile
            .expect("cognitive profile should deserialize");

        assert_eq!(cognitive.max_candidates_to_plan, 4);
        assert_eq!(
            cognitive.max_candidates_per_expansion,
            CognitiveProfile::default().max_candidates_per_expansion
        );
        assert_eq!(cognitive.max_plan_depth, 10);
        assert!(cognitive.speculative_acquisition);
        assert_eq!(cognitive.landmark_extraction_depth, 3);
    }

    #[test]
    fn test_scenario_def_artifact_posting_profile_deserializes_when_present() {
        let ron_str = r#"(
            seed: 1,
            places: [(name: "Nowhere", tags: [])],
            agents: [
                (
                    name: "Herald",
                    location: "Nowhere",
                    control: Ai,
                    artifact_posting_profile: (
                        threat_warning_ttl: 12,
                        office_vacancy_ttl: 34,
                        bounty_ttl: 56,
                    ),
                ),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        let profile = def.agents[0]
            .artifact_posting_profile
            .clone()
            .expect("artifact posting profile should deserialize");

        assert_eq!(
            profile,
            ArtifactPostingProfile {
                threat_warning_ttl: 12,
                office_vacancy_ttl: 34,
                bounty_ttl: 56,
            }
        );
    }

    #[test]
    fn test_scenario_def_artifact_posting_profile_omitted_field_stays_none() {
        let ron_str = r#"(
            seed: 1,
            places: [(name: "Nowhere", tags: [])],
            agents: [
                (
                    name: "Herald",
                    location: "Nowhere",
                    control: Ai,
                ),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        assert_eq!(def.agents[0].artifact_posting_profile, None);
    }

    #[test]
    fn test_patrol_route_def_deserializes_place_names() {
        let route: PatrolRouteDef = from_ron_str(
            r#"(
                assigned_places: ["Gate", "Market"],
            )"#,
        );

        assert_eq!(
            route.assigned_places,
            vec!["Gate".to_string(), "Market".to_string()]
        );
    }

    #[test]
    fn test_edge_def_bidirectional_default() {
        let ron_str = r#"(
            seed: 1,
            places: [
                (name: "A", tags: []),
                (name: "B", tags: []),
            ],
            edges: [
                (from: "A", to: "B", travel_ticks: 2),
            ],
        )"#;

        let def: ScenarioDef = from_ron_str(ron_str);
        assert!(
            def.edges[0].bidirectional,
            "bidirectional should default to true"
        );
    }
}
