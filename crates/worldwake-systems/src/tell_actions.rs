use std::collections::BTreeSet;
use std::num::NonZeroU32;
use worldwake_core::{
    ActionDefId, BodyCostPerTick, EntityId, EntityKind, EventTag, PerceptionSource, TellProfile,
    VisibilitySpec, World, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerId,
    ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress, ActionState,
    CommitOutcome, Constraint, DeterministicRng, DurationExpr, Interruptibility, Precondition,
    TargetSpec, TellActionPayload,
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
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let _ = tell_payload(def, &instance.payload)?;
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
        build_believed_entity_state, build_prototype_world, AgentBeliefStore, BodyCostPerTick,
        CauseRef, ControlSource, EntityId, EntityKind, EventLog, EventTag, Permille,
        PerceptionSource, Seed, TellProfile, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionError, ActionHandlerRegistry, ActionInstance, ActionPayload,
        ActionState, DeterministicRng, DurationExpr, Interruptibility, Precondition, TargetSpec,
        TellActionPayload,
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
}
