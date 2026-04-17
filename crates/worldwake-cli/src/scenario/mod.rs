//! Scenario system — RON-based world initialization.
//!
//! `types` defines the deserialization schema (`ScenarioDef` and sub-structs).
//! `spawn_scenario()` builds a fully initialized simulation from a `ScenarioDef`.

pub mod types;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use types::ScenarioDef;
use worldwake_core::{
    CarryCapacity, CauseRef, ControlSource, DeprivationExposure, EntityId, EntityKind, EventLog,
    ExplorationProfile, KnownRecipes, LastProactiveExplorationTick, LoadUnits, MerchandiseProfile,
    PatrolRoute, Place, ProductionOutputOwner, ProductionOutputOwnershipPolicy, ResourceSource,
    Seed, Tick, Topology, TravelEdge, TravelEdgeId, VisibilitySpec, WitnessData, WorkstationMarker,
    World, WorldTxn, default_commodity_decay_map, hash_world,
};
use worldwake_sim::{
    ControllerState, DeterministicRng, RecipeRegistry, ReplayRecordingConfig, ReplayState,
    Scheduler, SimulationState, SystemDispatchTable, SystemManifest,
};
use worldwake_systems::{
    ActionRegistries, build_canonical_production_recipe_registry, build_full_action_registries,
    dispatch_table,
};

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

const DEFAULT_AGENT_CARRY_CAPACITY: CarryCapacity = CarryCapacity(LoadUnits(20));

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
    let recipe_registry = build_canonical_production_recipe_registry();

    let topology = build_topology(def, &mut names, &mut place_names)?;
    let mut world = World::new(topology)?;
    world.set_commodity_decay(
        def.commodity_decay
            .clone()
            .unwrap_or_else(default_commodity_decay_map),
    );
    let mut event_log = EventLog::new();

    spawn_entities(
        def,
        &recipe_registry,
        &mut world,
        &mut event_log,
        &mut names,
        &place_names,
    )?;

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
    recipes: &RecipeRegistry,
    world: &mut World,
    event_log: &mut EventLog,
    names: &mut BTreeMap<String, EntityId>,
    place_names: &BTreeSet<String>,
) -> Result<(), ScenarioError> {
    let mut agent_locations: BTreeMap<EntityId, EntityId> = BTreeMap::new();
    let mut facility_locations: BTreeMap<EntityId, EntityId> = BTreeMap::new();

    let mut txn = WorldTxn::new(
        world,
        Tick(0),
        CauseRef::Bootstrap,
        None,
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );

    for place_def in &def.places {
        let place_id = resolve_name(names, &place_def.name, "place visibility_profile")?;
        if let Some(profile) = &place_def.visibility_profile {
            txn.set_component_place_visibility_profile(place_id, profile.clone())?;
        }
    }

    for agent_def in &def.agents {
        spawn_agent(&mut txn, recipes, agent_def, names, &mut agent_locations)?;
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
        txn.set_component_production_output_ownership_policy(
            facility_id,
            ProductionOutputOwnershipPolicy {
                output_owner: ProductionOutputOwner::Actor,
            },
        )?;
        facility_locations.insert(facility_id, place_id);
        if let Some(name) = &facility_def.name {
            names.insert(name.clone(), facility_id);
        }
    }

    for source_def in &def.resource_sources {
        let place_id = resolve_name(
            names,
            &source_def.location,
            &format!("resource source {:?} location", source_def.commodity),
        )?;
        let source_id = if let Some(facility_name) = &source_def.facility {
            let facility_id = resolve_name(
                names,
                facility_name,
                &format!("resource source {:?} facility", source_def.commodity),
            )?;
            let facility_place =
                facility_locations
                    .get(&facility_id)
                    .copied()
                    .ok_or_else(|| {
                        ScenarioError::Validation(format!(
                            "resource source {:?} facility '{}' is not a spawned facility",
                            source_def.commodity, facility_name
                        ))
                    })?;
            if facility_place != place_id {
                return Err(ScenarioError::Validation(format!(
                    "resource source {:?} facility '{}' is not at '{}'",
                    source_def.commodity, facility_name, source_def.location
                )));
            }
            facility_id
        } else {
            let source_id = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(source_id, place_id)?;
            txn.set_component_production_output_ownership_policy(
                source_id,
                ProductionOutputOwnershipPolicy {
                    output_owner: ProductionOutputOwner::Actor,
                },
            )?;
            facility_locations.insert(source_id, place_id);
            source_id
        };
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
    }

    txn.commit(event_log);
    Ok(())
}

/// Spawn a single agent with all optional component profiles.
fn spawn_agent(
    txn: &mut WorldTxn<'_>,
    recipes: &RecipeRegistry,
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
    let thresholds = agent_def.drive_thresholds.unwrap_or_default();
    txn.set_component_drive_thresholds(agent_id, thresholds)?;
    let drive_escalation_profile = agent_def
        .drive_escalation_profile
        .clone()
        .unwrap_or_default();
    txn.set_component_drive_escalation_profile(agent_id, drive_escalation_profile)?;
    let metabolism = agent_def.metabolism_profile.unwrap_or_default();
    txn.set_component_metabolism_profile(agent_id, metabolism)?;
    if let Some(profile) = agent_def.disposal_profile {
        txn.set_component_disposal_profile(agent_id, profile)?;
    }
    let exploration =
        agent_def
            .exploration_profile
            .map_or_else(ExplorationProfile::default, |profile| ExplorationProfile {
                curiosity_weight: profile.curiosity_weight,
                need_activation_threshold: profile.need_activation_threshold,
                frontier_depth: profile.frontier_depth,
                acquisition_failure_threshold: profile.acquisition_failure_threshold,
                exploration_arrival_boost: profile.exploration_arrival_boost,
                max_consecutive_explorations: profile.max_consecutive_explorations,
                visit_lookback_ticks: profile.visit_lookback_ticks,
                consecutive_exploration_count: 0,
            });
    txn.set_component_exploration_profile(agent_id, exploration)?;
    if let Some(profile) = agent_def.diversification_profile {
        txn.set_component_diversification_profile(agent_id, profile)?;
        txn.set_component_last_proactive_exploration_tick(
            agent_id,
            LastProactiveExplorationTick(None),
        )?;
    }
    let carry = agent_def
        .carry_capacity
        .unwrap_or(DEFAULT_AGENT_CARRY_CAPACITY);
    txn.set_component_carry_capacity(agent_id, carry)?;

    let perception = agent_def.perception_profile.unwrap_or_default();
    txn.set_component_perception_profile(agent_id, perception)?;
    let tell = agent_def.tell_profile.unwrap_or_default();
    txn.set_component_tell_profile(agent_id, tell)?;
    let cognitive = agent_def.cognitive_profile.unwrap_or_default();
    let execution_budget = agent_def.execution_budget.unwrap_or_default();
    txn.set_component_cognitive_profile(agent_id, cognitive)?;
    txn.set_component_execution_budget(agent_id, execution_budget)?;
    let epistemic = agent_def.epistemic_disposition.clone().unwrap_or_default();
    txn.set_component_epistemic_disposition_profile(agent_id, epistemic)?;
    let intention = agent_def.intention_disposition.clone().unwrap_or_default();
    txn.set_component_intention_disposition_profile(agent_id, intention)?;
    let communication = agent_def.communication_profile.clone().unwrap_or_default();
    txn.set_component_communication_profile(agent_id, communication)?;
    let preference = agent_def.preference_profile.unwrap_or_default();
    txn.set_component_preference_profile(agent_id, preference)?;
    let expectation_store = agent_def.expectation_store.clone().unwrap_or_default();
    txn.set_component_expectation_store(agent_id, expectation_store)?;
    let last_seen_memory = agent_def.last_seen_memory.clone().unwrap_or_default();
    txn.set_component_last_seen_memory(agent_id, last_seen_memory)?;
    let obligation_satiation_profile = agent_def
        .obligation_satiation_profile
        .clone()
        .unwrap_or_default();
    txn.set_component_obligation_satiation_profile(agent_id, obligation_satiation_profile)?;

    if let Some(ref combat) = agent_def.combat_profile {
        txn.set_component_combat_profile(agent_id, *combat)?;
    }
    if let Some(ref utility) = agent_def.utility_profile {
        txn.set_component_utility_profile(agent_id, utility.clone())?;
    }
    let artifact_posting = agent_def
        .artifact_posting_profile
        .clone()
        .unwrap_or_default();
    txn.set_component_artifact_posting_profile(agent_id, artifact_posting)?;
    if let Some(ref merch_def) = agent_def.merchandise_profile {
        let home_facility = merch_def
            .home_facility
            .as_ref()
            .map(|name| {
                resolve_name(
                    names,
                    name,
                    &format!("agent '{}' merchandise home_facility", agent_def.name),
                )
            })
            .transpose()?;

        let profile = MerchandiseProfile {
            sale_kinds: merch_def.sale_kinds.iter().copied().collect(),
            home_facility,
        };
        txn.set_component_merchandise_profile(agent_id, profile)?;
    }
    if let Some(ref trade_disp) = agent_def.trade_disposition {
        txn.set_component_trade_disposition_profile(agent_id, trade_disp.clone())?;
    }
    if let Some(ref profile) = agent_def.theft_disposition {
        txn.set_component_theft_disposition_profile(agent_id, profile.clone())?;
    }
    if let Some(ref profile) = agent_def.justice_disposition {
        txn.set_component_justice_disposition_profile(agent_id, profile.clone())?;
    }
    if let Some(ref profile) = agent_def.violation_disposition {
        txn.set_component_violation_disposition_profile(agent_id, profile.clone())?;
    }
    if let Some(ref profile) = agent_def.patrol_profile {
        txn.set_component_patrol_profile(agent_id, profile.clone())?;
    }
    if let Some(ref route_def) = agent_def.patrol_route {
        let assigned_places = route_def
            .assigned_places
            .iter()
            .map(|name| {
                resolve_name(
                    names,
                    name,
                    &format!("agent '{}' patrol route", agent_def.name),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        txn.set_component_patrol_route(
            agent_id,
            PatrolRoute {
                assigned_places,
                current_index: 0,
            },
        )?;
    }
    if let Some(ref profile) = agent_def.pursuit_profile {
        txn.set_component_pursuit_profile(agent_id, profile.clone())?;
    }
    if let Some(ref profile) = agent_def.contention_disposition {
        txn.set_component_contention_disposition_profile(agent_id, profile.clone())?;
    }
    if let Some(ref profile) = agent_def.commodity_valuation {
        txn.set_component_commodity_valuation_profile(agent_id, *profile)?;
    }
    if let Some(ref preferences) = agent_def.substitute_preferences {
        txn.set_component_substitute_preferences(agent_id, preferences.clone())?;
    }
    if let Some(recipe_names) = &agent_def.known_recipes {
        let recipe_ids = recipe_names
            .iter()
            .filter_map(|name| recipes.recipe_by_name(name).map(|(id, _)| id))
            .collect::<Vec<_>>();
        if !recipe_ids.is_empty() {
            txn.set_component_known_recipes(agent_id, KnownRecipes::with(recipe_ids))?;
        }
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
    mut event_log: EventLog,
    recipe_registry: RecipeRegistry,
) -> Result<SimulationState, ScenarioError> {
    event_log.set_compaction_interval(def.compaction_interval);
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
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::topology::PlaceTag;
    use worldwake_core::{
        ArtifactPostingProfile, BeliefConfidencePolicy, CarryCapacity, CognitiveProfile,
        CommodityDecayMap, CommodityKind, CommodityValuationProfile, CommunicationProfile,
        ContentionDispositionProfile, ControlSource, DisposalProfile, DiversificationProfile,
        DriveEscalationParams, DriveEscalationProfile, DriveThresholds,
        EpistemicDispositionProfile, ExecutionBudget, ExpectationStore, HomeostaticNeedId,
        HomeostaticNeeds, IntentionDispositionProfile, JusticeDispositionProfile,
        LastProactiveExplorationTick, LastSeenMemory, LoadUnits, MultiplierPermille,
        ObligationSatiationProfile, PatrolProfile, PatrolRoute, PerceptionProfile, Permille,
        PlaceVisibilityProfile, PreferenceProfile, PursuitProfile, Quantity, SubstitutePreferences,
        TellProfile, TheftDispositionProfile, ThresholdBand, TradeCategory,
        ViolationDispositionProfile, WorkstationTag, default_commodity_decay_map,
    };

    fn minimal_agent(name: &str, location: &str, control: ControlSource) -> AgentDef {
        AgentDef {
            name: name.into(),
            location: location.into(),
            control,
            needs: None,
            combat_profile: None,
            utility_profile: None,
            artifact_posting_profile: None,
            merchandise_profile: None,
            trade_disposition: None,
            perception_profile: None,
            tell_profile: None,
            cognitive_profile: None,
            execution_budget: None,
            epistemic_disposition: None,
            intention_disposition: None,
            communication_profile: None,
            preference_profile: None,
            expectation_store: None,
            last_seen_memory: None,
            obligation_satiation_profile: None,
            drive_thresholds: None,
            drive_escalation_profile: None,
            metabolism_profile: None,
            disposal_profile: None,
            exploration_profile: None,
            diversification_profile: None,
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
            known_recipes: None,
        }
    }

    /// Helper: build a minimal `ScenarioDef` with given places and agents.
    fn minimal_def() -> ScenarioDef {
        ScenarioDef {
            seed: 42,
            places: vec![PlaceDef {
                name: "Village".into(),
                tags: vec![PlaceTag::Village],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![minimal_agent("Alice", "Village", ControlSource::Human)],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
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
    fn test_spawn_minimal_scenario_uses_default_commodity_decay() {
        let spawned = spawn_scenario(&minimal_def()).unwrap();

        assert_eq!(
            spawned.state.world().commodity_decay(),
            &default_commodity_decay_map()
        );
    }

    #[test]
    fn test_spawn_scenario_applies_explicit_commodity_decay_override() {
        let mut def = minimal_def();
        def.commodity_decay = Some(CommodityDecayMap::from([(
            CommodityKind::Waste,
            NonZeroU32::new(17).unwrap(),
        )]));

        let spawned = spawn_scenario(&def).unwrap();

        assert_eq!(
            spawned.state.world().commodity_decay(),
            &CommodityDecayMap::from([(CommodityKind::Waste, NonZeroU32::new(17).unwrap())])
        );
    }

    #[test]
    fn test_spawn_agents_at_places() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![
                PlaceDef {
                    name: "Town".into(),
                    tags: vec![],
                    visibility_profile: None,
                },
                PlaceDef {
                    name: "Forest".into(),
                    tags: vec![],
                    visibility_profile: None,
                },
            ],
            edges: vec![],
            agents: vec![
                minimal_agent("Alice", "Town", ControlSource::Human),
                minimal_agent("Bob", "Forest", ControlSource::Ai),
            ],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
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
    fn test_spawn_agents_receive_default_carry_capacity() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Town".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![minimal_agent("Alice", "Town", ControlSource::Ai)],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let agent = world
            .entities_with_name_and_agent_data()
            .next()
            .expect("spawned scenario should contain one agent");

        assert_eq!(
            world.get_component_carry_capacity(agent),
            Some(&CarryCapacity(LoadUnits(20)))
        );
    }

    #[test]
    fn test_spawn_agents_keep_default_disposal_profile_when_override_absent() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Town".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![minimal_agent("Alice", "Town", ControlSource::Ai)],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let agent = world
            .entities_with_name_and_agent_data()
            .next()
            .expect("spawned scenario should contain one agent");

        assert_eq!(
            world.get_component_disposal_profile(agent),
            Some(&DisposalProfile::default())
        );
    }

    #[test]
    fn test_spawn_agents_apply_disposal_profile_override_when_present() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Town".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![AgentDef {
                disposal_profile: Some(DisposalProfile {
                    capacity_strain_threshold: Permille::new(950).unwrap(),
                }),
                ..minimal_agent("Alice", "Town", ControlSource::Ai)
            }],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let agent = world
            .entities_with_name_and_agent_data()
            .next()
            .expect("spawned scenario should contain one agent");

        assert_eq!(
            world.get_component_disposal_profile(agent),
            Some(&DisposalProfile {
                capacity_strain_threshold: Permille::new(950).unwrap(),
            })
        );
    }

    #[test]
    fn test_spawn_applies_place_visibility_profile_when_present() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Forest".into(),
                tags: vec![PlaceTag::Forest],
                visibility_profile: Some(PlaceVisibilityProfile {
                    base_concealment: Permille::new(400).unwrap(),
                }),
            }],
            edges: vec![],
            agents: vec![minimal_agent("Scout", "Forest", ControlSource::Ai)],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let forest = EntityId {
            slot: 0,
            generation: 0,
        };

        assert_eq!(
            world.get_component_place_visibility_profile(forest),
            Some(&PlaceVisibilityProfile {
                base_concealment: Permille::new(400).unwrap(),
            })
        );
    }

    #[test]
    fn test_spawn_leaves_place_visibility_profile_absent_by_default() {
        let spawned = spawn_scenario(&minimal_def()).unwrap();
        let world = spawned.state.world();
        let village = EntityId {
            slot: 0,
            generation: 0,
        };

        assert_eq!(world.get_component_place_visibility_profile(village), None);
    }

    #[test]
    fn test_spawn_items_at_place() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Market".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![minimal_agent("Trader", "Market", ControlSource::Ai)],
            items: vec![ItemDef {
                commodity: CommodityKind::Apple,
                quantity: Quantity(10),
                location: "Market".into(),
                container: false,
            }],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
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
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![minimal_agent("Warrior", "Camp", ControlSource::Human)],
            items: vec![ItemDef {
                commodity: CommodityKind::Sword,
                quantity: Quantity(1),
                location: "Warrior".into(),
                container: false,
            }],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
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
                    visibility_profile: None,
                },
                PlaceDef {
                    name: "B".into(),
                    tags: vec![],
                    visibility_profile: None,
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
            commodity_decay: None,
            compaction_interval: 0,
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
                    visibility_profile: None,
                },
                PlaceDef {
                    name: "Y".into(),
                    tags: vec![],
                    visibility_profile: None,
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
            commodity_decay: None,
            compaction_interval: 0,
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
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![minimal_agent("Lost", "Nowhere", ControlSource::Ai)],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
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
                    visibility_profile: None,
                },
                PlaceDef {
                    name: "Orchard".into(),
                    tags: vec![],
                    visibility_profile: None,
                },
            ],
            edges: vec![],
            agents: vec![],
            items: vec![],
            facilities: vec![FacilityDef {
                name: Some("Forge Bench".into()),
                workstation: WorkstationTag::Forge,
                location: "Smithy".into(),
            }],
            resource_sources: vec![ResourceSourceDef {
                commodity: CommodityKind::Apple,
                location: "Orchard".into(),
                facility: None,
                regeneration_ticks_per_unit: NonZeroU32::new(5),
                capacity: Quantity(20),
            }],
            commodity_decay: None,
            compaction_interval: 0,
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
    fn test_spawn_agent_resolves_known_recipe_names() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Orchard".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![AgentDef {
                known_recipes: Some(vec!["Harvest Apples".into(), "Unknown Recipe".into()]),
                ..minimal_agent("Forager", "Orchard", ControlSource::Ai)
            }],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let harvest_apples = spawned
            .state
            .recipe_registry()
            .recipe_by_name("Harvest Apples")
            .map(|(id, _)| id)
            .expect("canonical scenario recipe registry should include Harvest Apples");

        let agent = world
            .entities_with_name_and_agent_data()
            .next()
            .expect("scenario should spawn one agent");
        let known = world
            .get_component_known_recipes(agent)
            .expect("agent should receive known recipes from scenario");

        assert!(known.recipes.contains(&harvest_apples));
        assert_eq!(known.recipes.len(), 1);
        assert!(
            spawned
                .action_registries
                .defs
                .iter()
                .any(|def| def.name == "harvest:Harvest Apples"),
            "scenario action registries should include recipe-backed harvest actions"
        );
    }

    #[test]
    fn test_spawn_named_resource_source_attaches_to_facility() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Orchard".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![],
            items: vec![],
            facilities: vec![FacilityDef {
                name: Some("North Orchard".into()),
                workstation: WorkstationTag::OrchardRow,
                location: "Orchard".into(),
            }],
            resource_sources: vec![ResourceSourceDef {
                commodity: CommodityKind::Apple,
                location: "Orchard".into(),
                facility: Some("North Orchard".into()),
                regeneration_ticks_per_unit: NonZeroU32::new(2),
                capacity: Quantity(20),
            }],
            commodity_decay: None,
            compaction_interval: 0,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let orchard = EntityId {
            slot: 0,
            generation: 0,
        };

        let facilities_at_orchard = world
            .entities_effectively_at(orchard)
            .into_iter()
            .filter(|entity| world.get_component_workstation_marker(*entity).is_some())
            .collect::<Vec<_>>();

        assert_eq!(
            facilities_at_orchard.len(),
            1,
            "named source attachment should reuse the authored facility"
        );
        let facility = facilities_at_orchard[0];
        assert!(world.get_component_resource_source(facility).is_some());
        assert_eq!(
            world.get_component_workstation_marker(facility).unwrap().0,
            WorkstationTag::OrchardRow
        );
    }

    #[test]
    fn test_spawn_water_source_attaches_to_well_and_registers_harvest_water() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Village".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![AgentDef {
                name: "Water Bearer".into(),
                location: "Village".into(),
                control: ControlSource::Ai,
                known_recipes: Some(vec!["Harvest Water".into()]),
                ..minimal_agent("Water Bearer", "Village", ControlSource::Ai)
            }],
            items: vec![],
            facilities: vec![FacilityDef {
                name: Some("Village Well".into()),
                workstation: WorkstationTag::Well,
                location: "Village".into(),
            }],
            resource_sources: vec![ResourceSourceDef {
                commodity: CommodityKind::Water,
                location: "Village".into(),
                facility: Some("Village Well".into()),
                regeneration_ticks_per_unit: NonZeroU32::new(3),
                capacity: Quantity(15),
            }],
            commodity_decay: None,
            compaction_interval: 0,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let harvest_water = spawned
            .state
            .recipe_registry()
            .recipe_by_name("Harvest Water")
            .map(|(id, _)| id)
            .expect("canonical scenario recipe registry should include Harvest Water");

        let agent = world
            .entities_with_name_and_agent_data()
            .next()
            .expect("scenario should spawn one agent");
        let known = world
            .get_component_known_recipes(agent)
            .expect("agent should receive harvest water recipe from scenario");
        assert!(known.recipes.contains(&harvest_water));

        let village = EntityId {
            slot: 0,
            generation: 0,
        };
        let facilities_at_village = world
            .entities_effectively_at(village)
            .into_iter()
            .filter(|entity| world.get_component_workstation_marker(*entity).is_some())
            .collect::<Vec<_>>();
        assert_eq!(facilities_at_village.len(), 1);
        let well = facilities_at_village[0];
        assert_eq!(
            world.get_component_workstation_marker(well).unwrap().0,
            WorkstationTag::Well
        );
        assert_eq!(
            world
                .get_component_resource_source(well)
                .expect("well should carry attached resource source")
                .commodity,
            CommodityKind::Water
        );
        assert!(
            spawned
                .action_registries
                .defs
                .iter()
                .any(|def| def.name == "harvest:Harvest Water"),
            "scenario action registries should include recipe-backed water harvest actions"
        );
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
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![minimal_agent("Bot", "Void", ControlSource::Ai)],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
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
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![AgentDef {
                needs: Some(custom_needs),
                ..minimal_agent("Hungry", "Home", ControlSource::Ai)
            }],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();

        let agents: Vec<_> = world.entities_with_name_and_agent_data().collect();
        let needs = world.get_component_homeostatic_needs(agents[0]).unwrap();
        assert_eq!(needs.hunger, Permille::new(100).unwrap());
        assert_eq!(needs.thirst, Permille::new(200).unwrap());
        assert_eq!(needs.fatigue, Permille::new(50).unwrap());
    }

    #[test]
    fn test_spawn_agents_receive_default_universal_profiles() {
        let spawned = spawn_scenario(&minimal_def()).unwrap();
        let world = spawned.state.world();
        let agent = world
            .entities_with_name_and_agent_data()
            .next()
            .expect("spawned scenario should contain one agent");

        assert_eq!(
            world.get_component_perception_profile(agent),
            Some(&PerceptionProfile::default())
        );
        assert_eq!(
            world.get_component_tell_profile(agent),
            Some(&TellProfile::default())
        );
        assert_eq!(
            world.get_component_cognitive_profile(agent),
            Some(&CognitiveProfile::default())
        );
        assert_eq!(
            world.get_component_execution_budget(agent),
            Some(&ExecutionBudget::default())
        );
        assert_eq!(
            world.get_component_drive_escalation_profile(agent),
            Some(&DriveEscalationProfile::default())
        );
        assert_eq!(
            world.get_component_epistemic_disposition_profile(agent),
            Some(&EpistemicDispositionProfile::default())
        );
        assert_eq!(
            world.get_component_intention_disposition_profile(agent),
            Some(&IntentionDispositionProfile::default())
        );
        assert_eq!(
            world.get_component_communication_profile(agent),
            Some(&CommunicationProfile::default())
        );
        assert_eq!(
            world.get_component_preference_profile(agent),
            Some(&PreferenceProfile::default())
        );
        assert_eq!(
            world.get_component_expectation_store(agent),
            Some(&ExpectationStore::default())
        );
        assert_eq!(
            world.get_component_exploration_profile(agent),
            Some(&worldwake_core::ExplorationProfile::default())
        );
        assert_eq!(
            world.get_component_last_seen_memory(agent),
            Some(&LastSeenMemory::default())
        );
        assert_eq!(
            world.get_component_obligation_satiation_profile(agent),
            Some(&ObligationSatiationProfile::default())
        );
        assert_eq!(
            world.get_component_artifact_posting_profile(agent),
            Some(&ArtifactPostingProfile::default())
        );
        assert_eq!(world.get_component_diversification_profile(agent), None);
        assert_eq!(
            world.get_component_last_proactive_exploration_tick(agent),
            None
        );
    }

    #[test]
    fn test_spawn_agent_with_diversification_profile_sets_runtime_components() {
        let profile = DiversificationProfile {
            base_curiosity: Permille::new(400).unwrap(),
            comfort_threshold: Permille::new(450).unwrap(),
            curiosity_buildup_rate: Permille::new(5).unwrap(),
            exploration_cooldown_ticks: 60,
            familiarity_per_visit: Permille::new(150).unwrap(),
            familiarity_recovery_per_tick: Permille::new(2).unwrap(),
            familiarity_floor: Permille::new(50).unwrap(),
            max_exploration_hops: 3,
        };
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Town".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![AgentDef {
                diversification_profile: Some(profile),
                ..minimal_agent("Scout", "Town", ControlSource::Ai)
            }],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let agent = world
            .entities_with_name_and_agent_data()
            .next()
            .expect("spawned scenario should contain one agent");

        assert_eq!(
            world.get_component_diversification_profile(agent),
            Some(&profile)
        );
        assert_eq!(
            world.get_component_last_proactive_exploration_tick(agent),
            Some(&LastProactiveExplorationTick(None))
        );
    }

    #[test]
    fn test_spawn_agent_with_last_seen_memory_override() {
        let custom_memory = LastSeenMemory {
            records: BTreeMap::new(),
            capacity: 50,
        };
        let custom_expectation_store = ExpectationStore::default();
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Home".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![AgentDef {
                expectation_store: Some(custom_expectation_store.clone()),
                last_seen_memory: Some(custom_memory.clone()),
                ..minimal_agent("Searcher", "Home", ControlSource::Ai)
            }],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let agent = world
            .entities_with_name_and_agent_data()
            .next()
            .expect("spawned scenario should contain one agent");

        assert_eq!(
            world.get_component_expectation_store(agent),
            Some(&custom_expectation_store)
        );
        assert_eq!(
            world.get_component_last_seen_memory(agent),
            Some(&custom_memory)
        );
    }

    #[test]
    fn test_spawn_agent_with_artifact_posting_profile_override() {
        let custom_profile = ArtifactPostingProfile {
            threat_warning_ttl: 12,
            office_vacancy_ttl: 34,
            bounty_ttl: 56,
        };
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Home".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![AgentDef {
                artifact_posting_profile: Some(custom_profile.clone()),
                ..minimal_agent("Herald", "Home", ControlSource::Ai)
            }],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let agent = world
            .entities_with_name_and_agent_data()
            .next()
            .expect("spawned scenario should contain one agent");

        assert_eq!(
            world.get_component_artifact_posting_profile(agent),
            Some(&custom_profile)
        );
    }

    #[test]
    fn test_spawn_agent_with_profile_overrides() {
        let custom_perception = PerceptionProfile {
            entity_activation_threshold: Permille::new(250).unwrap(),
            claim_confidence_threshold: Permille::new(50).unwrap(),
            observation_buffer_capacity: 4,
            observation_budget: 7,
            need_salience_boost: Permille::new(500).unwrap(),
            need_salience_urgency_threshold: Permille::new(500).unwrap(),
            observation_fidelity: Permille::new(900).unwrap(),
            confidence_policy: BeliefConfidencePolicy::default(),
            institutional_memory_capacity: 9,
            consultation_speed_factor: Permille::new(650).unwrap(),
            contradiction_tolerance: Permille::new(125).unwrap(),
        };
        let custom_exploration = ExplorationProfileDef {
            curiosity_weight: Permille::new(275).unwrap(),
            need_activation_threshold: Permille::new(350).unwrap(),
            frontier_depth: 4,
            acquisition_failure_threshold: 6,
            exploration_arrival_boost: Permille::new(650).unwrap(),
            max_consecutive_explorations: 5,
            visit_lookback_ticks: 17,
        };
        let custom_thresholds = DriveThresholds::new(
            ThresholdBand::new(
                Permille::new(150).unwrap(),
                Permille::new(300).unwrap(),
                Permille::new(600).unwrap(),
                Permille::new(850).unwrap(),
            )
            .unwrap(),
            ThresholdBand::new(
                Permille::new(160).unwrap(),
                Permille::new(320).unwrap(),
                Permille::new(610).unwrap(),
                Permille::new(860).unwrap(),
            )
            .unwrap(),
            ThresholdBand::new(
                Permille::new(170).unwrap(),
                Permille::new(340).unwrap(),
                Permille::new(620).unwrap(),
                Permille::new(870).unwrap(),
            )
            .unwrap(),
            ThresholdBand::new(
                Permille::new(180).unwrap(),
                Permille::new(360).unwrap(),
                Permille::new(630).unwrap(),
                Permille::new(880).unwrap(),
            )
            .unwrap(),
            ThresholdBand::new(
                Permille::new(190).unwrap(),
                Permille::new(380).unwrap(),
                Permille::new(640).unwrap(),
                Permille::new(890).unwrap(),
            )
            .unwrap(),
            ThresholdBand::new(
                Permille::new(120).unwrap(),
                Permille::new(240).unwrap(),
                Permille::new(520).unwrap(),
                Permille::new(800).unwrap(),
            )
            .unwrap(),
            ThresholdBand::new(
                Permille::new(80).unwrap(),
                Permille::new(220).unwrap(),
                Permille::new(480).unwrap(),
                Permille::new(760).unwrap(),
            )
            .unwrap(),
        );

        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Town".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![AgentDef {
                perception_profile: Some(custom_perception),
                drive_thresholds: Some(custom_thresholds),
                exploration_profile: Some(custom_exploration),
                ..minimal_agent("Alice", "Town", ControlSource::Ai)
            }],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let agent = world
            .entities_with_name_and_agent_data()
            .next()
            .expect("spawned scenario should contain one agent");

        assert_eq!(
            world.get_component_perception_profile(agent),
            Some(&custom_perception)
        );
        assert_eq!(custom_perception.observation_budget, 7);
        assert_eq!(
            world.get_component_drive_thresholds(agent),
            Some(&custom_thresholds)
        );
        assert_eq!(
            world.get_component_exploration_profile(agent),
            Some(&worldwake_core::ExplorationProfile {
                curiosity_weight: Permille::new(275).unwrap(),
                need_activation_threshold: Permille::new(350).unwrap(),
                frontier_depth: 4,
                acquisition_failure_threshold: 6,
                exploration_arrival_boost: Permille::new(650).unwrap(),
                max_consecutive_explorations: 5,
                visit_lookback_ticks: 17,
                consecutive_exploration_count: 0,
            })
        );
    }

    #[test]
    fn test_spawn_agents_apply_obligation_satiation_profile_override_when_present() {
        let custom_profile = ObligationSatiationProfile {
            satiation_threshold: 4,
            window_ticks: 96,
            decay_per_execution: Permille::new(150).unwrap(),
            satiation_floor: Permille::new(125).unwrap(),
        };
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Town".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![AgentDef {
                obligation_satiation_profile: Some(custom_profile.clone()),
                ..minimal_agent("Alice", "Town", ControlSource::Ai)
            }],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let agent = world
            .entities_with_name_and_agent_data()
            .next()
            .expect("spawned scenario should contain one agent");

        assert_eq!(
            world.get_component_obligation_satiation_profile(agent),
            Some(&custom_profile)
        );
    }

    #[test]
    fn test_spawn_agents_apply_drive_escalation_profile_override_when_present() {
        let custom_profile = DriveEscalationProfile {
            per_need: BTreeMap::from([(
                HomeostaticNeedId::Dirtiness,
                DriveEscalationParams {
                    start_after_ticks: 40,
                    growth_per_tick: Permille::new(25).unwrap(),
                    max_multiplier: MultiplierPermille::new(2200).unwrap(),
                },
            )]),
            default_per_need: DriveEscalationParams {
                start_after_ticks: 80,
                growth_per_tick: Permille::new(15).unwrap(),
                max_multiplier: MultiplierPermille::new(1800).unwrap(),
            },
        };
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Town".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![AgentDef {
                drive_escalation_profile: Some(custom_profile.clone()),
                ..minimal_agent("Alice", "Town", ControlSource::Ai)
            }],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let agent = world
            .entities_with_name_and_agent_data()
            .next()
            .expect("spawned scenario should contain one agent");

        assert_eq!(
            world.get_component_drive_escalation_profile(agent),
            Some(&custom_profile)
        );
    }

    #[test]
    fn test_spawn_agent_applies_role_specific_profiles_and_patrol_route() {
        let theft_profile = TheftDispositionProfile {
            steal_duration_ticks: NonZeroU32::new(6).unwrap(),
            theft_motive_weight: Permille::new(620).unwrap(),
            witness_risk_penalty: Permille::new(180).unwrap(),
        };
        let justice_profile = JusticeDispositionProfile {
            accusation_motive_weight: Permille::new(700).unwrap(),
            fine_severity: Permille::new(450).unwrap(),
        };
        let violation_profile = ViolationDispositionProfile {
            investigation_duration_ticks: NonZeroU32::new(5).unwrap(),
            violation_memory_retention_ticks: 120,
            investigation_motive_weight: Permille::new(510).unwrap(),
            ownership_motive_bonus: Permille::new(280).unwrap(),
        };
        let patrol_profile = PatrolProfile {
            base_dwell_ticks: 12,
            dwell_vigilance_scale_ticks: 10,
            vigilance: Permille::new(700).unwrap(),
            route_adaptation_sensitivity: Permille::new(450).unwrap(),
            patrol_motive_weight: Permille::new(550).unwrap(),
        };
        let pursuit_profile = PursuitProfile {
            min_location_confidence: Permille::new(600).unwrap(),
            max_pursuit_travel_ticks: NonZeroU32::new(10).unwrap(),
        };
        let queue_profile = ContentionDispositionProfile {
            queue_patience_ticks: Some(NonZeroU32::new(8).unwrap()),
        };
        let valuation_profile = CommodityValuationProfile {
            recipe_opportunity_depth: std::num::NonZeroU8::new(2).unwrap(),
            recipe_place_horizon: 4,
            indirect_value_decay_per_step: Permille::new(140).unwrap(),
        };
        let substitute_preferences = SubstitutePreferences {
            preferences: BTreeMap::from([(
                TradeCategory::Food,
                vec![CommodityKind::Bread, CommodityKind::Apple],
            )]),
        };

        let def = ScenarioDef {
            seed: 7,
            places: vec![
                PlaceDef {
                    name: "Gate".into(),
                    tags: vec![],
                    visibility_profile: None,
                },
                PlaceDef {
                    name: "Market".into(),
                    tags: vec![],
                    visibility_profile: None,
                },
            ],
            edges: vec![],
            agents: vec![AgentDef {
                theft_disposition: Some(theft_profile.clone()),
                justice_disposition: Some(justice_profile.clone()),
                violation_disposition: Some(violation_profile.clone()),
                patrol_profile: Some(patrol_profile.clone()),
                patrol_route: Some(PatrolRouteDef {
                    assigned_places: vec!["Gate".into(), "Market".into()],
                }),
                pursuit_profile: Some(pursuit_profile.clone()),
                contention_disposition: Some(queue_profile.clone()),
                commodity_valuation: Some(valuation_profile),
                substitute_preferences: Some(substitute_preferences.clone()),
                ..minimal_agent("Guard", "Gate", ControlSource::Ai)
            }],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let agent = world
            .entities_with_name_and_agent_data()
            .next()
            .expect("spawned scenario should contain one agent");
        let gate = world
            .topology()
            .place_ids()
            .find(|&id| world.topology().place(id).unwrap().name == "Gate")
            .unwrap();
        let market = world
            .topology()
            .place_ids()
            .find(|&id| world.topology().place(id).unwrap().name == "Market")
            .unwrap();

        assert_eq!(
            world.get_component_theft_disposition_profile(agent),
            Some(&theft_profile)
        );
        assert_eq!(
            world.get_component_justice_disposition_profile(agent),
            Some(&justice_profile)
        );
        assert_eq!(
            world.get_component_violation_disposition_profile(agent),
            Some(&violation_profile)
        );
        assert_eq!(
            world.get_component_patrol_profile(agent),
            Some(&patrol_profile)
        );
        assert_eq!(
            world.get_component_patrol_route(agent),
            Some(&PatrolRoute {
                assigned_places: vec![gate, market],
                current_index: 0,
            })
        );
        assert_eq!(
            world.get_component_pursuit_profile(agent),
            Some(&pursuit_profile)
        );
        assert_eq!(
            world.get_component_contention_disposition_profile(agent),
            Some(&queue_profile)
        );
        assert_eq!(
            world.get_component_commodity_valuation_profile(agent),
            Some(&valuation_profile)
        );
        assert_eq!(
            world.get_component_substitute_preferences(agent),
            Some(&substitute_preferences)
        );
    }

    #[test]
    fn test_spawn_agent_leaves_role_specific_profiles_absent_by_default() {
        let spawned = spawn_scenario(&minimal_def()).unwrap();
        let world = spawned.state.world();
        let agent = world
            .entities_with_name_and_agent_data()
            .next()
            .expect("spawned scenario should contain one agent");

        assert_eq!(world.get_component_theft_disposition_profile(agent), None);
        assert_eq!(world.get_component_justice_disposition_profile(agent), None);
        assert_eq!(
            world.get_component_violation_disposition_profile(agent),
            None
        );
        assert_eq!(world.get_component_patrol_profile(agent), None);
        assert_eq!(world.get_component_patrol_route(agent), None);
        assert_eq!(world.get_component_pursuit_profile(agent), None);
        assert_eq!(
            world.get_component_contention_disposition_profile(agent),
            None
        );
        assert_eq!(world.get_component_commodity_valuation_profile(agent), None);
        assert_eq!(world.get_component_substitute_preferences(agent), None);
    }

    #[test]
    fn test_spawn_agent_rejects_invalid_patrol_route_place() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Gate".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![AgentDef {
                patrol_route: Some(PatrolRouteDef {
                    assigned_places: vec!["Gate".into(), "Nowhere".into()],
                }),
                ..minimal_agent("Guard", "Gate", ControlSource::Ai)
            }],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            commodity_decay: None,
            compaction_interval: 0,
        };

        let Err(error) = spawn_scenario(&def) else {
            panic!("invalid patrol route should fail");
        };
        assert_eq!(
            error.to_string(),
            "validation error: agent 'Guard' patrol route references nonexistent entity 'Nowhere'"
        );
    }
}
