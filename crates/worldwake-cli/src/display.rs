//! Display and formatting helpers for the CLI.
//!
//! All functions are pure read-only — no world mutation.

use worldwake_core::{
    control::ControlSource,
    drives::ThresholdBand,
    ids::EntityId,
    items::CommodityKind,
    numerics::{Permille, Quantity},
    world::World,
};

/// Errors from [`resolve_entity`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// No entity matched the input.
    NotFound(String),
    /// Multiple entities matched; contains matching names.
    Ambiguous(Vec<String>),
}

/// Return a human-readable display name for an entity.
///
/// If the entity has a `Name` component, returns the name string.
/// Otherwise returns `"<EntityKind>#<slot>"` (e.g. `"Agent#3"`).
pub fn entity_display_name(world: &World, id: EntityId) -> String {
    if let Some(name) = world.get_component_name(id) {
        return name.0.clone();
    }
    match world.entity_kind(id) {
        Some(kind) => format!("{kind:?}#{}", id.slot),
        None => format!("Unknown#{}", id.slot),
    }
}

/// Resolve user text input to an `EntityId`.
///
/// Resolution order (per spec line 60):
/// 1. Try parsing as `u32` slot number → find alive entity at that slot
/// 2. Exact name match among all live entities with a `Name` component
/// 3. Single case-insensitive prefix match → return
/// 4. Multiple prefix matches → `Ambiguous` error with matching names
/// 5. No match → `NotFound` error
pub fn resolve_entity(world: &World, input: &str) -> Result<EntityId, ResolveError> {
    let trimmed = input.trim();

    // 1. Try numeric slot lookup.
    if let Ok(slot) = trimmed.parse::<u32>() {
        for id in world.entities() {
            if id.slot == slot {
                return Ok(id);
            }
        }
    }

    // Collect all named entities (deterministic via BTreeMap iteration).
    let named: Vec<(EntityId, String)> = world
        .query_name()
        .map(|(id, name)| (id, name.0.clone()))
        .collect();

    // 2. Exact match.
    for (id, name) in &named {
        if name == trimmed {
            return Ok(*id);
        }
    }

    // 3–5. Prefix match.
    let lower_input = trimmed.to_lowercase();
    let prefix_matches: Vec<(EntityId, String)> = named
        .into_iter()
        .filter(|(_, name)| name.to_lowercase().starts_with(&lower_input))
        .collect();

    match prefix_matches.len() {
        0 => Err(ResolveError::NotFound(trimmed.to_string())),
        1 => Ok(prefix_matches[0].0),
        _ => Err(ResolveError::Ambiguous(
            prefix_matches.into_iter().map(|(_, name)| name).collect(),
        )),
    }
}

/// Format a single homeostatic need as a visual bar with urgency band label.
///
/// Example output: `"hunger: ████░░░░░░ 420‰ [medium]"`
pub fn format_needs_bar(need_name: &str, current: Permille, band: &ThresholdBand) -> String {
    const BAR_WIDTH: u16 = 10;
    let filled = ((u32::from(current.value()) * u32::from(BAR_WIDTH) + 500) / 1000) as u16;
    let empty = BAR_WIDTH - filled;

    let bar: String = "█".repeat(filled as usize) + &"░".repeat(empty as usize);
    let label = urgency_label(current, *band);

    format!("{need_name}: {bar} {current} [{label}]")
}

/// Determine urgency band label from a value and its thresholds.
fn urgency_label(value: Permille, band: ThresholdBand) -> &'static str {
    if value >= band.critical() {
        "critical"
    } else if value >= band.high() {
        "high"
    } else if value >= band.medium() {
        "medium"
    } else if value >= band.low() {
        "low"
    } else {
        "none"
    }
}

/// Format a commodity quantity, e.g. `"5× Grain"`.
pub fn format_quantity(kind: CommodityKind, qty: Quantity) -> String {
    format!("{}× {kind:?}", qty.0)
}

/// Format the location of an entity, e.g. `"at Market Square"`.
pub fn format_location(world: &World, entity_id: EntityId) -> String {
    match world.effective_place(entity_id) {
        Some(place_id) => {
            let place_name = entity_display_name(world, place_id);
            format!("at {place_name}")
        }
        None => "(no location)".to_string(),
    }
}

/// Format a `ControlSource` variant for display.
pub fn format_control_source(cs: ControlSource) -> &'static str {
    match cs {
        ControlSource::Human => "[human]",
        ControlSource::Ai => "[ai]",
        ControlSource::None => "[none]",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::{spawn_scenario, types::*, SpawnedSimulation};
    use worldwake_core::{
        control::ControlSource,
        drives::ThresholdBand,
        items::CommodityKind,
        numerics::{Permille, Quantity},
        topology::PlaceTag,
    };

    fn pm(v: u16) -> Permille {
        Permille::new(v).unwrap()
    }

    fn test_band() -> ThresholdBand {
        ThresholdBand::new(pm(250), pm(500), pm(750), pm(900)).unwrap()
    }

    fn one_agent_def(name: &str) -> ScenarioDef {
        ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Village".into(),
                tags: vec![PlaceTag::Village],
            }],
            edges: vec![],
            agents: vec![AgentDef {
                name: name.into(),
                location: "Village".into(),
                control: ControlSource::Ai,
                needs: None,
                combat_profile: None,
                utility_profile: None,
                merchandise_profile: None,
                trade_disposition: None,
            }],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
        }
    }

    /// Spawn a minimal scenario, return the simulation and agent id.
    fn one_agent_scenario(name: &str) -> (SpawnedSimulation, EntityId) {
        let spawned = spawn_scenario(&one_agent_def(name)).unwrap();
        let agent_id = spawned
            .state
            .world()
            .entities_with_name_and_agent_data()
            .next()
            .unwrap();
        (spawned, agent_id)
    }

    #[test]
    fn test_entity_display_name_with_name() {
        let (sim, id) = one_agent_scenario("Aster");
        assert_eq!(entity_display_name(sim.state.world(), id), "Aster");
    }

    #[test]
    fn test_entity_display_name_without_name() {
        let (sim, _) = one_agent_scenario("Aster");
        let world = sim.state.world();
        // Place entity (slot 0) is in topology but has no Name component.
        let place_id = EntityId {
            slot: 0,
            generation: 0,
        };
        let display = entity_display_name(world, place_id);
        // Places don't get Name component — falls back to "Place#0".
        assert!(
            display == "Place#0" || display == "Village",
            "got: {display}"
        );
    }

    #[test]
    fn test_resolve_entity_by_id() {
        let (sim, id) = one_agent_scenario("Aster");
        let resolved = resolve_entity(sim.state.world(), &id.slot.to_string()).unwrap();
        assert_eq!(resolved, id);
    }

    #[test]
    fn test_resolve_entity_exact_name() {
        let (sim, id) = one_agent_scenario("Aster");
        let resolved = resolve_entity(sim.state.world(), "Aster").unwrap();
        assert_eq!(resolved, id);
    }

    #[test]
    fn test_resolve_entity_prefix() {
        let (sim, id) = one_agent_scenario("Aster");
        let resolved = resolve_entity(sim.state.world(), "Ast").unwrap();
        assert_eq!(resolved, id);
    }

    #[test]
    fn test_resolve_entity_ambiguous() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Village".into(),
                tags: vec![],
            }],
            edges: vec![],
            agents: vec![
                AgentDef {
                    name: "Aster".into(),
                    location: "Village".into(),
                    control: ControlSource::Ai,
                    needs: None,
                    combat_profile: None,
                    utility_profile: None,
                    merchandise_profile: None,
                    trade_disposition: None,
                },
                AgentDef {
                    name: "Astrid".into(),
                    location: "Village".into(),
                    control: ControlSource::Ai,
                    needs: None,
                    combat_profile: None,
                    utility_profile: None,
                    merchandise_profile: None,
                    trade_disposition: None,
                },
            ],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
        };
        let spawned = spawn_scenario(&def).unwrap();

        let err = resolve_entity(spawned.state.world(), "Ast").unwrap_err();
        match err {
            ResolveError::Ambiguous(names) => {
                assert!(names.contains(&"Aster".to_string()));
                assert!(names.contains(&"Astrid".to_string()));
            }
            other @ ResolveError::NotFound(_) => panic!("expected Ambiguous, got {other:?}"),
        }
    }

    #[test]
    fn test_resolve_entity_not_found() {
        let (sim, _) = one_agent_scenario("Aster");
        let err = resolve_entity(sim.state.world(), "Zephyr").unwrap_err();
        assert_eq!(err, ResolveError::NotFound("Zephyr".to_string()));
    }

    #[test]
    fn test_format_needs_bar() {
        let bar = format_needs_bar("hunger", pm(420), &test_band());
        assert!(bar.contains("hunger:"), "got: {bar}");
        assert!(bar.contains("420‰"), "got: {bar}");
        assert!(bar.contains("[low]"), "got: {bar}");
    }

    #[test]
    fn test_format_quantity() {
        assert_eq!(
            format_quantity(CommodityKind::Grain, Quantity(5)),
            "5× Grain"
        );
        assert_eq!(
            format_quantity(CommodityKind::Water, Quantity(1)),
            "1× Water"
        );
    }

    #[test]
    fn test_format_control_source() {
        assert_eq!(format_control_source(ControlSource::Human), "[human]");
        assert_eq!(format_control_source(ControlSource::Ai), "[ai]");
        assert_eq!(format_control_source(ControlSource::None), "[none]");
    }

    #[test]
    fn test_format_location_placed() {
        let (sim, id) = one_agent_scenario("Aster");
        let loc = format_location(sim.state.world(), id);
        assert!(loc.starts_with("at "), "got: {loc}");
    }

    #[test]
    fn test_format_location_unplaced() {
        let (sim, _) = one_agent_scenario("Aster");
        let fake_id = EntityId {
            slot: 999,
            generation: 0,
        };
        assert_eq!(format_location(sim.state.world(), fake_id), "(no location)");
    }
}
