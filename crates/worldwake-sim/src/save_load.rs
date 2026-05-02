use crate::{SaveableRuntime, SimulationState};
use std::fmt;
use std::path::Path;

pub const SAVE_MAGIC: [u8; 4] = *b"WWAK";
pub const SAVE_FORMAT_VERSION: u32 = 62;

const SAVE_HEADER_LEN: usize = SAVE_MAGIC.len() + std::mem::size_of::<u32>();
const PAYLOAD_LEN_WIDTH: usize = std::mem::size_of::<u64>();

#[derive(Debug)]
pub enum SaveError {
    Io(std::io::Error),
    Serialization(String),
    RuntimeSerialization(String),
    InvalidMagic,
    UnsupportedVersion { found: u32, expected: u32 },
    Deserialization(String),
    RuntimeDeserialization(String),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(source) => write!(f, "save/load I/O failed: {source}"),
            Self::Serialization(message) => {
                write!(f, "failed to serialize simulation state: {message}")
            }
            Self::RuntimeSerialization(message) => {
                write!(f, "failed to serialize runtime state: {message}")
            }
            Self::InvalidMagic => f.write_str("save data does not start with Worldwake save magic"),
            Self::UnsupportedVersion { found, expected } => write!(
                f,
                "unsupported save format version {found}; expected {expected}"
            ),
            Self::Deserialization(message) => {
                write!(f, "failed to deserialize simulation state: {message}")
            }
            Self::RuntimeDeserialization(message) => {
                write!(f, "failed to deserialize runtime state: {message}")
            }
        }
    }
}

impl std::error::Error for SaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(source) => Some(source),
            Self::Serialization(_)
            | Self::RuntimeSerialization(_)
            | Self::InvalidMagic
            | Self::UnsupportedVersion { .. }
            | Self::Deserialization(_)
            | Self::RuntimeDeserialization(_) => None,
        }
    }
}

impl From<std::io::Error> for SaveError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn save(
    state: &SimulationState,
    runtime: Option<&dyn SaveableRuntime>,
    path: &Path,
) -> Result<(), SaveError> {
    let bytes = save_to_bytes(state, runtime)?;
    std::fs::write(path, bytes).map_err(SaveError::Io)
}

pub fn load(path: &Path) -> Result<(SimulationState, Option<Vec<u8>>), SaveError> {
    let bytes = std::fs::read(path).map_err(SaveError::Io)?;
    load_from_bytes(&bytes)
}

pub fn save_to_bytes(
    state: &SimulationState,
    runtime: Option<&dyn SaveableRuntime>,
) -> Result<Vec<u8>, SaveError> {
    let sim_payload =
        bincode::serialize(state).map_err(|error| SaveError::Serialization(error.to_string()))?;
    let runtime_payload = runtime
        .map(SaveableRuntime::save_runtime_state)
        .transpose()?
        .unwrap_or_default();
    let sim_payload_len = u64::try_from(sim_payload.len()).map_err(|_| {
        SaveError::Serialization("simulation payload exceeds u64 length".to_string())
    })?;
    let runtime_payload_len = u64::try_from(runtime_payload.len()).map_err(|_| {
        SaveError::RuntimeSerialization("runtime payload exceeds u64 length".to_string())
    })?;
    let mut bytes = Vec::with_capacity(
        SAVE_HEADER_LEN + PAYLOAD_LEN_WIDTH * 2 + sim_payload.len() + runtime_payload.len(),
    );
    bytes.extend_from_slice(&SAVE_MAGIC);
    bytes.extend_from_slice(&SAVE_FORMAT_VERSION.to_le_bytes());
    bytes.extend_from_slice(&sim_payload_len.to_le_bytes());
    bytes.extend_from_slice(&sim_payload);
    bytes.extend_from_slice(&runtime_payload_len.to_le_bytes());
    bytes.extend_from_slice(&runtime_payload);
    Ok(bytes)
}

pub fn load_from_bytes(bytes: &[u8]) -> Result<(SimulationState, Option<Vec<u8>>), SaveError> {
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

    match found {
        SAVE_FORMAT_VERSION => load_current_format(payload),
        _ => Err(SaveError::UnsupportedVersion {
            found,
            expected: SAVE_FORMAT_VERSION,
        }),
    }
}

fn load_current_format(bytes: &[u8]) -> Result<(SimulationState, Option<Vec<u8>>), SaveError> {
    let (sim_payload, rest) = split_length_prefixed_payload(bytes, "simulation")?;
    let state = bincode::deserialize(sim_payload)
        .map_err(|error| SaveError::Deserialization(error.to_string()))?;
    let (runtime_payload, trailing) = split_length_prefixed_payload(rest, "runtime")?;
    if !trailing.is_empty() {
        return Err(SaveError::Deserialization(
            "save data has trailing bytes after runtime payload".to_string(),
        ));
    }

    let runtime = (!runtime_payload.is_empty()).then(|| runtime_payload.to_vec());
    Ok((state, runtime))
}

fn split_length_prefixed_payload<'a>(
    bytes: &'a [u8],
    label: &str,
) -> Result<(&'a [u8], &'a [u8]), SaveError> {
    if bytes.len() < PAYLOAD_LEN_WIDTH {
        return Err(SaveError::Deserialization(format!(
            "save data is truncated before the {label} length prefix completes"
        )));
    }

    let (len_bytes, rest) = bytes.split_at(PAYLOAD_LEN_WIDTH);
    let payload_len = u64::from_le_bytes(
        len_bytes
            .try_into()
            .expect("validated fixed-width payload length"),
    );
    let payload_len = usize::try_from(payload_len).map_err(|_| {
        SaveError::Deserialization(format!(
            "{label} payload length does not fit into platform usize"
        ))
    })?;
    if rest.len() < payload_len {
        return Err(SaveError::Deserialization(format!(
            "save data is truncated before the full {label} payload completes"
        )));
    }

    Ok(rest.split_at(payload_len))
}

#[cfg(test)]
mod tests {
    use super::{
        SAVE_FORMAT_VERSION, SAVE_MAGIC, SaveError, load, load_from_bytes, save, save_to_bytes,
    };
    use crate::belief_view::BeliefStatus;
    use crate::{
        ActionDefRegistry, ActionDuration, ActionHandlerRegistry, ActionInstance, ActionInstanceId,
        ActionPayload, ActionState, ActionStatus, ControllerState, DeterministicRng, InputKind,
        RecipeDefinition, RecipeRegistry, ReplayCheckpoint, ReplayRecordingConfig, ReplayState,
        SaveableRuntime, Scheduler, SimulationState, SystemDispatchTable, SystemError,
        SystemExecutionContext, SystemId, SystemManifest, TickStepServices, step_tick,
    };
    use std::num::{NonZeroU32, NonZeroU64};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use worldwake_core::{
        AcquisitionQuantity, ActionDefId, ActionDomain, AgentBeliefStore, BeliefClaimKey,
        BeliefSnapshot, BeliefStatusTag, BelievedActivity, BelievedEntityState, BlockerKey,
        BlockerRecordedPayload, BlockingFact, BodyCostPerTick, CauseRef, ClaimId, ClaimValue,
        CommodityKind, CommodityPurpose, ControlSource, DecisionEventPayload, Discrepancy,
        EmitterTag, EntityBeliefAspect, EntityBeliefClaim, EntityId, EventLog, EventPayload,
        EventTag, EventView, EvidenceKindTag, EvidenceSummary, ExpectationBasis, ExpectationId,
        ExpectationMismatchPayload, ExpectationRecord, ExpectationState, ExpectationStore,
        GoalAbandonReason, GoalAbandonedPayload, GoalCommittedPayload, GoalKey, GoalKind,
        GoalOfferedPayload, GoalRejectionReason, GoalSuppressedPayload, GoalSuspendedPayload,
        GoalSwitchReason, GroundComfortTag, HomeostaticNeedId, LastSeenMemory, LastSeenProvenance,
        LastSeenRecord, LatrineFullness, MaterializationTag, MetabolismProfile, PendingEvent,
        PerceptionSource, PlaceDirtiness, PlanAdoptedPayload, PlanInvalidatedPayload,
        PlanInvalidationReason, PursuitInvalidationReasonTag, Quantity, RejectedAlternativeSummary,
        RepairAppliedPayload, RepairKind, ReplanReason, ReplanTriggeredPayload, ReservationId,
        RewardEncumbrance, Seed, ShelterTag, SleepEpisode, SleepEpisodeEndedPayload,
        SleepEpisodeStartedPayload, SleepQualityProfile, SleepRecoveryModifier, StateHash,
        SuspensionReason, Tick, TickRange, UniqueItemKind, VisibilitySpec, WakeCondition,
        WakeReason, WashBasinState, WashFacilityUsedPayload, WasteCreatedPayload, WasteSource,
        WitnessData, WorkstationMarker, WorkstationTag, World, WorldTxn, build_prototype_world,
        test_utils::{
            sample_preference_profile, sample_route_experience, sample_source_reliability,
        },
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

    fn believed_entity_state_with_activity(
        place: EntityId,
        observed_tick: Tick,
    ) -> BelievedEntityState {
        BelievedEntityState {
            believed_kind: None,
            last_known_place: Some(place),
            last_known_inventory: std::collections::BTreeMap::from([(
                CommodityKind::Apple,
                Quantity(3),
            )]),
            workstation_tag: Some(WorkstationTag::Mill),
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: Some(BelievedActivity {
                action_domain: ActionDomain::Production,
                target: Some(place),
                observed_tick,
            }),
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            ..BelievedEntityState::single_observation_defaults(
                observed_tick,
                PerceptionSource::DirectObservation,
            )
        }
    }

    fn populated_state() -> (SimulationState, EntityId, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        world.set_commodity_decay(std::collections::BTreeMap::from([(
            CommodityKind::Waste,
            std::num::NonZeroU32::new(17).unwrap(),
        )]));
        let mut event_log = EventLog::new();
        let actor = spawn_agent(&mut world, &mut event_log, Tick(0), "save-actor");
        let target = spawn_agent(&mut world, &mut event_log, Tick(1), "save-target");
        let mut profile_txn = new_txn(&mut world, Tick(1), CauseRef::Bootstrap);
        profile_txn
            .set_component_metabolism_profile(
                actor,
                MetabolismProfile {
                    min_sleep_ticks: NonZeroU32::new(11).unwrap(),
                    ..MetabolismProfile::default()
                },
            )
            .unwrap();
        let _ = profile_txn.commit(&mut event_log);
        let mut office_txn = new_txn(&mut world, Tick(1), CauseRef::Bootstrap);
        let office = office_txn.create_office("save-office").unwrap();
        office_txn
            .set_component_reward_encumbrance(
                office,
                RewardEncumbrance {
                    reservations: vec![worldwake_core::RewardReservation {
                        bounty_artifact: worldwake_core::test_utils::entity_id(99, 0),
                        commodity: CommodityKind::Coin,
                        quantity: Quantity(31),
                    }],
                },
            )
            .unwrap();
        let _ = office_txn.commit(&mut event_log);
        let belief_place = world.topology().place_ids().next().unwrap();
        let mut sleep_txn = new_txn(&mut world, Tick(2), CauseRef::Bootstrap);
        sleep_txn
            .set_component_sleep_episode(
                actor,
                SleepEpisode {
                    place: belief_place,
                    start_tick: Tick(2),
                    intended_min_ticks: NonZeroU32::new(4).unwrap(),
                    intended_max_ticks: NonZeroU32::new(40).unwrap(),
                    target_recovery: worldwake_core::Permille::new(750).unwrap(),
                    accumulated_recovery: worldwake_core::Permille::new(125).unwrap(),
                    recovery_modifier: SleepRecoveryModifier::new(1250),
                    wake_conditions: vec![
                        WakeCondition::TargetRecoveryReached,
                        WakeCondition::ScheduledCommitmentDue { tick: Tick(20) },
                    ],
                },
            )
            .unwrap();
        sleep_txn
            .set_component_sleep_quality_profile(
                belief_place,
                SleepQualityProfile {
                    shelter: ShelterTag::Shelter,
                    ground_comfort: GroundComfortTag::Soft,
                    recovery_modifier: SleepRecoveryModifier::new(1250),
                },
            )
            .unwrap();
        sleep_txn
            .set_component_place_dirtiness(
                belief_place,
                PlaceDirtiness {
                    value: worldwake_core::Permille::new(500).unwrap(),
                    decay_per_tick: worldwake_core::Permille::new(3).unwrap(),
                    dirtiness_per_use: worldwake_core::Permille::new(90).unwrap(),
                },
            )
            .unwrap();
        sleep_txn
            .set_component_latrine_fullness(
                belief_place,
                LatrineFullness {
                    fill: worldwake_core::Permille::new(650).unwrap(),
                    fill_per_use: worldwake_core::Permille::new(70).unwrap(),
                    critical_threshold: worldwake_core::Permille::new(850).unwrap(),
                },
            )
            .unwrap();
        let basin = sleep_txn.create_entity(worldwake_core::EntityKind::Facility);
        sleep_txn
            .set_component_workstation_marker(basin, WorkstationMarker(WorkstationTag::WashBasin))
            .unwrap();
        sleep_txn
            .set_component_wash_basin_state(
                basin,
                WashBasinState {
                    clean_water_units: 4,
                    max_clean_water: 12,
                    refill_per_tick: 2,
                    units_per_full_wash: 3,
                    dirtiness_level: worldwake_core::Permille::new(250).unwrap(),
                    dirtiness_per_use: worldwake_core::Permille::new(60).unwrap(),
                },
            )
            .unwrap();
        let _ = sleep_txn.commit(&mut event_log);
        let (reserved_item, reservation) =
            spawn_item_with_reservation(&mut world, &mut event_log, actor);
        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(
            target,
            believed_entity_state_with_activity(belief_place, Tick(3)),
        );
        beliefs.record_entity_claim(EntityBeliefClaim {
            claim_id: ClaimId(1),
            subject: target,
            aspect: EntityBeliefAspect::Location,
            value: ClaimValue::Place(Some(belief_place)),
            source: PerceptionSource::DirectObservation,
            acquired_tick: Tick(3),
            claimed_event_tick: Some(Tick(3)),
            confidence: worldwake_core::Permille::new(1000).unwrap(),
            refuted_at_tick: None,
        });
        beliefs.record_entity_claim(EntityBeliefClaim {
            claim_id: ClaimId(2),
            subject: target,
            aspect: EntityBeliefAspect::Alive,
            value: ClaimValue::Bool(true),
            source: PerceptionSource::DirectObservation,
            acquired_tick: Tick(3),
            claimed_event_tick: None,
            confidence: worldwake_core::Permille::new(1000).unwrap(),
            refuted_at_tick: Some(Tick(18)),
        });
        let mut belief_txn = new_txn(&mut world, Tick(3), CauseRef::Bootstrap);
        belief_txn
            .set_component_agent_belief_store(actor, beliefs)
            .unwrap();
        belief_txn
            .set_component_route_experience(actor, sample_route_experience())
            .unwrap();
        belief_txn
            .set_component_source_reliability(actor, sample_source_reliability())
            .unwrap();
        belief_txn
            .set_component_preference_profile(actor, sample_preference_profile())
            .unwrap();
        let mut expectation_store = ExpectationStore::default();
        expectation_store.records.insert(
            ExpectationId(1),
            ExpectationRecord {
                id: ExpectationId(1),
                owner: actor,
                subject: target,
                expected_place: belief_place,
                deadline_tick: Tick(8),
                grace_ticks: 3,
                basis: ExpectationBasis::RoutineReturn,
                state: ExpectationState::Active,
                created_tick: Tick(2),
            },
        );
        belief_txn
            .set_component_expectation_store(actor, expectation_store)
            .unwrap();
        belief_txn
            .set_component_last_seen_memory(
                actor,
                LastSeenMemory {
                    records: std::collections::BTreeMap::from([(
                        target,
                        LastSeenRecord {
                            subject: target,
                            place: belief_place,
                            observed_tick: Tick(3),
                            source: actor,
                            provenance: LastSeenProvenance::DirectObservation,
                        },
                    )]),
                    capacity: 7,
                },
            )
            .unwrap();
        let _ = belief_txn.commit(&mut event_log);
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(3),
            cause: CauseRef::SystemTick(Tick(3)),
            actor_id: None,
            action_name: None,
            target_ids: Vec::new(),
            evidence: Vec::new(),
            place_id: None,
            state_deltas: Vec::new(),
            observed_entities: std::collections::BTreeMap::new(),
            visibility: VisibilitySpec::Hidden,
            witness_data: WitnessData::default(),
            tags: std::collections::BTreeSet::from([worldwake_core::EventTag::System]),
            decision_payload: None,
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
                provenance: crate::RequestProvenance::External,
            },
        );
        scheduler.insert_action(ActionInstance {
            instance_id: ActionInstanceId(7),
            def_id: ActionDefId(4),
            payload: ActionPayload::None,
            actor,
            targets: vec![target],
            start_tick: Tick(2),
            remaining_duration: ActionDuration::new(5),
            status: ActionStatus::Active,
            reservation_ids: vec![reservation],
            local_state: Some(ActionState::Empty),
            body_cost_override: None,
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
            target,
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
            request_resolution_trace: None,
            politics_trace: None,
            perception_trace: None,
            institutional_knowledge_trace: None,
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

    fn decision_test_entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn decision_goal(kind: GoalKind) -> GoalKey {
        GoalKey::from(kind)
    }

    fn append_decision_event(
        state: &mut SimulationState,
        tick: Tick,
        actor: EntityId,
        place: EntityId,
        tag: EventTag,
        payload: DecisionEventPayload,
    ) {
        let _ = state
            .event_log_mut()
            .emit(PendingEvent::from_payload(EventPayload {
                tick,
                cause: CauseRef::SystemTick(tick),
                actor_id: Some(actor),
                action_name: None,
                target_ids: Vec::new(),
                evidence: Vec::new(),
                place_id: Some(place),
                state_deltas: Vec::new(),
                observed_entities: std::collections::BTreeMap::new(),
                visibility: VisibilitySpec::Hidden,
                witness_data: WitnessData::default(),
                tags: std::collections::BTreeSet::from([tag]),
                decision_payload: Some(payload),
            }));
    }

    fn sample_decision_events(
        actor: EntityId,
        target: EntityId,
        place: EntityId,
    ) -> Vec<(EventTag, DecisionEventPayload)> {
        let trade_goal = decision_goal(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let sleep_goal = decision_goal(GoalKind::Sleep);
        let produce_goal = decision_goal(GoalKind::ProduceCommodity {
            recipe_id: worldwake_core::RecipeId(7),
        });
        let move_goal = decision_goal(GoalKind::MoveCargo {
            commodity: CommodityKind::Water,
            destination: place,
        });
        let patrol_goal = decision_goal(GoalKind::Patrol { place });
        let office = decision_test_entity(301);
        let candidate = decision_test_entity(302);
        let support_goal = decision_goal(GoalKind::SupportCandidateForOffice { office, candidate });
        let explore_goal = decision_goal(GoalKind::ExploreLocation {
            target_place: place,
            motivating_need: worldwake_core::ExplorationMotivation::Proactive,
            hypothesis: worldwake_core::HypothesisKind::Proactive,
        });
        let claim_key = BeliefClaimKey {
            subject: target,
            aspect: EntityBeliefAspect::Inventory(CommodityKind::Bread),
        };
        let blocker_key = BlockerKey {
            goal_key: move_goal,
            place: Some(place),
            target: Some(target),
            action_def: Some(ActionDefId(6)),
        };

        vec![
            (
                EventTag::GoalOffered,
                DecisionEventPayload::GoalOffered(GoalOfferedPayload {
                    agent: actor,
                    goal_key: trade_goal,
                    emitter: EmitterTag::Enterprise,
                    source_evidence: EvidenceSummary {
                        evidence_kind_counts: std::collections::BTreeMap::from([
                            (EvidenceKindTag::LearnedOpportunity, 1),
                            (EvidenceKindTag::PerceptionObservation, 2),
                        ]),
                    },
                }),
            ),
            (
                EventTag::GoalSuppressed,
                DecisionEventPayload::GoalSuppressed(GoalSuppressedPayload {
                    agent: actor,
                    goal_key: sleep_goal,
                    reason: GoalRejectionReason::SuppressedByStressPolicy,
                }),
            ),
            (
                EventTag::GoalCommitted,
                DecisionEventPayload::GoalCommitted(GoalCommittedPayload {
                    agent: actor,
                    goal_key: produce_goal,
                    motive_score: 420,
                    rejected_alternatives: vec![RejectedAlternativeSummary {
                        goal_key: trade_goal,
                        rejection_reason: GoalRejectionReason::LowerMotive,
                        score_gap: 17,
                    }],
                }),
            ),
            (
                EventTag::GoalSuspended,
                DecisionEventPayload::GoalSuspended(GoalSuspendedPayload {
                    agent: actor,
                    goal_key: move_goal,
                    reason: SuspensionReason::RouteBlocked,
                }),
            ),
            (
                EventTag::GoalAbandoned,
                DecisionEventPayload::GoalAbandoned(GoalAbandonedPayload {
                    agent: actor,
                    goal_key: patrol_goal,
                    reason: GoalAbandonReason::GoalSwitched {
                        new_goal: explore_goal,
                        switch_kind: GoalSwitchReason::HigherPriorityGoal,
                    },
                }),
            ),
            (
                EventTag::SleepEpisodeStarted,
                DecisionEventPayload::SleepEpisodeStarted(SleepEpisodeStartedPayload {
                    sleeper: actor,
                    place,
                    intended_min_ticks: NonZeroU32::new(4).unwrap(),
                    intended_max_ticks: NonZeroU32::new(40).unwrap(),
                    target_recovery: worldwake_core::Permille::new(750).unwrap(),
                    wake_conditions: vec![
                        WakeCondition::IntendedDurationReached,
                        WakeCondition::ProjectedNeedBreach {
                            need: HomeostaticNeedId::Thirst,
                        },
                    ],
                    recovery_modifier: SleepRecoveryModifier::new(1250),
                }),
            ),
            (
                EventTag::SleepEpisodeEnded,
                DecisionEventPayload::SleepEpisodeEnded(SleepEpisodeEndedPayload {
                    sleeper: actor,
                    place,
                    start_tick: Tick(20),
                    end_tick: Tick(33),
                    end_reason: WakeReason::ProjectedNeedBreach {
                        need: HomeostaticNeedId::Thirst,
                        projected_breach_tick: Tick(34),
                    },
                    accumulated_recovery: worldwake_core::Permille::new(250).unwrap(),
                    final_fatigue: worldwake_core::Permille::new(500).unwrap(),
                }),
            ),
            (
                EventTag::WasteCreated,
                DecisionEventPayload::WasteCreated(WasteCreatedPayload {
                    creator: actor,
                    place,
                    waste_lot: decision_test_entity(303),
                    source: WasteSource::WildernessRelief,
                    place_dirtiness_delta: worldwake_core::Permille::new(80).unwrap(),
                }),
            ),
            (
                EventTag::WashFacilityUsed,
                DecisionEventPayload::WashFacilityUsed(WashFacilityUsedPayload {
                    user: actor,
                    basin: decision_test_entity(304),
                    water_consumed: 1,
                    agent_dirtiness_delta: worldwake_core::Permille::new(500).unwrap(),
                    basin_dirtiness_delta: worldwake_core::Permille::new(25).unwrap(),
                    partial: true,
                }),
            ),
            (
                EventTag::PlanAdopted,
                DecisionEventPayload::PlanAdopted(PlanAdoptedPayload {
                    agent: actor,
                    goal_key: trade_goal,
                    plan_step_count: 3,
                }),
            ),
            (
                EventTag::PlanInvalidated,
                DecisionEventPayload::PlanInvalidated(PlanInvalidatedPayload {
                    agent: actor,
                    goal_key: move_goal,
                    reason: PlanInvalidationReason::BeliefUpdate { claim_key },
                    belief_snapshot: Some(BeliefSnapshot {
                        confidence: worldwake_core::Permille::new(375).unwrap(),
                        status: BeliefStatusTag::Stale,
                        acquired_tick: Tick(14),
                    }),
                }),
            ),
            (
                EventTag::ExpectationMismatch,
                DecisionEventPayload::ExpectationMismatch(ExpectationMismatchPayload {
                    agent: actor,
                    goal_key: trade_goal,
                    step_index: 1,
                    expected_materializations: vec![MaterializationTag::SplitOffLot],
                    expectation_kind: None,
                    mismatch_detail: None,
                }),
            ),
            (
                EventTag::RepairApplied,
                DecisionEventPayload::RepairApplied(RepairAppliedPayload {
                    agent: actor,
                    goal_key: support_goal,
                    step_index: 2,
                    repair_kind: RepairKind::AlternateMerchant,
                    substitute_target: Some(target),
                }),
            ),
            (
                EventTag::ReplanTriggered,
                DecisionEventPayload::ReplanTriggered(ReplanTriggeredPayload {
                    agent: actor,
                    goal_key: move_goal,
                    reason: ReplanReason::PlanInvalidated {
                        reason: PlanInvalidationReason::PursuitInvalidated {
                            reason: PursuitInvalidationReasonTag::PlaceChanged,
                        },
                    },
                }),
            ),
            (
                EventTag::BlockerRecorded,
                DecisionEventPayload::BlockerRecorded(BlockerRecordedPayload {
                    agent: actor,
                    blocker_key,
                    discrepancy: Some(Discrepancy::RouteUnknown),
                    blocking_fact: Some(BlockingFact::NoKnownPath),
                    expires_tick: Tick(99),
                    belief_snapshot: Some(BeliefSnapshot {
                        confidence: worldwake_core::Permille::new(650).unwrap(),
                        status: BeliefStatusTag::Probable,
                        acquired_tick: Tick(18),
                    }),
                }),
            ),
        ]
    }

    struct MockRuntime {
        bytes: Vec<u8>,
    }

    impl SaveableRuntime for MockRuntime {
        fn save_runtime_state(&self) -> Result<Vec<u8>, SaveError> {
            Ok(self.bytes.clone())
        }
    }

    #[test]
    fn save_to_bytes_roundtrip_preserves_full_nondefault_state() {
        let (state, actor, target, reserved_item) = populated_state();

        let bytes = save_to_bytes(&state, None).unwrap();
        let (restored, runtime) = load_from_bytes(&bytes).unwrap();

        assert_eq!(&bytes[..SAVE_MAGIC.len()], &SAVE_MAGIC);
        assert_eq!(SAVE_FORMAT_VERSION, 62);
        assert_eq!(
            u32::from_le_bytes(
                bytes[SAVE_MAGIC.len()..SAVE_MAGIC.len() + std::mem::size_of::<u32>()]
                    .try_into()
                    .unwrap()
            ),
            SAVE_FORMAT_VERSION
        );
        assert_eq!(runtime, None);
        assert_eq!(restored, state);
        assert_eq!(restored.scheduler().active_actions().len(), 1);
        assert_eq!(restored.scheduler().input_queue().len(), 2);
        assert_eq!(restored.recipe_registry().len(), 1);
        assert_eq!(restored.replay_state().checkpoints().len(), 1);
        assert_eq!(restored.controller_state().controlled_entity(), Some(actor));
        assert_eq!(
            restored.world().commodity_decay(),
            state.world().commodity_decay()
        );
        assert_eq!(
            restored.world().get_component_sleep_episode(actor),
            state.world().get_component_sleep_episode(actor)
        );
        assert_eq!(
            restored
                .world()
                .get_component_metabolism_profile(actor)
                .unwrap()
                .min_sleep_ticks,
            NonZeroU32::new(11).unwrap()
        );
        let restored_sleep_place = state.world().topology().place_ids().next().unwrap();
        assert_eq!(
            restored
                .world()
                .get_component_sleep_quality_profile(restored_sleep_place),
            state
                .world()
                .get_component_sleep_quality_profile(restored_sleep_place)
        );
        assert_eq!(
            restored
                .world()
                .get_component_place_dirtiness(restored_sleep_place),
            state
                .world()
                .get_component_place_dirtiness(restored_sleep_place)
        );
        assert_eq!(
            restored
                .world()
                .get_component_latrine_fullness(restored_sleep_place),
            state
                .world()
                .get_component_latrine_fullness(restored_sleep_place)
        );
        let restored_basins: Vec<_> = restored.world().entities_with_wash_basin_state().collect();
        assert_eq!(restored_basins.len(), 1);
        let restored_basin = restored_basins[0];
        assert_eq!(
            restored
                .world()
                .get_component_wash_basin_state(restored_basin),
            state.world().get_component_wash_basin_state(restored_basin)
        );
        assert!(!restored.world().reservations_for(reserved_item).is_empty());
        let restored_belief = restored
            .world()
            .get_component_agent_belief_store(actor)
            .unwrap();
        let restored_summary = restored_belief.get_entity(&target).unwrap();
        assert_eq!(
            restored_summary.believed_activity,
            Some(BelievedActivity {
                action_domain: ActionDomain::Production,
                target: restored_summary.last_known_place,
                observed_tick: Tick(3),
            })
        );
        let restored_claims = restored_belief.entity_claims.get(&target).unwrap();
        assert_eq!(restored_claims.len(), 2);
        assert_eq!(restored_claims[0].claim_id, ClaimId(1));
        assert_eq!(restored_claims[0].refuted_at_tick, None);
        assert_eq!(restored_claims[1].claim_id, ClaimId(2));
        assert_eq!(restored_claims[1].refuted_at_tick, Some(Tick(18)));
        assert_eq!(restored_belief.next_claim_id, ClaimId(3));
        let restored_belief_place = restored_summary.last_known_place.unwrap();
        let restored_expectation_store = restored
            .world()
            .get_component_expectation_store(actor)
            .unwrap();
        assert_eq!(
            restored_expectation_store.records.get(&ExpectationId(1)),
            Some(&ExpectationRecord {
                id: ExpectationId(1),
                owner: actor,
                subject: target,
                expected_place: restored_belief_place,
                deadline_tick: Tick(8),
                grace_ticks: 3,
                basis: ExpectationBasis::RoutineReturn,
                state: ExpectationState::Active,
                created_tick: Tick(2),
            })
        );
        let restored_last_seen_memory = restored
            .world()
            .get_component_last_seen_memory(actor)
            .unwrap();
        assert_eq!(restored_last_seen_memory.capacity, 7);
        assert_eq!(
            restored_last_seen_memory.records.get(&target),
            Some(&LastSeenRecord {
                subject: target,
                place: restored_belief_place,
                observed_tick: Tick(3),
                source: actor,
                provenance: LastSeenProvenance::DirectObservation,
            })
        );
    }

    #[test]
    fn save_to_bytes_writes_current_format_version() {
        let (state, _, _, _) = populated_state();

        let bytes = save_to_bytes(&state, None).unwrap();
        let version_offset = SAVE_MAGIC.len();
        let version = u32::from_le_bytes(
            bytes[version_offset..version_offset + std::mem::size_of::<u32>()]
                .try_into()
                .unwrap(),
        );

        assert_eq!(version, SAVE_FORMAT_VERSION);
    }

    #[test]
    fn file_save_roundtrip_matches_in_memory_format() {
        let (state, _, _, _) = populated_state();
        let path = temp_save_path("roundtrip");
        let expected_bytes = save_to_bytes(&state, None).unwrap();

        save(&state, None, &path).unwrap();
        let file_bytes = std::fs::read(&path).unwrap();
        let (restored, runtime) = load(&path).unwrap();

        assert_eq!(file_bytes, expected_bytes);
        assert_eq!(runtime, None);
        assert_eq!(restored, state);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_to_bytes_roundtrip_preserves_runtime_payload() {
        let (state, _, _, _) = populated_state();
        let runtime = MockRuntime {
            bytes: vec![9, 8, 7, 6],
        };

        let bytes = save_to_bytes(&state, Some(&runtime)).unwrap();
        let (restored, runtime_payload) = load_from_bytes(&bytes).unwrap();

        assert_eq!(restored, state);
        assert_eq!(runtime_payload, Some(vec![9, 8, 7, 6]));
    }

    #[test]
    fn save_to_bytes_roundtrip_preserves_decision_event_payloads() {
        let (mut state, actor, target, _) = populated_state();
        let place = state.world().topology().place_ids().next().unwrap();
        let decision_events = sample_decision_events(actor, target, place);

        for (offset, (tag, payload)) in decision_events.iter().cloned().enumerate() {
            append_decision_event(
                &mut state,
                Tick(20 + u64::try_from(offset).unwrap()),
                actor,
                place,
                tag,
                payload,
            );
        }

        let bytes = save_to_bytes(&state, None).unwrap();
        let (restored, runtime) = load_from_bytes(&bytes).unwrap();

        assert_eq!(runtime, None);
        assert_eq!(restored, state);

        for event_id in restored.event_log().events_by_tag(EventTag::GoalOffered) {
            assert!(restored.event_log().get(*event_id).is_some());
        }

        let original_payloads: Vec<_> = state
            .event_log()
            .events_by_tag(EventTag::GoalOffered)
            .iter()
            .chain(state.event_log().events_by_tag(EventTag::GoalSuppressed))
            .chain(state.event_log().events_by_tag(EventTag::GoalCommitted))
            .chain(state.event_log().events_by_tag(EventTag::GoalSuspended))
            .chain(state.event_log().events_by_tag(EventTag::GoalAbandoned))
            .chain(
                state
                    .event_log()
                    .events_by_tag(EventTag::SleepEpisodeStarted),
            )
            .chain(state.event_log().events_by_tag(EventTag::SleepEpisodeEnded))
            .chain(state.event_log().events_by_tag(EventTag::WasteCreated))
            .chain(state.event_log().events_by_tag(EventTag::WashFacilityUsed))
            .chain(state.event_log().events_by_tag(EventTag::PlanAdopted))
            .chain(state.event_log().events_by_tag(EventTag::PlanInvalidated))
            .chain(
                state
                    .event_log()
                    .events_by_tag(EventTag::ExpectationMismatch),
            )
            .chain(state.event_log().events_by_tag(EventTag::RepairApplied))
            .chain(state.event_log().events_by_tag(EventTag::ReplanTriggered))
            .chain(state.event_log().events_by_tag(EventTag::BlockerRecorded))
            .map(|event_id| {
                state
                    .event_log()
                    .get(*event_id)
                    .unwrap()
                    .decision_payload()
                    .unwrap()
                    .clone()
            })
            .collect();
        let restored_payloads: Vec<_> = restored
            .event_log()
            .events_by_tag(EventTag::GoalOffered)
            .iter()
            .chain(restored.event_log().events_by_tag(EventTag::GoalSuppressed))
            .chain(restored.event_log().events_by_tag(EventTag::GoalCommitted))
            .chain(restored.event_log().events_by_tag(EventTag::GoalSuspended))
            .chain(restored.event_log().events_by_tag(EventTag::GoalAbandoned))
            .chain(
                restored
                    .event_log()
                    .events_by_tag(EventTag::SleepEpisodeStarted),
            )
            .chain(
                restored
                    .event_log()
                    .events_by_tag(EventTag::SleepEpisodeEnded),
            )
            .chain(restored.event_log().events_by_tag(EventTag::WasteCreated))
            .chain(
                restored
                    .event_log()
                    .events_by_tag(EventTag::WashFacilityUsed),
            )
            .chain(restored.event_log().events_by_tag(EventTag::PlanAdopted))
            .chain(
                restored
                    .event_log()
                    .events_by_tag(EventTag::PlanInvalidated),
            )
            .chain(
                restored
                    .event_log()
                    .events_by_tag(EventTag::ExpectationMismatch),
            )
            .chain(restored.event_log().events_by_tag(EventTag::RepairApplied))
            .chain(
                restored
                    .event_log()
                    .events_by_tag(EventTag::ReplanTriggered),
            )
            .chain(
                restored
                    .event_log()
                    .events_by_tag(EventTag::BlockerRecorded),
            )
            .map(|event_id| {
                restored
                    .event_log()
                    .get(*event_id)
                    .unwrap()
                    .decision_payload()
                    .unwrap()
                    .clone()
            })
            .collect();

        assert_eq!(restored_payloads, original_payloads);
        assert_eq!(restored_payloads.len(), decision_events.len());

        for (original, roundtrip) in original_payloads.iter().zip(&restored_payloads) {
            assert_eq!(
                bincode::serialize(roundtrip).unwrap(),
                bincode::serialize(original).unwrap()
            );
        }
    }

    #[test]
    fn belief_status_tag_serialization_matches_belief_status_ordinals() {
        let pairs = [
            (BeliefStatus::Certain, BeliefStatusTag::Certain),
            (BeliefStatus::Probable, BeliefStatusTag::Probable),
            (BeliefStatus::Stale, BeliefStatusTag::Stale),
            (BeliefStatus::Disputed, BeliefStatusTag::Disputed),
            (BeliefStatus::Contradicted, BeliefStatusTag::Contradicted),
        ];

        for (sim_status, core_status) in pairs {
            assert_eq!(
                bincode::serialize(&sim_status).unwrap(),
                bincode::serialize(&core_status).unwrap()
            );
        }
    }

    #[test]
    fn load_rejects_wrong_magic() {
        let (state, _, _, _) = populated_state();
        let mut bytes = save_to_bytes(&state, None).unwrap();
        bytes[..SAVE_MAGIC.len()].copy_from_slice(b"NOPE");

        let error = load_from_bytes(&bytes).unwrap_err();

        assert!(matches!(error, SaveError::InvalidMagic));
    }

    #[test]
    fn load_rejects_wrong_version() {
        let (state, _, _, _) = populated_state();
        let mut bytes = save_to_bytes(&state, None).unwrap();
        bytes[SAVE_MAGIC.len()..SAVE_MAGIC.len() + std::mem::size_of::<u32>()]
            .copy_from_slice(&(SAVE_FORMAT_VERSION - 1).to_le_bytes());

        let error = load_from_bytes(&bytes).unwrap_err();

        assert!(matches!(
            error,
            SaveError::UnsupportedVersion {
                found,
                expected: SAVE_FORMAT_VERSION
            } if found == SAVE_FORMAT_VERSION - 1
        ));
    }

    #[test]
    fn load_rejects_truncated_payload() {
        let (state, _, _, _) = populated_state();
        let bytes = save_to_bytes(&state, None).unwrap();

        let error = load_from_bytes(&bytes[..bytes.len() - 1]).unwrap_err();

        assert!(matches!(error, SaveError::Deserialization(_)));
    }

    #[test]
    fn load_rejects_trailing_bytes_after_runtime_payload() {
        let (state, _, _, _) = populated_state();
        let mut bytes = save_to_bytes(&state, None).unwrap();
        bytes.push(0xAA);

        let error = load_from_bytes(&bytes).unwrap_err();

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
        let (mut restored, runtime) =
            load_from_bytes(&save_to_bytes(&uninterrupted, None).unwrap()).unwrap();
        assert_eq!(runtime, None);

        advance_state(&mut uninterrupted, 4);
        advance_state(&mut restored, 4);

        assert_eq!(restored, uninterrupted);
    }
}
