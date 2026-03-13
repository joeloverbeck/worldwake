//! Handlers for world overview commands: `world`, `places`, `agents`, `goods`.
//!
//! All functions are read-only — zero world mutation.

use std::collections::BTreeMap;

use worldwake_core::{entity::EntityKind, ids::EntityId, items::CommodityKind};
use worldwake_sim::{ActionDuration, SimulationState};
use worldwake_systems::ActionRegistries;

use crate::commands::{CommandOutcome, CommandResult};
use crate::display::{entity_display_name, format_control_source, format_location};

/// `world` — summary of all places with agent/item counts.
#[allow(clippy::unnecessary_wraps)] // Must return CommandResult for dispatch interface.
pub fn handle_world(sim: &SimulationState) -> CommandResult {
    let world = sim.world();
    let tick = sim.scheduler().current_tick();

    println!("=== World Overview (tick {}) ===", tick.0);

    for place_id in world.topology().place_ids() {
        let place = world
            .topology()
            .place(place_id)
            .expect("place_id from topology must be valid");

        let entities = world.entities_effectively_at(place_id);

        let agent_count = entities
            .iter()
            .filter(|id| world.entity_kind(**id) == Some(EntityKind::Agent))
            .count();

        let item_count: u32 = entities
            .iter()
            .filter_map(|id| world.get_component_item_lot(*id))
            .map(|lot| lot.quantity.0)
            .sum();

        println!(
            "  {}: {} agents, {} items",
            place.name, agent_count, item_count
        );
    }

    Ok(CommandOutcome::Continue)
}

/// `places` — list places with tags and travel connections.
#[allow(clippy::unnecessary_wraps)] // Must return CommandResult for dispatch interface.
pub fn handle_places(sim: &SimulationState) -> CommandResult {
    let world = sim.world();

    println!("Places:");

    for place_id in world.topology().place_ids() {
        let place = world
            .topology()
            .place(place_id)
            .expect("place_id from topology must be valid");

        let tags: Vec<String> = place.tags.iter().map(|t| format!("{t:?}")).collect();
        let tag_str = if tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", tags.join(", "))
        };

        println!("  {}{}", place.name, tag_str);

        for edge_id in world.topology().outgoing_edges(place_id) {
            let edge = world
                .topology()
                .edge(*edge_id)
                .expect("outgoing edge id must be valid");
            let dest = world
                .topology()
                .place(edge.to())
                .expect("edge destination must be valid place");
            println!("    → {} ({} ticks)", dest.name, edge.travel_time_ticks());
        }
    }

    Ok(CommandOutcome::Continue)
}

/// `agents` — list all living agents with location and current action.
#[allow(clippy::unnecessary_wraps)] // Must return CommandResult for dispatch interface.
pub fn handle_agents(sim: &SimulationState, registries: &ActionRegistries) -> CommandResult {
    let world = sim.world();

    println!("Agents:");

    for (id, agent_data) in world.query_agent_data() {
        if !world.is_alive(id) {
            continue;
        }

        let name = entity_display_name(world, id);
        let control = format_control_source(agent_data.control_source);
        let location = format_location(world, id);

        // Find active action for this agent.
        let action_str = find_agent_action_str(sim, registries, id);

        println!("  {name} {control} {location} — {action_str}");
    }

    Ok(CommandOutcome::Continue)
}

/// Find and format the current action string for an agent.
fn find_agent_action_str(
    sim: &SimulationState,
    registries: &ActionRegistries,
    agent_id: EntityId,
) -> String {
    for action in sim.scheduler().active_actions().values() {
        if action.actor == agent_id {
            let action_name = registries
                .defs
                .get(action.def_id)
                .map_or("unknown", |def| def.name.as_str());
            let remaining = match action.remaining_duration {
                ActionDuration::Finite(t) => format!(" ({t} ticks left)"),
                ActionDuration::Indefinite => String::new(),
            };
            return format!("{action_name}{remaining}");
        }
    }
    "idle".to_string()
}

/// `goods` — global commodity totals with per-place breakdown.
#[allow(clippy::unnecessary_wraps)] // Must return CommandResult for dispatch interface.
pub fn handle_goods(sim: &SimulationState) -> CommandResult {
    let world = sim.world();

    // Aggregate: CommodityKind → (total, BTreeMap<place_name, subtotal>)
    let mut totals: BTreeMap<CommodityKind, (u32, BTreeMap<String, u32>)> = BTreeMap::new();

    for (lot_id, lot) in world.query_item_lot() {
        if !world.is_alive(lot_id) {
            continue;
        }

        let place_name = world
            .effective_place(lot_id)
            .and_then(|pid| world.topology().place(pid))
            .map_or_else(|| "unknown".to_string(), |p| p.name.clone());

        let entry = totals
            .entry(lot.commodity)
            .or_insert_with(|| (0, BTreeMap::new()));
        entry.0 += lot.quantity.0;
        *entry.1.entry(place_name).or_insert(0) += lot.quantity.0;
    }

    if totals.is_empty() {
        println!("No goods in the world.");
        return Ok(CommandOutcome::Continue);
    }

    println!("Goods:");

    for (kind, (total, by_place)) in &totals {
        let breakdown: Vec<String> = by_place
            .iter()
            .map(|(place, qty)| format!("{place}: {qty}"))
            .collect();
        println!("  {kind:?}: {total} total ({})", breakdown.join(", "));
    }

    Ok(CommandOutcome::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::{spawn_scenario, types::*};
    use worldwake_core::{
        control::ControlSource, items::CommodityKind, numerics::Quantity, topology::PlaceTag,
    };

    /// Multi-place scenario with agents, items, and travel edges.
    fn overview_scenario() -> crate::scenario::SpawnedSimulation {
        let def = ScenarioDef {
            seed: 42,
            places: vec![
                PlaceDef {
                    name: "Market Square".into(),
                    tags: vec![PlaceTag::Village, PlaceTag::Store],
                },
                PlaceDef {
                    name: "Forest Clearing".into(),
                    tags: vec![PlaceTag::Forest],
                },
                PlaceDef {
                    name: "Mountain Pass".into(),
                    tags: vec![PlaceTag::Road],
                },
            ],
            edges: vec![
                EdgeDef {
                    from: "Market Square".into(),
                    to: "Forest Clearing".into(),
                    travel_ticks: 3,
                    bidirectional: true,
                },
                EdgeDef {
                    from: "Market Square".into(),
                    to: "Mountain Pass".into(),
                    travel_ticks: 5,
                    bidirectional: true,
                },
            ],
            agents: vec![
                AgentDef {
                    name: "Kael".into(),
                    location: "Market Square".into(),
                    control: ControlSource::Human,
                    needs: None,
                    combat_profile: None,
                    utility_profile: None,
                    merchandise_profile: None,
                    trade_disposition: None,
                },
                AgentDef {
                    name: "Vara".into(),
                    location: "Market Square".into(),
                    control: ControlSource::Ai,
                    needs: None,
                    combat_profile: None,
                    utility_profile: None,
                    merchandise_profile: None,
                    trade_disposition: None,
                },
                AgentDef {
                    name: "Theron".into(),
                    location: "Forest Clearing".into(),
                    control: ControlSource::Ai,
                    needs: None,
                    combat_profile: None,
                    utility_profile: None,
                    merchandise_profile: None,
                    trade_disposition: None,
                },
            ],
            items: vec![
                ItemDef {
                    commodity: CommodityKind::Grain,
                    quantity: Quantity(30),
                    location: "Market Square".into(),
                    container: false,
                },
                ItemDef {
                    commodity: CommodityKind::Grain,
                    quantity: Quantity(15),
                    location: "Forest Clearing".into(),
                    container: false,
                },
                ItemDef {
                    commodity: CommodityKind::Water,
                    quantity: Quantity(20),
                    location: "Market Square".into(),
                    container: false,
                },
            ],
            facilities: vec![],
            resource_sources: vec![],
        };
        spawn_scenario(&def).unwrap()
    }

    /// Empty scenario: places but no items.
    fn empty_goods_scenario() -> crate::scenario::SpawnedSimulation {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Void".into(),
                tags: vec![],
            }],
            edges: vec![],
            agents: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
        };
        spawn_scenario(&def).unwrap()
    }

    // ── handle_world ─────────────────────────────────────────────────

    #[test]
    fn test_world_shows_all_places() {
        let spawned = overview_scenario();
        let world = spawned.state.world();
        let place_count = world.topology().place_ids().count();
        assert_eq!(place_count, 3);

        // Verify handler doesn't panic and returns Continue.
        let result = handle_world(&spawned.state);
        assert_eq!(result.unwrap(), CommandOutcome::Continue);
    }

    #[test]
    fn test_world_shows_population() {
        let spawned = overview_scenario();
        let world = spawned.state.world();

        // Market Square should have 2 agents.
        let market = world
            .topology()
            .place_ids()
            .find(|id| world.topology().place(*id).unwrap().name == "Market Square")
            .unwrap();
        let agents_at_market = world
            .entities_effectively_at(market)
            .iter()
            .filter(|id| world.entity_kind(**id) == Some(EntityKind::Agent))
            .count();
        assert_eq!(agents_at_market, 2);

        // Forest Clearing should have 1 agent.
        let forest = world
            .topology()
            .place_ids()
            .find(|id| world.topology().place(*id).unwrap().name == "Forest Clearing")
            .unwrap();
        let agents_at_forest = world
            .entities_effectively_at(forest)
            .iter()
            .filter(|id| world.entity_kind(**id) == Some(EntityKind::Agent))
            .count();
        assert_eq!(agents_at_forest, 1);

        // Mountain Pass should have 0 agents.
        let mountain = world
            .topology()
            .place_ids()
            .find(|id| world.topology().place(*id).unwrap().name == "Mountain Pass")
            .unwrap();
        let agents_at_mountain = world
            .entities_effectively_at(mountain)
            .iter()
            .filter(|id| world.entity_kind(**id) == Some(EntityKind::Agent))
            .count();
        assert_eq!(agents_at_mountain, 0);
    }

    // ── handle_places ────────────────────────────────────────────────

    #[test]
    fn test_places_shows_connections() {
        let spawned = overview_scenario();
        let world = spawned.state.world();

        // Market Square should have edges to Forest Clearing and Mountain Pass.
        let market = world
            .topology()
            .place_ids()
            .find(|id| world.topology().place(*id).unwrap().name == "Market Square")
            .unwrap();
        let edge_count = world.topology().outgoing_edges(market).len();
        assert_eq!(edge_count, 2);

        let result = handle_places(&spawned.state);
        assert_eq!(result.unwrap(), CommandOutcome::Continue);
    }

    #[test]
    fn test_places_shows_tags() {
        let spawned = overview_scenario();
        let world = spawned.state.world();

        let market = world
            .topology()
            .place_ids()
            .find(|id| world.topology().place(*id).unwrap().name == "Market Square")
            .unwrap();
        let place = world.topology().place(market).unwrap();
        assert!(place.tags.contains(&PlaceTag::Village));
        assert!(place.tags.contains(&PlaceTag::Store));
    }

    // ── handle_agents ────────────────────────────────────────────────

    #[test]
    fn test_agents_lists_all_living() {
        let spawned = overview_scenario();
        let living_count = spawned
            .state
            .world()
            .query_agent_data()
            .filter(|(id, _)| spawned.state.world().is_alive(*id))
            .count();
        assert_eq!(living_count, 3);

        let result = handle_agents(&spawned.state, &spawned.action_registries);
        assert_eq!(result.unwrap(), CommandOutcome::Continue);
    }

    #[test]
    fn test_agents_shows_location() {
        let spawned = overview_scenario();
        let world = spawned.state.world();

        // Kael should be at Market Square.
        let kael = world
            .query_name_and_agent_data()
            .find(|(_, n, _)| n.0 == "Kael")
            .map(|(id, _, _)| id)
            .unwrap();
        let place_id = world.effective_place(kael).unwrap();
        let place = world.topology().place(place_id).unwrap();
        assert_eq!(place.name, "Market Square");

        // Theron should be at Forest Clearing.
        let theron = world
            .query_name_and_agent_data()
            .find(|(_, n, _)| n.0 == "Theron")
            .map(|(id, _, _)| id)
            .unwrap();
        let place_id = world.effective_place(theron).unwrap();
        let place = world.topology().place(place_id).unwrap();
        assert_eq!(place.name, "Forest Clearing");
    }

    #[test]
    fn test_agents_shows_control_source() {
        let spawned = overview_scenario();
        let world = spawned.state.world();

        let kael = world
            .query_name_and_agent_data()
            .find(|(_, n, _)| n.0 == "Kael")
            .map(|(id, _, _)| id)
            .unwrap();
        assert_eq!(
            world.get_component_agent_data(kael).unwrap().control_source,
            ControlSource::Human
        );

        let vara = world
            .query_name_and_agent_data()
            .find(|(_, n, _)| n.0 == "Vara")
            .map(|(id, _, _)| id)
            .unwrap();
        assert_eq!(
            world.get_component_agent_data(vara).unwrap().control_source,
            ControlSource::Ai
        );
    }

    // ── handle_goods ─────────────────────────────────────────────────

    #[test]
    fn test_goods_aggregates_by_commodity() {
        let spawned = overview_scenario();
        let world = spawned.state.world();

        // Build the same aggregation the handler does.
        let mut totals: BTreeMap<CommodityKind, u32> = BTreeMap::new();
        for (lot_id, lot) in world.query_item_lot() {
            if world.is_alive(lot_id) {
                *totals.entry(lot.commodity).or_insert(0) += lot.quantity.0;
            }
        }

        assert_eq!(totals.get(&CommodityKind::Grain), Some(&45));
        assert_eq!(totals.get(&CommodityKind::Water), Some(&20));

        let result = handle_goods(&spawned.state);
        assert_eq!(result.unwrap(), CommandOutcome::Continue);
    }

    #[test]
    fn test_goods_empty_world() {
        let spawned = empty_goods_scenario();

        // No items → handler should print "No goods" message.
        let result = handle_goods(&spawned.state);
        assert_eq!(result.unwrap(), CommandOutcome::Continue);
    }
}
