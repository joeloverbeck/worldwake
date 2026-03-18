use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU64;

use worldwake_core::{
    build_believed_entity_state, ActionDefId, AgentBeliefStore, BeliefConfidencePolicy,
    BelievedEntityState, CauseRef, ControlSource, EntityId, EventLog, EventPayload, EventTag,
    PendingEvent, PerceptionProfile, PerceptionSource, Permille, Place, Seed,
    SocialObservationKind, StateHash, TellProfile, Tick, Topology, TravelEdge, TravelEdgeId,
    VisibilitySpec, WitnessData, World, WorldTxn,
};
use worldwake_sim::{
    get_affordances, record_tick_checkpoint, replay_and_verify, step_tick, ActionPayload,
    ActionRequestMode, DeterministicRng, InputKind, PerAgentBeliefView, RecipeRegistry,
    ReplayRecordingConfig, ReplayState, Scheduler, SimulationState, SystemManifest,
    TellActionPayload, TickStepResult, TickStepServices,
};
use worldwake_systems::{build_full_action_registries, dispatch_table};

fn entity(slot: u32) -> EntityId {
    EntityId {
        slot,
        generation: 0,
    }
}

fn nz64(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).unwrap()
}

fn perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        memory_capacity: 8,
        memory_retention_ticks: 64,
        observation_fidelity: Permille::new(1000).unwrap(),
        confidence_policy: BeliefConfidencePolicy::default(),
    }
}

fn blind_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        memory_capacity: 8,
        memory_retention_ticks: 64,
        observation_fidelity: Permille::new(0).unwrap(),
        confidence_policy: BeliefConfidencePolicy::default(),
    }
}

fn accepting_tell_profile() -> TellProfile {
    TellProfile {
        max_tell_candidates: 3,
        max_relay_chain_len: 3,
        acceptance_fidelity: Permille::new(1000).unwrap(),
    }
}

fn integration_topology(travel_ticks: u32) -> Topology {
    let mut topology = Topology::new();
    for (slot, name) in [(1, "Origin"), (2, "Destination")] {
        topology
            .add_place(
                entity(slot),
                Place {
                    name: name.to_string(),
                    capacity: None,
                    tags: BTreeSet::new(),
                },
            )
            .unwrap();
    }
    topology
        .add_edge(
            TravelEdge::new(TravelEdgeId(10), entity(1), entity(2), travel_ticks, None).unwrap(),
        )
        .unwrap();
    topology
        .add_edge(
            TravelEdge::new(TravelEdgeId(11), entity(2), entity(1), travel_ticks, None).unwrap(),
        )
        .unwrap();
    topology
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

fn commit_txn(txn: WorldTxn<'_>, event_log: &mut EventLog) {
    let _ = txn.commit(event_log);
}

struct TellHarness {
    world: World,
    event_log: EventLog,
    scheduler: Scheduler,
    controller: worldwake_sim::ControllerState,
    rng: DeterministicRng,
    defs: worldwake_sim::ActionDefRegistry,
    handlers: worldwake_sim::ActionHandlerRegistry,
    recipes: RecipeRegistry,
    systems: worldwake_sim::SystemDispatchTable,
    origin: EntityId,
    destination: EntityId,
    speaker: EntityId,
    listener: EntityId,
    bystander: Option<EntityId>,
    subject: EntityId,
}

impl TellHarness {
    fn new(
        speaker_place: EntityId,
        listener_place: EntityId,
        bystander_place: Option<EntityId>,
        speaker_belief_place: EntityId,
    ) -> Self {
        let mut world = World::new(integration_topology(10)).unwrap();
        let mut event_log = EventLog::new();
        let origin = entity(1);
        let destination = entity(2);

        let (speaker, listener, bystander, subject) = {
            let mut txn = new_txn(&mut world, 0);
            let speaker = txn.create_agent("Speaker", ControlSource::Ai).unwrap();
            let listener = txn.create_agent("Listener", ControlSource::Ai).unwrap();
            let subject = txn.create_agent("Subject", ControlSource::Ai).unwrap();
            let bystander =
                bystander_place.map(|_| txn.create_agent("Bystander", ControlSource::Ai).unwrap());

            txn.set_ground_location(speaker, speaker_place).unwrap();
            txn.set_ground_location(listener, listener_place).unwrap();
            txn.set_ground_location(subject, origin).unwrap();
            if let Some(bystander) = bystander {
                txn.set_ground_location(bystander, bystander_place.unwrap())
                    .unwrap();
            }

            for agent in [Some(speaker), Some(listener), bystander]
                .into_iter()
                .flatten()
            {
                txn.set_component_agent_belief_store(agent, AgentBeliefStore::new())
                    .unwrap();
                txn.set_component_perception_profile(agent, perception_profile())
                    .unwrap();
                txn.set_component_tell_profile(agent, accepting_tell_profile())
                    .unwrap();
            }

            commit_txn(txn, &mut event_log);
            (speaker, listener, bystander, subject)
        };

        let mut belief = build_believed_entity_state(
            &world,
            subject,
            Tick(0),
            PerceptionSource::DirectObservation,
        )
        .unwrap();
        belief.last_known_place = Some(speaker_belief_place);

        {
            let mut txn = new_txn(&mut world, 1);
            let mut store = AgentBeliefStore::new();
            store.update_entity(subject, belief);
            txn.set_component_agent_belief_store(speaker, store)
                .unwrap();
            commit_txn(txn, &mut event_log);
        }

        let recipes = RecipeRegistry::new();
        let registries = build_full_action_registries(&recipes).unwrap();

        Self {
            world,
            event_log,
            scheduler: Scheduler::new(SystemManifest::canonical()),
            controller: worldwake_sim::ControllerState::with_entity(speaker),
            rng: DeterministicRng::new(Seed([23; 32])),
            defs: registries.defs,
            handlers: registries.handlers,
            recipes,
            systems: dispatch_table(),
            origin,
            destination,
            speaker,
            listener,
            bystander,
            subject,
        }
    }

    fn action_def_id(&self, name: &str) -> ActionDefId {
        self.defs
            .iter()
            .find(|def| def.name == name)
            .map_or_else(|| panic!("missing action def {name}"), |def| def.id)
    }

    fn queue_travel(&mut self, destination: EntityId) {
        let tick = self.scheduler.current_tick();
        let def_id = self.action_def_id("travel");
        self.scheduler.input_queue_mut().enqueue(
            tick,
            InputKind::RequestAction {
                actor: self.speaker,
                def_id,
                targets: vec![destination],
                payload_override: None,
                mode: ActionRequestMode::Strict,
            },
        );
    }

    fn queue_tell(&mut self) {
        let tick = self.scheduler.current_tick();
        let def_id = self.action_def_id("tell");
        self.scheduler.input_queue_mut().enqueue(
            tick,
            InputKind::RequestAction {
                actor: self.speaker,
                def_id,
                targets: vec![self.listener],
                payload_override: Some(ActionPayload::Tell(TellActionPayload {
                    listener: self.listener,
                    subject_entity: self.subject,
                })),
                mode: ActionRequestMode::Strict,
            },
        );
    }

    fn step_once(&mut self) -> TickStepResult {
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
                systems: &self.systems,
                input_producer: None,
                action_trace: None,
                politics_trace: None,
            },
        )
        .unwrap()
    }

    fn listener_belief(&self) -> Option<&BelievedEntityState> {
        self.world
            .get_component_agent_belief_store(self.listener)
            .and_then(|store| store.get_entity(&self.subject))
    }

    fn bystander_store(&self) -> Option<&AgentBeliefStore> {
        self.bystander
            .and_then(|bystander| self.world.get_component_agent_belief_store(bystander))
    }
}

#[allow(clippy::too_many_lines)]
fn build_recorded_replay_state() -> (SimulationState, StateHash) {
    let recipes = RecipeRegistry::new();
    let registries = build_full_action_registries(&recipes).unwrap();
    let systems = dispatch_table();
    let origin = entity(1);
    let destination = entity(2);

    let mut world = World::new(integration_topology(10)).unwrap();
    let mut event_log = EventLog::new();

    let (speaker, listener, subject) = {
        let mut txn = new_txn(&mut world, 0);
        let speaker = txn.create_agent("Speaker", ControlSource::Ai).unwrap();
        let listener = txn.create_agent("Listener", ControlSource::Ai).unwrap();
        let subject = txn.create_agent("Subject", ControlSource::Ai).unwrap();
        txn.set_ground_location(speaker, origin).unwrap();
        txn.set_ground_location(listener, destination).unwrap();
        txn.set_ground_location(subject, origin).unwrap();

        txn.set_component_agent_belief_store(speaker, AgentBeliefStore::new())
            .unwrap();
        txn.set_component_perception_profile(speaker, blind_perception_profile())
            .unwrap();
        txn.set_component_tell_profile(speaker, accepting_tell_profile())
            .unwrap();

        txn.set_component_agent_belief_store(listener, AgentBeliefStore::new())
            .unwrap();
        txn.set_component_perception_profile(listener, perception_profile())
            .unwrap();
        txn.set_component_tell_profile(listener, accepting_tell_profile())
            .unwrap();

        commit_txn(txn, &mut event_log);
        (speaker, listener, subject)
    };

    let mut stale_belief = build_believed_entity_state(
        &world,
        subject,
        Tick(0),
        PerceptionSource::DirectObservation,
    )
    .unwrap();
    stale_belief.last_known_place = Some(destination);
    let listener_known = build_believed_entity_state(
        &world,
        listener,
        Tick(0),
        PerceptionSource::DirectObservation,
    )
    .unwrap();

    {
        let mut txn = new_txn(&mut world, 1);
        let mut store = AgentBeliefStore::new();
        store.update_entity(subject, stale_belief);
        store.update_entity(listener, listener_known);
        txn.set_component_agent_belief_store(speaker, store)
            .unwrap();
        commit_txn(txn, &mut event_log);
    }

    let scheduler = Scheduler::new(SystemManifest::canonical());
    let controller = worldwake_sim::ControllerState::with_entity(speaker);
    let rng = DeterministicRng::new(Seed([31; 32]));
    let mut state = SimulationState::new(
        world,
        event_log,
        scheduler,
        recipes,
        ReplayState::new(
            StateHash([0; 32]),
            Seed([31; 32]),
            Tick(0),
            ReplayRecordingConfig::disabled(),
        ),
        controller,
        rng,
    );
    let initial_hash = state.replay_bootstrap_hash().unwrap();
    *state.replay_state_mut() = ReplayState::new(
        initial_hash,
        state.rng_state().seed(),
        state.scheduler().current_tick(),
        ReplayRecordingConfig::every(nz64(1)),
    );
    let mut initial_state = state.clone();

    let travel_def = registries
        .defs
        .iter()
        .find(|def| def.name == "travel")
        .map(|def| def.id)
        .unwrap();
    let tell_def = registries
        .defs
        .iter()
        .find(|def| def.name == "tell")
        .map(|def| def.id)
        .unwrap();

    let current_tick = state.scheduler().current_tick();
    let travel_input = {
        state.scheduler_mut().input_queue_mut().enqueue(
            current_tick,
            InputKind::RequestAction {
                actor: speaker,
                def_id: travel_def,
                targets: vec![destination],
                payload_override: None,
                mode: ActionRequestMode::Strict,
            },
        )
    }
    .clone();
    state.replay_state_mut().record_input(travel_input).unwrap();

    for _ in 0..16 {
        let result = {
            let (world, event_log, scheduler, controller, rng, recipe_registry) =
                state.tick_parts_mut();
            step_tick(
                world,
                event_log,
                scheduler,
                controller,
                rng,
                TickStepServices {
                    action_defs: &registries.defs,
                    action_handlers: &registries.handlers,
                    recipe_registry,
                    systems: &systems,
                    input_producer: None,
                    action_trace: None,
                    politics_trace: None,
                },
            )
        }
        .unwrap();
        let _ = record_tick_checkpoint(&mut state, result.tick).unwrap();
        let terminal_tick = state.scheduler().current_tick();
        state
            .replay_state_mut()
            .set_terminal_tick(terminal_tick)
            .unwrap();
        if state.world().effective_place(speaker) == Some(destination) {
            break;
        }
    }
    assert_eq!(state.world().effective_place(speaker), Some(destination));

    let affordances = get_affordances(
        &PerAgentBeliefView::from_world(speaker, state.world()),
        speaker,
        &registries.defs,
        &registries.handlers,
    );
    assert!(
        affordances.iter().any(|affordance| {
            affordance.def_id == tell_def
                && affordance.bound_targets == vec![listener]
                && affordance.payload_override
                    == Some(ActionPayload::Tell(TellActionPayload {
                        listener,
                        subject_entity: subject,
                    }))
        }),
        "tell affordance missing before replay input; speaker_place={:?}, listener_place={:?}, known_subjects={:?}",
        state.world().effective_place(speaker),
        state.world().effective_place(listener),
        state.world()
            .get_component_agent_belief_store(speaker)
            .map(|store| store.known_entities.keys().copied().collect::<Vec<_>>())
    );

    let current_tick = state.scheduler().current_tick();
    let tell_input = {
        state.scheduler_mut().input_queue_mut().enqueue(
            current_tick,
            InputKind::RequestAction {
                actor: speaker,
                def_id: tell_def,
                targets: vec![listener],
                payload_override: Some(ActionPayload::Tell(TellActionPayload {
                    listener,
                    subject_entity: subject,
                })),
                mode: ActionRequestMode::Strict,
            },
        )
    }
    .clone();
    state.replay_state_mut().record_input(tell_input).unwrap();

    for _ in 0..8 {
        let result = {
            let (world, event_log, scheduler, controller, rng, recipe_registry) =
                state.tick_parts_mut();
            step_tick(
                world,
                event_log,
                scheduler,
                controller,
                rng,
                TickStepServices {
                    action_defs: &registries.defs,
                    action_handlers: &registries.handlers,
                    recipe_registry,
                    systems: &systems,
                    input_producer: None,
                    action_trace: None,
                    politics_trace: None,
                },
            )
        }
        .unwrap();
        let _ = record_tick_checkpoint(&mut state, result.tick).unwrap();
        let terminal_tick = state.scheduler().current_tick();
        state
            .replay_state_mut()
            .set_terminal_tick(terminal_tick)
            .unwrap();
        if state
            .event_log()
            .events_by_tag(EventTag::Discovery)
            .is_empty()
        {
            continue;
        }
        break;
    }

    assert!(!state.event_log().events_by_tag(EventTag::Social).is_empty());
    assert!(!state
        .event_log()
        .events_by_tag(EventTag::Discovery)
        .is_empty());

    let final_hash = state.replay_bootstrap_hash().unwrap();
    *initial_state.replay_state_mut() = state.replay_state().clone();

    (initial_state, final_hash)
}

#[test]
fn tell_propagation_requires_travel_and_tell_completion() {
    let mut harness = TellHarness::new(entity(1), entity(2), None, entity(1));

    assert!(harness.listener_belief().is_none());

    harness.queue_travel(harness.destination);
    for _ in 0..9 {
        let _ = harness.step_once();
        assert!(
            harness.listener_belief().is_none(),
            "listener should remain ignorant while speaker is still remote"
        );
    }

    for _ in 0..3 {
        if harness.world.effective_place(harness.speaker) == Some(harness.destination) {
            break;
        }
        let _ = harness.step_once();
        assert!(harness.listener_belief().is_none());
    }

    assert_eq!(
        harness.world.effective_place(harness.speaker),
        Some(harness.destination)
    );
    assert!(harness.listener_belief().is_none());

    harness.queue_tell();
    let _ = harness.step_once();
    assert!(
        harness.listener_belief().is_none(),
        "tell should not update the listener on the first tick"
    );

    let mut listener_updated = false;
    for _ in 0..3 {
        let _ = harness.step_once();
        if harness.listener_belief().is_some() {
            listener_updated = true;
            break;
        }
    }
    assert!(
        listener_updated,
        "listener should receive the belief after tell completes"
    );

    let transferred = harness.listener_belief().unwrap();
    assert_eq!(transferred.last_known_place, Some(harness.origin));
    assert_eq!(transferred.observed_tick, Tick(0));
    assert_eq!(
        transferred.source,
        PerceptionSource::Report {
            from: harness.speaker,
            chain_len: 1,
        }
    );
}

#[test]
fn hidden_event_at_empty_location_remains_isolated_from_remote_agents() {
    let mut world = World::new(integration_topology(10)).unwrap();
    let mut event_log = EventLog::new();
    let origin = entity(1);
    let destination = entity(2);
    let remote = {
        let mut txn = new_txn(&mut world, 0);
        let remote = txn.create_agent("Remote", ControlSource::Ai).unwrap();
        txn.set_ground_location(remote, destination).unwrap();
        txn.set_component_agent_belief_store(remote, AgentBeliefStore::new())
            .unwrap();
        txn.set_component_perception_profile(remote, perception_profile())
            .unwrap();
        commit_txn(txn, &mut event_log);
        remote
    };

    let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
        tick: Tick(0),
        cause: CauseRef::Bootstrap,
        actor_id: None,
        target_ids: Vec::new(),
        evidence: Vec::new(),
        place_id: Some(origin),
        state_deltas: Vec::new(),
        observed_entities: BTreeMap::new(),
        visibility: VisibilitySpec::SamePlace,
        witness_data: WitnessData::default(),
        tags: BTreeSet::from([EventTag::WorldMutation]),
    }));

    let recipes = RecipeRegistry::new();
    let registries = build_full_action_registries(&recipes).unwrap();
    let mut scheduler = Scheduler::new(SystemManifest::canonical());
    let mut controller = worldwake_sim::ControllerState::with_entity(remote);
    let mut rng = DeterministicRng::new(Seed([37; 32]));

    let _ = step_tick(
        &mut world,
        &mut event_log,
        &mut scheduler,
        &mut controller,
        &mut rng,
        TickStepServices {
            action_defs: &registries.defs,
            action_handlers: &registries.handlers,
            recipe_registry: &recipes,
            systems: &dispatch_table(),
            input_producer: None,
            action_trace: None,
            politics_trace: None,
        },
    )
    .unwrap();

    let store = world.get_component_agent_belief_store(remote).unwrap();
    assert!(store.known_entities.is_empty());
    assert!(store.social_observations.is_empty());
}

#[test]
fn bystander_observes_witnessed_telling_without_receiving_subject_belief() {
    let mut harness = TellHarness::new(entity(2), entity(2), Some(entity(2)), entity(1));

    let _ = harness.step_once();
    harness.queue_tell();
    let _ = harness.step_once();
    for _ in 0..3 {
        if harness.listener_belief().is_some() {
            break;
        }
        let _ = harness.step_once();
    }

    let listener_belief = harness.listener_belief().unwrap();
    assert_eq!(
        listener_belief.source,
        PerceptionSource::Report {
            from: harness.speaker,
            chain_len: 1,
        }
    );

    let bystander_store = harness.bystander_store().unwrap();
    assert!(bystander_store.get_entity(&harness.subject).is_none());
    assert!(bystander_store
        .social_observations
        .iter()
        .any(|observation| {
            observation.kind == SocialObservationKind::WitnessedTelling
                && observation.subjects == (harness.speaker, harness.listener)
                && observation.place == harness.destination
        }));
}

#[test]
fn replay_verification_accepts_recorded_tell_and_discovery_scenario() {
    let recipes = RecipeRegistry::new();
    let registries = build_full_action_registries(&recipes).unwrap();
    let systems = dispatch_table();
    let (initial_state, expected_final_hash) = build_recorded_replay_state();

    let final_hash = replay_and_verify(
        &initial_state,
        TickStepServices {
            action_defs: &registries.defs,
            action_handlers: &registries.handlers,
            recipe_registry: &recipes,
            systems: &systems,
            input_producer: None,
            action_trace: None,
            politics_trace: None,
        },
    )
    .unwrap();

    assert_eq!(final_hash, expected_final_hash);
}
