use crate::SimulationState;
use std::fmt;
use std::path::Path;

pub const SAVE_MAGIC: [u8; 4] = *b"WWAK";
pub const SAVE_FORMAT_VERSION: u32 = 2;

const SAVE_HEADER_LEN: usize = SAVE_MAGIC.len() + std::mem::size_of::<u32>();

#[derive(Debug)]
pub enum SaveError {
    Io(std::io::Error),
    Serialization(String),
    InvalidMagic,
    UnsupportedVersion { found: u32, expected: u32 },
    Deserialization(String),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(source) => write!(f, "save/load I/O failed: {source}"),
            Self::Serialization(message) => {
                write!(f, "failed to serialize simulation state: {message}")
            }
            Self::InvalidMagic => f.write_str("save data does not start with Worldwake save magic"),
            Self::UnsupportedVersion { found, expected } => write!(
                f,
                "unsupported save format version {found}; expected {expected}"
            ),
            Self::Deserialization(message) => {
                write!(f, "failed to deserialize simulation state: {message}")
            }
        }
    }
}

impl std::error::Error for SaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(source) => Some(source),
            Self::Serialization(_)
            | Self::InvalidMagic
            | Self::UnsupportedVersion { .. }
            | Self::Deserialization(_) => None,
        }
    }
}

impl From<std::io::Error> for SaveError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn save(state: &SimulationState, path: &Path) -> Result<(), SaveError> {
    let bytes = save_to_bytes(state)?;
    std::fs::write(path, bytes).map_err(SaveError::Io)
}

pub fn load(path: &Path) -> Result<SimulationState, SaveError> {
    let bytes = std::fs::read(path).map_err(SaveError::Io)?;
    load_from_bytes(&bytes)
}

pub fn save_to_bytes(state: &SimulationState) -> Result<Vec<u8>, SaveError> {
    let payload =
        bincode::serialize(state).map_err(|error| SaveError::Serialization(error.to_string()))?;
    let mut bytes = Vec::with_capacity(SAVE_HEADER_LEN + payload.len());
    bytes.extend_from_slice(&SAVE_MAGIC);
    bytes.extend_from_slice(&SAVE_FORMAT_VERSION.to_le_bytes());
    bytes.extend_from_slice(&payload);
    Ok(bytes)
}

pub fn load_from_bytes(bytes: &[u8]) -> Result<SimulationState, SaveError> {
    if bytes.len() < SAVE_HEADER_LEN {
        return Err(SaveError::Deserialization(
            "save data is truncated before the fixed header completes".to_string(),
        ));
    }

    let (magic, rest) = bytes.split_at(SAVE_MAGIC.len());
    if magic != SAVE_MAGIC {
        return Err(SaveError::InvalidMagic);
    }

    let (version_bytes, payload) = rest.split_at(std::mem::size_of::<u32>());
    let found = u32::from_le_bytes(
        version_bytes
            .try_into()
            .expect("validated fixed-width save header"),
    );
    if found != SAVE_FORMAT_VERSION {
        return Err(SaveError::UnsupportedVersion {
            found,
            expected: SAVE_FORMAT_VERSION,
        });
    }

    bincode::deserialize(payload).map_err(|error| SaveError::Deserialization(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{
        load, load_from_bytes, save, save_to_bytes, SaveError, SAVE_FORMAT_VERSION, SAVE_MAGIC,
    };
    use crate::{
        step_tick, ActionDefRegistry, ActionDuration, ActionHandlerRegistry, ActionInstance,
        ActionInstanceId, ActionPayload, ActionState, ActionStatus, ControllerState,
        DeterministicRng, InputKind, RecipeDefinition, RecipeRegistry, ReplayCheckpoint,
        ReplayRecordingConfig, ReplayState, Scheduler, SimulationState, SystemDispatchTable,
        SystemError, SystemExecutionContext, SystemId, SystemManifest, TickStepServices,
    };
    use std::num::NonZeroU64;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use worldwake_core::{
        build_prototype_world, ActionDefId, BodyCostPerTick, CauseRef, CommodityKind,
        ControlSource, EntityId, EventLog, EventPayload, PendingEvent, Quantity, ReservationId,
        Seed, StateHash, Tick, TickRange, UniqueItemKind, VisibilitySpec, WitnessData,
        WorkstationTag, World, WorldTxn,
    };

    fn state_hash(byte: u8) -> StateHash {
        StateHash([byte; 32])
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

    fn spawn_agent(
        world: &mut World,
        event_log: &mut EventLog,
        tick: Tick,
        name: &str,
    ) -> EntityId {
        let mut txn = new_txn(world, tick, CauseRef::Bootstrap);
        let agent = txn.create_agent(name, ControlSource::Ai).unwrap();
        let _ = txn.commit(event_log);
        agent
    }

    fn spawn_item_with_reservation(
        world: &mut World,
        event_log: &mut EventLog,
        reserver: EntityId,
    ) -> (EntityId, ReservationId) {
        let mut txn = new_txn(world, Tick(2), CauseRef::Bootstrap);
        let item = txn
            .create_item_lot(
                worldwake_core::CommodityKind::Bread,
                worldwake_core::Quantity(2),
            )
            .unwrap();
        let reservation = txn
            .try_reserve(item, reserver, TickRange::new(Tick(3), Tick(8)).unwrap())
            .unwrap();
        let _ = txn.commit(event_log);
        (item, reservation)
    }

    fn populated_recipe_registry() -> RecipeRegistry {
        let mut registry = RecipeRegistry::new();
        registry.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: std::num::NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: vec![UniqueItemKind::SimpleTool],
            body_cost_per_tick: BodyCostPerTick::zero(),
        });
        registry
    }

    fn populated_state() -> (SimulationState, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();
        let actor = spawn_agent(&mut world, &mut event_log, Tick(0), "save-actor");
        let target = spawn_agent(&mut world, &mut event_log, Tick(1), "save-target");
        let (reserved_item, reservation) =
            spawn_item_with_reservation(&mut world, &mut event_log, actor);
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(3),
            cause: CauseRef::SystemTick(Tick(3)),
            actor_id: None,
            target_ids: Vec::new(),
            evidence: Vec::new(),
            place_id: None,
            state_deltas: Vec::new(),
            observed_entities: std::collections::BTreeMap::new(),
            visibility: VisibilitySpec::Hidden,
            witness_data: WitnessData::default(),
            tags: std::collections::BTreeSet::from([worldwake_core::EventTag::System]),
        }));

        let mut scheduler = Scheduler::new_with_tick(Tick(3), SystemManifest::canonical());
        let _ = scheduler.input_queue_mut().enqueue(
            Tick(3),
            InputKind::SwitchControl {
                from: None,
                to: Some(actor),
            },
        );
        let _ = scheduler.input_queue_mut().enqueue(
            Tick(5),
            InputKind::RequestAction {
                actor,
                def_id: ActionDefId(9),
                targets: vec![target],
                payload_override: None,
                mode: crate::ActionRequestMode::Strict,
            },
        );
        scheduler.insert_action(ActionInstance {
            instance_id: ActionInstanceId(7),
            def_id: ActionDefId(4),
            payload: ActionPayload::None,
            actor,
            targets: vec![target],
            start_tick: Tick(2),
            remaining_duration: ActionDuration::Finite(5),
            status: ActionStatus::Active,
            reservation_ids: vec![reservation],
            local_state: Some(ActionState::Empty),
        });

        let mut rng = DeterministicRng::new(Seed([0x44; 32]));
        let _ = rng.next_u32();
        let _ = rng.next_u64();
        let recipe_registry = populated_recipe_registry();

        let initial_hash = SimulationState::replay_bootstrap_hash_parts(
            &world,
            &event_log,
            &scheduler,
            &recipe_registry,
            &ControllerState::with_entity(actor),
            &rng,
        )
        .unwrap();
        let mut replay_state = ReplayState::new(
            initial_hash,
            Seed([0x55; 32]),
            Tick(3),
            ReplayRecordingConfig::every(NonZeroU64::new(2).unwrap()),
        );
        replay_state
            .record_input(
                scheduler
                    .input_queue()
                    .iter_in_sequence_order()
                    .next()
                    .unwrap()
                    .clone(),
            )
            .unwrap();
        replay_state
            .record_checkpoint(ReplayCheckpoint {
                tick: Tick(4),
                event_log_hash: state_hash(0x12),
                world_state_hash: state_hash(0x34),
            })
            .unwrap();

        (
            SimulationState::new(
                world,
                event_log,
                scheduler,
                recipe_registry,
                replay_state,
                ControllerState::with_entity(actor),
                rng,
            ),
            actor,
            reserved_item,
        )
    }

    #[allow(clippy::needless_pass_by_value)]
    fn deterministic_system(context: SystemExecutionContext<'_>) -> Result<(), SystemError> {
        let roll = context.rng.next_u32();
        if context.system_id != SystemId::Needs || roll & 1 == 1 {
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
            &format!("{}-{}-{roll}", context.system_id.as_str(), context.tick.0),
            ControlSource::Ai,
        )
        .map_err(|error| SystemError::new(error.to_string()))?;
        let _ = txn.commit(context.event_log);
        Ok(())
    }

    fn deterministic_systems() -> SystemDispatchTable {
        SystemDispatchTable::from_handlers([deterministic_system; SystemId::ALL.len()])
    }

    fn services<'a>(
        action_defs: &'a ActionDefRegistry,
        action_handlers: &'a ActionHandlerRegistry,
        recipe_registry: &'a RecipeRegistry,
        systems: &'a SystemDispatchTable,
    ) -> TickStepServices<'a> {
        TickStepServices {
            action_defs,
            action_handlers,
            recipe_registry,
            systems,
            input_producer: None,
            action_trace: None,
        }
    }

    fn continuation_state() -> SimulationState {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();
        let first = spawn_agent(&mut world, &mut event_log, Tick(0), "continuation-a");
        let second = spawn_agent(&mut world, &mut event_log, Tick(1), "continuation-b");
        let mut scheduler = Scheduler::new_with_tick(Tick(2), SystemManifest::canonical());
        let first_input = scheduler
            .input_queue_mut()
            .enqueue(
                Tick(2),
                InputKind::SwitchControl {
                    from: Some(first),
                    to: Some(second),
                },
            )
            .clone();
        let second_input = scheduler
            .input_queue_mut()
            .enqueue(
                Tick(4),
                InputKind::SwitchControl {
                    from: Some(second),
                    to: Some(first),
                },
            )
            .clone();
        let mut rng = DeterministicRng::new(Seed([0x77; 32]));
        let _ = rng.next_u32();
        let controller = ControllerState::with_entity(first);
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
        let mut replay_state = ReplayState::new(
            initial_hash,
            Seed([0x77; 32]),
            Tick(2),
            ReplayRecordingConfig::disabled(),
        );
        replay_state.record_input(first_input).unwrap();
        replay_state.record_input(second_input).unwrap();

        SimulationState::new(
            world,
            event_log,
            scheduler,
            recipe_registry,
            replay_state,
            controller,
            rng,
        )
    }

    fn advance_state(state: &mut SimulationState, ticks: u64) {
        let action_defs = ActionDefRegistry::new();
        let action_handlers = ActionHandlerRegistry::new();
        let systems = deterministic_systems();

        for _ in 0..ticks {
            let recipe_registry = state.recipe_registry().clone();
            let (world, event_log, scheduler, controller, rng) = state.runtime_parts_mut();
            step_tick(
                world,
                event_log,
                scheduler,
                controller,
                rng,
                services(&action_defs, &action_handlers, &recipe_registry, &systems),
            )
            .unwrap();
        }
    }

    fn temp_save_path(test_name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "worldwake-{test_name}-{}-{nanos}.bin",
            std::process::id()
        ))
    }

    #[test]
    fn save_to_bytes_roundtrip_preserves_full_nondefault_state() {
        let (state, actor, reserved_item) = populated_state();

        let bytes = save_to_bytes(&state).unwrap();
        let restored = load_from_bytes(&bytes).unwrap();

        assert_eq!(&bytes[..SAVE_MAGIC.len()], &SAVE_MAGIC);
        assert_eq!(
            u32::from_le_bytes(
                bytes[SAVE_MAGIC.len()..SAVE_MAGIC.len() + std::mem::size_of::<u32>()]
                    .try_into()
                    .unwrap()
            ),
            SAVE_FORMAT_VERSION
        );
        assert_eq!(restored, state);
        assert_eq!(restored.scheduler().active_actions().len(), 1);
        assert_eq!(restored.scheduler().input_queue().len(), 2);
        assert_eq!(restored.recipe_registry().len(), 1);
        assert_eq!(restored.replay_state().checkpoints().len(), 1);
        assert_eq!(restored.controller_state().controlled_entity(), Some(actor));
        assert!(!restored.world().reservations_for(reserved_item).is_empty());
    }

    #[test]
    fn file_save_roundtrip_matches_in_memory_format() {
        let (state, _, _) = populated_state();
        let path = temp_save_path("roundtrip");
        let expected_bytes = save_to_bytes(&state).unwrap();

        save(&state, &path).unwrap();
        let file_bytes = std::fs::read(&path).unwrap();
        let restored = load(&path).unwrap();

        assert_eq!(file_bytes, expected_bytes);
        assert_eq!(restored, state);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn load_rejects_wrong_magic() {
        let (state, _, _) = populated_state();
        let mut bytes = save_to_bytes(&state).unwrap();
        bytes[..SAVE_MAGIC.len()].copy_from_slice(b"NOPE");

        let error = load_from_bytes(&bytes).unwrap_err();

        assert!(matches!(error, SaveError::InvalidMagic));
    }

    #[test]
    fn load_rejects_wrong_version() {
        let (state, _, _) = populated_state();
        let mut bytes = save_to_bytes(&state).unwrap();
        bytes[SAVE_MAGIC.len()..SAVE_MAGIC.len() + std::mem::size_of::<u32>()]
            .copy_from_slice(&(SAVE_FORMAT_VERSION + 1).to_le_bytes());

        let error = load_from_bytes(&bytes).unwrap_err();

        assert!(matches!(
            error,
            SaveError::UnsupportedVersion {
                found,
                expected: SAVE_FORMAT_VERSION
            } if found == SAVE_FORMAT_VERSION + 1
        ));
    }

    #[test]
    fn load_rejects_truncated_payload() {
        let (state, _, _) = populated_state();
        let bytes = save_to_bytes(&state).unwrap();

        let error = load_from_bytes(&bytes[..bytes.len() - 1]).unwrap_err();

        assert!(matches!(error, SaveError::Deserialization(_)));
    }

    #[test]
    fn load_rejects_truncated_header() {
        let error = load_from_bytes(&SAVE_MAGIC[..2]).unwrap_err();

        assert!(matches!(error, SaveError::Deserialization(_)));
    }

    #[test]
    fn loaded_state_continues_identically_to_uninterrupted_execution() {
        let mut uninterrupted = continuation_state();
        let mut restored = load_from_bytes(&save_to_bytes(&uninterrupted).unwrap()).unwrap();

        advance_state(&mut uninterrupted, 4);
        advance_state(&mut restored, 4);

        assert_eq!(restored, uninterrupted);
    }
}
