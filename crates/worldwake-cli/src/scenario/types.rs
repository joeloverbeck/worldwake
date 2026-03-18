//! Scenario definition types for RON-based world initialization.
//!
//! These are pure data types — no logic, just the schema for scenario files.
//! All location references use string names, resolved to `EntityId` during spawning.

use std::num::NonZeroU32;

use serde::Deserialize;
use worldwake_core::{
    combat::CombatProfile, control::ControlSource, items::CommodityKind, needs::HomeostaticNeeds,
    numerics::Quantity, production::WorkstationTag, topology::PlaceTag,
    trade::TradeDispositionProfile, utility_profile::UtilityProfile,
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
}

/// A place in the world graph.
#[derive(Clone, Debug, Deserialize)]
pub struct PlaceDef {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<PlaceTag>,
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
    pub merchandise_profile: Option<MerchandiseProfileDef>,
    #[serde(default)]
    pub trade_disposition: Option<TradeDispositionProfile>,
}

/// Scenario-specific merchandise profile using string names instead of `EntityId`.
///
/// `MerchandiseProfile` in core contains `home_market: Option<EntityId>`, which
/// cannot appear in a RON file before entities are spawned. This def uses a
/// place name string, resolved to `EntityId` during spawning.
#[derive(Clone, Debug, Deserialize)]
pub struct MerchandiseProfileDef {
    pub sale_kinds: Vec<CommodityKind>,
    #[serde(default)]
    pub home_market: Option<String>,
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
    pub workstation: WorkstationTag,
    pub location: String,
}

/// A resource source at a place.
#[derive(Clone, Debug, Deserialize)]
pub struct ResourceSourceDef {
    pub commodity: CommodityKind,
    pub location: String,
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
                        courage: 500,
                        care_weight: 200,
                    ),
                    merchandise_profile: (
                        sale_kinds: [Apple, Bread],
                        home_market: "Town",
                    ),
                    trade_disposition: (
                        negotiation_round_ticks: 2,
                        initial_offer_bias: 600,
                        concession_rate: 100,
                        demand_memory_retention_ticks: 50,
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
        assert!(bob.merchandise_profile.is_some());
        let merch = bob.merchandise_profile.as_ref().unwrap();
        assert_eq!(
            merch.sale_kinds,
            vec![CommodityKind::Apple, CommodityKind::Bread]
        );
        assert_eq!(merch.home_market, Some("Town".to_string()));
        assert!(bob.trade_disposition.is_some());

        assert_eq!(def.items.len(), 2);
        assert!(!def.items[0].container);
        assert_eq!(def.facilities.len(), 1);
        assert_eq!(def.facilities[0].workstation, WorkstationTag::Forge);
        assert_eq!(def.resource_sources.len(), 1);
        assert_eq!(def.resource_sources[0].capacity, Quantity(20));
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
        assert!(agent.merchandise_profile.is_none());
        assert!(agent.trade_disposition.is_none());
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
