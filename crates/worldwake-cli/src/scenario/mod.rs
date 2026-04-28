//! Scenario system — RON-based world initialization.
//!
//! `types` defines the deserialization schema (`ScenarioDef` and sub-structs).
//! `spawn_scenario()` builds a fully initialized simulation from a `ScenarioDef`.

pub mod lints;
pub mod types;

use std::collections::{BTreeMap, BTreeSet};
use std::num::{NonZeroU8, NonZeroU32};
use std::path::Path;

use types::ScenarioDef;
use worldwake_core::{
    ArtifactHeader, ArtifactKind, ArtifactState, BanditCamp, BanditFactionPolicy,
    BelievedInstitutionalClaim, CarryCapacity, CauseRef, Container, ContentionQueue, ControlSource,
    DeprivationExposure, EligibilityRule, EntityId, EntityKind, EventLog, ExpectationBasis,
    ExpectationOutcome, ExpectationRecord, ExpectationState, ExpectationStore, ExplorationProfile,
    InstitutionalBeliefKey, InstitutionalClaim, InstitutionalKnowledgeSource, KnownRecipes,
    LastProactiveExplorationTick, LastSeenMemory, LastSeenProvenance, LastSeenRecord, LoadUnits,
    MerchandiseProfile, Name, NoticeContent, NoticeTopic, OfficeData, OfficeForceProfile,
    OfficeForceState, PatrolRoute, Place, ProductionOutputOwner, ProductionOutputOwnershipPolicy,
    RecordData, RecordKind, ResourceExtractionQueues, ResourceSource, Seed, SocialObservation,
    SocialObservationDetail, Tick, Topology, TravelEdge, TravelEdgeId, VisibilitySpec, WitnessData,
    WorkstationMarker, World, WorldTxn, default_commodity_decay_map, hash_world, load_per_unit,
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
    LintFailure(lints::LintReport),
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
            Self::LintFailure(report) => {
                for failure in &report.failures {
                    writeln!(
                        f,
                        "lint failure: {:?} [{}] {}",
                        failure.rule,
                        failure.affected_agents.join(", "),
                        failure.detail
                    )?;
                }
                Ok(())
            }
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
    let report = lints::run_lints(def);
    let report = lints::filter_overrides(report, &def.scenario_lint_overrides)?;
    if !report.failures.is_empty() {
        return Err(ScenarioError::LintFailure(report));
    }
    spawn_scenario_inner(def)
}

pub fn spawn_scenario_ignoring_lints(
    def: &ScenarioDef,
) -> Result<SpawnedSimulation, ScenarioError> {
    let report = lints::run_lints(def);
    let _report = lints::filter_overrides(report, &def.scenario_lint_overrides)?;
    spawn_scenario_inner(def)
}

fn spawn_scenario_inner(def: &ScenarioDef) -> Result<SpawnedSimulation, ScenarioError> {
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
    world.set_harvest_trace_retention_ticks(
        def.harvest_trace_retention_ticks
            .unwrap_or(worldwake_core::HARVEST_TRACE_RETENTION_TICKS),
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

    for camp_def in &def.bandit_camps {
        spawn_bandit_camp(&mut txn, camp_def, names)?;
    }

    for office_def in &def.offices {
        spawn_office(&mut txn, office_def, names, &mut facility_locations)?;
    }

    for notice_def in &def.notices {
        spawn_notice(&mut txn, notice_def, names)?;
    }

    apply_agent_expectation_stores(&mut txn, def, names)?;

    for item_def in &def.items {
        spawn_item(&mut txn, item_def, names, place_names, &agent_locations)?;
    }

    for facility_def in &def.facilities {
        let place_id = resolve_name(
            names,
            &facility_def.location,
            &format!("facility {:?} location", facility_def.workstation),
        )?;
        let facility_id = if let Some(storage) = &facility_def.merchant_storage {
            let owner_id = resolve_name(
                names,
                &storage.owner,
                &format!(
                    "facility {:?} merchant_storage owner",
                    facility_def.workstation
                ),
            )?;
            let (facility_id, _, _) = txn.create_merchant_facility(
                place_id,
                owner_id,
                storage.stock_capacity,
                storage.display_capacity,
            )?;
            txn.set_component_workstation_marker(
                facility_id,
                WorkstationMarker(facility_def.workstation),
            )?;
            facility_id
        } else {
            let facility_id = txn.create_entity(EntityKind::Facility);
            txn.set_component_workstation_marker(
                facility_id,
                WorkstationMarker(facility_def.workstation),
            )?;
            txn.set_ground_location(facility_id, place_id)?;
            facility_id
        };
        txn.set_component_production_output_ownership_policy(
            facility_id,
            ProductionOutputOwnershipPolicy {
                output_owner: ProductionOutputOwner::Actor,
            },
        )?;
        if let Some(policy) = &facility_def.contention_policy {
            txn.set_component_contention_policy(facility_id, policy.clone())?;
            txn.set_component_contention_queue(facility_id, ContentionQueue::default())?;
        }
        if let Some(name) = &facility_def.name {
            txn.set_component_name(facility_id, worldwake_core::Name(name.clone()))?;
        }
        facility_locations.insert(facility_id, place_id);
        if let Some(name) = &facility_def.name {
            names.insert(name.clone(), facility_id);
        }
    }

    for agent_def in &def.agents {
        let Some(merch_def) = &agent_def.merchandise_profile else {
            continue;
        };
        let Some(home_facility_name) = &merch_def.home_facility else {
            continue;
        };
        let agent_id = resolve_name(names, &agent_def.name, "agent merchandise owner")?;
        let home_facility = resolve_name(
            names,
            home_facility_name,
            &format!("agent '{}' merchandise home_facility", agent_def.name),
        )?;
        let sale_kinds = merch_def.sale_kinds.iter().copied().collect();
        txn.set_component_merchandise_profile(
            agent_id,
            MerchandiseProfile {
                sale_kinds,
                home_facility: Some(home_facility),
            },
        )?;
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
        let extraction_slots =
            NonZeroU8::new(source_def.extraction_slots).unwrap_or(NonZeroU8::MIN);
        txn.set_component_resource_source(
            source_id,
            ResourceSource {
                commodity: source_def.commodity,
                available_quantity: source_def.capacity,
                max_quantity: source_def.capacity,
                regeneration_ticks_per_unit: source_def.regeneration_ticks_per_unit,
                last_regeneration_tick: None,
                extraction_slots,
                extraction_duration_ticks: NonZeroU32::new(source_def.extraction_duration_ticks)
                    .unwrap_or(NonZeroU32::MIN),
            },
        )?;
        txn.set_component_resource_extraction_queues(
            source_id,
            ResourceExtractionQueues {
                queues: vec![ContentionQueue::default(); extraction_slots.get() as usize],
            },
        )?;
    }

    for hostility_def in &def.hostilities {
        let subject = resolve_name(names, &hostility_def.subject, "hostility subject")?;
        let target = resolve_name(names, &hostility_def.target, "hostility target")?;
        txn.add_hostility(subject, target)?;
    }

    txn.commit(event_log);
    Ok(())
}

fn spawn_bandit_camp(
    txn: &mut WorldTxn<'_>,
    camp_def: &types::BanditCampDef,
    names: &mut BTreeMap<String, EntityId>,
) -> Result<(), ScenarioError> {
    let place = resolve_name(
        names,
        &camp_def.place,
        &format!("bandit camp '{}' place", camp_def.faction),
    )?;
    let faction = txn.create_faction(&camp_def.faction)?;
    names.insert(camp_def.faction.clone(), faction);

    for member_name in &camp_def.members {
        let member = resolve_name(
            names,
            member_name,
            &format!("bandit camp '{}' member", camp_def.faction),
        )?;
        txn.add_member(member, faction)?;
    }

    let rally_place = camp_def
        .policy
        .rally_place
        .as_ref()
        .map(|name| {
            resolve_name(
                names,
                name,
                &format!("bandit camp '{}' rally_place", camp_def.faction),
            )
        })
        .transpose()?;
    txn.set_component_bandit_faction_policy(
        faction,
        BanditFactionPolicy {
            min_regroup_count: camp_def.policy.min_regroup_count,
            establishment_duration_ticks: camp_def.policy.establishment_duration_ticks,
            abandonment_grace_ticks: camp_def.policy.abandonment_grace_ticks,
            flee_wound_threshold: camp_def.policy.flee_wound_threshold,
            rally_place,
        },
    )?;

    let supplies = if let Some(supplies_def) = &camp_def.supplies {
        let container = txn.create_container(supplies_def.container.clone())?;
        txn.set_ground_location(container, place)?;
        txn.set_owner(container, faction)?;
        let lot = txn.create_item_lot(supplies_def.commodity, supplies_def.quantity)?;
        txn.set_ground_location(lot, place)?;
        txn.set_owner(lot, faction)?;
        txn.put_into_container(lot, container)?;
        container
    } else {
        let container = txn.create_container(worldwake_core::Container {
            capacity: LoadUnits(1),
            allowed_commodities: None,
            allows_unique_items: false,
            allows_nested_containers: false,
        })?;
        txn.set_ground_location(container, place)?;
        txn.set_owner(container, faction)?;
        container
    };

    if let Some(supplies_def) = &camp_def.supplies {
        let required = load_per_unit(supplies_def.commodity)
            .0
            .checked_mul(supplies_def.quantity.0)
            .map(LoadUnits)
            .ok_or_else(|| {
                ScenarioError::Validation(format!(
                    "bandit camp '{}' supplies load overflowed",
                    camp_def.faction
                ))
            })?;
        if supplies_def.container.capacity.0 < required.0 {
            return Err(ScenarioError::Validation(format!(
                "bandit camp '{}' supplies container capacity {} is below required load {}",
                camp_def.faction, supplies_def.container.capacity.0, required.0
            )));
        }
    }

    txn.set_component_bandit_camp(
        place,
        BanditCamp {
            faction,
            supplies,
            empty_since_tick: None,
        },
    )?;
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
    names.insert(agent_def.name.clone(), agent_id);

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
    let agenda_profile = agent_def.agenda_profile.unwrap_or_default();
    let execution_budget = agent_def.execution_budget.unwrap_or_default();
    txn.set_component_cognitive_profile(agent_id, cognitive)?;
    txn.set_component_agenda_profile(agent_id, agenda_profile)?;
    txn.set_component_execution_budget(agent_id, execution_budget)?;
    let epistemic = agent_def.epistemic_disposition.clone().unwrap_or_default();
    txn.set_component_epistemic_disposition_profile(agent_id, epistemic)?;
    let intention = agent_def.intention_disposition.clone().unwrap_or_default();
    txn.set_component_intention_disposition_profile(agent_id, intention)?;
    let communication = agent_def.communication_profile.clone().unwrap_or_default();
    txn.set_component_communication_profile(agent_id, communication)?;
    let preference = agent_def.preference_profile.unwrap_or_default();
    txn.set_component_preference_profile(agent_id, preference)?;
    txn.set_component_expectation_store(agent_id, ExpectationStore::default())?;
    let last_seen_memory = last_seen_memory_from_def(agent_def.last_seen_memory.as_ref(), names)?;
    txn.set_component_last_seen_memory(agent_id, last_seen_memory)?;
    if let Some(observations_def) = agent_def.social_observations.as_ref() {
        let mut belief_store = txn
            .get_component_agent_belief_store(agent_id)
            .cloned()
            .unwrap_or_default();
        belief_store
            .social_observations
            .extend(social_observations_from_def(observations_def, names)?);
        txn.set_component_agent_belief_store(agent_id, belief_store)?;
    }
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
        let profile = MerchandiseProfile {
            sale_kinds: merch_def.sale_kinds.iter().copied().collect(),
            home_facility: None,
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
    Ok(())
}

fn apply_agent_expectation_stores(
    txn: &mut WorldTxn<'_>,
    def: &ScenarioDef,
    names: &BTreeMap<String, EntityId>,
) -> Result<(), ScenarioError> {
    for agent_def in &def.agents {
        let agent_id = resolve_name(names, &agent_def.name, "agent expectation owner")?;
        let expectation_store =
            expectation_store_from_def(agent_id, agent_def.expectation_store.as_ref(), names)?;
        txn.set_component_expectation_store(agent_id, expectation_store)?;
    }

    Ok(())
}

fn spawn_office(
    txn: &mut WorldTxn<'_>,
    office_def: &types::OfficeDef,
    names: &mut BTreeMap<String, EntityId>,
    facility_locations: &mut BTreeMap<EntityId, EntityId>,
) -> Result<(), ScenarioError> {
    let seat = resolve_name(
        names,
        &office_def.seat,
        &format!("office '{}' seat", office_def.name),
    )?;

    let eligibility_rules = office_def
        .eligibility_rules
        .iter()
        .map(|rule| match rule {
            types::EligibilityRuleDef::FactionMember(name) => resolve_name(
                names,
                name,
                &format!("office '{}' faction eligibility", office_def.name),
            )
            .map(EligibilityRule::FactionMember),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let initial_holder = office_def
        .initial_holder
        .as_ref()
        .map(|name| {
            resolve_name(
                names,
                name,
                &format!("office '{}' initial holder", office_def.name),
            )
        })
        .transpose()?;

    let office = txn.create_office(&office_def.name)?;
    txn.set_ground_location(office, seat)?;
    txn.set_component_office_data(
        office,
        OfficeData {
            title: office_def.name.clone(),
            seat,
            jurisdiction: BTreeSet::from([seat]),
            succession_law: office_def.succession_law.clone(),
            eligibility_rules,
            succession_period_ticks: office_def.succession_period_ticks,
            vacancy_since: initial_holder.is_none().then_some(Tick(0)),
        },
    )?;
    if matches!(
        office_def.succession_law,
        worldwake_core::SuccessionLaw::Force
    ) {
        let hold_ticks = u32::try_from(office_def.succession_period_ticks)
            .ok()
            .and_then(std::num::NonZeroU32::new)
            .ok_or_else(|| {
                ScenarioError::Validation(format!(
                    "office '{}' force succession_period_ticks must fit in non-zero u32",
                    office_def.name
                ))
            })?;
        txn.set_component_office_force_profile(
            office,
            OfficeForceProfile {
                uncontested_hold_ticks: hold_ticks,
                vacancy_claim_grace_ticks: std::num::NonZeroU32::new(1).unwrap(),
                challenger_presence_grace_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        )?;
        txn.set_component_office_force_state(
            office,
            OfficeForceState {
                control_since: None,
                challenged_since: None,
                contested_since: None,
                last_uncontested_tick: None,
            },
        )?;
    }

    for kind in [RecordKind::OfficeRegister, RecordKind::SupportLedger] {
        let exists = txn
            .query_record_data()
            .any(|(_, record)| record.record_kind == kind && record.home_place == seat);
        if !exists {
            let record = txn.create_record(RecordData {
                record_kind: kind,
                home_place: seat,
                issuer: seat,
                consultation_ticks: 4,
                max_entries_per_consult: 6,
                entries: Vec::new(),
                next_entry_id: 0,
            })?;
            facility_locations.insert(record, seat);
        }
    }
    let crime_register_exists = txn.query_record_data().any(|(_, record)| {
        record.record_kind == RecordKind::CrimeRegister
            && record.home_place == seat
            && record.issuer == office
    });
    if !crime_register_exists {
        let crime_register = txn.create_record(RecordData {
            record_kind: RecordKind::CrimeRegister,
            home_place: seat,
            issuer: office,
            consultation_ticks: 4,
            max_entries_per_consult: 6,
            entries: Vec::new(),
            next_entry_id: 0,
        })?;
        facility_locations.insert(crime_register, seat);
    }

    let office_register = txn
        .query_record_data()
        .find_map(|(record, data)| {
            (data.record_kind == RecordKind::OfficeRegister && data.home_place == seat)
                .then_some(record)
        })
        .ok_or_else(|| {
            ScenarioError::Validation(format!(
                "office '{}' requires an OfficeRegister at its seat",
                office_def.name
            ))
        })?;
    let office_holder_claim = InstitutionalClaim::OfficeHolder {
        office,
        holder: initial_holder,
        effective_tick: Tick(0),
    };
    let office_holder_entry_id = txn.append_record_entry(office_register, office_holder_claim)?;
    if let Some(initial_holder) = initial_holder {
        txn.assign_office(office, initial_holder)?;
        txn.project_institutional_belief(
            initial_holder,
            InstitutionalBeliefKey::OfficeHolderOf { office },
            BelievedInstitutionalClaim {
                claim: office_holder_claim,
                source: InstitutionalKnowledgeSource::RecordConsultation {
                    record: office_register,
                    entry_id: office_holder_entry_id,
                },
                learned_tick: Tick(0),
                learned_at: Some(seat),
            },
        )?;
    }

    if let Some(treasury) = &office_def.treasury {
        let container_name = treasury
            .container_name
            .clone()
            .unwrap_or_else(|| format!("{} Treasury", office_def.name));
        let capacity = load_per_unit(treasury.commodity)
            .0
            .checked_mul(treasury.quantity.0)
            .map(LoadUnits)
            .ok_or_else(|| {
                ScenarioError::Validation(format!(
                    "office '{}' treasury capacity overflowed for {:?} quantity {}",
                    office_def.name, treasury.commodity, treasury.quantity.0
                ))
            })?;
        let treasury_container = txn.create_container(Container {
            capacity,
            allowed_commodities: Some(BTreeSet::from([treasury.commodity])),
            allows_unique_items: false,
            allows_nested_containers: false,
        })?;
        txn.set_component_name(treasury_container, Name(container_name))?;
        txn.set_ground_location(treasury_container, seat)?;
        txn.set_owner(treasury_container, office)?;

        let lot = txn.create_item_lot(treasury.commodity, treasury.quantity)?;
        txn.put_into_container(lot, treasury_container)?;
        txn.set_owner(lot, office)?;
    }

    facility_locations.insert(office, seat);
    names.insert(office_def.name.clone(), office);
    Ok(())
}

fn spawn_notice(
    txn: &mut WorldTxn<'_>,
    notice_def: &types::NoticeDef,
    names: &BTreeMap<String, EntityId>,
) -> Result<(), ScenarioError> {
    let issuer = resolve_name(names, &notice_def.issuer, "notice issuer")?;
    let location = resolve_name(names, &notice_def.location, "notice location")?;
    let issuing_authority = notice_def
        .issuing_authority
        .as_ref()
        .map(|name| resolve_name(names, name, "notice issuing_authority"))
        .transpose()?;
    let jurisdiction = notice_def
        .jurisdiction
        .as_ref()
        .map(|name| resolve_name(names, name, "notice jurisdiction"))
        .transpose()?;
    let topic = notice_topic_from_def(&notice_def.topic, names)?;

    let artifact = txn.create_entity(EntityKind::SocialArtifact);
    txn.set_component_artifact_header(
        artifact,
        ArtifactHeader {
            kind: ArtifactKind::Notice,
            issuer,
            issuing_authority,
            created_at: Tick(0),
            expires_at: notice_def.expires_at.map(Tick),
            state: ArtifactState::Active,
            jurisdiction,
        },
    )?;
    txn.set_component_notice_content(artifact, NoticeContent { topic })?;
    txn.set_ground_location(artifact, location)?;
    Ok(())
}

fn notice_topic_from_def(
    topic: &types::NoticeTopicDef,
    names: &BTreeMap<String, EntityId>,
) -> Result<NoticeTopic, ScenarioError> {
    match topic {
        types::NoticeTopicDef::ThreatWarning { place } => Ok(NoticeTopic::ThreatWarning {
            place: resolve_name(names, place, "notice threat_warning place")?,
        }),
        types::NoticeTopicDef::OfficeVacancy { office } => Ok(NoticeTopic::OfficeVacancy {
            office: resolve_name(names, office, "notice office_vacancy office")?,
        }),
        types::NoticeTopicDef::CommodityShortage { commodity, place } => {
            Ok(NoticeTopic::CommodityShortage {
                commodity: *commodity,
                place: resolve_name(names, place, "notice commodity_shortage place")?,
            })
        }
    }
}

fn expectation_store_from_def(
    owner: EntityId,
    def: Option<&types::ExpectationStoreDef>,
    names: &BTreeMap<String, EntityId>,
) -> Result<ExpectationStore, ScenarioError> {
    let Some(def) = def else {
        return Ok(ExpectationStore::default());
    };

    let mut store = ExpectationStore::default();
    for record_def in &def.records {
        let subject = resolve_name(names, &record_def.subject, "expectation subject")?;
        let expected_place = resolve_name(
            names,
            &record_def.expected_place,
            "expectation expected_place",
        )?;
        let basis = expectation_basis_from_def(&record_def.basis, names)?;
        let state = expectation_state_from_def(&record_def.state, names)?;
        store.allocate_record(|id| ExpectationRecord {
            id,
            owner,
            subject,
            expected_place,
            deadline_tick: Tick(record_def.deadline_tick),
            grace_ticks: record_def.grace_ticks,
            basis,
            state,
            created_tick: Tick(record_def.created_tick),
        });
    }
    Ok(store)
}

fn expectation_basis_from_def(
    def: &types::ExpectationBasisDef,
    names: &BTreeMap<String, EntityId>,
) -> Result<ExpectationBasis, ScenarioError> {
    Ok(match def {
        types::ExpectationBasisDef::DutyAssignment { office } => ExpectationBasis::DutyAssignment {
            office: resolve_name(names, office, "expectation duty office")?,
        },
        types::ExpectationBasisDef::DeliveryCommitment {
            commodity,
            quantity,
        } => ExpectationBasis::DeliveryCommitment {
            commodity: *commodity,
            quantity: *quantity,
        },
        types::ExpectationBasisDef::RoutineReturn => ExpectationBasis::RoutineReturn,
        types::ExpectationBasisDef::EscortObligation { charge } => {
            ExpectationBasis::EscortObligation {
                charge: resolve_name(names, charge, "expectation escort charge")?,
            }
        }
        types::ExpectationBasisDef::SocialPromise => ExpectationBasis::SocialPromise,
        types::ExpectationBasisDef::PlanStepCompletion {
            step_index,
            kind_tag,
        } => ExpectationBasis::PlanStepCompletion {
            step_index: *step_index,
            kind_tag: *kind_tag,
        },
    })
}

fn expectation_state_from_def(
    def: &types::ExpectationStateDef,
    names: &BTreeMap<String, EntityId>,
) -> Result<ExpectationState, ScenarioError> {
    Ok(match def {
        types::ExpectationStateDef::Active => ExpectationState::Active,
        types::ExpectationStateDef::Overdue => ExpectationState::Overdue,
        types::ExpectationStateDef::Resolved { outcome } => ExpectationState::Resolved {
            outcome: expectation_outcome_from_def(outcome, names)?,
        },
        types::ExpectationStateDef::Expired => ExpectationState::Expired,
    })
}

fn expectation_outcome_from_def(
    def: &types::ExpectationOutcomeDef,
    names: &BTreeMap<String, EntityId>,
) -> Result<ExpectationOutcome, ScenarioError> {
    Ok(match def {
        types::ExpectationOutcomeDef::Fulfilled => ExpectationOutcome::Fulfilled,
        types::ExpectationOutcomeDef::FoundSafe { at_place } => ExpectationOutcome::FoundSafe {
            at_place: resolve_name(names, at_place, "expectation outcome place")?,
        },
        types::ExpectationOutcomeDef::FoundWounded { at_place } => {
            ExpectationOutcome::FoundWounded {
                at_place: resolve_name(names, at_place, "expectation outcome place")?,
            }
        }
        types::ExpectationOutcomeDef::FoundDead { at_place } => ExpectationOutcome::FoundDead {
            at_place: resolve_name(names, at_place, "expectation outcome place")?,
        },
        types::ExpectationOutcomeDef::NotFound => ExpectationOutcome::NotFound,
        types::ExpectationOutcomeDef::ReturnedLate => ExpectationOutcome::ReturnedLate,
    })
}

fn last_seen_memory_from_def(
    def: Option<&types::LastSeenMemoryDef>,
    names: &BTreeMap<String, EntityId>,
) -> Result<LastSeenMemory, ScenarioError> {
    let Some(def) = def else {
        return Ok(LastSeenMemory::default());
    };

    let mut records = BTreeMap::new();
    for record_def in &def.records {
        let subject = resolve_name(names, &record_def.subject, "last_seen subject")?;
        records.insert(
            subject,
            LastSeenRecord {
                subject,
                place: resolve_name(names, &record_def.place, "last_seen place")?,
                observed_tick: Tick(record_def.observed_tick),
                source: resolve_name(names, &record_def.source, "last_seen source")?,
                provenance: last_seen_provenance_from_def(&record_def.provenance, names)?,
            },
        );
    }

    Ok(LastSeenMemory {
        records,
        capacity: def.capacity,
    })
}

fn last_seen_provenance_from_def(
    def: &types::LastSeenProvenanceDef,
    names: &BTreeMap<String, EntityId>,
) -> Result<LastSeenProvenance, ScenarioError> {
    Ok(match def {
        types::LastSeenProvenanceDef::DirectObservation => LastSeenProvenance::DirectObservation,
        types::LastSeenProvenanceDef::Hearsay {
            original_observer,
            chain_depth,
        } => LastSeenProvenance::Hearsay {
            original_observer: resolve_name(
                names,
                original_observer,
                "last_seen hearsay original_observer",
            )?,
            chain_depth: *chain_depth,
        },
    })
}

fn social_observations_from_def(
    defs: &[types::SocialObservationDef],
    names: &BTreeMap<String, EntityId>,
) -> Result<Vec<SocialObservation>, ScenarioError> {
    defs.iter()
        .map(|def| {
            let place = resolve_name(names, &def.place, "social observation place")?;
            let detail = match &def.detail {
                types::SocialObservationDetailDef::WitnessedConflict { actor, target } => {
                    SocialObservationDetail::WitnessedConflict {
                        actor: resolve_name(names, actor, "social observation actor")?,
                        target: resolve_name(names, target, "social observation target")?,
                    }
                }
            };
            Ok(SocialObservation {
                detail,
                place,
                observed_tick: Tick(def.observed_tick),
                source: def.source,
            })
        })
        .collect()
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
        txn.set_owner(item_id, location_id)?;
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
    use worldwake_core::SuccessionLaw;
    use worldwake_core::topology::PlaceTag;
    use worldwake_core::{
        AgendaProfile, ArtifactPostingProfile, BeliefConfidencePolicy, CarryCapacity,
        CognitiveProfile, CommodityDecayMap, CommodityKind, CommodityValuationProfile,
        CommunicationProfile, ContentionDispositionProfile, ContentionPolicy, ControlSource,
        DisposalProfile, DiversificationProfile, DriveEscalationParams, DriveEscalationProfile,
        DriveThresholds, EpistemicDispositionProfile, ExecutionBudget, ExpectationStore,
        HomeostaticNeedId, HomeostaticNeeds, IntentionDispositionProfile,
        JusticeDispositionProfile, LastProactiveExplorationTick, LastSeenMemory, LoadUnits,
        MultiplierPermille, ObligationSatiationProfile, PatrolProfile, PatrolRoute,
        PerceptionProfile, Permille, PlaceVisibilityProfile, PreferenceProfile, PursuitProfile,
        Quantity, SubstitutePreferences, TellProfile, TheftDispositionProfile, ThresholdBand,
        TradeCategory, ViolationDispositionProfile, WorkstationTag, default_commodity_decay_map,
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
            agenda_profile: None,
            execution_budget: None,
            epistemic_disposition: None,
            intention_disposition: None,
            communication_profile: None,
            preference_profile: None,
            expectation_store: None,
            last_seen_memory: None,
            social_observations: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
        }
    }

    #[test]
    fn spawn_scenario_authors_bandit_camp_policy_membership_and_supplies() {
        let mut def = minimal_def();
        def.places[0].tags.push(PlaceTag::Camp);
        def.agents = vec![minimal_agent("Rook", "Village", ControlSource::Human)];
        def.bandit_camps = vec![BanditCampDef {
            faction: "Road Wolves".into(),
            place: "Village".into(),
            members: vec!["Rook".into()],
            policy: BanditFactionPolicyDef {
                min_regroup_count: 1,
                establishment_duration_ticks: NonZeroU32::new(2).unwrap(),
                abandonment_grace_ticks: NonZeroU32::new(3).unwrap(),
                flee_wound_threshold: Permille::new(650).unwrap(),
                rally_place: Some("Village".into()),
            },
            supplies: Some(BanditCampSuppliesDef {
                commodity: CommodityKind::Apple,
                quantity: Quantity(4),
                container: worldwake_core::Container {
                    capacity: LoadUnits(4),
                    allowed_commodities: None,
                    allows_unique_items: false,
                    allows_nested_containers: false,
                },
            }),
        }];

        let spawned = spawn_scenario(&def).expect("scenario should spawn");
        let world = spawned.state.world();
        let village = spawned
            .state
            .world()
            .topology()
            .place_ids()
            .next()
            .expect("minimal topology should have a place");
        let camp = world
            .get_component_bandit_camp(village)
            .cloned()
            .expect("Village should have authored bandit camp");
        let faction = camp.faction;

        assert_eq!(world.members_of(faction).len(), 1);
        assert_eq!(
            world
                .get_component_bandit_faction_policy(faction)
                .expect("faction should have bandit policy")
                .rally_place,
            Some(village)
        );
        assert_eq!(world.owner_of(camp.supplies), Some(faction));
        assert_eq!(world.direct_contents_of(camp.supplies).len(), 1);
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
        assert_eq!(
            world
                .get_component_metabolism_profile(agent_id)
                .unwrap()
                .min_sleep_ticks,
            NonZeroU32::new(8).unwrap()
        );
    }

    #[test]
    fn test_spawn_scenario_applies_authored_hostility() {
        let mut def = minimal_def();
        def.agents
            .push(minimal_agent("Intruder", "Village", ControlSource::Ai));
        def.hostilities.push(HostilityDef {
            subject: "Alice".into(),
            target: "Intruder".into(),
        });

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let alice = world
            .query_name_and_agent_data()
            .find_map(|(entity, name, _)| (name.0 == "Alice").then_some(entity))
            .expect("Alice should spawn");
        let intruder = world
            .query_name_and_agent_data()
            .find_map(|(entity, name, _)| (name.0 == "Intruder").then_some(entity))
            .expect("Intruder should spawn");

        assert_eq!(world.hostile_targets_of(alice), vec![intruder]);
        assert_eq!(world.hostile_towards(intruder), vec![alice]);
    }

    #[test]
    fn test_spawn_notice_artifact_from_scenario() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Square".into(),
                tags: vec![PlaceTag::Village],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![minimal_agent("Herald", "Square", ControlSource::Human)],
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![NoticeDef {
                issuer: "Herald".into(),
                location: "Square".into(),
                issuing_authority: None,
                expires_at: Some(18),
                jurisdiction: Some("Square".into()),
                topic: NoticeTopicDef::ThreatWarning {
                    place: "Square".into(),
                },
            }],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let square = EntityId {
            slot: 0,
            generation: 0,
        };
        let herald = world
            .entities_with_name_and_agent_data()
            .find(|entity| {
                world
                    .get_component_name(*entity)
                    .is_some_and(|name| name.0 == "Herald")
            })
            .expect("scenario should spawn notice issuer");
        let notice = world
            .entities_effectively_at(square)
            .into_iter()
            .find(|entity| world.get_component_artifact_header(*entity).is_some())
            .expect("scenario should spawn a notice artifact at the square");

        assert_eq!(
            world.get_component_artifact_header(notice),
            Some(&ArtifactHeader {
                kind: ArtifactKind::Notice,
                issuer: herald,
                issuing_authority: None,
                created_at: Tick(0),
                expires_at: Some(Tick(18)),
                state: ArtifactState::Active,
                jurisdiction: Some(square),
            })
        );
        assert_eq!(
            world.get_component_notice_content(notice),
            Some(&NoticeContent {
                topic: NoticeTopic::ThreatWarning { place: square },
            })
        );
    }

    #[test]
    fn test_spawn_agent_with_social_observations_override() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Square".into(),
                tags: vec![PlaceTag::Village],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![
                minimal_agent("Rival", "Square", ControlSource::None),
                AgentDef {
                    social_observations: Some(vec![SocialObservationDef {
                        place: "Square".into(),
                        observed_tick: 7,
                        source: worldwake_core::PerceptionSource::DirectObservation,
                        detail: SocialObservationDetailDef::WitnessedConflict {
                            actor: "Claimant".into(),
                            target: "Rival".into(),
                        },
                    }]),
                    ..minimal_agent("Claimant", "Square", ControlSource::Ai)
                },
            ],
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let square = EntityId {
            slot: 0,
            generation: 0,
        };
        let claimant = world
            .entities_with_name_and_agent_data()
            .find(|entity| {
                world
                    .get_component_name(*entity)
                    .is_some_and(|name| name.0 == "Claimant")
            })
            .expect("scenario should spawn claimant");
        let rival = world
            .entities_with_name_and_agent_data()
            .find(|entity| {
                world
                    .get_component_name(*entity)
                    .is_some_and(|name| name.0 == "Rival")
            })
            .expect("scenario should spawn rival");

        assert_eq!(
            world
                .get_component_agent_belief_store(claimant)
                .expect("scenario should keep claimant belief store")
                .social_observations,
            vec![SocialObservation {
                detail: SocialObservationDetail::WitnessedConflict {
                    actor: claimant,
                    target: rival,
                },
                place: square,
                observed_tick: Tick(7),
                source: worldwake_core::PerceptionSource::DirectObservation,
            }]
        );
    }

    #[test]
    fn test_spawn_agent_applies_authored_agenda_profile() {
        let def = ScenarioDef {
            agents: vec![AgentDef {
                agenda_profile: Some(AgendaProfile {
                    pending_capacity: 20,
                    suspended_capacity: 4,
                    revive_cooldown_ticks: 2,
                }),
                ..minimal_agent("Alice", "Village", ControlSource::Ai)
            }],
            ..minimal_def()
        };
        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let agent = world.entities_with_name_and_agent_data().next().unwrap();

        assert_eq!(
            world.get_component_agenda_profile(agent),
            Some(&AgendaProfile {
                pending_capacity: 20,
                suspended_capacity: 4,
                revive_cooldown_ticks: 2,
            })
        );
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
    fn test_spawn_minimal_scenario_uses_default_harvest_trace_retention() {
        let spawned = spawn_scenario(&minimal_def()).unwrap();

        assert_eq!(
            spawned.state.world().harvest_trace_retention_ticks(),
            worldwake_core::HARVEST_TRACE_RETENTION_TICKS,
        );
    }

    #[test]
    fn test_spawn_scenario_applies_harvest_trace_retention_override() {
        let mut def = minimal_def();
        def.harvest_trace_retention_ticks = Some(75);

        let spawned = spawn_scenario(&def).unwrap();

        assert_eq!(spawned.state.world().harvest_trace_retention_ticks(), 75,);
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![ItemDef {
                commodity: CommodityKind::Apple,
                quantity: Quantity(10),
                location: "Market".into(),
                container: false,
            }],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![ItemDef {
                commodity: CommodityKind::Sword,
                quantity: Quantity(1),
                location: "Warrior".into(),
                container: false,
            }],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
        assert_eq!(world.owner_of(sword_id), Some(warrior));
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![FacilityDef {
                name: Some("Forge Bench".into()),
                workstation: WorkstationTag::Forge,
                location: "Smithy".into(),
                merchant_storage: None,
                contention_policy: None,
            }],
            resource_sources: vec![ResourceSourceDef {
                commodity: CommodityKind::Apple,
                location: "Orchard".into(),
                facility: None,
                regeneration_ticks_per_unit: NonZeroU32::new(5),
                capacity: Quantity(20),
                extraction_slots: 1,
                extraction_duration_ticks: 1,
            }],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![FacilityDef {
                name: Some("North Orchard".into()),
                workstation: WorkstationTag::OrchardRow,
                location: "Orchard".into(),
                merchant_storage: None,
                contention_policy: None,
            }],
            resource_sources: vec![ResourceSourceDef {
                commodity: CommodityKind::Apple,
                location: "Orchard".into(),
                facility: Some("North Orchard".into()),
                regeneration_ticks_per_unit: NonZeroU32::new(2),
                capacity: Quantity(20),
                extraction_slots: 1,
                extraction_duration_ticks: 1,
            }],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![FacilityDef {
                name: Some("Village Well".into()),
                workstation: WorkstationTag::Well,
                location: "Village".into(),
                merchant_storage: None,
                contention_policy: None,
            }],
            resource_sources: vec![ResourceSourceDef {
                commodity: CommodityKind::Water,
                location: "Village".into(),
                facility: Some("Village Well".into()),
                regeneration_ticks_per_unit: NonZeroU32::new(3),
                capacity: Quantity(15),
                extraction_slots: 1,
                extraction_duration_ticks: 1,
            }],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
    fn test_spawn_merchant_storage_facility_creates_stock_policy_owned_by_agent() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Market".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![minimal_agent("Merchant", "Market", ControlSource::Ai)],
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![FacilityDef {
                name: Some("Merchant Stall".into()),
                workstation: WorkstationTag::Forge,
                location: "Market".into(),
                merchant_storage: Some(MerchantStorageDef {
                    owner: "Merchant".into(),
                    stock_capacity: LoadUnits(200),
                    display_capacity: Some(LoadUnits(100)),
                }),
                contention_policy: None,
            }],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let market = EntityId {
            slot: 0,
            generation: 0,
        };
        let merchant = world
            .entities_with_name_and_agent_data()
            .find(|entity| {
                world
                    .get_component_name(*entity)
                    .is_some_and(|name| name.0 == "Merchant")
            })
            .expect("scenario should spawn merchant");
        let stall = world
            .entities_effectively_at(market)
            .into_iter()
            .find(|entity| {
                world
                    .get_component_name(*entity)
                    .is_some_and(|name| name.0 == "Merchant Stall")
            })
            .expect("scenario should spawn named merchant stall");
        let policy = world
            .get_component_stock_storage_policy(stall)
            .expect("merchant stall should have stock storage policy");

        assert_eq!(world.owner_of(stall), Some(merchant));
        assert_eq!(world.owner_of(policy.stock_container), Some(merchant));
        let display = policy
            .display_container
            .expect("merchant stall should have display container");
        assert_eq!(world.owner_of(display), Some(merchant));
        assert_eq!(
            world.get_component_workstation_marker(stall).unwrap().0,
            WorkstationTag::Forge
        );
    }

    #[test]
    fn test_spawn_facility_contention_policy_seeds_queue_state() {
        let policy = ContentionPolicy {
            grant_hold_ticks: NonZeroU32::new(3).unwrap(),
            auto_promote: true,
            max_waiters: Some(2),
        };
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Village".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![],
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![FacilityDef {
                name: Some("Village Well".into()),
                workstation: WorkstationTag::Well,
                location: "Village".into(),
                merchant_storage: None,
                contention_policy: Some(policy.clone()),
            }],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let well = world
            .query_name()
            .find_map(|(entity, name)| (name.0 == "Village Well").then_some(entity))
            .expect("scenario should spawn named well");

        assert_eq!(world.get_component_contention_policy(well), Some(&policy));
        assert!(
            world
                .get_component_contention_queue(well)
                .is_some_and(|queue| queue.waiting.is_empty() && queue.granted.is_none())
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
        let custom_memory = types::LastSeenMemoryDef {
            records: Vec::new(),
            capacity: 50,
        };
        let custom_expectation_store = types::ExpectationStoreDef::default();
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let agent = world
            .entities_with_name_and_agent_data()
            .next()
            .expect("spawned scenario should contain one agent");

        assert_eq!(
            world.get_component_expectation_store(agent),
            Some(&ExpectationStore::default())
        );
        assert_eq!(
            world.get_component_last_seen_memory(agent),
            Some(&LastSeenMemory {
                records: BTreeMap::new(),
                capacity: 50,
            })
        );
    }

    #[test]
    fn spawn_scenario_resolves_agent_duty_assignments_after_offices_spawn() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Hall".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![
                AgentDef {
                    expectation_store: Some(types::ExpectationStoreDef {
                        records: vec![types::ExpectationRecordDef {
                            subject: "Subject".into(),
                            expected_place: "Hall".into(),
                            deadline_tick: 0,
                            grace_ticks: 0,
                            basis: types::ExpectationBasisDef::DutyAssignment {
                                office: "Warden".into(),
                            },
                            state: types::ExpectationStateDef::Overdue,
                            created_tick: 0,
                        }],
                    }),
                    ..minimal_agent("Holder", "Hall", ControlSource::Ai)
                },
                minimal_agent("Subject", "Hall", ControlSource::None),
            ],
            bandit_camps: Vec::new(),
            offices: vec![OfficeDef {
                name: "Warden".into(),
                seat: "Hall".into(),
                succession_law: SuccessionLaw::Force,
                succession_period_ticks: 3,
                initial_holder: None,
                eligibility_rules: Vec::new(),
                treasury: None,
            }],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
        };

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let holder = world
            .query_name_and_agent_data()
            .find_map(|(entity, name, _)| (name.0 == "Holder").then_some(entity))
            .unwrap();
        let warden = world
            .query_name()
            .find_map(|(entity, name)| (name.0 == "Warden").then_some(entity))
            .unwrap();
        let store = world.get_component_expectation_store(holder).unwrap();

        assert!(store.records.values().any(|record| {
            matches!(
                record.basis,
                ExpectationBasis::DutyAssignment { office } if office == warden
            )
        }));
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
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
            bandit_camps: Vec::new(),
            offices: vec![],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
        };

        let Err(error) = spawn_scenario(&def) else {
            panic!("invalid patrol route should fail");
        };
        assert_eq!(
            error.to_string(),
            "validation error: agent 'Guard' patrol route references nonexistent entity 'Nowhere'"
        );
    }

    #[test]
    fn test_spawn_office_creates_local_crime_register_for_office_issuer() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Square".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![minimal_agent("Holder", "Square", ControlSource::Ai)],
            bandit_camps: Vec::new(),
            offices: vec![OfficeDef {
                name: "Marshal".into(),
                seat: "Square".into(),
                succession_law: SuccessionLaw::Force,
                succession_period_ticks: 2,
                initial_holder: Some("Holder".into()),
                eligibility_rules: vec![],
                treasury: None,
            }],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
        };

        let spawned = spawn_scenario(&def).expect("office scenario should spawn");
        let world = spawned.state.world();
        let office = world
            .query_name()
            .find_map(|(entity, name)| (name.0 == "Marshal").then_some(entity))
            .expect("spawned office should exist");
        let seat = world
            .get_component_office_data(office)
            .expect("spawned office should have OfficeData")
            .seat;
        let holder = world
            .query_name()
            .find_map(|(entity, name)| (name.0 == "Holder").then_some(entity))
            .expect("spawned holder should exist");
        let crime_register = world
            .query_record_data()
            .find_map(|(entity, data)| {
                (data.record_kind == RecordKind::CrimeRegister).then_some((entity, data))
            })
            .expect("office should spawn a colocated crime register");
        let office_register = world
            .query_record_data()
            .find_map(|(entity, data)| {
                (data.record_kind == RecordKind::OfficeRegister && data.home_place == seat)
                    .then_some((entity, data))
            })
            .expect("office should spawn with a colocated office register");
        let holder_beliefs = world
            .get_component_agent_belief_store(holder)
            .expect("initial holder should have a belief store");

        assert_eq!(crime_register.1.home_place, seat);
        assert_eq!(crime_register.1.issuer, office);
        assert_eq!(world.office_holder(office), Some(holder));
        assert!(office_register.1.entries.iter().any(|entry| {
            matches!(
                entry.claim,
                InstitutionalClaim::OfficeHolder {
                    office: claim_office,
                    holder: Some(claim_holder),
                    effective_tick: Tick(0),
                } if claim_office == office && claim_holder == holder
            )
        }));
        assert_eq!(
            holder_beliefs.believed_office_holder(office),
            worldwake_core::InstitutionalBeliefRead::Certain(Some(holder))
        );
    }

    #[test]
    fn spawn_office_with_treasury_creates_owned_container_and_lots() {
        let def = ScenarioDef {
            seed: 1,
            places: vec![PlaceDef {
                name: "Square".into(),
                tags: vec![],
                visibility_profile: None,
            }],
            edges: vec![],
            agents: vec![],
            bandit_camps: Vec::new(),
            offices: vec![OfficeDef {
                name: "Marshal".into(),
                seat: "Square".into(),
                succession_law: SuccessionLaw::Force,
                succession_period_ticks: 2,
                initial_holder: None,
                eligibility_rules: vec![],
                treasury: Some(TreasuryDef {
                    commodity: CommodityKind::Coin,
                    quantity: Quantity(12),
                    container_name: None,
                }),
            }],
            notices: vec![],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
            hostilities: vec![],
            commodity_decay: None,
            survival_health_contract: None,
            compaction_interval: 0,
            scenario_lint_overrides: BTreeMap::new(),
            harvest_trace_retention_ticks: None,
        };

        let spawned = spawn_scenario(&def).expect("office treasury scenario should spawn");
        let world = spawned.state.world();
        let office = world
            .query_name()
            .find_map(|(entity, name)| (name.0 == "Marshal").then_some(entity))
            .expect("spawned office should exist");
        let seat = world
            .get_component_office_data(office)
            .expect("spawned office should have OfficeData")
            .seat;
        let treasury_container = world
            .query_name()
            .find_map(|(entity, name)| (name.0 == "Marshal Treasury").then_some(entity))
            .expect("default-named treasury container should exist");

        assert!(world.get_component_container(treasury_container).is_some());
        assert_eq!(world.owner_of(treasury_container), Some(office));
        assert_eq!(world.effective_place(treasury_container), Some(seat));

        let (lot, treasury_lot) = world
            .query_item_lot()
            .find(|(entity, lot)| {
                lot.commodity == CommodityKind::Coin
                    && lot.quantity == Quantity(12)
                    && world.direct_container(*entity) == Some(treasury_container)
            })
            .expect("treasury coin lot should be inside the treasury container");

        assert_eq!(world.owner_of(lot), Some(office));
        assert_eq!(treasury_lot.quantity, Quantity(12));
        assert_eq!(
            world.controlled_commodity_quantity(office, CommodityKind::Coin),
            Quantity(12)
        );
        assert!(
            !world.ground_entities_at(seat).contains(&lot),
            "treasury lot should not be a loose place-floor item"
        );
    }

    #[test]
    fn spawn_scenario_resource_source_defaults_to_one_slot_one_tick() {
        let mut def = minimal_def();
        def.resource_sources = vec![ResourceSourceDef {
            commodity: CommodityKind::Apple,
            location: "Village".into(),
            facility: None,
            regeneration_ticks_per_unit: NonZeroU32::new(5),
            capacity: Quantity(20),
            extraction_slots: 1,
            extraction_duration_ticks: 1,
        }];

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let (_, source) = world
            .query_resource_source()
            .next()
            .expect("resource source should be spawned");
        assert_eq!(source.extraction_slots.get(), 1);
        assert_eq!(source.extraction_duration_ticks.get(), 1);
    }

    #[test]
    fn spawn_scenario_resource_source_explicit_extraction_fields() {
        let mut def = minimal_def();
        def.resource_sources = vec![ResourceSourceDef {
            commodity: CommodityKind::Water,
            location: "Village".into(),
            facility: None,
            regeneration_ticks_per_unit: NonZeroU32::new(3),
            capacity: Quantity(40),
            extraction_slots: 5,
            extraction_duration_ticks: 4,
        }];

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let (_, source) = world
            .query_resource_source()
            .next()
            .expect("resource source should be spawned");
        assert_eq!(source.extraction_slots.get(), 5);
        assert_eq!(source.extraction_duration_ticks.get(), 4);
    }

    #[test]
    fn scenario_spawn_registers_queue_per_slot() {
        let mut def = minimal_def();
        def.resource_sources = vec![ResourceSourceDef {
            commodity: CommodityKind::Water,
            location: "Village".into(),
            facility: None,
            regeneration_ticks_per_unit: NonZeroU32::new(3),
            capacity: Quantity(40),
            extraction_slots: 5,
            extraction_duration_ticks: 4,
        }];

        let spawned = spawn_scenario(&def).unwrap();
        let world = spawned.state.world();
        let (source_entity, source) = world
            .query_resource_source()
            .next()
            .expect("resource source should be spawned");
        assert_eq!(source.extraction_slots.get(), 5);

        let queues = world
            .get_component_resource_extraction_queues(source_entity)
            .expect("scenario spawn should register ResourceExtractionQueues");
        assert_eq!(queues.queues.len(), source.extraction_slots.get() as usize);
        for slot in &queues.queues {
            assert_eq!(slot, &ContentionQueue::default());
        }
    }
}
