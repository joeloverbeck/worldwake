use std::num::{NonZeroU32, NonZeroU64};

use worldwake_core::{
    build_prototype_world, hash_serializable, verify_live_lot_conservation, CarryCapacity,
    CauseRef, CommodityKind, CombatProfile, CombatWeaponRef, ControlSource, DeadAt, EventLog,
    LoadUnits, Quantity, Seed, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
};
use worldwake_sim::{
    record_tick_checkpoint, replay_and_verify, ActionDefId, ActionDefRegistry,
    ActionHandlerRegistry, ActionPayload, CombatActionPayload, ControllerState, DeterministicRng,
    InputKind, RecipeRegistry, ReplayRecordingConfig, ReplayState, Scheduler, SimulationState,
    SystemDispatchTable, SystemManifest, TickStepError, TickStepResult, TickStepServices, step_tick,
};
use worldwake_systems::{dispatch_table, register_attack_action, register_loot_action};

fn pm(value: u16) -> worldwake_core::Permille {
    worldwake_core::Permille::new(value).unwrap()
}

fn nz(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}

fn nz64(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).unwrap()
}

fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
    WorldTxn::new(
        world,
        Tick(tick),
        CauseRef::Bootstrap,
        None,
        None,
        VisibilitySpec::SamePlace,
        WitnessData::default(),
    )
}

fn commit_txn(txn: WorldTxn<'_>) {
    let mut log = EventLog::new();
    let _ = txn.commit(&mut log);
}

fn attacker_profile() -> CombatProfile {
    CombatProfile::new(
        pm(1000),
        pm(700),
        pm(1000),
        pm(0),
        pm(0),
        pm(0),
        pm(0),
        pm(120),
        pm(30),
        nz(6),
    )
}

fn fragile_target_profile() -> CombatProfile {
    CombatProfile::new(
        pm(150),
        pm(100),
        pm(0),
        pm(0),
        pm(0),
        pm(0),
        pm(0),
        pm(80),
        pm(10),
        nz(6),
    )
}

fn spawn_guard_with_profile(
    world: &mut World,
    tick: u64,
    control: ControlSource,
    profile: CombatProfile,
) -> worldwake_core::EntityId {
    let place = world.topology().place_ids().next().unwrap();
    let mut txn = new_txn(world, tick);
    let actor = txn.create_agent("Guard", control).unwrap();
    txn.set_ground_location(actor, place).unwrap();
    txn.set_component_combat_profile(actor, profile).unwrap();
    txn.set_component_wound_list(actor, worldwake_core::WoundList::default())
        .unwrap();
    txn.set_component_carry_capacity(actor, CarryCapacity(LoadUnits(20)))
        .unwrap();
    commit_txn(txn);
    actor
}

fn add_carried_lot(
    world: &mut World,
    actor: worldwake_core::EntityId,
    tick: u64,
    commodity: CommodityKind,
    quantity: u32,
) -> worldwake_core::EntityId {
    let place = world.effective_place(actor).unwrap();
    let mut txn = new_txn(world, tick);
    let lot = txn.create_item_lot(commodity, Quantity(quantity)).unwrap();
    txn.set_ground_location(lot, place).unwrap();
    txn.set_possessor(lot, actor).unwrap();
    commit_txn(txn);
    lot
}

struct CombatHarness {
    world: World,
    event_log: EventLog,
    scheduler: Scheduler,
    controller: ControllerState,
    rng: DeterministicRng,
    recipe_registry: RecipeRegistry,
    replay_state: ReplayState,
    defs: ActionDefRegistry,
    handlers: ActionHandlerRegistry,
    systems: SystemDispatchTable,
    attacker: worldwake_core::EntityId,
    target: worldwake_core::EntityId,
}

impl CombatHarness {
    fn new(replay_config: ReplayRecordingConfig) -> Self {
        let mut world = World::new(build_prototype_world()).unwrap();
        let attacker =
            spawn_guard_with_profile(&mut world, 1, ControlSource::Ai, attacker_profile());
        let target =
            spawn_guard_with_profile(&mut world, 2, ControlSource::Ai, fragile_target_profile());
        let _sword = add_carried_lot(&mut world, attacker, 3, CommodityKind::Sword, 1);
        let _bread = add_carried_lot(&mut world, target, 4, CommodityKind::Bread, 3);

        let event_log = EventLog::new();
        let scheduler = Scheduler::new(SystemManifest::canonical());
        let recipe_registry = RecipeRegistry::new();
        let controller = ControllerState::with_entity(attacker);
        let rng = DeterministicRng::new(Seed([41; 32]));
        let initial_hash = hash_serializable(&(
            &world,
            &event_log,
            &scheduler,
            &recipe_registry,
            &controller,
            &rng,
        ))
        .unwrap();
        let replay = ReplayState::new(initial_hash, Seed([41; 32]), Tick(0), replay_config);

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let _ = register_attack_action(&mut defs, &mut handlers);
        let _ = register_loot_action(&mut defs, &mut handlers);

        Self {
            world,
            event_log,
            scheduler,
            controller,
            rng,
            recipe_registry,
            replay_state: replay,
            defs,
            handlers,
            systems: dispatch_table(),
            attacker,
            target,
        }
    }

    fn services(&self) -> TickStepServices<'_> {
        TickStepServices {
            action_defs: &self.defs,
            action_handlers: &self.handlers,
            systems: &self.systems,
        }
    }

    fn action_def_id(&self, name: &str) -> ActionDefId {
        self.defs
            .iter()
            .find(|def| def.name == name)
            .map_or_else(|| panic!("missing action def {name}"), |def| def.id)
    }

    fn queue_attack(&mut self, actor: worldwake_core::EntityId, target: worldwake_core::EntityId) {
        let def_id = self.action_def_id("attack");
        let tick = self.scheduler.current_tick();
        self.scheduler.input_queue_mut().enqueue(
            tick,
            InputKind::RequestAction {
                actor,
                def_id,
                targets: vec![target],
                payload_override: Some(ActionPayload::Combat(CombatActionPayload {
                    target,
                    weapon: CombatWeaponRef::Commodity(CommodityKind::Sword),
                })),
            },
        );
    }

    fn queue_loot(&mut self, actor: worldwake_core::EntityId, target: worldwake_core::EntityId) {
        let def_id = self.action_def_id("loot");
        let tick = self.scheduler.current_tick();
        self.scheduler.input_queue_mut().enqueue(
            tick,
            InputKind::RequestAction {
                actor,
                def_id,
                targets: vec![target],
                payload_override: None,
            },
        );
    }

    fn queue_attack_recorded(
        &mut self,
        actor: worldwake_core::EntityId,
        target: worldwake_core::EntityId,
    ) {
        let def_id = self.action_def_id("attack");
        let tick = self.scheduler.current_tick();
        let input = {
            self.scheduler.input_queue_mut().enqueue(
                tick,
                InputKind::RequestAction {
                    actor,
                    def_id,
                    targets: vec![target],
                    payload_override: Some(ActionPayload::Combat(CombatActionPayload {
                        target,
                        weapon: CombatWeaponRef::Commodity(CommodityKind::Sword),
                    })),
                },
            )
        }
        .clone();
        self.replay_state.record_input(input).unwrap();
    }

    fn queue_loot_recorded(
        &mut self,
        actor: worldwake_core::EntityId,
        target: worldwake_core::EntityId,
    ) {
        let def_id = self.action_def_id("loot");
        let tick = self.scheduler.current_tick();
        let input = {
            self.scheduler.input_queue_mut().enqueue(
                tick,
                InputKind::RequestAction {
                    actor,
                    def_id,
                    targets: vec![target],
                    payload_override: None,
                },
            )
        }
        .clone();
        self.replay_state.record_input(input).unwrap();
    }

    fn step_once(&mut self) -> Result<TickStepResult, TickStepError> {
        let services = TickStepServices {
            action_defs: &self.defs,
            action_handlers: &self.handlers,
            systems: &self.systems,
        };
        step_tick(
            &mut self.world,
            &mut self.event_log,
            &mut self.scheduler,
            &mut self.controller,
            &mut self.rng,
            services,
        )
    }

    fn step_once_recorded(&mut self) -> TickStepResult {
        let result = self.step_once().unwrap();
        let mut state = self.snapshot_state(self.replay_state.clone());
        let _ = record_tick_checkpoint(&mut state, result.tick).unwrap();
        self.replay_state = state.replay_state().clone();
        self.replay_state
            .set_terminal_tick(self.scheduler.current_tick())
            .unwrap();
        result
    }

    fn run_until_no_active_actions(&mut self, max_ticks: u32, record: bool) {
        for _ in 0..max_ticks {
            if record {
                let _ = self.step_once_recorded();
            } else {
                let _ = self.step_once().unwrap();
            }
            if self.scheduler.active_actions().is_empty() {
                return;
            }
        }

        panic!("actions did not complete within {max_ticks} ticks");
    }

    fn snapshot_state(&self, replay_state: ReplayState) -> SimulationState {
        SimulationState::new(
            self.world.clone(),
            self.event_log.clone(),
            self.scheduler.clone(),
            self.recipe_registry.clone(),
            replay_state,
            self.controller.clone(),
            self.rng.clone(),
        )
    }
}

#[test]
fn scheduler_combat_death_and_loot_preserve_conservation() {
    let mut harness = CombatHarness::new(ReplayRecordingConfig::disabled());

    verify_live_lot_conservation(&harness.world, CommodityKind::Sword, 1).unwrap();
    verify_live_lot_conservation(&harness.world, CommodityKind::Bread, 3).unwrap();

    harness.queue_attack(harness.attacker, harness.target);
    harness.run_until_no_active_actions(6, false);

    assert_eq!(
        harness.world.get_component_dead_at(harness.target),
        Some(&DeadAt(Tick(3)))
    );

    harness.queue_loot(harness.attacker, harness.target);
    harness.run_until_no_active_actions(2, false);

    assert_eq!(
        harness
            .world
            .controlled_commodity_quantity(harness.attacker, CommodityKind::Bread),
        Quantity(3)
    );
    assert_eq!(
        harness
            .world
            .controlled_commodity_quantity(harness.target, CommodityKind::Bread),
        Quantity(0)
    );
    verify_live_lot_conservation(&harness.world, CommodityKind::Sword, 1).unwrap();
    verify_live_lot_conservation(&harness.world, CommodityKind::Bread, 3).unwrap();
}

#[test]
fn scheduler_rejects_new_attack_requests_from_dead_actors() {
    let mut harness = CombatHarness::new(ReplayRecordingConfig::disabled());
    let attack_id = harness.action_def_id("attack");

    harness.queue_attack(harness.attacker, harness.target);
    harness.run_until_no_active_actions(6, false);
    assert!(harness.world.get_component_dead_at(harness.target).is_some());

    harness.queue_attack(harness.target, harness.attacker);
    let error = harness.step_once().unwrap_err();

    assert_eq!(
        error,
        TickStepError::RequestedAffordanceUnavailable {
            actor: harness.target,
            def_id: attack_id,
            targets: vec![harness.attacker],
            payload_override: Some(ActionPayload::Combat(CombatActionPayload {
                target: harness.attacker,
                weapon: CombatWeaponRef::Commodity(CommodityKind::Sword),
            })),
        }
    );
}

#[test]
fn combat_replay_matches_recorded_scheduler_outcome() {
    let mut harness = CombatHarness::new(ReplayRecordingConfig::every(nz64(1)));
    let mut initial_state = harness.snapshot_state(harness.replay_state.clone());

    harness.queue_attack_recorded(harness.attacker, harness.target);
    harness.run_until_no_active_actions(6, true);
    harness.queue_loot_recorded(harness.attacker, harness.target);
    harness.run_until_no_active_actions(2, true);

    let expected_final_hash = harness
        .snapshot_state(harness.replay_state.clone())
        .replay_bootstrap_hash()
        .unwrap();
    *initial_state.replay_state_mut() = harness.replay_state.clone();

    let actual_final_hash = replay_and_verify(&initial_state, harness.services()).unwrap();

    assert_eq!(actual_final_hash, expected_final_hash);
}
