use std::collections::BTreeSet;
use std::num::NonZeroU32;
use worldwake_core::{
    ActionDefId, BodyCostPerTick, EntityId, EntityKind, EventTag, PerceptionProfile,
    PerceptionSource, TellProfile, VisibilitySpec, World, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionAbortRequestReason, ActionDef, ActionDefRegistry, ActionError,
    ActionHandler, ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload,
    ActionProgress, ActionState, CommitOutcome, Constraint, DeterministicRng, DurationExpr,
    Interruptibility, PayloadEntityRole, Precondition, TargetSpec, TellActionPayload,
};

pub fn register_tell_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(start_tell, tick_tell, commit_tell, abort_tell)
            .with_payload_override_validator(validate_tell_payload_override)
            .with_authoritative_payload_validator(validate_tell_payload_authoritatively),
    );
    let id = ActionDefId(defs.len() as u32);
    defs.register(tell_action_def(id, handler))
}

fn tell_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "tell".to_string(),
        domain: worldwake_sim::ActionDomain::Social,
        actor_constraints: vec![Constraint::ActorAlive],
        targets: vec![TargetSpec::EntityAtActorPlace {
            kind: EntityKind::Agent,
        }],
        preconditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetAtActorPlace(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Agent,
            },
            Precondition::TargetAlive(0),
        ],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::Fixed(NonZeroU32::new(2).unwrap()),
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetAtActorPlace(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Agent,
            },
            Precondition::TargetAlive(0),
        ],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::Social, EventTag::WorldMutation]),
        payload: ActionPayload::None,
        handler,
    }
}

fn tell_payload<'a>(
    def: &ActionDef,
    payload: &'a ActionPayload,
) -> Result<&'a TellActionPayload, ActionError> {
    payload.as_tell().ok_or_else(|| {
        ActionError::PreconditionFailed(format!(
            "action def {} requires Tell payload",
            def.id
        ))
    })
}

fn belief_chain_len(source: PerceptionSource) -> u8 {
    match source {
        PerceptionSource::DirectObservation | PerceptionSource::Inference => 0,
        PerceptionSource::Report { chain_len, .. } | PerceptionSource::Rumor { chain_len } => {
            chain_len
        }
    }
}

fn degrade_source(speaker: EntityId, source: PerceptionSource) -> PerceptionSource {
    match source {
        PerceptionSource::DirectObservation => PerceptionSource::Report {
            from: speaker,
            chain_len: 1,
        },
        PerceptionSource::Report { chain_len, .. } | PerceptionSource::Rumor { chain_len } => {
            PerceptionSource::Rumor {
                chain_len: chain_len.saturating_add(1),
            }
        }
        PerceptionSource::Inference => PerceptionSource::Rumor { chain_len: 1 },
    }
}

fn passes_acceptance_check(fidelity: u16, rng: &mut DeterministicRng) -> bool {
    match fidelity {
        0 => false,
        1000 => true,
        value => rng.next_range(0, 1000) < u32::from(value),
    }
}

fn validate_tell_context(
    txn: &WorldTxn<'_>,
    instance: &ActionInstance,
    payload: &TellActionPayload,
) -> Result<EntityId, ActionError> {
    let listener = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    if payload.listener != listener {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::PayloadEntityMismatch {
                role: PayloadEntityRole::Target,
                expected: listener,
                actual: payload.listener,
            },
        ));
    }

    let actor_place = txn.effective_place(instance.actor).ok_or({
        ActionError::AbortRequested(ActionAbortRequestReason::ActorNotPlaced {
            actor: instance.actor,
        })
    })?;
    if txn.effective_place(listener) != Some(actor_place) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetNotColocated {
                actor: instance.actor,
                target: listener,
            },
        ));
    }

    Ok(listener)
}

fn tell_profile(world: &WorldTxn<'_>, entity: EntityId) -> TellProfile {
    world
        .get_component_tell_profile(entity)
        .copied()
        .unwrap_or_else(TellProfile::default)
}

fn perception_profile(world: &WorldTxn<'_>, entity: EntityId) -> PerceptionProfile {
    world
        .get_component_perception_profile(entity)
        .copied()
        .unwrap_or_else(PerceptionProfile::default)
}

fn validate_tell_payload_override(
    _def: &ActionDef,
    _actor: EntityId,
    _targets: &[EntityId],
    payload: &ActionPayload,
    _view: &dyn worldwake_sim::RuntimeBeliefView,
) -> bool {
    payload.as_tell().is_some()
}

fn validate_tell_payload_authoritatively(
    def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let payload = tell_payload(def, payload)?;
    let listener = *targets.first().ok_or(ActionError::InvalidTarget(actor))?;

    if payload.listener != listener {
        return Err(ActionError::PreconditionFailed(format!(
            "tell payload listener {} does not match bound target {}",
            payload.listener, listener
        )));
    }
    if listener == actor {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} cannot tell themselves"
        )));
    }

    let beliefs = world.get_component_agent_belief_store(actor).ok_or_else(|| {
        ActionError::PreconditionFailed(format!("actor {actor} lacks AgentBeliefStore"))
    })?;
    let belief = beliefs.get_entity(&payload.subject_entity).ok_or_else(|| {
        ActionError::PreconditionFailed(format!(
            "actor {actor} lacks belief about subject {}",
            payload.subject_entity
        ))
    })?;

    let relay_limit = world
        .get_component_tell_profile(actor)
        .copied()
        .unwrap_or_else(TellProfile::default)
        .max_relay_chain_len;
    let chain_len = belief_chain_len(belief.source);
    if chain_len > relay_limit {
        return Err(ActionError::PreconditionFailed(format!(
            "subject {} chain length {} exceeds actor {actor} relay limit {}",
            payload.subject_entity, chain_len, relay_limit
        )));
    }

    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn start_tell(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let _ = tell_payload(def, &instance.payload)?;
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_tell(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

#[allow(clippy::unnecessary_wraps)]
fn commit_tell(
    def: &ActionDef,
    instance: &ActionInstance,
    rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = tell_payload(def, &instance.payload)?;
    let listener = validate_tell_context(txn, instance, payload)?;
    let speaker = instance.actor;

    let Some(speaker_beliefs) = txn.get_component_agent_belief_store(speaker) else {
        return Ok(CommitOutcome::empty());
    };
    let Some(speaker_belief) = speaker_beliefs.get_entity(&payload.subject_entity).cloned() else {
        return Ok(CommitOutcome::empty());
    };

    let relay_limit = tell_profile(txn, speaker).max_relay_chain_len;
    if belief_chain_len(speaker_belief.source) > relay_limit {
        return Ok(CommitOutcome::empty());
    }

    let listener_profile = tell_profile(txn, listener);
    if !passes_acceptance_check(listener_profile.acceptance_fidelity.value(), rng) {
        return Ok(CommitOutcome::empty());
    }

    let mut transferred = speaker_belief.clone();
    transferred.source = degrade_source(speaker, speaker_belief.source);

    let mut listener_beliefs = txn
        .get_component_agent_belief_store(listener)
        .cloned()
        .unwrap_or_default();
    listener_beliefs.update_entity(payload.subject_entity, transferred);
    listener_beliefs.enforce_capacity(&perception_profile(txn, listener), txn.tick());
    txn.set_component_agent_belief_store(listener, listener_beliefs)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_tell(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _reason: &AbortReason,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{register_tell_action, validate_tell_payload_authoritatively};
    use std::collections::BTreeSet;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_believed_entity_state, build_prototype_world, ActionDefId, AgentBeliefStore,
        BodyCostPerTick, CauseRef, ControlSource, EntityId, EntityKind, EventLog, EventTag,
        Permille, PerceptionProfile, PerceptionSource, Seed, TellProfile, Tick, VisibilitySpec,
        WitnessData, World, WorldTxn,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionError, ActionHandlerRegistry, ActionInstance, ActionPayload,
        ActionState, ActionStatus, DeterministicRng, DurationExpr, Interruptibility, Precondition,
        TargetSpec, TellActionPayload,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
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

    fn new_action_txn(
        world: &mut World,
        actor: EntityId,
        visibility: VisibilitySpec,
        tick: u64,
    ) -> WorldTxn<'_> {
        let place = world.effective_place(actor);
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            Some(actor),
            place,
            visibility,
            WitnessData::default(),
        )
    }

    fn test_rng(seed: u8) -> DeterministicRng {
        DeterministicRng::new(Seed([seed; 32]))
    }

    fn world_with_speaker_listener_and_subject(
        source: PerceptionSource,
    ) -> (World, EntityId, EntityId, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();

        let (speaker, listener, subject) = {
            let mut txn = new_txn(&mut world, 1);
            let speaker = txn.create_agent("Speaker", ControlSource::Ai).unwrap();
            let listener = txn.create_agent("Listener", ControlSource::Ai).unwrap();
            let subject = txn.create_agent("Subject", ControlSource::Ai).unwrap();
            txn.set_component_tell_profile(
                listener,
                TellProfile {
                    max_tell_candidates: 3,
                    max_relay_chain_len: 3,
                    acceptance_fidelity: Permille::new(1000).unwrap(),
                },
            )
            .unwrap();
            for entity in [speaker, listener, subject] {
                txn.set_ground_location(entity, place).unwrap();
            }
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (speaker, listener, subject)
        };

        let belief = build_believed_entity_state(&world, subject, Tick(2), source).unwrap();
        let mut store = world
            .get_component_agent_belief_store(speaker)
            .cloned()
            .unwrap_or_else(AgentBeliefStore::new);
        store.update_entity(subject, belief);

        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_agent_belief_store(speaker, store).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        (world, place, speaker, listener, subject)
    }

    fn tell_test_setup(
        source: PerceptionSource,
    ) -> (
        ActionDefRegistry,
        ActionHandlerRegistry,
        ActionDefId,
        World,
        EntityId,
        EntityId,
        EntityId,
        EntityId,
    ) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let (world, place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(source);
        (defs, handlers, tell_id, world, place, speaker, listener, subject)
    }

    fn tell_instance(
        tell_id: ActionDefId,
        speaker: EntityId,
        listener: EntityId,
        subject: EntityId,
    ) -> ActionInstance {
        ActionInstance {
            instance_id: worldwake_sim::ActionInstanceId(0),
            def_id: tell_id,
            payload: ActionPayload::Tell(TellActionPayload {
                listener,
                subject_entity: subject,
            }),
            actor: speaker,
            targets: vec![listener],
            start_tick: Tick(5),
            remaining_duration: worldwake_sim::ActionDuration::Finite(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        }
    }

    fn commit_tell_and_finalize_event(
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        tell_id: ActionDefId,
        world: &mut World,
        instance: &ActionInstance,
        seed: u8,
        tick: u64,
    ) {
        let def = defs.get(tell_id).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let mut rng = test_rng(seed);
        let mut txn = new_action_txn(world, instance.actor, def.visibility, tick);

        (handler.on_commit)(def, instance, &mut rng, &mut txn).unwrap();
        txn.add_tag(EventTag::ActionCommitted);
        for tag in &def.causal_event_tags {
            txn.add_tag(*tag);
        }
        for target in &instance.targets {
            txn.add_target(*target);
        }

        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn register_tell_action_creates_expected_definition() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();

        assert_eq!(tell.name, "tell");
        assert_eq!(tell.domain, worldwake_sim::ActionDomain::Social);
        assert_eq!(
            tell.targets,
            vec![TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }]
        );
        assert_eq!(tell.duration, DurationExpr::Fixed(NonZeroU32::new(2).unwrap()));
        assert_eq!(tell.body_cost_per_tick, BodyCostPerTick::zero());
        assert_eq!(tell.interruptibility, Interruptibility::FreelyInterruptible);
        assert_eq!(tell.visibility, VisibilitySpec::SamePlace);
        assert_eq!(
            tell.causal_event_tags,
            BTreeSet::from([EventTag::Social, EventTag::WorldMutation])
        );
        assert!(handlers.get(tell.handler).is_some());
        assert_eq!(tell.payload, ActionPayload::None);
        assert!(tell.preconditions.contains(&Precondition::TargetAlive(0)));
        assert!(tell.commit_conditions.contains(&Precondition::TargetAlive(0)));
    }

    #[test]
    fn tell_payload_validator_rejects_non_tell_payload() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (world, _place, speaker, listener, _subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[listener],
            &ActionPayload::None,
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn tell_payload_validator_rejects_listener_target_mismatch() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (world, _place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);
        let other_listener = entity(999);

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[listener],
            &ActionPayload::Tell(TellActionPayload {
                listener: other_listener,
                subject_entity: subject,
            }),
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn tell_payload_validator_rejects_self_targeting() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (world, _place, speaker, _listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[speaker],
            &ActionPayload::Tell(TellActionPayload {
                listener: speaker,
                subject_entity: subject,
            }),
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn tell_payload_validator_rejects_unknown_subject_belief() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (world, _place, speaker, listener, _subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[listener],
            &ActionPayload::Tell(TellActionPayload {
                listener,
                subject_entity: entity(404),
            }),
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn tell_payload_validator_rejects_subjects_beyond_relay_limit() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (mut world, _place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::Rumor { chain_len: 4 });

        {
            let mut txn = new_txn(&mut world, 4);
            txn.set_component_tell_profile(
                speaker,
                TellProfile {
                    max_tell_candidates: 3,
                    max_relay_chain_len: 2,
                    acceptance_fidelity: Permille::new(800).unwrap(),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[listener],
            &ActionPayload::Tell(TellActionPayload {
                listener,
                subject_entity: subject,
            }),
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn tell_payload_validator_accepts_known_relayable_subject() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (world, _place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::Report {
                from: entity(77),
                chain_len: 2,
            });

        assert_eq!(
            validate_tell_payload_authoritatively(
                tell,
                &defs,
                speaker,
                &[listener],
                &ActionPayload::Tell(TellActionPayload {
                    listener,
                    subject_entity: subject,
                }),
                &world,
            ),
            Ok(())
        );
    }

    #[test]
    fn tell_action_starts_with_tell_payload() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let handler = handlers.get(tell.handler).unwrap();
        let (mut world, _place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);
        let instance = ActionInstance {
            instance_id: worldwake_sim::ActionInstanceId(0),
            def_id: tell_id,
            payload: ActionPayload::Tell(TellActionPayload {
                listener,
                subject_entity: subject,
            }),
            actor: speaker,
            targets: vec![listener],
            start_tick: Tick(5),
            remaining_duration: worldwake_sim::ActionDuration::Finite(2),
            status: worldwake_sim::ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };
        let mut rng = test_rng(1);
        let mut txn = new_txn(&mut world, 5);

        assert_eq!(
            (handler.on_start)(tell, &instance, &mut rng, &mut txn).unwrap(),
            Some(ActionState::Empty)
        );
    }

    #[test]
    fn tell_commit_transfers_direct_observation_as_report_and_preserves_tick() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        let transferred = listener_store.get_entity(&subject).unwrap();
        assert_eq!(transferred.observed_tick, Tick(2));
        assert_eq!(
            transferred.source,
            PerceptionSource::Report {
                from: speaker,
                chain_len: 1,
            }
        );
    }

    #[test]
    fn tell_commit_degrades_report_to_rumor() {
        let report_source = PerceptionSource::Report {
            from: entity(77),
            chain_len: 2,
        };
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(report_source);
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let transferred = world
            .get_component_agent_belief_store(listener)
            .unwrap()
            .get_entity(&subject)
            .unwrap();
        assert_eq!(transferred.source, PerceptionSource::Rumor { chain_len: 3 });
    }

    #[test]
    fn tell_commit_degrades_rumor_to_deeper_rumor() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::Rumor { chain_len: 3 });
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let transferred = world
            .get_component_agent_belief_store(listener)
            .unwrap()
            .get_entity(&subject)
            .unwrap();
        assert_eq!(transferred.source, PerceptionSource::Rumor { chain_len: 4 });
    }

    #[test]
    fn tell_commit_degrades_inference_to_first_hand_rumor() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::Inference);
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let transferred = world
            .get_component_agent_belief_store(listener)
            .unwrap()
            .get_entity(&subject)
            .unwrap();
        assert_eq!(transferred.source, PerceptionSource::Rumor { chain_len: 1 });
    }

    #[test]
    fn tell_commit_skips_when_speaker_no_longer_has_subject_belief() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.set_component_agent_belief_store(speaker, AgentBeliefStore::new())
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        assert!(listener_store.get_entity(&subject).is_none());
    }

    #[test]
    fn tell_commit_respects_listener_acceptance_fidelity() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.set_component_tell_profile(
                listener,
                TellProfile {
                    max_tell_candidates: 3,
                    max_relay_chain_len: 3,
                    acceptance_fidelity: Permille::new(0).unwrap(),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        assert!(listener_store.get_entity(&subject).is_none());
    }

    #[test]
    fn tell_commit_keeps_listener_newer_belief() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let newer = build_believed_entity_state(
            &world,
            subject,
            Tick(7),
            PerceptionSource::DirectObservation,
        )
        .unwrap();
        {
            let mut store = world
                .get_component_agent_belief_store(listener)
                .cloned()
                .unwrap_or_default();
            store.update_entity(subject, newer.clone());

            let mut txn = new_txn(&mut world, 7);
            txn.set_component_agent_belief_store(listener, store).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let retained = world
            .get_component_agent_belief_store(listener)
            .unwrap()
            .get_entity(&subject)
            .unwrap();
        assert_eq!(retained, &newer);
    }

    #[test]
    fn tell_commit_enforces_listener_memory_capacity() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let older_subject = {
            let place = world.topology().place_ids().next().unwrap();
            let mut txn = new_txn(&mut world, 4);
            let older_subject = txn.create_agent("OlderSubject", ControlSource::Ai).unwrap();
            txn.set_ground_location(older_subject, place).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            older_subject
        };
        let older_belief = build_believed_entity_state(
            &world,
            older_subject,
            Tick(1),
            PerceptionSource::DirectObservation,
        )
        .unwrap();
        {
            let mut store = world
                .get_component_agent_belief_store(listener)
                .cloned()
                .unwrap_or_default();
            store.update_entity(older_subject, older_belief);

            let mut txn = new_txn(&mut world, 6);
            txn.set_component_agent_belief_store(listener, store).unwrap();
            txn.set_component_perception_profile(
                listener,
                PerceptionProfile {
                    memory_capacity: 1,
                    memory_retention_ticks: 100,
                    observation_fidelity: Permille::new(1000).unwrap(),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        assert!(listener_store.get_entity(&older_subject).is_none());
        assert!(listener_store.get_entity(&subject).is_some());
        assert_eq!(listener_store.known_entities.len(), 1);
    }

    #[test]
    fn tell_commit_rechecks_relay_limit_against_current_belief() {
        let report_source = PerceptionSource::Report {
            from: entity(77),
            chain_len: 2,
        };
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(report_source);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.set_component_tell_profile(
                speaker,
                TellProfile {
                    max_tell_candidates: 3,
                    max_relay_chain_len: 1,
                    acceptance_fidelity: Permille::new(800).unwrap(),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        assert!(listener_store.get_entity(&subject).is_none());
    }

    #[test]
    fn tell_commit_relies_on_scheduler_event_transaction_shape() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let def = defs.get(tell_id).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let instance = tell_instance(tell_id, speaker, listener, subject);
        let mut rng = test_rng(1);
        let mut txn = new_action_txn(&mut world, speaker, def.visibility, 8);

        (handler.on_commit)(def, &instance, &mut rng, &mut txn).unwrap();
        txn.add_tag(EventTag::ActionCommitted);
        for tag in &def.causal_event_tags {
            txn.add_tag(*tag);
        }
        for target in &instance.targets {
            txn.add_target(*target);
        }

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.actor_id, Some(speaker));
        assert_eq!(record.target_ids, vec![listener]);
        assert_eq!(record.visibility, VisibilitySpec::SamePlace);
        assert!(record.tags.contains(&EventTag::ActionCommitted));
        assert!(record.tags.contains(&EventTag::Social));
        assert!(record.tags.contains(&EventTag::WorldMutation));
    }
}
