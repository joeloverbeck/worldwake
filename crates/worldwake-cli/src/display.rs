//! Display and formatting helpers for the CLI.
//!
//! All functions are pure read-only — no world mutation.

use worldwake_core::{
    control::ControlSource,
    delta::{
        ComponentDelta, EntityDelta, QuantityDelta, RelationDelta, ReservationDelta, StateDelta,
    },
    drives::ThresholdBand,
    ids::EntityId,
    items::CommodityKind,
    numerics::{Permille, Quantity},
    world::World,
    Tick,
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
/// Resolution order:
/// 1. `Name` component → name string (e.g. `"Merchant Vara"`)
/// 2. Topology place → place name (e.g. `"Market Square"`)
/// 3. `ItemLot` component → quantity + commodity (e.g. `"5× Grain"`)
/// 4. `WorkstationMarker` → workstation tag (e.g. `"Mill"`)
/// 5. Fallback → `"<EntityKind>#<slot>"` (e.g. `"Agent#3"`)
pub fn entity_display_name(world: &World, id: EntityId) -> String {
    if let Some(name) = world.get_component_name(id) {
        return name.0.clone();
    }
    // Places are stored in topology with names but lack Name components.
    if let Some(place) = world.topology().place(id) {
        return place.name.clone();
    }
    // Item lots: show as "5× Grain".
    if let Some(lot) = world.get_component_item_lot(id) {
        return format_quantity(lot.commodity, lot.quantity);
    }
    // Workstations/facilities: show the workstation tag.
    if let Some(wm) = world.get_component_workstation_marker(id) {
        return format!("{:?}", wm.0);
    }
    // Resource sources: show as "Apple source".
    if let Some(rs) = world.get_component_resource_source(id) {
        return format!("{:?} source", rs.commodity);
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

/// Format the location of an entity, e.g. `"at Market Square"` or
/// `"in transit to Eldergrove Forest (2 ticks remaining)"`.
pub fn format_location(world: &World, entity_id: EntityId, current_tick: Tick) -> String {
    if let Some(place_id) = world.effective_place(entity_id) {
        let place_name = entity_display_name(world, place_id);
        return format!("at {place_name}");
    }
    if world.is_in_transit(entity_id) {
        if let Some(transit) = world.get_component_in_transit_on_edge(entity_id) {
            let dest = entity_display_name(world, transit.destination);
            let remaining = transit.arrival_tick.0.saturating_sub(current_tick.0);
            return format!("in transit to {dest} ({remaining} ticks remaining)");
        }
    }
    "(no location)".to_string()
}

/// Format a `ControlSource` variant for display.
pub fn format_control_source(cs: ControlSource) -> &'static str {
    match cs {
        ControlSource::Human => "[human]",
        ControlSource::Ai => "[ai]",
        ControlSource::None => "[none]",
    }
}

/// Format a state delta for human-readable event display.
pub fn format_state_delta(world: &World, delta: &StateDelta) -> String {
    match delta {
        StateDelta::Entity(EntityDelta::Created { entity, kind }) => {
            let name = entity_display_name(world, *entity);
            format!("{name} ({kind:?}) created")
        }
        StateDelta::Entity(EntityDelta::Archived { entity, kind }) => {
            let name = entity_display_name(world, *entity);
            format!("{name} ({kind:?}) archived")
        }
        StateDelta::Component(ComponentDelta::Set {
            entity,
            component_kind,
            ..
        }) => {
            let name = entity_display_name(world, *entity);
            format!("{component_kind:?}: set on {name}")
        }
        StateDelta::Component(ComponentDelta::Removed {
            entity,
            component_kind,
            ..
        }) => {
            let name = entity_display_name(world, *entity);
            format!("{component_kind:?}: removed from {name}")
        }
        StateDelta::Relation(RelationDelta::Added {
            relation_kind,
            relation,
        }) => format_relation_delta("added", *relation_kind, relation, world),
        StateDelta::Relation(RelationDelta::Removed {
            relation_kind,
            relation,
        }) => format_relation_delta("removed", *relation_kind, relation, world),
        StateDelta::Quantity(QuantityDelta::Changed {
            entity,
            commodity,
            before,
            after,
        }) => {
            let name = entity_display_name(world, *entity);
            format!("{commodity:?} on {name}: {} → {}", before.0, after.0)
        }
        StateDelta::Reservation(ReservationDelta::Created { .. }) => {
            "Reservation created".to_string()
        }
        StateDelta::Reservation(ReservationDelta::Released { .. }) => {
            "Reservation released".to_string()
        }
    }
}

/// Helper: format a relation delta with resolved entity names.
fn format_relation_delta(
    verb: &str,
    kind: worldwake_core::delta::RelationKind,
    value: &worldwake_core::delta::RelationValue,
    world: &World,
) -> String {
    use worldwake_core::delta::RelationValue;
    match value {
        RelationValue::LocatedIn { entity, place } => {
            let e = entity_display_name(world, *entity);
            let p = entity_display_name(world, *place);
            format!("LocatedIn: {verb} ({e} → {p})")
        }
        RelationValue::InTransit { entity } => {
            let e = entity_display_name(world, *entity);
            format!("InTransit: {verb} ({e})")
        }
        RelationValue::PossessedBy { entity, holder } => {
            let e = entity_display_name(world, *entity);
            let h = entity_display_name(world, *holder);
            format!("PossessedBy: {verb} ({e} → {h})")
        }
        RelationValue::ContainedBy { entity, container } => {
            let e = entity_display_name(world, *entity);
            let c = entity_display_name(world, *container);
            format!("ContainedBy: {verb} ({e} → {c})")
        }
        RelationValue::OwnedBy { entity, owner } => {
            let e = entity_display_name(world, *entity);
            let o = entity_display_name(world, *owner);
            format!("OwnedBy: {verb} ({e} → {o})")
        }
        RelationValue::HostileTo { subject, target } => {
            let s = entity_display_name(world, *subject);
            let t = entity_display_name(world, *target);
            format!("HostileTo: {verb} ({s} → {t})")
        }
        _ => format!("{kind:?}: {verb}"),
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
                perception_profile: None,
                tell_profile: None,
                reasoning_profile: None,
                epistemic_disposition: None,
                intention_disposition: None,
                communication_profile: None,
                preference_profile: None,
                drive_thresholds: None,
                metabolism_profile: None,
                carry_capacity: None,
                theft_disposition: None,
                justice_disposition: None,
                violation_disposition: None,
                patrol_profile: None,
                patrol_route: None,
                pursuit_profile: None,
                contention_disposition: None,
                commodity_valuation: None,
                substitute_preferences: None,
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
        // Places resolve via topology to their place name.
        assert_eq!(display, "Village");
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
                    perception_profile: None,
                    tell_profile: None,
                    reasoning_profile: None,
                    epistemic_disposition: None,
                    intention_disposition: None,
                    communication_profile: None,
                    preference_profile: None,
                    drive_thresholds: None,
                    metabolism_profile: None,
                    carry_capacity: None,
                    theft_disposition: None,
                    justice_disposition: None,
                    violation_disposition: None,
                    patrol_profile: None,
                    patrol_route: None,
                    pursuit_profile: None,
                    contention_disposition: None,
                    commodity_valuation: None,
                    substitute_preferences: None,
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
                    perception_profile: None,
                    tell_profile: None,
                    reasoning_profile: None,
                    epistemic_disposition: None,
                    intention_disposition: None,
                    communication_profile: None,
                    preference_profile: None,
                    drive_thresholds: None,
                    metabolism_profile: None,
                    carry_capacity: None,
                    theft_disposition: None,
                    justice_disposition: None,
                    violation_disposition: None,
                    patrol_profile: None,
                    patrol_route: None,
                    pursuit_profile: None,
                    contention_disposition: None,
                    commodity_valuation: None,
                    substitute_preferences: None,
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
        let loc = format_location(sim.state.world(), id, Tick(0));
        assert!(loc.starts_with("at "), "got: {loc}");
    }

    #[test]
    fn test_format_location_unplaced() {
        let (sim, _) = one_agent_scenario("Aster");
        let fake_id = EntityId {
            slot: 999,
            generation: 0,
        };
        assert_eq!(
            format_location(sim.state.world(), fake_id, Tick(0)),
            "(no location)"
        );
    }
}
