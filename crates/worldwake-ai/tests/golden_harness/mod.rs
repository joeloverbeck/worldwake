//! Shared infrastructure for the golden end-to-end test suite.
//!
//! Provides `GoldenHarness`, helper functions, recipe builders, and world
//! setup utilities used by all golden test files.

// Each test binary uses a different subset of harness items.
#![allow(dead_code)]

use std::num::NonZeroU32;

use worldwake_ai::{AgentTickDriver, PlanningBudget};
use worldwake_core::{
    build_believed_entity_state, build_prototype_world, hash_serializable, prototype_place_entity,
    AgentBeliefStore, BlockedIntentMemory, BodyCostPerTick, CarryCapacity, CauseRef, CombatProfile,
    CombatStance, CommodityKind, ControlSource, DeprivationExposure, DriveThresholds, EntityId,
    EntityKind, EventLog, ExclusiveFacilityPolicy, FacilityQueueDispositionProfile,
    FacilityUseQueue, HomeostaticNeeds, KnownRecipes, LoadUnits, MetabolismProfile,
    PerceptionSource, Permille, PrototypePlace, Quantity, RecipeId, ResourceSource, Seed, Tick,
    VisibilitySpec, WitnessData, WorkstationMarker, WorkstationTag, World, WorldTxn, WoundList,
};
use worldwake_sim::{
    load_from_bytes, save_to_bytes, step_tick, ActionDefRegistry, ActionHandlerRegistry,
    AutonomousControllerRuntime, ControllerState, DeterministicRng, RecipeDefinition,
    RecipeRegistry, ReplayRecordingConfig, ReplayState, Scheduler, SimulationState, SystemManifest,
    TickStepResult, TickStepServices,
};
use worldwake_systems::{build_full_action_registries, dispatch_table};

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
) -> EntityId {
    let mut txn = new_txn(world, 0);
    let ws = txn.create_entity(EntityKind::Facility);
    txn.set_ground_location(ws, place).unwrap();
    txn.set_component_workstation_marker(ws, WorkstationMarker(tag))
        .unwrap();
    txn.set_component_resource_source(ws, source).unwrap();
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
    commit_txn(txn, event_log);
    ws
}

pub fn place_workstation(
    world: &mut World,
    event_log: &mut EventLog,
    place: EntityId,
    tag: WorkstationTag,
) -> EntityId {
    let mut txn = new_txn(world, 0);
    let ws = txn.create_entity(EntityKind::Facility);
    txn.set_ground_location(ws, place).unwrap();
    txn.set_component_workstation_marker(ws, WorkstationMarker(tag))
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
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldwake_sim::{PerAgentBeliefView, RuntimeBeliefView};

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
}
