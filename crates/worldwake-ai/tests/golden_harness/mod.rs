//! Shared infrastructure for the golden end-to-end test suite.
//!
//! Provides `GoldenHarness`, helper functions, recipe builders, and world
//! setup utilities used by all golden test files.

// Each test binary uses a different subset of harness items.
#![allow(dead_code)]

mod timeline;

use std::num::NonZeroU32;

use worldwake_ai::{AgentTickDriver, PlanningBudget};
use worldwake_core::{
    build_believed_entity_state, build_prototype_world, hash_serializable, prototype_place_entity,
    to_shared_belief_snapshot, AgentBeliefStore, BelievedEntityState, BlockedIntentMemory,
    BodyCostPerTick, CarryCapacity, CauseRef, CombatProfile, CombatStance, CommodityKind,
    ComponentDelta, ComponentKind, ComponentValue, ControlSource, DeprivationExposure,
    DriveThresholds, EligibilityRule, EntityId, EntityKind, EventId, EventLog, EventRecord,
    EventTag, EventView, ExclusiveFacilityPolicy, FacilityQueueDispositionProfile,
    FacilityUseQueue, FactionData, FactionPurpose, HomeostaticNeeds, KnownRecipes, LoadUnits,
    MetabolismProfile, OfficeData, PerceptionProfile, PerceptionSource, Permille, PrototypePlace,
    Quantity, RecipeId, RelationDelta, RelationValue, ResourceSource, Seed, StateDelta,
    SuccessionLaw, TellMemoryKey, TellProfile, Tick, ToldBeliefMemory, VisibilitySpec, WitnessData,
    WorkstationMarker, WorkstationTag, World, WorldTxn, WoundList,
};
use worldwake_sim::{
    load_from_bytes, save_to_bytes, step_tick, ActionDefRegistry, ActionHandlerRegistry,
    ActionTraceSink, AutonomousControllerRuntime, ControllerState, DeterministicRng,
    PoliticalTraceSink, RecipeDefinition, RecipeRegistry, ReplayRecordingConfig, ReplayState,
    Scheduler, SimulationState, SystemManifest, TickStepResult, TickStepServices,
};
use worldwake_systems::{build_full_action_registries, dispatch_table};

// Re-export so test files using `use golden_harness::*` get the ownership types.
#[allow(unused_imports)]
pub use timeline::{CrossLayerTimelineBuilder, TimelineLayer};
pub use worldwake_core::{ProductionOutputOwner, ProductionOutputOwnershipPolicy};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn pm(value: u16) -> Permille {
    Permille::new(value).unwrap()
}

pub fn nz(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}

/// Village Square — central hub, slot 0.
pub const VILLAGE_SQUARE: EntityId = prototype_place_entity(PrototypePlace::VillageSquare);
/// Orchard Farm — slot 1.
pub const ORCHARD_FARM: EntityId = prototype_place_entity(PrototypePlace::OrchardFarm);
/// Public Latrine — sanitation facility in the village.
pub const PUBLIC_LATRINE: EntityId = prototype_place_entity(PrototypePlace::PublicLatrine);

pub fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
    WorldTxn::new(
        world,
        worldwake_core::Tick(tick),
        CauseRef::Bootstrap,
        None,
        None,
        VisibilitySpec::SamePlace,
        WitnessData::default(),
    )
}

pub fn commit_txn(txn: WorldTxn<'_>, event_log: &mut EventLog) {
    let _ = txn.commit(event_log);
}

pub fn first_event_id_matching(
    log: &EventLog,
    mut predicate: impl FnMut(EventId, &EventRecord) -> bool,
) -> Option<EventId> {
    (0..log.len())
        .map(|index| EventId(index as u64))
        .find(|event_id| {
            log.get(*event_id)
                .is_some_and(|record| predicate(*event_id, record))
        })
}

pub fn first_tagged_event_id_matching(
    log: &EventLog,
    tag: EventTag,
    mut predicate: impl FnMut(EventId, &EventRecord) -> bool,
) -> Option<EventId> {
    log.events_by_tag(tag).iter().copied().find(|event_id| {
        log.get(*event_id)
            .is_some_and(|record| predicate(*event_id, record))
    })
}

pub fn event_sets_component(
    record: &impl EventView,
    entity: EntityId,
    component_kind: ComponentKind,
    after_matches: impl Fn(&ComponentValue) -> bool,
) -> bool {
    record.state_deltas().iter().any(|delta| {
        matches!(
            delta,
            StateDelta::Component(ComponentDelta::Set {
                entity: changed_entity,
                component_kind: changed_kind,
                after,
                ..
            }) if *changed_entity == entity
                && *changed_kind == component_kind
                && after_matches(after)
        )
    })
}

pub fn event_adds_relation(record: &impl EventView, relation: &RelationValue) -> bool {
    record.state_deltas().iter().any(|delta| {
        matches!(
            delta,
            StateDelta::Relation(RelationDelta::Added {
                relation: changed_relation,
                ..
            }) if changed_relation == relation
        )
    })
}

pub fn event_removes_relation(record: &impl EventView, relation: &RelationValue) -> bool {
    record.state_deltas().iter().any(|delta| {
        matches!(
            delta,
            StateDelta::Relation(RelationDelta::Removed {
                relation: changed_relation,
                ..
            }) if changed_relation == relation
        )
    })
}

pub fn assert_event_order(earlier: EventId, later: EventId, description: &str) {
    assert!(
        earlier < later,
        "{description}: expected {earlier:?} to precede {later:?}"
    );
}

pub fn seed_actor_beliefs(
    world: &mut World,
    event_log: &mut EventLog,
    actor: EntityId,
    entities: &[EntityId],
    observed_tick: Tick,
    source: PerceptionSource,
) {
    let mut store = world
        .get_component_agent_belief_store(actor)
        .cloned()
        .unwrap_or_else(AgentBeliefStore::new);
    for entity in entities {
        if *entity == actor {
            continue;
        }
        if let Some(snapshot) = build_believed_entity_state(world, *entity, observed_tick, source) {
            store.update_entity(*entity, snapshot);
        }
    }

    let mut txn = WorldTxn::new(
        world,
        observed_tick,
        CauseRef::Bootstrap,
        None,
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.set_component_agent_belief_store(actor, store)
        .expect("golden harness should keep belief stores writable");
    commit_txn(txn, event_log);
}

pub fn seed_actor_local_beliefs(
    world: &mut World,
    event_log: &mut EventLog,
    actor: EntityId,
    observed_tick: Tick,
    source: PerceptionSource,
) {
    let entities = world
        .effective_place(actor)
        .map(|place| {
            world
                .entities_effectively_at(place)
                .into_iter()
                .filter(|entity| *entity != actor)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    seed_actor_beliefs(world, event_log, actor, &entities, observed_tick, source);
}

pub fn seed_actor_world_beliefs(
    world: &mut World,
    event_log: &mut EventLog,
    actor: EntityId,
    observed_tick: Tick,
    source: PerceptionSource,
) {
    let entities = world
        .entities()
        .filter(|entity| *entity != actor)
        .collect::<Vec<_>>();
    seed_actor_beliefs(world, event_log, actor, &entities, observed_tick, source);
}

pub fn set_agent_tell_profile(
    world: &mut World,
    event_log: &mut EventLog,
    agent: EntityId,
    tell_profile: TellProfile,
) {
    let mut txn = new_txn(world, 0);
    txn.set_component_tell_profile(agent, tell_profile)
        .expect("golden harness should keep tell profiles writable");
    commit_txn(txn, event_log);
}

pub fn set_agent_perception_profile(
    world: &mut World,
    event_log: &mut EventLog,
    agent: EntityId,
    perception_profile: PerceptionProfile,
) {
    let mut txn = new_txn(world, 0);
    txn.set_component_perception_profile(agent, perception_profile)
        .expect("golden harness should keep perception profiles writable");
    commit_txn(txn, event_log);
}

pub fn seed_belief(
    world: &mut World,
    event_log: &mut EventLog,
    agent: EntityId,
    subject: EntityId,
    believed_state: BelievedEntityState,
) {
    let mut store = world
        .get_component_agent_belief_store(agent)
        .cloned()
        .unwrap_or_else(AgentBeliefStore::new);
    store.update_entity(subject, believed_state);

    let mut txn = new_txn(world, 0);
    txn.set_component_agent_belief_store(agent, store)
        .expect("golden harness should keep belief stores writable");
    commit_txn(txn, event_log);
}

pub fn seed_belief_from_world(
    world: &mut World,
    event_log: &mut EventLog,
    agent: EntityId,
    subject: EntityId,
    observed_tick: Tick,
    source: PerceptionSource,
) -> BelievedEntityState {
    let belief = build_believed_entity_state(world, subject, observed_tick, source)
        .expect("golden harness should only seed beliefs for observable subjects");
    seed_belief(world, event_log, agent, subject, belief.clone());
    belief
}

pub fn seed_told_belief_memory(
    world: &mut World,
    event_log: &mut EventLog,
    speaker: EntityId,
    listener: EntityId,
    subject: EntityId,
    shared_state: &BelievedEntityState,
    told_tick: Tick,
) {
    let mut store = world
        .get_component_agent_belief_store(speaker)
        .cloned()
        .unwrap_or_else(AgentBeliefStore::new);
    store.record_told_belief(
        TellMemoryKey {
            counterparty: listener,
            subject,
        },
        ToldBeliefMemory {
            shared_state: to_shared_belief_snapshot(shared_state),
            told_tick,
        },
    );

    let mut txn = new_txn(world, 0);
    txn.set_component_agent_belief_store(speaker, store)
        .expect("golden harness should keep belief stores writable");
    commit_txn(txn, event_log);
}

pub fn agent_belief_about(
    world: &World,
    agent: EntityId,
    subject: EntityId,
) -> Option<&BelievedEntityState> {
    world
        .get_component_agent_belief_store(agent)?
        .get_entity(&subject)
}

pub fn agent_belief_count(world: &World, agent: EntityId) -> usize {
    world
        .get_component_agent_belief_store(agent)
        .map_or(0, |store| store.known_entities.len())
}

pub fn build_harvest_apple_recipe() -> RecipeDefinition {
    RecipeDefinition {
        name: "Harvest Apples".to_string(),
        inputs: vec![],
        outputs: vec![(CommodityKind::Apple, Quantity(2))],
        work_ticks: nz(3),
        required_workstation_tag: Some(WorkstationTag::OrchardRow),
        required_tool_kinds: vec![],
        body_cost_per_tick: BodyCostPerTick::new(pm(3), pm(2), pm(5), pm(1)),
    }
}

pub fn build_harvest_grain_recipe() -> RecipeDefinition {
    RecipeDefinition {
        name: "Harvest Grain".to_string(),
        inputs: vec![],
        outputs: vec![(CommodityKind::Grain, Quantity(2))],
        work_ticks: nz(3),
        required_workstation_tag: Some(WorkstationTag::FieldPlot),
        required_tool_kinds: vec![],
        body_cost_per_tick: BodyCostPerTick::new(pm(3), pm(2), pm(5), pm(1)),
    }
}

pub fn build_bake_bread_recipe() -> RecipeDefinition {
    RecipeDefinition {
        name: "Bake Bread".to_string(),
        inputs: vec![(CommodityKind::Firewood, Quantity(1))],
        outputs: vec![(CommodityKind::Bread, Quantity(1))],
        work_ticks: nz(3),
        required_workstation_tag: Some(WorkstationTag::Mill),
        required_tool_kinds: vec![],
        body_cost_per_tick: BodyCostPerTick::new(pm(3), pm(2), pm(5), pm(1)),
    }
}

pub fn build_recipes() -> RecipeRegistry {
    let mut recipes = RecipeRegistry::new();
    recipes.register(build_harvest_apple_recipe());
    recipes
}

pub fn build_multi_recipe_registry() -> RecipeRegistry {
    let mut recipes = RecipeRegistry::new();
    recipes.register(build_harvest_apple_recipe());
    recipes.register(build_harvest_grain_recipe());
    recipes.register(build_bake_bread_recipe());
    recipes
}

pub fn build_full_registries(
    recipes: &RecipeRegistry,
) -> (ActionDefRegistry, ActionHandlerRegistry) {
    let registries = build_full_action_registries(recipes).unwrap();
    (registries.defs, registries.handlers)
}

pub fn default_combat_profile() -> CombatProfile {
    CombatProfile::new(
        pm(1000), // wound_capacity
        pm(700),  // incapacitation_threshold
        pm(500),  // attack_skill
        pm(500),  // guard_skill
        pm(80),   // defend_bonus
        pm(25),   // natural_clot_resistance
        pm(18),   // natural_recovery_rate
        pm(120),  // unarmed_wound_severity
        pm(35),   // unarmed_bleed_rate
        nz(6),    // unarmed_attack_ticks
    )
}

/// Create a fully-equipped agent at a given place with specified needs.
pub fn seed_agent(
    world: &mut World,
    event_log: &mut EventLog,
    name: &str,
    place: EntityId,
    needs: HomeostaticNeeds,
    metabolism: MetabolismProfile,
    utility: worldwake_core::UtilityProfile,
) -> EntityId {
    seed_agent_with_recipes(
        world,
        event_log,
        name,
        place,
        needs,
        metabolism,
        utility,
        KnownRecipes::with([RecipeId(0)]),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn seed_agent_with_recipes(
    world: &mut World,
    event_log: &mut EventLog,
    name: &str,
    place: EntityId,
    needs: HomeostaticNeeds,
    metabolism: MetabolismProfile,
    utility: worldwake_core::UtilityProfile,
    known_recipes: KnownRecipes,
) -> EntityId {
    let mut txn = new_txn(world, 0);
    let agent = txn.create_agent(name, ControlSource::Ai).unwrap();
    txn.set_ground_location(agent, place).unwrap();
    txn.set_component_homeostatic_needs(agent, needs).unwrap();
    txn.set_component_deprivation_exposure(agent, DeprivationExposure::default())
        .unwrap();
    txn.set_component_drive_thresholds(agent, DriveThresholds::default())
        .unwrap();
    txn.set_component_metabolism_profile(agent, metabolism)
        .unwrap();
    txn.set_component_utility_profile(agent, utility).unwrap();
    txn.set_component_combat_profile(agent, default_combat_profile())
        .unwrap();
    txn.set_component_wound_list(agent, WoundList::default())
        .unwrap();
    txn.set_component_blocked_intent_memory(agent, BlockedIntentMemory::default())
        .unwrap();
    txn.set_component_carry_capacity(agent, CarryCapacity(LoadUnits(50)))
        .unwrap();
    txn.set_component_known_recipes(agent, known_recipes)
        .unwrap();
    commit_txn(txn, event_log);
    agent
}

/// Give an agent possession of a commodity lot at the same place.
pub fn give_commodity(
    world: &mut World,
    event_log: &mut EventLog,
    agent: EntityId,
    place: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
) -> EntityId {
    let mut txn = new_txn(world, 0);
    let lot = txn.create_item_lot(commodity, quantity).unwrap();
    txn.set_ground_location(lot, place).unwrap();
    txn.set_possessor(lot, agent).unwrap();
    commit_txn(txn, event_log);
    lot
}

pub fn set_queue_patience(
    world: &mut World,
    event_log: &mut EventLog,
    agent: EntityId,
    queue_patience_ticks: Option<NonZeroU32>,
) {
    let mut txn = new_txn(world, 0);
    txn.set_component_facility_queue_disposition_profile(
        agent,
        FacilityQueueDispositionProfile {
            queue_patience_ticks,
        },
    )
    .unwrap();
    commit_txn(txn, event_log);
}

/// Place a workstation+resource-source entity at a location.
/// For harvest recipes (no inputs), the workstation entity itself must carry
/// the `ResourceSource` component — candidate generation checks
/// `view.resource_source(workstation)`.
pub fn place_workstation_with_source(
    world: &mut World,
    event_log: &mut EventLog,
    place: EntityId,
    tag: WorkstationTag,
    source: ResourceSource,
    ownership_policy: ProductionOutputOwner,
) -> EntityId {
    let mut txn = new_txn(world, 0);
    let ws = txn.create_entity(EntityKind::Facility);
    txn.set_ground_location(ws, place).unwrap();
    txn.set_component_workstation_marker(ws, WorkstationMarker(tag))
        .unwrap();
    txn.set_component_resource_source(ws, source).unwrap();
    txn.set_component_production_output_ownership_policy(
        ws,
        ProductionOutputOwnershipPolicy {
            output_owner: ownership_policy,
        },
    )
    .unwrap();
    commit_txn(txn, event_log);
    ws
}

pub fn place_exclusive_workstation_with_source(
    world: &mut World,
    event_log: &mut EventLog,
    place: EntityId,
    tag: WorkstationTag,
    source: ResourceSource,
    grant_hold_ticks: NonZeroU32,
    ownership_policy: ProductionOutputOwner,
) -> EntityId {
    let mut txn = new_txn(world, 0);
    let ws = txn.create_entity(EntityKind::Facility);
    txn.set_ground_location(ws, place).unwrap();
    txn.set_component_workstation_marker(ws, WorkstationMarker(tag))
        .unwrap();
    txn.set_component_resource_source(ws, source).unwrap();
    txn.set_component_exclusive_facility_policy(ws, ExclusiveFacilityPolicy { grant_hold_ticks })
        .unwrap();
    txn.set_component_facility_use_queue(ws, FacilityUseQueue::default())
        .unwrap();
    txn.set_component_production_output_ownership_policy(
        ws,
        ProductionOutputOwnershipPolicy {
            output_owner: ownership_policy,
        },
    )
    .unwrap();
    commit_txn(txn, event_log);
    ws
}

pub fn place_workstation(
    world: &mut World,
    event_log: &mut EventLog,
    place: EntityId,
    tag: WorkstationTag,
    ownership_policy: ProductionOutputOwner,
) -> EntityId {
    let mut txn = new_txn(world, 0);
    let ws = txn.create_entity(EntityKind::Facility);
    txn.set_ground_location(ws, place).unwrap();
    txn.set_component_workstation_marker(ws, WorkstationMarker(tag))
        .unwrap();
    txn.set_component_production_output_ownership_policy(
        ws,
        ProductionOutputOwnershipPolicy {
            output_owner: ownership_policy,
        },
    )
    .unwrap();
    commit_txn(txn, event_log);
    ws
}

pub fn add_hostility(
    world: &mut World,
    event_log: &mut EventLog,
    subject: EntityId,
    target: EntityId,
) {
    let mut txn = new_txn(world, 0);
    txn.add_hostility(subject, target).unwrap();
    commit_txn(txn, event_log);
}

// ---------------------------------------------------------------------------
// Office / Faction / Political helpers
// ---------------------------------------------------------------------------

/// Create a vacant Office entity with `OfficeData` at a jurisdiction.
pub fn seed_office(
    world: &mut World,
    event_log: &mut EventLog,
    title: &str,
    jurisdiction: EntityId,
    succession_law: SuccessionLaw,
    succession_period_ticks: u64,
    eligibility_rules: Vec<EligibilityRule>,
) -> EntityId {
    let mut txn = new_txn(world, 0);
    let office = txn.create_office(title).unwrap();
    txn.set_component_office_data(
        office,
        OfficeData {
            title: title.to_string(),
            jurisdiction,
            succession_law,
            eligibility_rules,
            succession_period_ticks,
            vacancy_since: Some(Tick(0)),
        },
    )
    .unwrap();
    commit_txn(txn, event_log);
    office
}

/// Create a Faction entity with `FactionData`.
pub fn seed_faction(
    world: &mut World,
    event_log: &mut EventLog,
    name: &str,
    purpose: FactionPurpose,
) -> EntityId {
    let mut txn = new_txn(world, 0);
    let faction = txn.create_faction(name).unwrap();
    txn.set_component_faction_data(
        faction,
        FactionData {
            name: name.to_string(),
            purpose,
        },
    )
    .unwrap();
    commit_txn(txn, event_log);
    faction
}

/// Add `member_of` relation between an agent and a faction.
pub fn add_faction_membership(
    world: &mut World,
    event_log: &mut EventLog,
    agent: EntityId,
    faction: EntityId,
) {
    let mut txn = new_txn(world, 0);
    txn.add_member(agent, faction).unwrap();
    commit_txn(txn, event_log);
}

/// Seed loyalty relation between two agents.
pub fn set_loyalty(
    world: &mut World,
    event_log: &mut EventLog,
    subject: EntityId,
    target: EntityId,
    value: Permille,
) {
    let mut txn = new_txn(world, 0);
    txn.set_loyalty(subject, target, value).unwrap();
    commit_txn(txn, event_log);
}

/// Pre-declare support for a candidate at an office.
pub fn declare_support(
    world: &mut World,
    event_log: &mut EventLog,
    supporter: EntityId,
    office: EntityId,
    candidate: EntityId,
) {
    let mut txn = new_txn(world, 0);
    txn.declare_support(supporter, office, candidate).unwrap();
    commit_txn(txn, event_log);
}

/// Update an agent's `UtilityProfile.courage` field.
pub fn set_courage(world: &mut World, event_log: &mut EventLog, agent: EntityId, value: Permille) {
    let mut profile = world
        .get_component_utility_profile(agent)
        .cloned()
        .expect("agent should have a UtilityProfile");
    profile.courage = value;
    let mut txn = new_txn(world, 0);
    txn.set_component_utility_profile(agent, profile).unwrap();
    commit_txn(txn, event_log);
}

/// Create a `UtilityProfile` with a high enterprise weight for political goal
/// generation. All other weights use defaults.
pub fn enterprise_weighted_utility(enterprise: Permille) -> worldwake_core::UtilityProfile {
    worldwake_core::UtilityProfile {
        enterprise_weight: enterprise,
        ..worldwake_core::UtilityProfile::default()
    }
}

// ---------------------------------------------------------------------------
// GoldenHarness
// ---------------------------------------------------------------------------

pub struct GoldenHarness {
    pub world: World,
    pub event_log: EventLog,
    pub scheduler: Scheduler,
    pub controller: ControllerState,
    pub rng: DeterministicRng,
    pub defs: ActionDefRegistry,
    pub handlers: ActionHandlerRegistry,
    pub recipes: RecipeRegistry,
    pub driver: AgentTickDriver,
    pub action_trace: Option<ActionTraceSink>,
    pub politics_trace: Option<PoliticalTraceSink>,
}

impl GoldenHarness {
    pub fn new(seed: Seed) -> Self {
        Self::with_recipes(seed, build_recipes())
    }

    pub fn with_recipes(seed: Seed, recipes: RecipeRegistry) -> Self {
        let world = World::new(build_prototype_world()).unwrap();
        let (defs, handlers) = build_full_registries(&recipes);

        Self {
            world,
            event_log: EventLog::new(),
            scheduler: Scheduler::new(SystemManifest::canonical()),
            controller: ControllerState::new(),
            rng: DeterministicRng::new(seed),
            defs,
            handlers,
            recipes,
            driver: AgentTickDriver::new(PlanningBudget::default()),
            action_trace: None,
            politics_trace: None,
        }
    }

    pub fn enable_action_tracing(&mut self) {
        self.action_trace = Some(ActionTraceSink::new());
    }

    pub fn action_trace_sink(&self) -> Option<&ActionTraceSink> {
        self.action_trace.as_ref()
    }

    pub fn enable_politics_tracing(&mut self) {
        self.politics_trace = Some(PoliticalTraceSink::new());
    }

    pub fn politics_trace_sink(&self) -> Option<&PoliticalTraceSink> {
        self.politics_trace.as_ref()
    }

    pub fn step_once(&mut self) -> TickStepResult {
        let mut controllers = AutonomousControllerRuntime::new(vec![&mut self.driver]);
        step_tick(
            &mut self.world,
            &mut self.event_log,
            &mut self.scheduler,
            &mut self.controller,
            &mut self.rng,
            TickStepServices {
                action_defs: &self.defs,
                action_handlers: &self.handlers,
                recipe_registry: &self.recipes,
                systems: &dispatch_table(),
                input_producer: Some(&mut controllers),
                action_trace: self.action_trace.as_mut(),
                politics_trace: self.politics_trace.as_mut(),
            },
        )
        .unwrap()
    }

    pub fn snapshot_state(&self) -> SimulationState {
        let replay_state = ReplayState::new(
            hash_serializable(&(
                &self.world,
                &self.event_log,
                &self.scheduler,
                &self.recipes,
                &self.controller,
                &self.rng,
            ))
            .expect("golden harness runtime roots should hash canonically"),
            self.rng.seed(),
            self.scheduler.current_tick(),
            ReplayRecordingConfig::disabled(),
        );

        SimulationState::new(
            self.world.clone(),
            self.event_log.clone(),
            self.scheduler.clone(),
            self.recipes.clone(),
            replay_state,
            self.controller.clone(),
            self.rng.clone(),
        )
    }

    pub fn save_load_roundtrip(&self) -> SimulationState {
        load_from_bytes(
            &save_to_bytes(&self.snapshot_state())
                .expect("golden harness simulation state should serialize"),
        )
        .expect("golden harness simulation state should deserialize")
    }

    pub fn from_simulation_state(state: &SimulationState) -> Self {
        let recipes = state.recipe_registry().clone();
        let (defs, handlers) = build_full_registries(&recipes);

        Self {
            world: state.world().clone(),
            event_log: state.event_log().clone(),
            scheduler: state.scheduler().clone(),
            controller: state.controller_state().clone(),
            rng: state.rng_state().clone(),
            defs,
            handlers,
            recipes,
            driver: AgentTickDriver::new(PlanningBudget::default()),
            action_trace: None,
            politics_trace: None,
        }
    }

    pub fn agent_hunger(&self, agent: EntityId) -> Permille {
        self.world
            .get_component_homeostatic_needs(agent)
            .map_or(pm(0), |n| n.hunger)
    }

    pub fn agent_thirst(&self, agent: EntityId) -> Permille {
        self.world
            .get_component_homeostatic_needs(agent)
            .map_or(pm(0), |n| n.thirst)
    }

    pub fn agent_bladder(&self, agent: EntityId) -> Permille {
        self.world
            .get_component_homeostatic_needs(agent)
            .map_or(pm(0), |n| n.bladder)
    }

    pub fn agent_dirtiness(&self, agent: EntityId) -> Permille {
        self.world
            .get_component_homeostatic_needs(agent)
            .map_or(pm(0), |n| n.dirtiness)
    }

    pub fn agent_wound_load(&self, agent: EntityId) -> u32 {
        self.world
            .get_component_wound_list(agent)
            .map_or(0, WoundList::wound_load)
    }

    pub fn agent_is_dead(&self, agent: EntityId) -> bool {
        self.world.get_component_dead_at(agent).is_some()
    }

    pub fn agent_has_active_action(&self, agent: EntityId) -> bool {
        self.scheduler
            .active_actions()
            .values()
            .any(|instance| instance.actor == agent)
    }

    pub fn agent_active_action_name(&self, agent: EntityId) -> Option<&str> {
        self.scheduler
            .active_actions()
            .values()
            .find(|instance| instance.actor == agent)
            .and_then(|instance| self.defs.get(instance.def_id))
            .map(|def| def.name.as_str())
    }

    pub fn agent_combat_stance(&self, agent: EntityId) -> Option<CombatStance> {
        self.world.get_component_combat_stance(agent).copied()
    }

    pub fn agent_commodity_qty(&self, agent: EntityId, kind: CommodityKind) -> Quantity {
        self.world.controlled_commodity_quantity(agent, kind)
    }

    pub fn agent_active_loot_target(&self, agent: EntityId) -> Option<EntityId> {
        self.scheduler
            .active_actions()
            .values()
            .find(|instance| instance.actor == agent)
            .and_then(|instance| instance.payload.as_loot())
            .map(|loot| loot.target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use worldwake_sim::{PerAgentBeliefView, RuntimeBeliefView};

    fn emit_test_event(
        log: &mut EventLog,
        tick: u64,
        tags: &[EventTag],
        state_deltas: Vec<StateDelta>,
    ) -> EventId {
        log.emit(worldwake_core::PendingEvent::from_payload(
            worldwake_core::EventPayload {
                tick: Tick(tick),
                cause: CauseRef::Bootstrap,
                actor_id: None,
                target_ids: Vec::new(),
                evidence: Vec::new(),
                place_id: None,
                state_deltas,
                observed_entities: BTreeMap::new(),
                visibility: VisibilitySpec::Hidden,
                witness_data: WitnessData::default(),
                tags: tags.iter().copied().collect::<BTreeSet<_>>(),
            },
        ))
    }

    #[test]
    fn setup_does_not_seed_remote_beliefs_by_default() {
        let mut h = GoldenHarness::new(Seed([41; 32]));
        let observer = seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Observer",
            VILLAGE_SQUARE,
            HomeostaticNeeds::default(),
            MetabolismProfile::default(),
            worldwake_core::UtilityProfile::default(),
        );
        let remote = seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Remote",
            ORCHARD_FARM,
            HomeostaticNeeds::default(),
            MetabolismProfile::default(),
            worldwake_core::UtilityProfile::default(),
        );

        let view = PerAgentBeliefView::from_world(observer, &h.world);
        assert_eq!(
            view.effective_place(remote),
            None,
            "default setup should not leak remote entity knowledge"
        );
    }

    #[test]
    fn explicit_local_belief_seeding_is_bounded_to_colocated_entities() {
        let mut h = GoldenHarness::new(Seed([42; 32]));
        let observer = seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Observer",
            VILLAGE_SQUARE,
            HomeostaticNeeds::default(),
            MetabolismProfile::default(),
            worldwake_core::UtilityProfile::default(),
        );
        let local = seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Local",
            VILLAGE_SQUARE,
            HomeostaticNeeds::default(),
            MetabolismProfile::default(),
            worldwake_core::UtilityProfile::default(),
        );
        let remote = seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Remote",
            ORCHARD_FARM,
            HomeostaticNeeds::default(),
            MetabolismProfile::default(),
            worldwake_core::UtilityProfile::default(),
        );

        seed_actor_local_beliefs(
            &mut h.world,
            &mut h.event_log,
            observer,
            Tick(0),
            PerceptionSource::DirectObservation,
        );

        let view = PerAgentBeliefView::from_world(observer, &h.world);
        assert_eq!(view.effective_place(local), Some(VILLAGE_SQUARE));
        assert_eq!(
            view.effective_place(remote),
            None,
            "bounded local seeding must not leak remote knowledge"
        );
    }

    #[test]
    fn profile_override_helpers_update_agent_components() {
        let mut h = GoldenHarness::new(Seed([43; 32]));
        let agent = seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Talker",
            VILLAGE_SQUARE,
            HomeostaticNeeds::default(),
            MetabolismProfile::default(),
            worldwake_core::UtilityProfile::default(),
        );

        let tell_profile = TellProfile {
            max_tell_candidates: 2,
            max_relay_chain_len: 1,
            acceptance_fidelity: pm(250),
            ..TellProfile::default()
        };
        let perception_profile = PerceptionProfile {
            memory_capacity: 5,
            memory_retention_ticks: 17,
            observation_fidelity: pm(600),
            confidence_policy: worldwake_core::BeliefConfidencePolicy::default(),
        };

        set_agent_tell_profile(&mut h.world, &mut h.event_log, agent, tell_profile);
        set_agent_perception_profile(&mut h.world, &mut h.event_log, agent, perception_profile);

        assert_eq!(
            h.world.get_component_tell_profile(agent),
            Some(&tell_profile)
        );
        assert_eq!(
            h.world.get_component_perception_profile(agent),
            Some(&perception_profile)
        );
    }

    #[test]
    fn seed_belief_accessors_and_count_reflect_seeded_state() {
        let mut h = GoldenHarness::new(Seed([44; 32]));
        let agent = seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Observer",
            VILLAGE_SQUARE,
            HomeostaticNeeds::default(),
            MetabolismProfile::default(),
            worldwake_core::UtilityProfile::default(),
        );
        let subject = seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Subject",
            ORCHARD_FARM,
            HomeostaticNeeds::default(),
            MetabolismProfile::default(),
            worldwake_core::UtilityProfile::default(),
        );

        assert_eq!(agent_belief_count(&h.world, agent), 0);

        let mut belief = build_believed_entity_state(
            &h.world,
            subject,
            Tick(5),
            PerceptionSource::Report {
                from: agent,
                chain_len: 1,
            },
        )
        .expect("subject should produce a belief snapshot");
        belief.last_known_place = Some(ORCHARD_FARM);
        belief.last_known_inventory = BTreeMap::from([(CommodityKind::Apple, Quantity(7))]);

        seed_belief(
            &mut h.world,
            &mut h.event_log,
            agent,
            subject,
            belief.clone(),
        );

        assert_eq!(agent_belief_count(&h.world, agent), 1);
        assert_eq!(agent_belief_about(&h.world, agent, subject), Some(&belief));
    }

    #[test]
    fn seed_belief_replaces_same_subject_when_tick_is_equal_or_newer() {
        let mut h = GoldenHarness::new(Seed([45; 32]));
        let agent = seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Observer",
            VILLAGE_SQUARE,
            HomeostaticNeeds::default(),
            MetabolismProfile::default(),
            worldwake_core::UtilityProfile::default(),
        );
        let subject = seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Subject",
            VILLAGE_SQUARE,
            HomeostaticNeeds::default(),
            MetabolismProfile::default(),
            worldwake_core::UtilityProfile::default(),
        );

        let mut earlier = build_believed_entity_state(
            &h.world,
            subject,
            Tick(3),
            PerceptionSource::DirectObservation,
        )
        .expect("subject should produce a belief snapshot");
        earlier.last_known_place = Some(VILLAGE_SQUARE);
        earlier.last_known_inventory = BTreeMap::from([(CommodityKind::Apple, Quantity(1))]);
        seed_belief(
            &mut h.world,
            &mut h.event_log,
            agent,
            subject,
            earlier.clone(),
        );

        let mut newer = earlier.clone();
        newer.observed_tick = Tick(4);
        newer.last_known_place = Some(ORCHARD_FARM);
        newer.last_known_inventory = BTreeMap::from([(CommodityKind::Apple, Quantity(9))]);
        seed_belief(
            &mut h.world,
            &mut h.event_log,
            agent,
            subject,
            newer.clone(),
        );

        assert_eq!(agent_belief_about(&h.world, agent, subject), Some(&newer));
    }

    #[test]
    fn seed_belief_preserves_newer_existing_belief_against_older_input() {
        let mut h = GoldenHarness::new(Seed([46; 32]));
        let agent = seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Observer",
            VILLAGE_SQUARE,
            HomeostaticNeeds::default(),
            MetabolismProfile::default(),
            worldwake_core::UtilityProfile::default(),
        );
        let subject = seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Subject",
            ORCHARD_FARM,
            HomeostaticNeeds::default(),
            MetabolismProfile::default(),
            worldwake_core::UtilityProfile::default(),
        );

        let mut newer = build_believed_entity_state(
            &h.world,
            subject,
            Tick(8),
            PerceptionSource::DirectObservation,
        )
        .expect("subject should produce a belief snapshot");
        newer.last_known_place = Some(ORCHARD_FARM);
        newer.last_known_inventory = BTreeMap::from([(CommodityKind::Apple, Quantity(8))]);
        seed_belief(
            &mut h.world,
            &mut h.event_log,
            agent,
            subject,
            newer.clone(),
        );

        let mut older = newer.clone();
        older.observed_tick = Tick(7);
        older.last_known_place = Some(VILLAGE_SQUARE);
        older.last_known_inventory = BTreeMap::from([(CommodityKind::Apple, Quantity(2))]);
        seed_belief(&mut h.world, &mut h.event_log, agent, subject, older);

        assert_eq!(agent_belief_about(&h.world, agent, subject), Some(&newer));
    }

    #[test]
    fn seed_belief_from_world_builds_and_stores_snapshot() {
        let mut h = GoldenHarness::new(Seed([47; 32]));
        let agent = seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Observer",
            VILLAGE_SQUARE,
            HomeostaticNeeds::default(),
            MetabolismProfile::default(),
            worldwake_core::UtilityProfile::default(),
        );
        let subject = seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Subject",
            ORCHARD_FARM,
            HomeostaticNeeds::default(),
            MetabolismProfile::default(),
            worldwake_core::UtilityProfile::default(),
        );

        let belief = seed_belief_from_world(
            &mut h.world,
            &mut h.event_log,
            agent,
            subject,
            Tick(6),
            PerceptionSource::DirectObservation,
        );

        assert_eq!(belief.observed_tick, Tick(6));
        assert_eq!(belief.source, PerceptionSource::DirectObservation);
        assert_eq!(agent_belief_about(&h.world, agent, subject), Some(&belief));
    }

    #[test]
    fn seed_told_belief_memory_records_requested_entry_and_preserves_beliefs() {
        let mut h = GoldenHarness::new(Seed([48; 32]));
        let speaker = seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Speaker",
            VILLAGE_SQUARE,
            HomeostaticNeeds::default(),
            MetabolismProfile::default(),
            worldwake_core::UtilityProfile::default(),
        );
        let listener = seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Listener",
            VILLAGE_SQUARE,
            HomeostaticNeeds::default(),
            MetabolismProfile::default(),
            worldwake_core::UtilityProfile::default(),
        );
        let subject = seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Subject",
            ORCHARD_FARM,
            HomeostaticNeeds::default(),
            MetabolismProfile::default(),
            worldwake_core::UtilityProfile::default(),
        );

        let listener_belief = seed_belief_from_world(
            &mut h.world,
            &mut h.event_log,
            speaker,
            listener,
            Tick(0),
            PerceptionSource::DirectObservation,
        );
        let subject_belief = seed_belief_from_world(
            &mut h.world,
            &mut h.event_log,
            speaker,
            subject,
            Tick(1),
            PerceptionSource::DirectObservation,
        );

        seed_told_belief_memory(
            &mut h.world,
            &mut h.event_log,
            speaker,
            listener,
            subject,
            &subject_belief,
            Tick(9),
        );

        let store = h
            .world
            .get_component_agent_belief_store(speaker)
            .expect("speaker should have a belief store");
        let memory = store
            .told_beliefs
            .get(&TellMemoryKey {
                counterparty: listener,
                subject,
            })
            .expect("told memory should be recorded for the requested listener and subject");

        assert_eq!(memory.told_tick, Tick(9));
        assert_eq!(
            memory.shared_state,
            to_shared_belief_snapshot(&subject_belief)
        );
        assert_eq!(
            agent_belief_about(&h.world, speaker, listener),
            Some(&listener_belief)
        );
        assert_eq!(
            agent_belief_about(&h.world, speaker, subject),
            Some(&subject_belief)
        );
    }

    #[test]
    fn event_log_helpers_match_component_and_relation_deltas() {
        let office = EntityId {
            slot: 91,
            generation: 0,
        };
        let incumbent = EntityId {
            slot: 92,
            generation: 0,
        };
        let challenger = EntityId {
            slot: 93,
            generation: 0,
        };
        let death_tick = Tick(7);

        let mut log = EventLog::new();
        let death_event_id = emit_test_event(
            &mut log,
            death_tick.0,
            &[EventTag::Combat],
            vec![StateDelta::Component(ComponentDelta::Set {
                entity: incumbent,
                component_kind: ComponentKind::DeadAt,
                before: None,
                after: ComponentValue::DeadAt(worldwake_core::DeadAt(death_tick)),
            })],
        );
        let vacancy_relation = RelationValue::OfficeHolder {
            office,
            holder: incumbent,
        };
        let install_relation = RelationValue::OfficeHolder {
            office,
            holder: challenger,
        };
        let vacancy_event_id = emit_test_event(
            &mut log,
            8,
            &[EventTag::Political],
            vec![StateDelta::Relation(RelationDelta::Removed {
                relation_kind: worldwake_core::RelationKind::OfficeHolder,
                relation: vacancy_relation.clone(),
            })],
        );
        let install_event_id = emit_test_event(
            &mut log,
            13,
            &[EventTag::Political],
            vec![StateDelta::Relation(RelationDelta::Added {
                relation_kind: worldwake_core::RelationKind::OfficeHolder,
                relation: install_relation.clone(),
            })],
        );

        let matched_death = first_tagged_event_id_matching(&log, EventTag::Combat, |_, record| {
            event_sets_component(
                record,
                incumbent,
                ComponentKind::DeadAt,
                |after| matches!(after, ComponentValue::DeadAt(worldwake_core::DeadAt(tick)) if *tick == death_tick),
            )
        });
        let matched_vacancy =
            first_tagged_event_id_matching(&log, EventTag::Political, |_, record| {
                event_removes_relation(record, &vacancy_relation)
            });
        let matched_install =
            first_tagged_event_id_matching(&log, EventTag::Political, |_, record| {
                event_adds_relation(record, &install_relation)
            });

        assert_eq!(matched_death, Some(death_event_id));
        assert_eq!(matched_vacancy, Some(vacancy_event_id));
        assert_eq!(matched_install, Some(install_event_id));
    }

    #[test]
    fn event_log_helpers_preserve_append_order() {
        let entity = EntityId {
            slot: 101,
            generation: 0,
        };
        let mut log = EventLog::new();

        let earlier = emit_test_event(
            &mut log,
            1,
            &[EventTag::Combat],
            vec![StateDelta::Component(ComponentDelta::Set {
                entity,
                component_kind: ComponentKind::DeadAt,
                before: None,
                after: ComponentValue::DeadAt(worldwake_core::DeadAt(Tick(1))),
            })],
        );
        let later = emit_test_event(&mut log, 2, &[EventTag::Political], Vec::new());

        let matched_earlier = first_event_id_matching(&log, |event_id, record| {
            event_id == earlier
                && event_sets_component(record, entity, ComponentKind::DeadAt, |_| true)
        })
        .expect("expected to find earlier event");
        let matched_later =
            first_tagged_event_id_matching(&log, EventTag::Political, |event_id, _| {
                event_id == later
            })
            .expect("expected to find later event");

        assert_event_order(
            matched_earlier,
            matched_later,
            "append-only ordering must hold",
        );
    }
}
