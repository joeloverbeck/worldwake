use std::collections::BTreeSet;
use worldwake_core::{
    is_incapacitated, ActionDefId, AskWitnessMemory, AskWitnessMemoryKey, BelievedEntityState,
    BodyCostPerTick, EntityId, EntityKind, EventTag, PerceptionSource, VisibilitySpec, World,
    WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionAbortRequestReason, ActionDef, ActionDefRegistry, ActionError,
    ActionHandler, ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload,
    ActionProgress, ActionState, AskWitnessPayload, CommitOutcome, Constraint, DeterministicRng,
    DurationExpr, Interruptibility, Precondition, RuntimeBeliefView, TargetSpec,
};

pub fn register_ask_witness_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(
            start_ask_witness,
            tick_ask_witness,
            commit_ask_witness,
            abort_ask_witness,
        )
        .with_affordance_payloads(enumerate_ask_witness_payloads)
        .with_payload_override_validator(validate_ask_witness_payload_override)
        .with_authoritative_payload_validator(validate_ask_witness_payload_authoritatively),
    );
    let id = ActionDefId(defs.len() as u32);
    defs.register(ask_witness_action_def(id, handler))
}

fn ask_witness_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    let preconditions = vec![
        Precondition::ActorAlive,
        Precondition::TargetExists(0),
        Precondition::TargetAtActorPlace(0),
        Precondition::TargetKind {
            target_index: 0,
            kind: EntityKind::Agent,
        },
        Precondition::TargetAlive(0),
    ];

    ActionDef {
        id,
        name: "ask_witness".to_string(),
        domain: worldwake_core::ActionDomain::Epistemic,
        actor_constraints: vec![Constraint::ActorAlive, Constraint::ActorNotIncapacitated],
        targets: vec![TargetSpec::EntityAtActorPlace {
            kind: EntityKind::Agent,
        }],
        preconditions: preconditions.clone(),
        reservation_requirements: Vec::new(),
        duration: DurationExpr::ActorWitnessQueryDisposition,
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: preconditions,
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::Social, EventTag::Discovery]),
        payload: ActionPayload::None,
        handler,
    }
}

fn ask_witness_payload<'a>(
    def: &ActionDef,
    payload: &'a ActionPayload,
) -> Result<&'a AskWitnessPayload, ActionError> {
    payload.as_ask_witness().ok_or_else(|| {
        ActionError::PreconditionFailed(format!(
            "action def {} requires AskWitness payload",
            def.id
        ))
    })
}

fn ask_witness_memory_key(payload: &AskWitnessPayload) -> AskWitnessMemoryKey {
    AskWitnessMemoryKey {
        counterparty: payload.target,
        topic_entity: payload.topic_entity,
        topic_commodity: payload.topic_commodity,
    }
}

fn is_authoritatively_incapacitated(world: &World, entity: EntityId) -> bool {
    world
        .get_component_wound_list(entity)
        .zip(world.get_component_combat_profile(entity))
        .is_some_and(|(wounds, profile)| is_incapacitated(wounds, profile))
}

fn matches_ask_topic(
    entity: EntityId,
    state: &BelievedEntityState,
    payload: &AskWitnessPayload,
) -> bool {
    let entity_match = payload
        .topic_entity
        .is_none_or(|topic_entity| entity == topic_entity);
    let commodity_match = payload.topic_commodity.is_none_or(|commodity| {
        state
            .resource_source
            .as_ref()
            .is_some_and(|resource| resource.commodity == commodity)
    });
    entity_match && commodity_match
}

fn enumerate_ask_witness_payloads(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    view: &dyn RuntimeBeliefView,
) -> Vec<ActionPayload> {
    let Some(target) = targets.first().copied() else {
        return Vec::new();
    };
    if target == actor || view.epistemic_disposition_profile(actor).is_none() {
        return Vec::new();
    }

    let mut payloads = BTreeSet::new();
    for (entity, state) in view.known_entity_beliefs(actor) {
        let payload = AskWitnessPayload {
            target,
            topic_entity: Some(entity),
            topic_commodity: state
                .resource_source
                .as_ref()
                .map(|resource| resource.commodity),
        };
        if view
            .ask_witness_memory(actor, &ask_witness_memory_key(&payload))
            .is_none()
        {
            payloads.insert(ActionPayload::AskWitness(payload));
        }
    }

    payloads.into_iter().collect()
}

fn validate_ask_witness_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(target) = targets.first().copied() else {
        return false;
    };
    if view.epistemic_disposition_profile(actor).is_none() {
        return false;
    }
    let Some(payload) = payload.as_ask_witness() else {
        return false;
    };
    if payload.target != target || payload.target == actor {
        return false;
    }
    if payload.topic_entity.is_none() && payload.topic_commodity.is_none() {
        return false;
    }

    view.ask_witness_memory(actor, &ask_witness_memory_key(payload))
        .is_none()
}

fn validate_ask_witness_payload_authoritatively(
    def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let Some(target) = targets.first().copied() else {
        return Err(ActionError::InvalidTarget(actor));
    };
    if world
        .get_component_epistemic_disposition_profile(actor)
        .is_none()
    {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} lacks EpistemicDispositionProfile"
        )));
    }

    let payload = ask_witness_payload(def, payload)?;
    if payload.target != target {
        return Err(ActionError::PreconditionFailed(format!(
            "ask_witness payload target {} does not match bound target {}",
            payload.target, target
        )));
    }
    if payload.target == actor {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} cannot ask_witness self"
        )));
    }
    if payload.topic_entity.is_none() && payload.topic_commodity.is_none() {
        return Err(ActionError::PreconditionFailed(
            "ask_witness requires topic_entity and/or topic_commodity".to_string(),
        ));
    }
    if world.get_component_dead_at(target).is_some() {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} is not alive"
        )));
    }
    if world.entity_kind(target) != Some(EntityKind::Agent) {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} is not an agent"
        )));
    }
    if world.effective_place(actor) != world.effective_place(target) {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} is not colocated with actor {actor}"
        )));
    }
    if is_authoritatively_incapacitated(world, target) {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} is incapacitated"
        )));
    }
    Ok(())
}

fn validate_ask_witness_context(
    txn: &WorldTxn<'_>,
    instance: &ActionInstance,
    payload: &AskWitnessPayload,
) -> Result<EntityId, ActionError> {
    let target = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    if payload.target != target {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::PayloadEntityMismatch {
                role: worldwake_sim::PayloadEntityRole::Counterparty,
                expected: target,
                actual: payload.target,
            },
        ));
    }
    let actor_place = txn.effective_place(instance.actor).ok_or_else(|| {
        ActionError::AbortRequested(ActionAbortRequestReason::ActorNotPlaced {
            actor: instance.actor,
        })
    })?;
    if txn.effective_place(target) != Some(actor_place) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetNotColocated {
                actor: instance.actor,
                target,
            },
        ));
    }
    if txn.get_component_dead_at(target).is_some() {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetNotAlive { target },
        ));
    }
    if txn.entity_kind(target) != Some(EntityKind::Agent) {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} is not an agent"
        )));
    }
    if is_authoritatively_incapacitated(txn, target) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetIncapacitated { target },
        ));
    }
    Ok(target)
}

fn start_ask_witness(
    def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let payload = ask_witness_payload(def, &instance.payload)?;
    validate_ask_witness_context(txn, instance, payload)?;
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_ask_witness(
    def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    let payload = ask_witness_payload(def, &instance.payload)?;
    validate_ask_witness_context(txn, instance, payload)?;
    Ok(ActionProgress::Continue)
}

fn commit_ask_witness(
    def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = ask_witness_payload(def, &instance.payload)?;
    let target = validate_ask_witness_context(txn, instance, payload)?;
    let target_store = txn
        .get_component_agent_belief_store(target)
        .cloned()
        .unwrap_or_default();
    let actor_profile = txn
        .get_component_epistemic_disposition_profile(instance.actor)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "actor {} lacks EpistemicDispositionProfile",
                instance.actor
            ))
        })?;
    let actor_perception = txn
        .get_component_perception_profile(instance.actor)
        .copied()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "actor {} lacks PerceptionProfile",
                instance.actor
            ))
        })?;

    let mut actor_store = txn
        .get_component_agent_belief_store(instance.actor)
        .cloned()
        .unwrap_or_default();
    for (entity, state) in target_store
        .known_entities
        .iter()
        .filter(|(entity, state)| matches_ask_topic(**entity, state, payload))
    {
        let mut transferred = state.clone();
        transferred.source = PerceptionSource::Report {
            from: target,
            chain_len: 1,
        };
        actor_store.update_entity(*entity, transferred);
    }
    actor_store.enforce_capacity(&actor_perception, txn.tick());
    actor_store.record_asked_witness(
        ask_witness_memory_key(payload),
        AskWitnessMemory {
            asked_tick: txn.tick(),
        },
    );
    actor_store.enforce_ask_witness_memory(txn.tick(), actor_profile.ask_memory_retention_ticks);
    txn.set_component_agent_belief_store(instance.actor, actor_store)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;

    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_ask_witness(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _reason: &AbortReason,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_believed_entity_state, build_prototype_world, BodyPart, CauseRef, CombatProfile,
        CombatWeaponRef, ControlSource, DeadAt, EpistemicDispositionProfile, EventLog, Permille,
        Seed, Tick, WitnessData, Wound, WoundCause, WoundId, WoundList,
    };
    use worldwake_sim::{
        get_affordances, start_action, tick_action, ActionExecutionAuthority,
        ActionInstanceId, Affordance, PerAgentBeliefView, TickOutcome,
    };

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn new_world() -> World {
        World::new(build_prototype_world()).unwrap()
    }

    fn first_two_places(world: &World) -> (EntityId, EntityId) {
        let places = world.topology().place_ids().collect::<Vec<_>>();
        (places[0], places[1])
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

    fn spawn_actor(world: &mut World, place: EntityId, name: &str) -> EntityId {
        let mut txn = new_txn(world, 1);
        let actor = txn.create_agent(name, ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, place).unwrap();
        commit_txn(txn);
        actor
    }

    fn set_epistemic_profile(world: &mut World, actor: EntityId, duration: u32) {
        let mut txn = new_txn(world, 1);
        txn.set_component_epistemic_disposition_profile(
            actor,
            EpistemicDispositionProfile {
                stale_evidence_barrier_threshold: pm(400),
                witness_query_duration_ticks: nz(duration),
                ask_memory_retention_ticks: 12,
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    fn seed_entity_belief(
        world: &mut World,
        actor: EntityId,
        subject: EntityId,
        observed_tick: u64,
    ) {
        seed_entity_belief_with_source(
            world,
            actor,
            subject,
            observed_tick,
            PerceptionSource::Rumor { chain_len: 1 },
        );
    }

    fn seed_entity_belief_with_source(
        world: &mut World,
        actor: EntityId,
        subject: EntityId,
        observed_tick: u64,
        source: PerceptionSource,
    ) {
        let state =
            build_believed_entity_state(world, subject, Tick(observed_tick), source).unwrap();
        let mut txn = new_txn(world, observed_tick);
        let mut store = txn
            .get_component_agent_belief_store(actor)
            .cloned()
            .unwrap_or_default();
        store.update_entity(subject, state);
        txn.set_component_agent_belief_store(actor, store).unwrap();
        commit_txn(txn);
    }

    fn set_ground_location(world: &mut World, entity: EntityId, place: EntityId, tick: u64) {
        let mut txn = new_txn(world, tick);
        txn.set_ground_location(entity, place).unwrap();
        commit_txn(txn);
    }

    fn setup_registries_with_ask() -> (ActionDefRegistry, ActionHandlerRegistry, ActionDefId) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let ask_id = register_ask_witness_action(&mut defs, &mut handlers);
        (defs, handlers, ask_id)
    }

    fn manual_ask_affordance(
        actor: EntityId,
        def_id: ActionDefId,
        target: EntityId,
        payload: AskWitnessPayload,
    ) -> Affordance {
        Affordance {
            def_id,
            actor,
            bound_targets: vec![target],
            payload_override: Some(ActionPayload::AskWitness(payload)),
            explanation: None,
        }
    }

    fn run_action_to_completion(
        world: &mut World,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        affordance: &Affordance,
        seed: [u8; 32],
        start_tick: u64,
        commit_tick: u64,
    ) -> TickOutcome {
        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed(seed));
        let mut next_instance_id = ActionInstanceId(1);
        let instance_id = start_action(
            affordance,
            defs,
            handlers,
            ActionExecutionAuthority {
                world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            &mut next_instance_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(start_tick)),
        )
        .unwrap();

        for tick in (start_tick + 1)..=commit_tick {
            let outcome = tick_action(
                instance_id,
                defs,
                handlers,
                ActionExecutionAuthority {
                    world,
                    event_log: &mut event_log,
                    active_actions: &mut active_actions,
                    rng: &mut rng,
                },
                worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(tick)),
            )
            .unwrap();
            if tick == commit_tick {
                return outcome;
            }
        }

        panic!("action did not reach requested commit tick");
    }

    fn kill_entity(world: &mut World, entity: EntityId, tick: u64) {
        let mut txn = new_txn(world, tick);
        txn.set_component_dead_at(entity, DeadAt(Tick(tick)))
            .unwrap();
        commit_txn(txn);
    }

    fn incapacitate_entity(world: &mut World, entity: EntityId, tick: u64) {
        let mut txn = new_txn(world, tick);
        if txn.get_component_combat_profile(entity).is_none() {
            txn.set_component_combat_profile(
                entity,
                CombatProfile::new(
                    pm(1000),
                    pm(700),
                    pm(600),
                    pm(550),
                    pm(75),
                    pm(20),
                    pm(15),
                    pm(120),
                    pm(30),
                    nz(6),
                    nz(10),
                ),
            )
            .unwrap();
        }
        txn.set_component_wound_list(
            entity,
            WoundList {
                wounds: vec![Wound {
                    id: WoundId(1),
                    body_part: BodyPart::Torso,
                    cause: WoundCause::Combat {
                        attacker: entity,
                        weapon: CombatWeaponRef::Unarmed,
                    },
                    severity: pm(1000),
                    inflicted_at: Tick(tick),
                    bleed_rate_per_tick: pm(0),
                }],
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    #[test]
    fn register_ask_witness_action_creates_expected_definition() {
        let (defs, handlers, def_id) = setup_registries_with_ask();
        let def = defs.get(def_id).unwrap();

        assert!(handlers.get(def.handler).is_some());
        assert_eq!(def.name, "ask_witness");
        assert_eq!(def.domain, worldwake_core::ActionDomain::Epistemic);
        assert_eq!(
            def.targets,
            vec![TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }]
        );
        assert_eq!(def.duration, DurationExpr::ActorWitnessQueryDisposition);
        assert_eq!(def.interruptibility, Interruptibility::FreelyInterruptible);
        assert_eq!(def.visibility, VisibilitySpec::SamePlace);
        assert_eq!(
            def.causal_event_tags,
            BTreeSet::from([EventTag::Social, EventTag::Discovery])
        );
    }

    #[test]
    fn ask_witness_transfers_belief_with_report_provenance_and_records_memory() {
        let mut world = new_world();
        let (place, other_place) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place, "Aster");
        let witness = spawn_actor(&mut world, place, "Bram");
        let subject = spawn_actor(&mut world, other_place, "Cyra");
        set_epistemic_profile(&mut world, actor, 2);
        seed_entity_belief(&mut world, actor, subject, 1);
        seed_entity_belief_with_source(
            &mut world,
            witness,
            subject,
            2,
            PerceptionSource::DirectObservation,
        );

        let payload = AskWitnessPayload {
            target: witness,
            topic_entity: Some(subject),
            topic_commodity: None,
        };
        let (defs, handlers, ask_id) = setup_registries_with_ask();
        let affordance = manual_ask_affordance(actor, ask_id, witness, payload.clone());

        match run_action_to_completion(&mut world, &defs, &handlers, &affordance, [9; 32], 2, 4) {
            TickOutcome::Committed { .. } => {}
            other => panic!("expected ask_witness commit, got {other:?}"),
        }

        let store = world.get_component_agent_belief_store(actor).unwrap();
        let belief = store.get_entity(&subject).unwrap();
        assert_eq!(belief.last_known_place, Some(other_place));
        assert_eq!(belief.observed_tick, Tick(2));
        assert_eq!(
            belief.source,
            PerceptionSource::Report {
                from: witness,
                chain_len: 1,
            }
        );
        assert_eq!(
            store
                .ask_witness_memory(&ask_witness_memory_key(&payload), Tick(4), 12)
                .map(|memory| memory.asked_tick),
            Some(Tick(4))
        );
    }

    #[test]
    fn ask_witness_noop_still_records_ask_memory() {
        let mut world = new_world();
        let (place, other_place) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place, "Aster");
        let witness = spawn_actor(&mut world, place, "Bram");
        let subject = spawn_actor(&mut world, other_place, "Cyra");
        set_epistemic_profile(&mut world, actor, 2);
        seed_entity_belief(&mut world, actor, subject, 1);

        let payload = AskWitnessPayload {
            target: witness,
            topic_entity: Some(subject),
            topic_commodity: None,
        };
        let (defs, handlers, ask_id) = setup_registries_with_ask();
        let affordance = manual_ask_affordance(actor, ask_id, witness, payload.clone());

        match run_action_to_completion(&mut world, &defs, &handlers, &affordance, [10; 32], 2, 4) {
            TickOutcome::Committed { .. } => {}
            other => panic!("expected ask_witness commit, got {other:?}"),
        }

        let store = world.get_component_agent_belief_store(actor).unwrap();
        let belief = store.get_entity(&subject).unwrap();
        assert_eq!(belief.observed_tick, Tick(1));
        assert_eq!(belief.source, PerceptionSource::Rumor { chain_len: 1 });
        assert_eq!(
            store
                .ask_witness_memory(&ask_witness_memory_key(&payload), Tick(4), 12)
                .map(|memory| memory.asked_tick),
            Some(Tick(4))
        );
    }

    #[test]
    fn ask_witness_affordance_suppresses_recent_reask() {
        let mut world = new_world();
        let (place, other_place) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place, "Aster");
        let witness = spawn_actor(&mut world, place, "Bram");
        let subject = spawn_actor(&mut world, other_place, "Cyra");
        set_epistemic_profile(&mut world, actor, 2);
        seed_entity_belief(&mut world, actor, subject, 1);

        let payload = AskWitnessPayload {
            target: witness,
            topic_entity: Some(subject),
            topic_commodity: None,
        };
        {
            let mut txn = new_txn(&mut world, 4);
            let mut store = txn
                .get_component_agent_belief_store(actor)
                .cloned()
                .unwrap_or_default();
            store.record_asked_witness(
                ask_witness_memory_key(&payload),
                AskWitnessMemory {
                    asked_tick: Tick(3),
                },
            );
            txn.set_component_agent_belief_store(actor, store).unwrap();
            commit_txn(txn);
        }

        let (defs, handlers, _) = setup_registries_with_ask();
        let view = PerAgentBeliefView::from_world(actor, &world);
        let affordances = get_affordances(&view, actor, &defs, &handlers);
        assert!(
            !affordances.iter().any(|affordance| {
                affordance
                    .payload_override
                    .as_ref()
                    .and_then(ActionPayload::as_ask_witness)
                    == Some(&payload)
            }),
            "recent ask should suppress same witness/topic affordance"
        );
    }

    #[test]
    fn ask_witness_rejects_payload_without_topic() {
        let mut world = new_world();
        let (place, _) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place, "Aster");
        let witness = spawn_actor(&mut world, place, "Bram");
        set_epistemic_profile(&mut world, actor, 2);

        let (defs, handlers, ask_id) = setup_registries_with_ask();
        let def = defs.get(ask_id).unwrap();
        let handler = handlers.get(def.handler).unwrap();

        let error = (handler.authoritative_payload_is_valid)(
            def,
            &defs,
            actor,
            &[witness],
            &ActionPayload::AskWitness(AskWitnessPayload {
                target: witness,
                topic_entity: None,
                topic_commodity: None,
            }),
            &world,
        )
        .unwrap_err();

        assert!(matches!(error, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn ask_witness_aborts_when_target_moves_before_commit() {
        let mut world = new_world();
        let (place, other_place) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place, "Aster");
        let witness = spawn_actor(&mut world, place, "Bram");
        let subject = spawn_actor(&mut world, other_place, "Cyra");
        set_epistemic_profile(&mut world, actor, 2);
        seed_entity_belief(&mut world, actor, subject, 1);

        let payload = AskWitnessPayload {
            target: witness,
            topic_entity: Some(subject),
            topic_commodity: None,
        };
        let (defs, handlers, ask_id) = setup_registries_with_ask();
        let affordance = manual_ask_affordance(actor, ask_id, witness, payload.clone());

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([11; 32]));
        let mut next_instance_id = ActionInstanceId(1);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            &mut next_instance_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(2)),
        )
        .unwrap();
        assert_eq!(
            tick_action(
                instance_id,
                &defs,
                &handlers,
                ActionExecutionAuthority {
                    world: &mut world,
                    event_log: &mut event_log,
                    active_actions: &mut active_actions,
                    rng: &mut rng,
                },
                worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(3)),
            )
            .unwrap(),
            TickOutcome::Continuing
        );
        set_ground_location(&mut world, witness, other_place, 3);

        match tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(4)),
        )
        .unwrap()
        {
            TickOutcome::Aborted { reason, .. } => assert!(matches!(
                reason,
                worldwake_sim::AbortReason::ExternalAbort {
                    kind: worldwake_sim::ExternalAbortReason::HandlerRequested {
                        reason: ActionAbortRequestReason::TargetNotColocated { .. }
                    },
                    ..
                }
            )),
            other => panic!("expected ask_witness abort, got {other:?}"),
        }

        let store = world.get_component_agent_belief_store(actor).unwrap();
        assert_eq!(store.get_entity(&subject).unwrap().observed_tick, Tick(1));
        assert_eq!(
            store.ask_witness_memory(&ask_witness_memory_key(&payload), Tick(4), 12),
            None
        );
    }

    #[test]
    fn ask_witness_aborts_when_target_dies_before_commit() {
        let mut world = new_world();
        let (place, other_place) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place, "Aster");
        let witness = spawn_actor(&mut world, place, "Bram");
        let subject = spawn_actor(&mut world, other_place, "Cyra");
        set_epistemic_profile(&mut world, actor, 2);
        seed_entity_belief(&mut world, actor, subject, 1);

        let payload = AskWitnessPayload {
            target: witness,
            topic_entity: Some(subject),
            topic_commodity: None,
        };
        let (defs, handlers, ask_id) = setup_registries_with_ask();
        let affordance = manual_ask_affordance(actor, ask_id, witness, payload.clone());

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([12; 32]));
        let mut next_instance_id = ActionInstanceId(1);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            &mut next_instance_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(2)),
        )
        .unwrap();
        let _ = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(3)),
        )
        .unwrap();
        kill_entity(&mut world, witness, 3);

        match tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(4)),
        )
        .unwrap()
        {
            TickOutcome::Aborted { .. } => {}
            other => panic!("expected ask_witness abort, got {other:?}"),
        }

        let store = world.get_component_agent_belief_store(actor).unwrap();
        assert_eq!(
            store.ask_witness_memory(&ask_witness_memory_key(&payload), Tick(4), 12),
            None
        );
    }

    #[test]
    fn ask_witness_aborts_when_target_becomes_incapacitated_before_commit() {
        let mut world = new_world();
        let (place, other_place) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place, "Aster");
        let witness = spawn_actor(&mut world, place, "Bram");
        let subject = spawn_actor(&mut world, other_place, "Cyra");
        set_epistemic_profile(&mut world, actor, 2);
        seed_entity_belief(&mut world, actor, subject, 1);

        let payload = AskWitnessPayload {
            target: witness,
            topic_entity: Some(subject),
            topic_commodity: None,
        };
        let (defs, handlers, ask_id) = setup_registries_with_ask();
        let affordance = manual_ask_affordance(actor, ask_id, witness, payload.clone());

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([13; 32]));
        let mut next_instance_id = ActionInstanceId(1);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            &mut next_instance_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(2)),
        )
        .unwrap();
        let _ = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(3)),
        )
        .unwrap();
        incapacitate_entity(&mut world, witness, 3);

        match tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(4)),
        )
        .unwrap()
        {
            TickOutcome::Aborted { reason, .. } => assert!(matches!(
                reason,
                worldwake_sim::AbortReason::ExternalAbort {
                    kind: worldwake_sim::ExternalAbortReason::HandlerRequested {
                        reason: ActionAbortRequestReason::TargetIncapacitated { .. }
                    },
                    ..
                }
            )),
            other => panic!("expected ask_witness abort, got {other:?}"),
        }

        let store = world.get_component_agent_belief_store(actor).unwrap();
        assert_eq!(
            store.ask_witness_memory(&ask_witness_memory_key(&payload), Tick(4), 12),
            None
        );
    }
}
