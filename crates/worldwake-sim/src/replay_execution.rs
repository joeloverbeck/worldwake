use crate::{
    step_tick, ControllerState, DeterministicRng, InputQueueError, ReplayCheckpoint, ReplayState,
    ReplayStateError, Scheduler, TickStepServices,
};
use std::fmt;
use worldwake_core::{
    hash_event_log, hash_serializable, hash_world, CanonicalError, EventLog, StateHash, Tick,
    World,
};

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
            Self::Canonical(source) => write!(f, "failed to hash replay checkpoint state: {source}"),
            Self::InputQueue(source) => write!(f, "failed to rebuild scheduler input queue: {source}"),
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
    initial_world: &World,
    initial_event_log: &EventLog,
    initial_scheduler: &Scheduler,
    initial_controller: &ControllerState,
    replay: &ReplayState,
    services: TickStepServices<'_>,
) -> Result<StateHash, Vec<ReplayError>> {
    let mut errors = Vec::new();
    let initial_hash = match hash_replay_roots(
        initial_world,
        initial_event_log,
        initial_scheduler,
        initial_controller,
    ) {
        Ok(hash) => hash,
        Err(source) => panic!("replay initial-state hashing failed unexpectedly: {source}"),
    };
    if initial_hash != replay.initial_state_hash() {
        errors.push(ReplayError::InitialStateHashMismatch {
            expected: replay.initial_state_hash(),
            actual: initial_hash,
        });
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut world = initial_world.clone();
    let mut event_log = initial_event_log.clone();
    let mut scheduler = initial_scheduler.clone();
    let mut controller = initial_controller.clone();
    let mut rng = DeterministicRng::new(replay.master_seed());

    scheduler
        .input_queue_mut()
        .replace_with_recorded(replay.input_log())
        .expect("replay input log must remain reconstructible");

    let mut checkpoint_index = 0usize;
    while scheduler.current_tick() < replay.terminal_tick() {
        let result = step_tick(
            &mut world,
            &mut event_log,
            &mut scheduler,
            &mut controller,
            &mut rng,
            services,
        )
        .unwrap_or_else(|error| panic!("replay tick stepping failed unexpectedly: {error}"));

        while let Some(checkpoint) = replay.checkpoints().get(checkpoint_index) {
            if checkpoint.tick != result.tick {
                break;
            }

            verify_checkpoint(checkpoint, &world, &event_log, &mut errors);
            checkpoint_index += 1;
        }
    }

    if errors.is_empty() {
        Ok(
            hash_replay_roots(&world, &event_log, &scheduler, &controller)
                .expect("replay final-state hashing failed unexpectedly"),
        )
    } else {
        Err(errors)
    }
}

pub fn record_tick_checkpoint(
    replay: &mut ReplayState,
    tick: Tick,
    world: &World,
    event_log: &EventLog,
) -> Result<bool, ReplayCheckpointError> {
    if !replay.should_checkpoint(tick) {
        return Ok(false);
    }

    replay.record_checkpoint(ReplayCheckpoint {
        tick,
        event_log_hash: hash_event_log(event_log)?,
        world_state_hash: hash_world(world)?,
    })?;
    Ok(true)
}

pub fn seed_replay_inputs_from_scheduler(
    replay: &mut ReplayState,
    scheduler: &Scheduler,
) -> Result<usize, ReplayCheckpointError> {
    let mut recorded = 0usize;
    for input in scheduler.input_queue().iter_in_sequence_order().cloned() {
        replay.record_input(input)?;
        recorded = recorded
            .checked_add(1)
            .expect("replay seeded input counter overflowed");
    }

    Ok(recorded)
}

fn verify_checkpoint(
    checkpoint: &ReplayCheckpoint,
    world: &World,
    event_log: &EventLog,
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

fn hash_replay_roots(
    world: &World,
    event_log: &EventLog,
    scheduler: &Scheduler,
    controller: &ControllerState,
) -> Result<StateHash, CanonicalError> {
    hash_serializable(&(world, event_log, scheduler, controller))
}

#[cfg(test)]
mod tests {
    use super::{
        hash_replay_roots, record_tick_checkpoint, replay_and_verify,
        seed_replay_inputs_from_scheduler, ReplayError,
    };
    use crate::{
        ActionDefRegistry, ActionHandlerRegistry, ControllerState, DeterministicRng, InputKind,
        ReplayCheckpoint, ReplayRecordingConfig, ReplayState, Scheduler, SystemDispatchTable,
        SystemError, SystemExecutionContext, SystemId, SystemManifest, TickStepServices,
    };
    use std::num::NonZeroU64;
    use worldwake_core::{
        build_prototype_world, CauseRef, ControlSource, EntityId, EventLog, Seed, StateHash, Tick,
        VisibilitySpec, WitnessData, World, WorldTxn,
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

    fn build_initial_state() -> (
        World,
        EventLog,
        Scheduler,
        ControllerState,
        EntityId,
        EntityId,
    ) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();
        let first = spawn_agent(&mut world, &mut event_log, 1);
        let second = spawn_agent(&mut world, &mut event_log, 2);

        (
            world,
            event_log,
            Scheduler::new(SystemManifest::canonical()),
            ControllerState::with_entity(first),
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
    ) -> (
        World,
        EventLog,
        Scheduler,
        ControllerState,
        ReplayState,
        StateHash,
    ) {
        let (initial_world, initial_event_log, mut initial_scheduler, initial_controller, first, second) =
            build_initial_state();
        let mut world = initial_world.clone();
        let mut event_log = initial_event_log.clone();
        let mut scheduler = initial_scheduler.clone();
        let mut controller = initial_controller.clone();
        let seed = Seed([17; 32]);

        if seed_initial_queue {
            let _ = scheduler
                .input_queue_mut()
                .enqueue(Tick(0), InputKind::SwitchControl {
                    from: Some(first),
                    to: Some(second),
                });
            let _ = scheduler
                .input_queue_mut()
                .enqueue(Tick(0), InputKind::SwitchControl {
                    from: Some(second),
                    to: Some(first),
                });
            initial_scheduler = scheduler.clone();
        }

        let mut replay = ReplayState::new(
            hash_replay_roots(
                &initial_world,
                &initial_event_log,
                &initial_scheduler,
                &initial_controller,
            )
            .unwrap(),
            seed,
            initial_scheduler.current_tick(),
            checkpoint_config,
        );

        if seed_initial_queue {
            seed_replay_inputs_from_scheduler(&mut replay, &scheduler).unwrap();
        } else {
            enqueue_recorded_input(
                &mut scheduler,
                &mut replay,
                Tick(0),
                InputKind::SwitchControl {
                    from: Some(first),
                    to: Some(second),
                },
            );
            enqueue_recorded_input(
                &mut scheduler,
                &mut replay,
                Tick(0),
                InputKind::SwitchControl {
                    from: Some(second),
                    to: Some(first),
                },
            );
        }
        if total_ticks > 3 {
            enqueue_recorded_input(
                &mut scheduler,
                &mut replay,
                Tick(3),
                InputKind::SwitchControl {
                    from: Some(first),
                    to: Some(second),
                },
            );
        }
        if total_ticks > 7 {
            enqueue_recorded_input(
                &mut scheduler,
                &mut replay,
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
        let mut rng = DeterministicRng::new(seed);

        for _ in 0..total_ticks {
            let result = crate::step_tick(
                &mut world,
                &mut event_log,
                &mut scheduler,
                &mut controller,
                &mut rng,
                services(&action_defs, &action_handlers, &systems),
            )
            .unwrap();
            let _ = record_tick_checkpoint(&mut replay, result.tick, &world, &event_log).unwrap();
            replay.set_terminal_tick(scheduler.current_tick()).unwrap();
        }

        let final_hash = hash_replay_roots(&world, &event_log, &scheduler, &controller).unwrap();

        (
            initial_world,
            initial_event_log,
            initial_scheduler,
            initial_controller,
            replay,
            final_hash,
        )
    }

    fn replay_with_recording(
        initial_world: &World,
        initial_event_log: &EventLog,
        initial_scheduler: &Scheduler,
        initial_controller: &ControllerState,
        replay: &ReplayState,
    ) -> Result<StateHash, Vec<ReplayError>> {
        let action_defs = ActionDefRegistry::new();
        let action_handlers = ActionHandlerRegistry::new();
        let systems = deterministic_systems();

        replay_and_verify(
            initial_world,
            initial_event_log,
            initial_scheduler,
            initial_controller,
            replay,
            services(&action_defs, &action_handlers, &systems),
        )
    }

    #[test]
    fn replay_determinism_matches_recorded_checkpoints_and_final_hash() {
        let (
            initial_world,
            initial_event_log,
            initial_scheduler,
            initial_controller,
            replay,
            expected_final_hash,
        ) = build_recording(
            ReplayRecordingConfig::every(NonZeroU64::new(1).unwrap()),
            12,
            false,
        );
        let original_world = initial_world.clone();
        let original_event_log = initial_event_log.clone();
        let original_scheduler = initial_scheduler.clone();
        let original_controller = initial_controller.clone();

        let actual_final_hash = replay_with_recording(
            &initial_world,
            &initial_event_log,
            &initial_scheduler,
            &initial_controller,
            &replay,
        )
        .unwrap();

        assert_eq!(actual_final_hash, expected_final_hash);
        assert_eq!(initial_world, original_world);
        assert_eq!(initial_event_log, original_event_log);
        assert_eq!(initial_scheduler, original_scheduler);
        assert_eq!(initial_controller, original_controller);
    }

    #[test]
    fn replay_zero_inputs_and_zero_ticks_returns_initial_hash() {
        let (world, event_log, scheduler, controller, _, _) = build_initial_state();
        let replay = ReplayState::new(
            hash_replay_roots(&world, &event_log, &scheduler, &controller).unwrap(),
            Seed([23; 32]),
            scheduler.current_tick(),
            ReplayRecordingConfig::disabled(),
        );

        let final_hash =
            replay_with_recording(&world, &event_log, &scheduler, &controller, &replay).unwrap();

        assert_eq!(
            final_hash,
            hash_replay_roots(&world, &event_log, &scheduler, &controller).unwrap()
        );
    }

    #[test]
    fn replay_preserves_same_tick_input_ordering() {
        let (
            initial_world,
            initial_event_log,
            initial_scheduler,
            initial_controller,
            replay,
            expected_final_hash,
        ) = build_recording(ReplayRecordingConfig::disabled(), 1, false);

        let actual_final_hash = replay_with_recording(
            &initial_world,
            &initial_event_log,
            &initial_scheduler,
            &initial_controller,
            &replay,
        )
        .unwrap();

        assert_eq!(actual_final_hash, expected_final_hash);
    }

    #[test]
    fn replay_rejects_initial_state_hash_mismatch() {
        let (
            initial_world,
            initial_event_log,
            initial_scheduler,
            initial_controller,
            replay,
            _,
        ) = build_recording(ReplayRecordingConfig::disabled(), 2, false);
        let mut wrong_controller = initial_controller.clone();
        wrong_controller
            .switch_control(initial_controller.controlled_entity(), Some(entity(999)))
            .unwrap();

        let errors = replay_with_recording(
            &initial_world,
            &initial_event_log,
            &initial_scheduler,
            &wrong_controller,
            &replay,
        )
        .unwrap_err();

        assert!(matches!(
            errors.as_slice(),
            [ReplayError::InitialStateHashMismatch { .. }]
        ));
    }

    #[test]
    fn replay_reconstructs_nonempty_initial_scheduler_queue_and_sequence_offset() {
        let (
            initial_world,
            initial_event_log,
            initial_scheduler,
            initial_controller,
            replay,
            expected_final_hash,
        ) = build_recording(
            ReplayRecordingConfig::every(NonZeroU64::new(1).unwrap()),
            2,
            true,
        );

        assert!(!initial_scheduler.input_queue().is_empty());
        assert_eq!(initial_scheduler.input_queue().next_sequence_no(), 2);

        let final_hash = replay_with_recording(
            &initial_world,
            &initial_event_log,
            &initial_scheduler,
            &initial_controller,
            &replay,
        )
        .unwrap();

        assert_eq!(final_hash, expected_final_hash);
    }

    #[test]
    fn replay_reports_world_checkpoint_mismatch() {
        let (
            initial_world,
            initial_event_log,
            initial_scheduler,
            initial_controller,
            replay,
            _,
        ) = build_recording(
            ReplayRecordingConfig::every(NonZeroU64::new(1).unwrap()),
            4,
            false,
        );
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

        let errors = replay_with_recording(
            &initial_world,
            &initial_event_log,
            &initial_scheduler,
            &initial_controller,
            &corrupted_replay,
        )
        .unwrap_err();

        assert!(errors.contains(&ReplayError::WorldCheckpointMismatch {
            tick: Tick(2),
            expected: StateHash([0x55; 32]),
            actual: replay.checkpoints()[2].world_state_hash,
        }));
    }

    #[test]
    fn record_tick_checkpoint_respects_disabled_and_interval_configs() {
        let (world, event_log, scheduler, controller, _, _) = build_initial_state();
        let initial_hash = hash_replay_roots(&world, &event_log, &scheduler, &controller).unwrap();
        let mut disabled =
            ReplayState::new(
                initial_hash,
                Seed([1; 32]),
                scheduler.current_tick(),
                ReplayRecordingConfig::disabled(),
            );
        let mut every_five = ReplayState::new(
            initial_hash,
            Seed([1; 32]),
            scheduler.current_tick(),
            ReplayRecordingConfig::every(NonZeroU64::new(5).unwrap()),
        );

        assert!(!record_tick_checkpoint(&mut disabled, Tick(0), &world, &event_log).unwrap());
        assert!(disabled.checkpoints().is_empty());

        assert!(record_tick_checkpoint(&mut every_five, Tick(0), &world, &event_log).unwrap());
        assert!(!record_tick_checkpoint(&mut every_five, Tick(1), &world, &event_log).unwrap());
        assert!(record_tick_checkpoint(&mut every_five, Tick(5), &world, &event_log).unwrap());
        assert_eq!(
            every_five
                .checkpoints()
                .iter()
                .map(|checkpoint| checkpoint.tick)
                .collect::<Vec<_>>(),
            vec![Tick(0), Tick(5)]
        );
    }

    #[test]
    fn seed_replay_inputs_from_scheduler_copies_pending_inputs_in_queue_order() {
        let (world, event_log, mut scheduler, controller, first, second) = build_initial_state();
        let initial_hash = hash_replay_roots(&world, &event_log, &scheduler, &controller).unwrap();
        let mut replay =
            ReplayState::new(
                initial_hash,
                Seed([1; 32]),
                scheduler.current_tick(),
                ReplayRecordingConfig::disabled(),
            );

        let first_input = scheduler
            .input_queue_mut()
            .enqueue(Tick(5), InputKind::SwitchControl {
                from: Some(first),
                to: Some(second),
            })
            .clone();
        let second_input = scheduler
            .input_queue_mut()
            .enqueue(Tick(2), InputKind::SwitchControl {
                from: Some(second),
                to: Some(first),
            })
            .clone();

        let recorded = seed_replay_inputs_from_scheduler(&mut replay, &scheduler).unwrap();

        assert_eq!(recorded, 2);
        assert_eq!(replay.input_log(), &[first_input, second_input]);
    }

    #[test]
    fn replay_uses_terminal_tick_when_no_inputs_or_checkpoints_exist() {
        let (initial_world, initial_event_log, initial_scheduler, initial_controller, _, _) =
            build_initial_state();
        let mut replay = ReplayState::new(
            hash_replay_roots(
                &initial_world,
                &initial_event_log,
                &initial_scheduler,
                &initial_controller,
            )
            .unwrap(),
            Seed([42; 32]),
            initial_scheduler.current_tick(),
            ReplayRecordingConfig::disabled(),
        );
        replay.set_terminal_tick(Tick(3)).unwrap();

        let final_hash = replay_with_recording(
            &initial_world,
            &initial_event_log,
            &initial_scheduler,
            &initial_controller,
            &replay,
        )
        .unwrap();

        let mut world = initial_world.clone();
        let mut event_log = initial_event_log.clone();
        let mut scheduler = initial_scheduler.clone();
        let mut controller = initial_controller.clone();
        let action_defs = ActionDefRegistry::new();
        let action_handlers = ActionHandlerRegistry::new();
        let systems = deterministic_systems();
        let mut rng = DeterministicRng::new(replay.master_seed());

        for _ in 0..3 {
            crate::step_tick(
                &mut world,
                &mut event_log,
                &mut scheduler,
                &mut controller,
                &mut rng,
                services(&action_defs, &action_handlers, &systems),
            )
            .unwrap();
        }

        assert_eq!(
            final_hash,
            hash_replay_roots(&world, &event_log, &scheduler, &controller).unwrap()
        );
    }
}
