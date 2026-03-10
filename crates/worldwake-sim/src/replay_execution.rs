use crate::{
    step_tick, InputQueueError, ReplayCheckpoint, ReplayStateError, SimulationState,
    TickStepServices,
};
use std::fmt;
use worldwake_core::{hash_event_log, hash_world, CanonicalError, StateHash, Tick};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayError {
    InitialStateHashMismatch {
        expected: StateHash,
        actual: StateHash,
    },
    EventLogCheckpointMismatch {
        tick: Tick,
        expected: StateHash,
        actual: StateHash,
    },
    WorldCheckpointMismatch {
        tick: Tick,
        expected: StateHash,
        actual: StateHash,
    },
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitialStateHashMismatch { expected, actual } => write!(
                f,
                "initial replay roots hash mismatch: expected {expected}, got {actual}"
            ),
            Self::EventLogCheckpointMismatch {
                tick,
                expected,
                actual,
            } => write!(
                f,
                "replay event-log checkpoint mismatch at tick {tick}: expected {expected}, got {actual}"
            ),
            Self::WorldCheckpointMismatch {
                tick,
                expected,
                actual,
            } => write!(
                f,
                "replay world checkpoint mismatch at tick {tick}: expected {expected}, got {actual}"
            ),
        }
    }
}

impl std::error::Error for ReplayError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayCheckpointError {
    Canonical(CanonicalError),
    InputQueue(InputQueueError),
    ReplayState(ReplayStateError),
}

impl fmt::Display for ReplayCheckpointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canonical(source) => {
                write!(f, "failed to hash replay checkpoint state: {source}")
            }
            Self::InputQueue(source) => {
                write!(f, "failed to rebuild scheduler input queue: {source}")
            }
            Self::ReplayState(source) => write!(f, "failed to append replay checkpoint: {source}"),
        }
    }
}

impl std::error::Error for ReplayCheckpointError {}

impl From<CanonicalError> for ReplayCheckpointError {
    fn from(value: CanonicalError) -> Self {
        Self::Canonical(value)
    }
}

impl From<ReplayStateError> for ReplayCheckpointError {
    fn from(value: ReplayStateError) -> Self {
        Self::ReplayState(value)
    }
}

impl From<InputQueueError> for ReplayCheckpointError {
    fn from(value: InputQueueError) -> Self {
        Self::InputQueue(value)
    }
}

pub fn replay_and_verify(
    initial_state: &SimulationState,
    services: TickStepServices<'_>,
) -> Result<StateHash, Vec<ReplayError>> {
    let mut errors = Vec::new();
    let initial_hash = match initial_state.replay_bootstrap_hash() {
        Ok(hash) => hash,
        Err(source) => panic!("replay initial-state hashing failed unexpectedly: {source}"),
    };
    if initial_hash != initial_state.replay_state().initial_state_hash() {
        errors.push(ReplayError::InitialStateHashMismatch {
            expected: initial_state.replay_state().initial_state_hash(),
            actual: initial_hash,
        });
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut state = initial_state.clone();
    let replay_inputs = state.replay_state().input_log().to_vec();

    state
        .scheduler_mut()
        .input_queue_mut()
        .replace_with_recorded(&replay_inputs)
        .expect("replay input log must remain reconstructible");

    let mut checkpoint_index = 0usize;
    while state.scheduler().current_tick() < state.replay_state().terminal_tick() {
        let result = {
            let (world, event_log, scheduler, controller, rng) = state.runtime_parts_mut();
            step_tick(world, event_log, scheduler, controller, rng, services)
        }
        .unwrap_or_else(|error| panic!("replay tick stepping failed unexpectedly: {error}"));

        let checkpoints = state.replay_state().checkpoints();
        while let Some(checkpoint) = checkpoints.get(checkpoint_index) {
            if checkpoint.tick != result.tick {
                break;
            }

            verify_checkpoint(checkpoint, state.world(), state.event_log(), &mut errors);
            checkpoint_index += 1;
        }
    }

    if errors.is_empty() {
        Ok(state
            .replay_bootstrap_hash()
            .expect("replay final-state hashing failed unexpectedly"))
    } else {
        Err(errors)
    }
}

pub fn record_tick_checkpoint(
    state: &mut SimulationState,
    tick: Tick,
) -> Result<bool, ReplayCheckpointError> {
    if !state.replay_state().should_checkpoint(tick) {
        return Ok(false);
    }

    let event_log_hash = hash_event_log(state.event_log())?;
    let world_state_hash = hash_world(state.world())?;
    state
        .replay_state_mut()
        .record_checkpoint(ReplayCheckpoint {
            tick,
            event_log_hash,
            world_state_hash,
        })?;
    Ok(true)
}

pub fn seed_replay_inputs_from_scheduler(
    state: &mut SimulationState,
) -> Result<usize, ReplayCheckpointError> {
    let mut recorded = 0usize;
    let inputs = state
        .scheduler()
        .input_queue()
        .iter_in_sequence_order()
        .cloned()
        .collect::<Vec<_>>();
    for input in inputs {
        state.replay_state_mut().record_input(input)?;
        recorded = recorded
            .checked_add(1)
            .expect("replay seeded input counter overflowed");
    }

    Ok(recorded)
}

fn verify_checkpoint(
    checkpoint: &ReplayCheckpoint,
    world: &worldwake_core::World,
    event_log: &worldwake_core::EventLog,
    errors: &mut Vec<ReplayError>,
) {
    let actual_event_log_hash =
        hash_event_log(event_log).expect("replay event-log hashing failed unexpectedly");
    if actual_event_log_hash != checkpoint.event_log_hash {
        errors.push(ReplayError::EventLogCheckpointMismatch {
            tick: checkpoint.tick,
            expected: checkpoint.event_log_hash,
            actual: actual_event_log_hash,
        });
    }

    let actual_world_hash = hash_world(world).expect("replay world hashing failed unexpectedly");
    if actual_world_hash != checkpoint.world_state_hash {
        errors.push(ReplayError::WorldCheckpointMismatch {
            tick: checkpoint.tick,
            expected: checkpoint.world_state_hash,
            actual: actual_world_hash,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{
        record_tick_checkpoint, replay_and_verify, seed_replay_inputs_from_scheduler, ReplayError,
    };
    use crate::{
        ActionDefRegistry, ActionHandlerRegistry, ControllerState, DeterministicRng, InputKind,
        RecipeDefinition, RecipeRegistry, ReplayCheckpoint, ReplayRecordingConfig, ReplayState,
        Scheduler, SimulationState, SystemDispatchTable, SystemError, SystemExecutionContext,
        SystemId, SystemManifest, TickStepServices,
    };
    use std::num::NonZeroU64;
    use worldwake_core::{
        build_prototype_world, BodyCostPerTick, CauseRef, CommodityKind, ControlSource, EntityId,
        EventLog, Quantity, Seed, StateHash, Tick, VisibilitySpec, WitnessData, WorkstationTag,
        World, WorldTxn,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn new_txn(world: &mut World, tick: Tick, cause: CauseRef) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            tick,
            cause,
            None,
            None,
            VisibilitySpec::Hidden,
            WitnessData::default(),
        )
    }

    fn spawn_agent(world: &mut World, event_log: &mut EventLog, slot: u64) -> EntityId {
        let mut txn = new_txn(world, Tick(slot), CauseRef::Bootstrap);
        let agent = txn
            .create_agent(&format!("agent-{slot}"), ControlSource::Ai)
            .unwrap();
        let _ = txn.commit(event_log);
        agent
    }

    fn populated_recipe_registry() -> RecipeRegistry {
        let mut registry = RecipeRegistry::new();
        registry.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: std::num::NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: vec![CommodityKind::Water],
            body_cost_per_tick: BodyCostPerTick::zero(),
        });
        registry
    }

    #[allow(clippy::needless_pass_by_value)]
    fn deterministic_world_system(context: SystemExecutionContext<'_>) -> Result<(), SystemError> {
        let marker = context.rng.next_u32();
        if marker & 1 == 1 {
            return Ok(());
        }

        let mut txn = WorldTxn::new(
            context.world,
            context.tick,
            CauseRef::SystemTick(context.tick),
            None,
            None,
            VisibilitySpec::Hidden,
            WitnessData::default(),
        );
        txn.create_agent(
            &format!(
                "{}-{}-{}",
                context.system_id.as_str(),
                context.tick.0,
                context.rng.next_u32()
            ),
            ControlSource::Ai,
        )
        .map_err(|error| SystemError::new(error.to_string()))?;
        let _ = txn.commit(context.event_log);
        Ok(())
    }

    fn deterministic_systems() -> SystemDispatchTable {
        fn dispatch(context: SystemExecutionContext<'_>) -> Result<(), SystemError> {
            if context.system_id == SystemId::Needs {
                return deterministic_world_system(context);
            }

            let _ = context.rng.next_u32();
            Ok(())
        }

        SystemDispatchTable::from_handlers([dispatch; SystemId::ALL.len()])
    }

    fn build_initial_state(advance_initial_rng: bool) -> (SimulationState, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();
        let first = spawn_agent(&mut world, &mut event_log, 1);
        let second = spawn_agent(&mut world, &mut event_log, 2);
        let scheduler = Scheduler::new(SystemManifest::canonical());
        let controller = ControllerState::with_entity(first);
        let mut rng = DeterministicRng::new(Seed([17; 32]));
        if advance_initial_rng {
            let _ = rng.next_u32();
        }
        let recipe_registry = populated_recipe_registry();

        let initial_hash = SimulationState::replay_bootstrap_hash_parts(
            &world,
            &event_log,
            &scheduler,
            &recipe_registry,
            &controller,
            &rng,
        )
        .unwrap();
        let replay = ReplayState::new(
            initial_hash,
            Seed([17; 32]),
            scheduler.current_tick(),
            ReplayRecordingConfig::disabled(),
        );

        (
            SimulationState::new(
                world,
                event_log,
                scheduler,
                recipe_registry,
                replay,
                controller,
                rng,
            ),
            first,
            second,
        )
    }

    fn services<'a>(
        action_defs: &'a ActionDefRegistry,
        action_handlers: &'a ActionHandlerRegistry,
        systems: &'a SystemDispatchTable,
    ) -> TickStepServices<'a> {
        TickStepServices {
            action_defs,
            action_handlers,
            systems,
        }
    }

    fn enqueue_recorded_input(
        scheduler: &mut Scheduler,
        replay: &mut ReplayState,
        tick: Tick,
        kind: InputKind,
    ) {
        let input = scheduler.input_queue_mut().enqueue(tick, kind).clone();
        replay.record_input(input).unwrap();
    }

    #[allow(clippy::too_many_lines)]
    fn build_recording(
        checkpoint_config: ReplayRecordingConfig,
        total_ticks: u64,
        seed_initial_queue: bool,
        advance_initial_rng: bool,
    ) -> (SimulationState, StateHash) {
        let (mut initial_state, first, second) = build_initial_state(advance_initial_rng);
        *initial_state.replay_state_mut() = ReplayState::new(
            initial_state.replay_bootstrap_hash().unwrap(),
            initial_state.replay_state().master_seed(),
            initial_state.scheduler().current_tick(),
            checkpoint_config,
        );
        let mut state = initial_state.clone();

        if seed_initial_queue {
            let _ = state.scheduler_mut().input_queue_mut().enqueue(
                Tick(0),
                InputKind::SwitchControl {
                    from: Some(first),
                    to: Some(second),
                },
            );
            let _ = state.scheduler_mut().input_queue_mut().enqueue(
                Tick(0),
                InputKind::SwitchControl {
                    from: Some(second),
                    to: Some(first),
                },
            );
            *initial_state.scheduler_mut() = state.scheduler().clone();
            *initial_state.replay_state_mut() = ReplayState::new(
                initial_state.replay_bootstrap_hash().unwrap(),
                initial_state.replay_state().master_seed(),
                initial_state.scheduler().current_tick(),
                checkpoint_config,
            );
            *state.replay_state_mut() = initial_state.replay_state().clone();
        }

        if seed_initial_queue {
            seed_replay_inputs_from_scheduler(&mut state).unwrap();
        } else {
            let (scheduler, replay) = state.scheduler_and_replay_mut();
            enqueue_recorded_input(
                scheduler,
                replay,
                Tick(0),
                InputKind::SwitchControl {
                    from: Some(first),
                    to: Some(second),
                },
            );
            let (scheduler, replay) = state.scheduler_and_replay_mut();
            enqueue_recorded_input(
                scheduler,
                replay,
                Tick(0),
                InputKind::SwitchControl {
                    from: Some(second),
                    to: Some(first),
                },
            );
        }
        if total_ticks > 3 {
            let (scheduler, replay) = state.scheduler_and_replay_mut();
            enqueue_recorded_input(
                scheduler,
                replay,
                Tick(3),
                InputKind::SwitchControl {
                    from: Some(first),
                    to: Some(second),
                },
            );
        }
        if total_ticks > 7 {
            let (scheduler, replay) = state.scheduler_and_replay_mut();
            enqueue_recorded_input(
                scheduler,
                replay,
                Tick(7),
                InputKind::SwitchControl {
                    from: Some(second),
                    to: Some(first),
                },
            );
        }

        let action_defs = ActionDefRegistry::new();
        let action_handlers = ActionHandlerRegistry::new();
        let systems = deterministic_systems();

        for _ in 0..total_ticks {
            let result = {
                let (world, event_log, scheduler, controller, rng) = state.runtime_parts_mut();
                crate::step_tick(
                    world,
                    event_log,
                    scheduler,
                    controller,
                    rng,
                    services(&action_defs, &action_handlers, &systems),
                )
            }
            .unwrap();
            let _ = record_tick_checkpoint(&mut state, result.tick).unwrap();
            let next_tick = state.scheduler().current_tick();
            state
                .replay_state_mut()
                .set_terminal_tick(next_tick)
                .unwrap();
        }

        let final_hash = state.replay_bootstrap_hash().unwrap();

        *initial_state.replay_state_mut() = state.replay_state().clone();

        (initial_state, final_hash)
    }

    fn replay_with_recording(
        initial_state: &SimulationState,
    ) -> Result<StateHash, Vec<ReplayError>> {
        let action_defs = ActionDefRegistry::new();
        let action_handlers = ActionHandlerRegistry::new();
        let systems = deterministic_systems();

        replay_and_verify(
            initial_state,
            services(&action_defs, &action_handlers, &systems),
        )
    }

    #[test]
    fn replay_determinism_matches_recorded_checkpoints_and_final_hash() {
        let (initial_state, expected_final_hash) = build_recording(
            ReplayRecordingConfig::every(NonZeroU64::new(1).unwrap()),
            12,
            false,
            false,
        );
        let original_state = initial_state.clone();

        let actual_final_hash = replay_with_recording(&initial_state).unwrap();

        assert_eq!(actual_final_hash, expected_final_hash);
        assert_eq!(initial_state, original_state);
    }

    #[test]
    fn replay_zero_inputs_and_zero_ticks_returns_initial_hash() {
        let (mut state, _, _) = build_initial_state(false);
        *state.replay_state_mut() = ReplayState::new(
            state.replay_bootstrap_hash().unwrap(),
            Seed([23; 32]),
            state.scheduler().current_tick(),
            ReplayRecordingConfig::disabled(),
        );

        let final_hash = replay_with_recording(&state).unwrap();

        assert_eq!(final_hash, state.replay_bootstrap_hash().unwrap());
    }

    #[test]
    fn replay_preserves_same_tick_input_ordering() {
        let (initial_state, expected_final_hash) =
            build_recording(ReplayRecordingConfig::disabled(), 1, false, false);

        let actual_final_hash = replay_with_recording(&initial_state).unwrap();

        assert_eq!(actual_final_hash, expected_final_hash);
    }

    #[test]
    fn replay_rejects_initial_state_hash_mismatch() {
        let (initial_state, _) =
            build_recording(ReplayRecordingConfig::disabled(), 2, false, false);
        let mut wrong_state = initial_state.clone();
        let current_controller = wrong_state.controller_state().clone();
        wrong_state
            .controller_state_mut()
            .switch_control(current_controller.controlled_entity(), Some(entity(999)))
            .unwrap();

        let errors = replay_with_recording(&wrong_state).unwrap_err();

        assert!(matches!(
            errors.as_slice(),
            [ReplayError::InitialStateHashMismatch { .. }]
        ));
    }

    #[test]
    fn replay_reconstructs_nonempty_initial_scheduler_queue_and_sequence_offset() {
        let (initial_state, expected_final_hash) = build_recording(
            ReplayRecordingConfig::every(NonZeroU64::new(1).unwrap()),
            2,
            true,
            false,
        );

        assert!(!initial_state.scheduler().input_queue().is_empty());
        assert_eq!(
            initial_state.scheduler().input_queue().next_sequence_no(),
            2
        );

        let final_hash = replay_with_recording(&initial_state).unwrap();

        assert_eq!(final_hash, expected_final_hash);
    }

    #[test]
    fn replay_uses_recorded_initial_rng_state_instead_of_reseeding_from_master_seed() {
        let (initial_state, expected_final_hash) = build_recording(
            ReplayRecordingConfig::every(NonZeroU64::new(1).unwrap()),
            3,
            false,
            true,
        );

        let final_hash = replay_with_recording(&initial_state).unwrap();

        assert_eq!(final_hash, expected_final_hash);
    }

    #[test]
    fn replay_reports_world_checkpoint_mismatch() {
        let (initial_state, _) = build_recording(
            ReplayRecordingConfig::every(NonZeroU64::new(1).unwrap()),
            4,
            false,
            false,
        );
        let replay = initial_state.replay_state();
        let mut corrupted_replay = ReplayState::new(
            replay.initial_state_hash(),
            replay.master_seed(),
            replay.terminal_tick(),
            *replay.config(),
        );

        for input in replay.input_log() {
            corrupted_replay.record_input(input.clone()).unwrap();
        }
        for checkpoint in replay.checkpoints() {
            let checkpoint = if checkpoint.tick == Tick(2) {
                ReplayCheckpoint {
                    tick: checkpoint.tick,
                    event_log_hash: checkpoint.event_log_hash,
                    world_state_hash: StateHash([0x55; 32]),
                }
            } else {
                *checkpoint
            };
            corrupted_replay.record_checkpoint(checkpoint).unwrap();
        }

        let mut corrupted_state = initial_state.clone();
        *corrupted_state.replay_state_mut() = corrupted_replay;

        let errors = replay_with_recording(&corrupted_state).unwrap_err();

        assert!(errors.contains(&ReplayError::WorldCheckpointMismatch {
            tick: Tick(2),
            expected: StateHash([0x55; 32]),
            actual: initial_state.replay_state().checkpoints()[2].world_state_hash,
        }));
    }

    #[test]
    fn record_tick_checkpoint_respects_disabled_and_interval_configs() {
        let (mut disabled, _, _) = build_initial_state(false);
        let initial_hash = disabled.replay_bootstrap_hash().unwrap();
        *disabled.replay_state_mut() = ReplayState::new(
            initial_hash,
            Seed([1; 32]),
            disabled.scheduler().current_tick(),
            ReplayRecordingConfig::disabled(),
        );
        let mut every_five = disabled.clone();
        *every_five.replay_state_mut() = ReplayState::new(
            initial_hash,
            Seed([1; 32]),
            every_five.scheduler().current_tick(),
            ReplayRecordingConfig::every(NonZeroU64::new(5).unwrap()),
        );

        assert!(!record_tick_checkpoint(&mut disabled, Tick(0)).unwrap());
        assert!(disabled.replay_state().checkpoints().is_empty());

        assert!(record_tick_checkpoint(&mut every_five, Tick(0)).unwrap());
        assert!(!record_tick_checkpoint(&mut every_five, Tick(1)).unwrap());
        assert!(record_tick_checkpoint(&mut every_five, Tick(5)).unwrap());
        assert_eq!(
            every_five
                .replay_state()
                .checkpoints()
                .iter()
                .map(|checkpoint| checkpoint.tick)
                .collect::<Vec<_>>(),
            vec![Tick(0), Tick(5)]
        );
    }

    #[test]
    fn seed_replay_inputs_from_scheduler_copies_pending_inputs_in_queue_order() {
        let (mut state, first, second) = build_initial_state(false);
        let initial_hash = state.replay_bootstrap_hash().unwrap();
        *state.replay_state_mut() = ReplayState::new(
            initial_hash,
            Seed([1; 32]),
            state.scheduler().current_tick(),
            ReplayRecordingConfig::disabled(),
        );

        let first_input = state
            .scheduler_mut()
            .input_queue_mut()
            .enqueue(
                Tick(5),
                InputKind::SwitchControl {
                    from: Some(first),
                    to: Some(second),
                },
            )
            .clone();
        let second_input = state
            .scheduler_mut()
            .input_queue_mut()
            .enqueue(
                Tick(2),
                InputKind::SwitchControl {
                    from: Some(second),
                    to: Some(first),
                },
            )
            .clone();

        let recorded = seed_replay_inputs_from_scheduler(&mut state).unwrap();

        assert_eq!(recorded, 2);
        assert_eq!(
            state.replay_state().input_log(),
            &[first_input, second_input]
        );
    }

    #[test]
    fn replay_uses_terminal_tick_when_no_inputs_or_checkpoints_exist() {
        let (mut initial_state, _, _) = build_initial_state(false);
        *initial_state.replay_state_mut() = ReplayState::new(
            initial_state.replay_bootstrap_hash().unwrap(),
            Seed([42; 32]),
            initial_state.scheduler().current_tick(),
            ReplayRecordingConfig::disabled(),
        );
        initial_state
            .replay_state_mut()
            .set_terminal_tick(Tick(3))
            .unwrap();

        let final_hash = replay_with_recording(&initial_state).unwrap();

        let mut state = initial_state.clone();
        let action_defs = ActionDefRegistry::new();
        let action_handlers = ActionHandlerRegistry::new();
        let systems = deterministic_systems();

        for _ in 0..3 {
            let (world, event_log, scheduler, controller, rng) = state.runtime_parts_mut();
            crate::step_tick(
                world,
                event_log,
                scheduler,
                controller,
                rng,
                services(&action_defs, &action_handlers, &systems),
            )
            .unwrap();
        }

        assert_eq!(final_hash, state.replay_bootstrap_hash().unwrap());
    }
}
