//! Scenario system — RON-based world initialization.
//!
//! `types` defines the deserialization schema (`ScenarioDef` and sub-structs).
//! `spawn_scenario()` builds a fully initialized simulation from a `ScenarioDef`.

pub mod types;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use types::ScenarioDef;
use worldwake_core::{
    hash_world, CauseRef, ControlSource, DeprivationExposure, DriveThresholds, EntityId,
    EntityKind, EventLog, MerchandiseProfile, MetabolismProfile, Place, ResourceSource, Seed,
    Tick, Topology, TravelEdge, TravelEdgeId, VisibilitySpec, WitnessData, WorkstationMarker,
    World, WorldTxn,
};
use worldwake_sim::{
    ControllerState, DeterministicRng, RecipeRegistry, ReplayRecordingConfig, ReplayState,
    Scheduler, SimulationState, SystemDispatchTable, SystemManifest,
};
use worldwake_systems::{build_full_action_registries, dispatch_table, ActionRegistries};

/// Bundled result of scenario spawning: persistent simulation state plus
/// transient runtime artifacts (action registries, dispatch table).
///
/// `SimulationState` is serializable (save/load). Registries and dispatch
/// tables are derived from the recipe registry and must be rebuilt after load.
pub struct SpawnedSimulation {
    pub state: SimulationState,
    pub action_registries: ActionRegistries,
    pub dispatch_table: SystemDispatchTable,
}

/// Errors that can occur during scenario loading or spawning.
#[derive(Debug)]
pub enum ScenarioError {
    Io(std::io::Error),
    Parse(ron::error::SpannedError),
    Validation(String),
    World(worldwake_core::WorldError),
}

impl From<std::io::Error> for ScenarioError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<ron::error::SpannedError> for ScenarioError {
    fn from(e: ron::error::SpannedError) -> Self {
        Self::Parse(e)
    }
}

impl From<worldwake_core::WorldError> for ScenarioError {
    fn from(e: worldwake_core::WorldError) -> Self {
        Self::World(e)
    }
}

impl std::fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Parse(e) => write!(f, "RON parse error: {e}"),
            Self::Validation(msg) => write!(f, "validation error: {msg}"),
            Self::World(e) => write!(f, "world error: {e}"),
        }
    }
}

impl std::error::Error for ScenarioError {}

/// RON deserialization options matching the scenario format.
/// Uses `UNWRAP_NEWTYPES` (for `Permille`, `Quantity`, etc.) and
/// `IMPLICIT_SOME` (for optional fields like `combat_profile`).
fn ron_options() -> ron::Options {
    ron::Options::default()
        .with_default_extension(ron::extensions::Extensions::UNWRAP_NEWTYPES)
        .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
}

/// Load and parse a RON scenario file into a `ScenarioDef`.
pub fn load_scenario_file(path: &Path) -> Result<ScenarioDef, ScenarioError> {
    let contents = std::fs::read_to_string(path)?;
    let def: ScenarioDef = ron_options().from_str(&contents)?;
    Ok(def)
}

/// Build a fully initialized simulation from a scenario definition.
///
/// Bootstrap sequence:
/// 1. Build `Topology` from places + edges
/// 2. `World::new(topology)`
/// 3. Spawn agents, items, facilities, resource sources via `WorldTxn`
/// 4. Build action registries and dispatch table
/// 5. Assemble `SimulationState` + runtime artifacts into `SpawnedSimulation`
pub fn spawn_scenario(def: &ScenarioDef) -> Result<SpawnedSimulation, ScenarioError> {
    let mut names: BTreeMap<String, EntityId> = BTreeMap::new();
    let mut place_names: BTreeSet<String> = BTreeSet::new();

    let topology = build_topology(def, &mut names, &mut place_names)?;
    let mut world = World::new(topology)?;
    let mut event_log = EventLog::new();

    spawn_entities(def, &mut world, &mut event_log, &mut names, &place_names)?;

    let recipe_registry = RecipeRegistry::new();
    let action_registries = build_full_action_registries(&recipe_registry).map_err(|orphans| {
        ScenarioError::Validation(format!(
            "action registry incomplete: {} orphaned defs",
            orphans.len()
        ))
    })?;
    let dispatch = dispatch_table();

    let state = assemble_state(def, &names, world, event_log, recipe_registry)?;

    Ok(SpawnedSimulation {
        state,
        action_registries,
        dispatch_table: dispatch,
    })
}

/// Build the topology graph from place and edge definitions.
fn build_topology(
    def: &ScenarioDef,
    names: &mut BTreeMap<String, EntityId>,
    place_names: &mut BTreeSet<String>,
) -> Result<Topology, ScenarioError> {
    let mut topology = Topology::new();
    let mut next_edge_id: u32 = 0;

    for (slot, place_def) in def.places.iter().enumerate() {
        let place_id = EntityId {
            slot: u32::try_from(slot)
                .map_err(|_| ScenarioError::Validation("too many places (exceeds u32)".into()))?,
            generation: 0,
        };

        if names.contains_key(&place_def.name) {
            return Err(ScenarioError::Validation(format!(
                "duplicate place name: '{}'",
                place_def.name
            )));
        }

        topology.add_place(
            place_id,
            Place {
                name: place_def.name.clone(),
                capacity: None,
                tags: place_def.tags.iter().copied().collect(),
            },
        )?;

        place_names.insert(place_def.name.clone());
        names.insert(place_def.name.clone(), place_id);
    }

    for edge_def in &def.edges {
        let from = resolve_name(names, &edge_def.from, "edge 'from'")?;
        let to = resolve_name(names, &edge_def.to, "edge 'to'")?;

        topology.add_edge(TravelEdge::new(
            TravelEdgeId(next_edge_id),
            from,
            to,
            edge_def.travel_ticks,
            None,
        )?)?;
        next_edge_id += 1;

        if edge_def.bidirectional {
            topology.add_edge(TravelEdge::new(
                TravelEdgeId(next_edge_id),
                to,
                from,
                edge_def.travel_ticks,
                None,
            )?)?;
            next_edge_id += 1;
        }
    }

    Ok(topology)
}

/// Spawn all entities (agents, items, facilities, resource sources) via a single `WorldTxn`.
fn spawn_entities(
    def: &ScenarioDef,
    world: &mut World,
    event_log: &mut EventLog,
    names: &mut BTreeMap<String, EntityId>,
    place_names: &BTreeSet<String>,
) -> Result<(), ScenarioError> {
    let mut agent_locations: BTreeMap<EntityId, EntityId> = BTreeMap::new();

    let mut txn = WorldTxn::new(
        world,
        Tick(0),
        CauseRef::Bootstrap,
        None,
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );

    for agent_def in &def.agents {
        spawn_agent(&mut txn, agent_def, names, &mut agent_locations)?;
    }

    for item_def in &def.items {
        spawn_item(&mut txn, item_def, names, place_names, &agent_locations)?;
    }

    for facility_def in &def.facilities {
        let place_id = resolve_name(
            names,
            &facility_def.location,
            &format!("facility {:?} location", facility_def.workstation),
        )?;
        let facility_id = txn.create_entity(EntityKind::Facility);
        txn.set_component_workstation_marker(
            facility_id,
            WorkstationMarker(facility_def.workstation),
        )?;
        txn.set_ground_location(facility_id, place_id)?;
    }

    for source_def in &def.resource_sources {
        let place_id = resolve_name(
            names,
            &source_def.location,
            &format!("resource source {:?} location", source_def.commodity),
        )?;
        let source_id = txn.create_entity(EntityKind::Facility);
        txn.set_component_resource_source(
            source_id,
            ResourceSource {
                commodity: source_def.commodity,
                available_quantity: source_def.capacity,
                max_quantity: source_def.capacity,
                regeneration_ticks_per_unit: source_def.regeneration_ticks_per_unit,
                last_regeneration_tick: None,
            },
        )?;
        txn.set_ground_location(source_id, place_id)?;
    }

    txn.commit(event_log);
    Ok(())
}

/// Spawn a single agent with all optional component profiles.
fn spawn_agent(
    txn: &mut WorldTxn<'_>,
    agent_def: &types::AgentDef,
    names: &mut BTreeMap<String, EntityId>,
    agent_locations: &mut BTreeMap<EntityId, EntityId>,
) -> Result<(), ScenarioError> {
    let place_id = resolve_name(
        names,
        &agent_def.location,
        &format!("agent '{}' location", agent_def.name),
    )?;

    let agent_id = txn.create_agent(&agent_def.name, agent_def.control)?;

    let needs = agent_def.needs.unwrap_or_default();
    txn.set_component_homeostatic_needs(agent_id, needs)?;
    txn.set_component_deprivation_exposure(agent_id, DeprivationExposure::default())?;
    txn.set_component_drive_thresholds(agent_id, DriveThresholds::default())?;
    txn.set_component_metabolism_profile(agent_id, MetabolismProfile::default())?;

    if let Some(ref combat) = agent_def.combat_profile {
        txn.set_component_combat_profile(agent_id, *combat)?;
    }
    if let Some(ref utility) = agent_def.utility_profile {
        txn.set_component_utility_profile(agent_id, utility.clone())?;
    }
    if let Some(ref merch_def) = agent_def.merchandise_profile {
        let home_market = merch_def
            .home_market
            .as_ref()
            .map(|name| {
                resolve_name(
                    names,
                    name,
                    &format!("agent '{}' merchandise home_market", agent_def.name),
                )
            })
            .transpose()?;

        let profile = MerchandiseProfile {
            sale_kinds: merch_def.sale_kinds.iter().copied().collect(),
            home_market,
        };
        txn.set_component_merchandise_profile(agent_id, profile)?;
    }
    if let Some(ref trade_disp) = agent_def.trade_disposition {
        txn.set_component_trade_disposition_profile(agent_id, trade_disp.clone())?;
    }

    txn.set_ground_location(agent_id, place_id)?;
    agent_locations.insert(agent_id, place_id);
    names.insert(agent_def.name.clone(), agent_id);
    Ok(())
}

/// Spawn a single item lot at a place or on an agent.
fn spawn_item(
    txn: &mut WorldTxn<'_>,
    item_def: &types::ItemDef,
    names: &BTreeMap<String, EntityId>,
    place_names: &BTreeSet<String>,
    agent_locations: &BTreeMap<EntityId, EntityId>,
) -> Result<(), ScenarioError> {
    let location_id = resolve_name(
        names,
        &item_def.location,
        &format!("item {:?} location", item_def.commodity),
    )?;

    let item_id = txn.create_item_lot(item_def.commodity, item_def.quantity)?;

    if place_names.contains(&item_def.location) {
        txn.set_ground_location(item_id, location_id)?;
    } else {
        let agent_place = agent_locations.get(&location_id).ok_or_else(|| {
            ScenarioError::Validation(format!(
                "item {:?} location '{}' is not a place or agent",
                item_def.commodity, item_def.location
            ))
        })?;
        txn.set_ground_location(item_id, *agent_place)?;
        txn.set_possessor(item_id, location_id)?;
    }

    Ok(())
}

/// Assemble the final `SimulationState` from all spawned world data.
fn assemble_state(
    def: &ScenarioDef,
    names: &BTreeMap<String, EntityId>,
    world: World,
    event_log: EventLog,
    recipe_registry: RecipeRegistry,
) -> Result<SimulationState, ScenarioError> {
    let controller_state = def
        .agents
        .iter()
        .find(|a| a.control == ControlSource::Human)
        .and_then(|a| names.get(&a.name))
        .map_or_else(ControllerState::new, |&id| ControllerState::with_entity(id));

    let seed_bytes = seed_from_u64(def.seed);
    let rng = DeterministicRng::new(Seed(seed_bytes));

    let initial_hash = hash_world(&world)
        .map_err(|e| ScenarioError::Validation(format!("failed to hash initial world: {e}")))?;
    let replay_state = ReplayState::new(
        initial_hash,
        Seed(seed_bytes),
        Tick(0),
        ReplayRecordingConfig::disabled(),
    );

    let scheduler = Scheduler::new_with_tick(Tick(0), SystemManifest::canonical());

    Ok(SimulationState::new(
        world,
        event_log,
        scheduler,
        recipe_registry,
        replay_state,
        controller_state,
        rng,
    ))
}

/// Resolve a name to an `EntityId`, returning a descriptive validation error.
fn resolve_name(
    names: &BTreeMap<String, EntityId>,
    name: &str,
    context: &str,
) -> Result<EntityId, ScenarioError> {
    names.get(name).copied().ok_or_else(|| {
        ScenarioError::Validation(format!("{context} references nonexistent entity '{name}'"))
    })
}

/// Convert a u64 scenario seed into a 32-byte seed array.
fn seed_from_u64(seed: u64) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&seed.to_le_bytes());
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::types::*;
    use std::num::NonZeroU32;
    use worldwake_core::topology::PlaceTag;
    use worldwake_core::{
        CommodityKind, ControlSource, HomeostaticNeeds, Permille, Quantity, WorkstationTag,
    };

    /// Helper: build a minimal `ScenarioDef` with given places and agents.
    fn minimal_def() -> ScenarioDef {
        ScenarioDef {
            seed: 42,
            places: vec![PlaceDef {
                name: "Village".into(),
                tags: vec![PlaceTag::Village],
            }],
            edges: vec![],
            agents: vec![AgentDef {
                name: "Alice".into(),
                location: "Village".into(),
                control: ControlSource::Human,
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

    #[test]
    fn test_spawn_minimal_scenario() {
        let def = minimal_def();
        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();

        // 1 place in topology
        assert_eq!(world.topology().place_ids().count(), 1);

        // 1 agent exists
        let agents: Vec<_> = world.entities_with_name_and_agent_data().collect();
        assert_eq!(agents.len(), 1);

        let agent_id = agents[0];
        let name = world.get_component_name(agent_id).unwrap();
        assert_eq!(name.0, "Alice");

        // Agent is at Village
        let place_id = world.effective_place(agent_id).unwrap();
        let place = world.topology().place(place_id).unwrap();
        assert_eq!(place.name, "Village");
    }

    #[test]
    fn test_spawn_agents_at_places() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![
                PlaceDef {
                    name: "Town".into(),
                    tags: vec![],
                },
                PlaceDef {
                    name: "Forest".into(),
                    tags: vec![],
                },
            ],
            edges: vec![],
            agents: vec![
                AgentDef {
                    name: "Alice".into(),
                    location: "Town".into(),
                    control: ControlSource::Human,
                    needs: None,
                    combat_profile: None,
                    utility_profile: None,
                    merchandise_profile: None,
                    trade_disposition: None,
                },
                AgentDef {
                    name: "Bob".into(),
                    location: "Forest".into(),
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
        let world = spawned.state.world();

        let agents: Vec<_> = world.entities_with_name_and_agent_data().collect();
        assert_eq!(agents.len(), 2);

        // Find Alice and Bob by name
        let alice = agents
            .iter()
            .find(|&&id| world.get_component_name(id).unwrap().0 == "Alice")
            .unwrap();
        let bob = agents
            .iter()
            .find(|&&id| world.get_component_name(id).unwrap().0 == "Bob")
            .unwrap();

        // Verify placements
        let alice_place = world.effective_place(*alice).unwrap();
        assert_eq!(world.topology().place(alice_place).unwrap().name, "Town");

        let bob_place = world.effective_place(*bob).unwrap();
        assert_eq!(world.topology().place(bob_place).unwrap().name, "Forest");
    }

    #[test]
    fn test_spawn_items_at_place() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Market".into(),
                tags: vec![],
            }],
            edges: vec![],
            agents: vec![AgentDef {
                name: "Trader".into(),
                location: "Market".into(),
                control: ControlSource::Ai,
                needs: None,
                combat_profile: None,
                utility_profile: None,
                merchandise_profile: None,
                trade_disposition: None,
            }],
            items: vec![ItemDef {
                commodity: CommodityKind::Apple,
                quantity: Quantity(10),
                location: "Market".into(),
                container: false,
            }],
            facilities: vec![],
            resource_sources: vec![],
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();

        // Find item lot at Market
        let market_id = EntityId {
            slot: 0,
            generation: 0,
        };
        let entities_at_market = world.entities_effectively_at(market_id);

        // Should have the agent + the item lot
        assert!(entities_at_market.len() >= 2);

        // Find the item lot
        let item = entities_at_market
            .iter()
            .find(|&&id| world.get_component_item_lot(id).is_some());
        assert!(item.is_some(), "item lot should be at Market");

        let item_id = *item.unwrap();
        let lot = world.get_component_item_lot(item_id).unwrap();
        assert_eq!(lot.commodity, CommodityKind::Apple);
        assert_eq!(lot.quantity, Quantity(10));
    }

    #[test]
    fn test_spawn_items_on_agent() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Camp".into(),
                tags: vec![],
            }],
            edges: vec![],
            agents: vec![AgentDef {
                name: "Warrior".into(),
                location: "Camp".into(),
                control: ControlSource::Human,
                needs: None,
                combat_profile: None,
                utility_profile: None,
                merchandise_profile: None,
                trade_disposition: None,
            }],
            items: vec![ItemDef {
                commodity: CommodityKind::Sword,
                quantity: Quantity(1),
                location: "Warrior".into(),
                container: false,
            }],
            facilities: vec![],
            resource_sources: vec![],
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();

        // Find the warrior agent
        let agents: Vec<_> = world.entities_with_name_and_agent_data().collect();
        let warrior = agents[0];

        // Find the sword item
        let possessions = world.possessions_of(warrior);
        assert_eq!(possessions.len(), 1, "warrior should possess 1 item");

        let sword_id = possessions[0];
        let lot = world.get_component_item_lot(sword_id).unwrap();
        assert_eq!(lot.commodity, CommodityKind::Sword);
        assert_eq!(lot.quantity, Quantity(1));
    }

    #[test]
    fn test_spawn_with_edges() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![
                PlaceDef {
                    name: "A".into(),
                    tags: vec![],
                },
                PlaceDef {
                    name: "B".into(),
                    tags: vec![],
                },
            ],
            edges: vec![EdgeDef {
                from: "A".into(),
                to: "B".into(),
                travel_ticks: 5,
                bidirectional: false,
            }],
            agents: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let topo = world.topology();

        let a = EntityId {
            slot: 0,
            generation: 0,
        };
        let b = EntityId {
            slot: 1,
            generation: 0,
        };

        // A → B exists
        let outgoing_a = topo.outgoing_edges(a);
        assert_eq!(outgoing_a.len(), 1);
        assert_eq!(topo.edge(outgoing_a[0]).unwrap().to(), b);

        // B → A does NOT exist (not bidirectional)
        let outgoing_b = topo.outgoing_edges(b);
        assert!(outgoing_b.is_empty());
    }

    #[test]
    fn test_spawn_bidirectional_edge() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![
                PlaceDef {
                    name: "X".into(),
                    tags: vec![],
                },
                PlaceDef {
                    name: "Y".into(),
                    tags: vec![],
                },
            ],
            edges: vec![EdgeDef {
                from: "X".into(),
                to: "Y".into(),
                travel_ticks: 3,
                bidirectional: true,
            }],
            agents: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
        };

        let spawned = spawn_scenario(&def).unwrap();
        let topo = spawned.state.world().topology();

        let x = EntityId {
            slot: 0,
            generation: 0,
        };
        let y = EntityId {
            slot: 1,
            generation: 0,
        };

        // X → Y exists
        let outgoing_x = topo.outgoing_edges(x);
        assert_eq!(outgoing_x.len(), 1);
        assert_eq!(topo.edge(outgoing_x[0]).unwrap().to(), y);

        // Y → X also exists
        let outgoing_y = topo.outgoing_edges(y);
        assert_eq!(outgoing_y.len(), 1);
        assert_eq!(topo.edge(outgoing_y[0]).unwrap().to(), x);
    }

    #[test]
    fn test_spawn_human_control() {
        let def = minimal_def();
        let spawned = spawn_scenario(&def).unwrap();

        // Alice is Human-controlled → ControllerState should track her
        let controlled = spawned.state.controller_state().controlled_entity();
        assert!(controlled.is_some(), "human agent should be tracked");

        let agent_id = controlled.unwrap();
        let name = spawned.state.world().get_component_name(agent_id).unwrap();
        assert_eq!(name.0, "Alice");
    }

    #[test]
    fn test_spawn_invalid_place_ref() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Town".into(),
                tags: vec![],
            }],
            edges: vec![],
            agents: vec![AgentDef {
                name: "Lost".into(),
                location: "Nowhere".into(), // does not exist
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
        };

        let result = spawn_scenario(&def);
        let Err(err) = result else {
            panic!("expected error for nonexistent place reference");
        };
        match err {
            ScenarioError::Validation(msg) => {
                assert!(
                    msg.contains("Nowhere"),
                    "error should mention the bad name: {msg}"
                );
            }
            other => panic!("expected Validation error, got: {other:?}"),
        }
    }

    #[test]
    fn test_spawn_facilities_and_sources() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![
                PlaceDef {
                    name: "Smithy".into(),
                    tags: vec![],
                },
                PlaceDef {
                    name: "Orchard".into(),
                    tags: vec![],
                },
            ],
            edges: vec![],
            agents: vec![],
            items: vec![],
            facilities: vec![FacilityDef {
                workstation: WorkstationTag::Forge,
                location: "Smithy".into(),
            }],
            resource_sources: vec![ResourceSourceDef {
                commodity: CommodityKind::Apple,
                location: "Orchard".into(),
                regeneration_ticks_per_unit: NonZeroU32::new(5),
                capacity: Quantity(20),
            }],
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();

        let smithy = EntityId {
            slot: 0,
            generation: 0,
        };
        let orchard = EntityId {
            slot: 1,
            generation: 0,
        };

        // Find workstation at Smithy
        let at_smithy = world.entities_effectively_at(smithy);
        let forge = at_smithy
            .iter()
            .find(|&&id| world.get_component_workstation_marker(id).is_some());
        assert!(forge.is_some(), "forge should be at Smithy");
        let marker = world
            .get_component_workstation_marker(*forge.unwrap())
            .unwrap();
        assert_eq!(marker.0, WorkstationTag::Forge);

        // Find resource source at Orchard
        let at_orchard = world.entities_effectively_at(orchard);
        let source = at_orchard
            .iter()
            .find(|&&id| world.get_component_resource_source(id).is_some());
        assert!(source.is_some(), "apple source should be at Orchard");
        let rs = world
            .get_component_resource_source(*source.unwrap())
            .unwrap();
        assert_eq!(rs.commodity, CommodityKind::Apple);
        assert_eq!(rs.max_quantity, Quantity(20));
        assert_eq!(rs.available_quantity, Quantity(20));
        assert_eq!(rs.regeneration_ticks_per_unit, NonZeroU32::new(5));
    }

    #[test]
    fn test_spawn_determinism() {
        let def1 = minimal_def();
        let def2 = minimal_def();

        let spawned1 = spawn_scenario(&def1).unwrap();
        let spawned2 = spawn_scenario(&def2).unwrap();

        assert_eq!(
            spawned1.state.hash().unwrap(),
            spawned2.state.hash().unwrap(),
            "same ScenarioDef with same seed must produce identical SimulationState"
        );
    }

    #[test]
    fn test_spawn_no_human_agent() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Void".into(),
                tags: vec![],
            }],
            edges: vec![],
            agents: vec![AgentDef {
                name: "Bot".into(),
                location: "Void".into(),
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
        };

        let spawned = spawn_scenario(&def).unwrap();
        assert_eq!(
            spawned.state.controller_state().controlled_entity(),
            None,
            "no human agent → ControllerState should be empty"
        );
    }

    #[test]
    fn test_spawn_agent_with_needs_override() {
        let custom_needs = HomeostaticNeeds::new(
            Permille::new(100).unwrap(),
            Permille::new(200).unwrap(),
            Permille::new(50).unwrap(),
            Permille::new(0).unwrap(),
            Permille::new(0).unwrap(),
        );

        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Home".into(),
                tags: vec![],
            }],
            edges: vec![],
            agents: vec![AgentDef {
                name: "Hungry".into(),
                location: "Home".into(),
                control: ControlSource::Ai,
                needs: Some(custom_needs),
                combat_profile: None,
                utility_profile: None,
                merchandise_profile: None,
                trade_disposition: None,
            }],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();

        let agents: Vec<_> = world.entities_with_name_and_agent_data().collect();
        let needs = world.get_component_homeostatic_needs(agents[0]).unwrap();
        assert_eq!(needs.hunger, Permille::new(100).unwrap());
        assert_eq!(needs.thirst, Permille::new(200).unwrap());
        assert_eq!(needs.fatigue, Permille::new(50).unwrap());
    }
}
