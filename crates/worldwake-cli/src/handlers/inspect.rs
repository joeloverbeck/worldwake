//! Inspection command handlers: look, inspect, inventory, needs, relations.
//!
//! All handlers are read-only — zero world mutation.

use worldwake_core::{
    drives::ThresholdBand, ids::EntityId, load::load_of_entity, numerics::Permille, world::World,
};
use worldwake_sim::SimulationState;

use crate::commands::{CommandError, CommandOutcome, CommandResult};
use crate::display::{
    entity_display_name, format_location, format_needs_bar, format_quantity, resolve_entity,
    ResolveError,
};

/// Display-only threshold band for needs bars (same as tick.rs).
///
/// Derived read-model — not authoritative state.
/// When per-agent `DriveThresholds` exist, prefer those.
fn default_display_band() -> ThresholdBand {
    ThresholdBand::new(
        Permille::new(250).unwrap(),
        Permille::new(500).unwrap(),
        Permille::new(750).unwrap(),
        Permille::new(900).unwrap(),
    )
    .unwrap()
}

/// Return a threshold band for a specific need, preferring the agent's
/// `DriveThresholds` component if present, falling back to a default.
fn need_band(world: &World, entity: EntityId, need: &str) -> ThresholdBand {
    if let Some(dt) = world.get_component_drive_thresholds(entity) {
        return match need {
            "hunger" => dt.hunger,
            "thirst" => dt.thirst,
            "fatigue" => dt.fatigue,
            "bladder" => dt.bladder,
            "dirtiness" => dt.dirtiness,
            _ => default_display_band(),
        };
    }
    default_display_band()
}

/// Format a `ResolveError` into a user-friendly `CommandError`.
fn resolve_error_to_command_error(err: ResolveError) -> CommandError {
    match err {
        ResolveError::NotFound(input) => {
            CommandError::new(format!("no entity matching \"{input}\""))
        }
        ResolveError::Ambiguous(names) => {
            CommandError::new(format!("ambiguous — matches: {}", names.join(", ")))
        }
    }
}

/// Describe the controlled agent's current location, co-located entities,
/// and travel connections.
pub fn handle_look(sim: &SimulationState) -> CommandResult {
    let entity = sim
        .controller_state()
        .controlled_entity()
        .ok_or_else(|| CommandError::new("no controlled agent (observer mode)"))?;

    let world = sim.world();

    let place_id = world
        .effective_place(entity)
        .ok_or_else(|| CommandError::new("controlled agent has no location"))?;

    // Place name and tags.
    if let Some(place) = world.topology().place(place_id) {
        let tags: Vec<String> = place.tags.iter().map(|t| format!("{t:?}")).collect();
        if tags.is_empty() {
            println!("{}", place.name);
        } else {
            println!("{} [{}]", place.name, tags.join(", "));
        }
    } else {
        println!("{}", entity_display_name(world, place_id));
    }

    // Entities at the same place (excluding self).
    let colocated = world.entities_effectively_at(place_id);
    let others: Vec<EntityId> = colocated.into_iter().filter(|id| *id != entity).collect();
    if others.is_empty() {
        println!("  (nobody else here)");
    } else {
        println!("  Entities here:");
        for other in &others {
            let name = entity_display_name(world, *other);
            let kind = world
                .entity_kind(*other)
                .map_or("?".to_string(), |k| format!("{k:?}"));
            println!("    {name} ({kind})");
        }
    }

    // Travel connections.
    let topo = world.topology();
    let outgoing = topo.outgoing_edges(place_id);
    if outgoing.is_empty() {
        println!("  No travel connections.");
    } else {
        println!("  Connections:");
        for edge_id in outgoing {
            if let Some(edge) = topo.edge(*edge_id) {
                let dest_id = edge.to();
                let dest_name = topo
                    .place(dest_id)
                    .map_or_else(|| entity_display_name(world, dest_id), |p| p.name.clone());
                println!("    -> {} ({} ticks)", dest_name, edge.travel_time_ticks());
            }
        }
    }

    Ok(CommandOutcome::Continue)
}

/// Show all components on a resolved entity.
pub fn handle_inspect(sim: &SimulationState, entity_input: &str) -> CommandResult {
    let world = sim.world();
    let entity = resolve_entity(world, entity_input).map_err(resolve_error_to_command_error)?;

    let name = entity_display_name(world, entity);
    let kind = world
        .entity_kind(entity)
        .map_or("Unknown".to_string(), |k| format!("{k:?}"));
    println!("{name} ({kind}) #{}", entity.slot);

    // Check each component type and print if present.
    if let Some(agent_data) = world.get_component_agent_data(entity) {
        println!("  AgentData: control={:?}", agent_data.control_source);
    }
    if let Some(needs) = world.get_component_homeostatic_needs(entity) {
        println!(
            "  HomeostaticNeeds: hunger={}, thirst={}, fatigue={}, bladder={}, dirtiness={}",
            needs.hunger, needs.thirst, needs.fatigue, needs.bladder, needs.dirtiness
        );
    }
    if let Some(wounds) = world.get_component_wound_list(entity) {
        println!("  WoundList: {} wound(s)", wounds.wounds.len());
        for w in &wounds.wounds {
            println!(
                "    {:?} on {:?}, severity={}, tick={}",
                w.cause, w.body_part, w.severity, w.inflicted_at.0
            );
        }
    }
    if let Some(cp) = world.get_component_combat_profile(entity) {
        println!("  CombatProfile: {cp:?}");
    }
    if let Some(dead) = world.get_component_dead_at(entity) {
        println!("  DeadAt: tick {}", dead.0 .0);
    }
    if let Some(up) = world.get_component_utility_profile(entity) {
        println!("  UtilityProfile: {up:?}");
    }
    if let Some(dt) = world.get_component_drive_thresholds(entity) {
        println!("  DriveThresholds: {dt:?}");
    }
    if let Some(mp) = world.get_component_metabolism_profile(entity) {
        println!("  MetabolismProfile: {mp:?}");
    }
    if let Some(dep) = world.get_component_deprivation_exposure(entity) {
        println!("  DeprivationExposure: {dep:?}");
    }
    if let Some(bim) = world.get_component_blocked_intent_memory(entity) {
        println!("  BlockedIntentMemory: {} entries", bim.intents.len());
    }
    if let Some(lot) = world.get_component_item_lot(entity) {
        println!("  ItemLot: {:?} x{}", lot.commodity, lot.quantity.0);
    }
    if let Some(ui) = world.get_component_unique_item(entity) {
        println!("  UniqueItem: {:?} name={:?}", ui.kind, ui.name);
    }
    if let Some(c) = world.get_component_container(entity) {
        println!("  Container: capacity={}", c.capacity.0);
    }
    if let Some(cc) = world.get_component_carry_capacity(entity) {
        println!("  CarryCapacity: {}", cc.0 .0);
    }
    if let Some(kr) = world.get_component_known_recipes(entity) {
        println!("  KnownRecipes: {:?}", kr.recipes);
    }
    if let Some(mp) = world.get_component_merchandise_profile(entity) {
        println!("  MerchandiseProfile: {mp:?}");
    }
    if let Some(tdp) = world.get_component_trade_disposition_profile(entity) {
        println!("  TradeDispositionProfile: {tdp:?}");
    }
    if let Some(dm) = world.get_component_demand_memory(entity) {
        println!("  DemandMemory: {dm:?}");
    }
    if let Some(sp) = world.get_component_substitute_preferences(entity) {
        println!("  SubstitutePreferences: {sp:?}");
    }
    if let Some(wm) = world.get_component_workstation_marker(entity) {
        println!("  WorkstationMarker: {:?}", wm.0);
    }
    if let Some(pj) = world.get_component_production_job(entity) {
        println!("  ProductionJob: {pj:?}");
    }
    if let Some(rs) = world.get_component_resource_source(entity) {
        println!(
            "  ResourceSource: {:?} {}/{}",
            rs.commodity, rs.available_quantity.0, rs.max_quantity.0
        );
    }
    if let Some(transit) = world.get_component_in_transit_on_edge(entity) {
        println!("  InTransitOnEdge: {transit:?}");
    }
    if let Some(stance) = world.get_component_combat_stance(entity) {
        println!("  CombatStance: {stance:?}");
    }

    // Location.
    println!("  Location: {}", format_location(world, entity));

    Ok(CommandOutcome::Continue)
}

/// Show items carried/possessed by an entity.
pub fn handle_inventory(sim: &SimulationState, entity_input: Option<&str>) -> CommandResult {
    let world = sim.world();

    let entity = match entity_input {
        Some(input) => resolve_entity(world, input).map_err(resolve_error_to_command_error)?,
        None => sim.controller_state().controlled_entity().ok_or_else(|| {
            CommandError::new("no controlled agent (observer mode) — specify an entity")
        })?,
    };

    let name = entity_display_name(world, entity);
    println!("{name} inventory:");

    let possessions = world.possessions_of(entity);
    if possessions.is_empty() {
        println!("  (no items)");
    } else {
        for item_id in &possessions {
            if let Some(lot) = world.get_component_item_lot(*item_id) {
                println!("  {}", format_quantity(lot.commodity, lot.quantity));
            } else if let Some(unique) = world.get_component_unique_item(*item_id) {
                let fallback = format!("{:?}", unique.kind);
                let item_name = unique.name.as_deref().unwrap_or(&fallback);
                println!("  {item_name} ({:?})", unique.kind);
            } else if let Some(container) = world.get_component_container(*item_id) {
                let container_name = entity_display_name(world, *item_id);
                let contents_count = world.direct_contents_of(*item_id).len();
                println!(
                    "  [container] {container_name} (cap={}, {contents_count} items)",
                    container.capacity.0
                );
            } else {
                let item_name = entity_display_name(world, *item_id);
                println!("  {item_name}");
            }
        }
    }

    // Total load vs capacity.
    if let Ok(total_load) = load_of_entity(world, entity) {
        if let Some(capacity) = world.get_component_carry_capacity(entity) {
            println!("  Load: {}/{}", total_load.0, capacity.0 .0);
        }
    }

    Ok(CommandOutcome::Continue)
}

/// Show all 5 homeostatic needs for an agent.
pub fn handle_needs(sim: &SimulationState, entity_input: Option<&str>) -> CommandResult {
    let world = sim.world();

    let entity = match entity_input {
        Some(input) => resolve_entity(world, input).map_err(resolve_error_to_command_error)?,
        None => sim.controller_state().controlled_entity().ok_or_else(|| {
            CommandError::new("no controlled agent (observer mode) — specify an entity")
        })?,
    };

    // Must be an agent.
    if world.get_component_agent_data(entity).is_none() {
        let name = entity_display_name(world, entity);
        return Err(CommandError::new(format!("{name} is not an agent")));
    }

    let name = entity_display_name(world, entity);

    let needs = world.get_component_homeostatic_needs(entity);
    match needs {
        Some(n) => {
            println!("{name} needs:");
            println!(
                "  {}",
                format_needs_bar("hunger", n.hunger, &need_band(world, entity, "hunger"))
            );
            println!(
                "  {}",
                format_needs_bar("thirst", n.thirst, &need_band(world, entity, "thirst"))
            );
            println!(
                "  {}",
                format_needs_bar("fatigue", n.fatigue, &need_band(world, entity, "fatigue"))
            );
            println!(
                "  {}",
                format_needs_bar("bladder", n.bladder, &need_band(world, entity, "bladder"))
            );
            println!(
                "  {}",
                format_needs_bar(
                    "dirtiness",
                    n.dirtiness,
                    &need_band(world, entity, "dirtiness")
                )
            );
        }
        None => {
            println!("{name}: (no needs data)");
        }
    }

    Ok(CommandOutcome::Continue)
}

/// Show all relations involving an entity.
pub fn handle_relations(sim: &SimulationState, entity_input: &str) -> CommandResult {
    let world = sim.world();
    let entity = resolve_entity(world, entity_input).map_err(resolve_error_to_command_error)?;

    let name = entity_display_name(world, entity);
    println!("{name} relations:");

    let mut any = false;

    // Placement.
    if let Some(place_id) = world.effective_place(entity) {
        let place_name = entity_display_name(world, place_id);
        println!("  placed at {place_name}");
        any = true;
    }
    if world.is_in_transit(entity) {
        println!("  in transit");
        any = true;
    }

    // Containment.
    if let Some(container_id) = world.direct_container(entity) {
        let container_name = entity_display_name(world, container_id);
        println!("  contained in {container_name}");
        any = true;
    }
    let contents = world.direct_contents_of(entity);
    if !contents.is_empty() {
        for c in &contents {
            println!("  contains {}", entity_display_name(world, *c));
        }
        any = true;
    }

    // Possession.
    if let Some(holder_id) = world.possessor_of(entity) {
        let holder_name = entity_display_name(world, holder_id);
        println!("  possessed by {holder_name}");
        any = true;
    }
    let possessions = world.possessions_of(entity);
    for p in &possessions {
        let item_name = entity_display_name(world, *p);
        if let Some(lot) = world.get_component_item_lot(*p) {
            println!(
                "  possesses {} ({:?})",
                format_quantity(lot.commodity, lot.quantity),
                item_name
            );
        } else {
            println!("  possesses {item_name}");
        }
        any = true;
    }

    // Ownership.
    if let Some(owner_id) = world.owner_of(entity) {
        let owner_name = entity_display_name(world, owner_id);
        println!("  owned by {owner_name}");
        any = true;
    }

    // Social: factions.
    let factions = world.factions_of(entity);
    for f in &factions {
        println!("  member of {}", entity_display_name(world, *f));
        any = true;
    }

    // Social: loyalty.
    let loyal_targets = world.loyal_targets_of(entity);
    for (target, strength) in &loyal_targets {
        println!(
            "  loyal to {} ({})",
            entity_display_name(world, *target),
            strength
        );
        any = true;
    }

    // Social: hostility.
    let hostile_targets = world.hostile_targets_of(entity);
    for h in &hostile_targets {
        println!("  hostile to {}", entity_display_name(world, *h));
        any = true;
    }

    // Social: offices.
    let offices = world.offices_held_by(entity);
    for o in &offices {
        println!("  holds office {}", entity_display_name(world, *o));
        any = true;
    }

    if !any {
        println!("  (no relations)");
    }

    Ok(CommandOutcome::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::{spawn_scenario, types::*, SpawnedSimulation};
    use worldwake_core::{
        control::ControlSource,
        ids::EntityId,
        items::CommodityKind,
        needs::HomeostaticNeeds,
        numerics::{Permille, Quantity},
        topology::PlaceTag,
    };

    fn pm(v: u16) -> Permille {
        Permille::new(v).unwrap()
    }

    /// Scenario with a human agent at a place with connections, items, and another agent.
    fn rich_scenario() -> (SpawnedSimulation, EntityId) {
        let def = ScenarioDef {
            seed: 42,
            places: vec![
                PlaceDef {
                    name: "Market Square".into(),
                    tags: vec![PlaceTag::Village, PlaceTag::Store],
                },
                PlaceDef {
                    name: "Dark Forest".into(),
                    tags: vec![PlaceTag::Forest],
                },
            ],
            edges: vec![EdgeDef {
                from: "Market Square".into(),
                to: "Dark Forest".into(),
                travel_ticks: 5,
                bidirectional: true,
            }],
            agents: vec![
                AgentDef {
                    name: "Aster".into(),
                    location: "Market Square".into(),
                    control: ControlSource::Human,
                    needs: Some(HomeostaticNeeds::new(
                        pm(300),
                        pm(400),
                        pm(100),
                        pm(50),
                        pm(20),
                    )),
                    combat_profile: None,
                    utility_profile: None,
                    merchandise_profile: None,
                    trade_disposition: None,
                },
                AgentDef {
                    name: "Kael".into(),
                    location: "Market Square".into(),
                    control: ControlSource::Ai,
                    needs: Some(HomeostaticNeeds::new(
                        pm(100),
                        pm(200),
                        pm(50),
                        pm(30),
                        pm(10),
                    )),
                    combat_profile: None,
                    utility_profile: None,
                    merchandise_profile: None,
                    trade_disposition: None,
                },
            ],
            items: vec![ItemDef {
                commodity: CommodityKind::Grain,
                quantity: Quantity(5),
                location: "Aster".into(),
                container: false,
            }],
            facilities: vec![],
            resource_sources: vec![],
        };
        let spawned = spawn_scenario(&def).unwrap();
        let human_id = spawned
            .state
            .world()
            .query_name_and_agent_data()
            .find(|(_, name, ad)| name.0 == "Aster" && ad.control_source == ControlSource::Human)
            .map(|(id, _, _)| id)
            .unwrap();
        (spawned, human_id)
    }

    // ---- look tests ----

    #[test]
    fn test_look_shows_place_name() {
        let (spawned, _) = rich_scenario();
        // handle_look prints to stdout; we verify it returns Ok.
        let result = handle_look(&spawned.state);
        assert_eq!(result.unwrap(), CommandOutcome::Continue);
        // The place name "Market Square" is in the output (verified via topology).
        let entity = spawned
            .state
            .controller_state()
            .controlled_entity()
            .unwrap();
        let place_id = spawned.state.world().effective_place(entity).unwrap();
        let place = spawned.state.world().topology().place(place_id).unwrap();
        assert_eq!(place.name, "Market Square");
    }

    #[test]
    fn test_look_shows_colocated_entities() {
        let (spawned, human_id) = rich_scenario();
        let world = spawned.state.world();
        let place_id = world.effective_place(human_id).unwrap();
        let colocated = world.entities_effectively_at(place_id);
        // At least two entities at Market Square (Aster + Kael + item lot).
        assert!(
            colocated.len() >= 2,
            "expected at least 2 entities at Market Square, got {}",
            colocated.len()
        );
        let result = handle_look(&spawned.state);
        assert_eq!(result.unwrap(), CommandOutcome::Continue);
    }

    #[test]
    fn test_look_shows_travel_connections() {
        let (spawned, human_id) = rich_scenario();
        let world = spawned.state.world();
        let place_id = world.effective_place(human_id).unwrap();
        let topo = world.topology();
        let outgoing = topo.outgoing_edges(place_id);
        assert!(
            !outgoing.is_empty(),
            "Market Square should have travel connections"
        );
        let result = handle_look(&spawned.state);
        assert_eq!(result.unwrap(), CommandOutcome::Continue);
    }

    // ---- inspect tests ----

    #[test]
    fn test_inspect_shows_components() {
        let (spawned, _) = rich_scenario();
        let result = handle_inspect(&spawned.state, "Aster");
        assert_eq!(result.unwrap(), CommandOutcome::Continue);
        // Aster should have AgentData and Name components.
        let world = spawned.state.world();
        let entity = resolve_entity(world, "Aster").unwrap();
        assert!(world.get_component_agent_data(entity).is_some());
        assert!(world.get_component_name(entity).is_some());
    }

    #[test]
    fn test_inspect_unknown_entity() {
        let (spawned, _) = rich_scenario();
        let result = handle_inspect(&spawned.state, "Nonexistent");
        let err = result.unwrap_err();
        assert!(
            err.message.contains("no entity matching"),
            "expected not-found error, got: {}",
            err.message
        );
    }

    // ---- inventory tests ----

    #[test]
    fn test_inventory_controlled_agent() {
        let (spawned, human_id) = rich_scenario();
        // Aster should have items (Grain was placed at "Aster").
        let possessions = spawned.state.world().possessions_of(human_id);
        assert!(
            !possessions.is_empty(),
            "Aster should have possessions from scenario"
        );
        let result = handle_inventory(&spawned.state, None);
        assert_eq!(result.unwrap(), CommandOutcome::Continue);
    }

    #[test]
    fn test_inventory_named_entity() {
        let (spawned, _) = rich_scenario();
        let result = handle_inventory(&spawned.state, Some("Aster"));
        assert_eq!(result.unwrap(), CommandOutcome::Continue);
    }

    #[test]
    fn test_inventory_empty() {
        let (spawned, _) = rich_scenario();
        // Kael has no items.
        let result = handle_inventory(&spawned.state, Some("Kael"));
        assert_eq!(result.unwrap(), CommandOutcome::Continue);
        // Verify Kael indeed has no possessions.
        let world = spawned.state.world();
        let kael = resolve_entity(world, "Kael").unwrap();
        assert!(world.possessions_of(kael).is_empty());
    }

    // ---- needs tests ----

    #[test]
    fn test_needs_shows_all_five() {
        let (spawned, human_id) = rich_scenario();
        let result = handle_needs(&spawned.state, None);
        assert_eq!(result.unwrap(), CommandOutcome::Continue);
        // Verify all 5 need fields are present.
        let needs = spawned
            .state
            .world()
            .get_component_homeostatic_needs(human_id)
            .expect("agent should have needs");
        assert!(needs.hunger.value() > 0);
        assert!(needs.thirst.value() > 0);
        assert!(needs.fatigue.value() > 0);
        assert!(needs.bladder.value() > 0);
        assert!(needs.dirtiness.value() > 0);
    }

    #[test]
    fn test_needs_non_agent() {
        let (spawned, human_id) = rich_scenario();
        // Find an item lot (non-agent entity).
        let possessions = spawned.state.world().possessions_of(human_id);
        assert!(!possessions.is_empty(), "need at least one item");
        let item_id = possessions[0];
        let result = handle_needs(&spawned.state, Some(&item_id.slot.to_string()));
        let err = result.unwrap_err();
        assert!(
            err.message.contains("not an agent"),
            "expected non-agent error, got: {}",
            err.message
        );
    }

    // ---- relations tests ----

    #[test]
    fn test_relations_shows_placement() {
        let (spawned, _) = rich_scenario();
        let result = handle_relations(&spawned.state, "Aster");
        assert_eq!(result.unwrap(), CommandOutcome::Continue);
        // Verify Aster is placed.
        let world = spawned.state.world();
        let entity = resolve_entity(world, "Aster").unwrap();
        assert!(
            world.effective_place(entity).is_some(),
            "Aster should be placed"
        );
    }
}
