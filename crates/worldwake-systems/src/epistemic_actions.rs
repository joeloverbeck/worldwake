use std::collections::BTreeSet;
use worldwake_core::{
    build_believed_entity_state, is_incapacitated, ActionDefId, AskWitnessMemory,
    AskWitnessMemoryKey, BelievedEntityState, BodyCostPerTick, EntityId, EntityKind, EventTag,
    PerceptionSource, Quantity, VerificationSubject, ViolationKind, VisibilitySpec, World,
    WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionAbortRequestReason, ActionDef, ActionDefRegistry, ActionError,
    ActionHandler, ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload,
    ActionProgress, ActionState, CommitOutcome, Constraint, DeterministicRng, DurationExpr,
    Interruptibility, Precondition, RuntimeBeliefView, TargetSpec, AskWitnessPayload,
    VerifyBeliefPayload,
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

pub fn register_verify_belief_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(
            start_verify_belief,
            tick_verify_belief,
            commit_verify_belief,
            abort_verify_belief,
        )
        .with_affordance_payloads(enumerate_verify_belief_payloads)
        .with_payload_override_validator(validate_verify_belief_payload_override)
        .with_authoritative_payload_validator(validate_verify_belief_payload_authoritatively),
    );
    let id = ActionDefId(defs.len() as u32);
    defs.register(verify_belief_action_def(id, handler))
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
        domain: worldwake_sim::ActionDomain::Epistemic,
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

fn verify_belief_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    let preconditions = vec![
        Precondition::ActorAlive,
        Precondition::TargetExists(0),
        Precondition::TargetKind {
            target_index: 0,
            kind: EntityKind::Place,
        },
    ];
    let commit_conditions = vec![
        Precondition::ActorAlive,
        Precondition::TargetExists(0),
        Precondition::TargetKind {
            target_index: 0,
            kind: EntityKind::Place,
        },
    ];

    ActionDef {
        id,
        name: "verify_belief".to_string(),
        domain: worldwake_sim::ActionDomain::Epistemic,
        actor_constraints: vec![Constraint::ActorAlive, Constraint::ActorNotIncapacitated],
        targets: vec![TargetSpec::ActorPlace],
        preconditions: preconditions.clone(),
        reservation_requirements: Vec::new(),
        duration: DurationExpr::ActorVerificationDisposition,
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions,
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::Discovery]),
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
    let entity_match = payload.topic_entity.is_none_or(|topic_entity| entity == topic_entity);
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
    if target == actor || view.verification_disposition_profile(actor).is_none() {
        return Vec::new();
    }

    let mut payloads = BTreeSet::new();
    for (entity, state) in view.known_entity_beliefs(actor) {
        let payload = AskWitnessPayload {
            target,
            topic_entity: Some(entity),
            topic_commodity: state.resource_source.as_ref().map(|resource| resource.commodity),
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
    if view.verification_disposition_profile(actor).is_none() {
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
        .get_component_verification_disposition_profile(actor)
        .is_none()
    {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} lacks VerificationDispositionProfile"
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
    instance: &ActionInstance,
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
        .get_component_verification_disposition_profile(instance.actor)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "actor {} lacks VerificationDispositionProfile",
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
    _reason: &AbortReason,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

fn subject_place(subject: VerificationSubject) -> EntityId {
    match subject {
        VerificationSubject::EntityLocation { place, .. }
        | VerificationSubject::SupplyAvailability { place, .. } => place,
    }
}

fn subject_matches_belief(
    entity: EntityId,
    state: &BelievedEntityState,
    subject: VerificationSubject,
) -> bool {
    match subject {
        VerificationSubject::EntityLocation {
            entity: subject_entity,
            place,
        } => entity == subject_entity && state.last_known_place == Some(place),
        VerificationSubject::SupplyAvailability {
            commodity,
            source,
            place,
        } => {
            entity == source
                && state.last_known_place == Some(place)
                && state
                    .resource_source
                    .as_ref()
                    .is_some_and(|resource| resource.commodity == commodity)
        }
    }
}

fn actor_has_verifiable_subject(
    world: &World,
    actor: EntityId,
    subject: VerificationSubject,
) -> bool {
    world
        .get_component_agent_belief_store(actor)
        .into_iter()
        .flat_map(|store| store.known_entities.iter())
        .any(|(entity, state)| subject_matches_belief(*entity, state, subject))
}

fn enumerate_verify_belief_payloads(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    view: &dyn RuntimeBeliefView,
) -> Vec<ActionPayload> {
    let Some(place) = targets.first().copied() else {
        return Vec::new();
    };
    if view.verification_disposition_profile(actor).is_none() {
        return Vec::new();
    }

    view.known_entity_beliefs(actor)
        .into_iter()
        .filter(|(_, state)| state.last_known_place == Some(place))
        .flat_map(|(entity, state)| {
            let mut payloads = vec![ActionPayload::VerifyBelief(VerifyBeliefPayload {
                subject: VerificationSubject::EntityLocation { entity, place },
            })];
            if let Some(resource) = state.resource_source {
                payloads.push(ActionPayload::VerifyBelief(VerifyBeliefPayload {
                    subject: VerificationSubject::SupplyAvailability {
                        commodity: resource.commodity,
                        source: entity,
                        place,
                    },
                }));
            }
            payloads
        })
        .collect()
}

fn validate_verify_belief_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(place) = targets.first().copied() else {
        return false;
    };
    if view.verification_disposition_profile(actor).is_none() {
        return false;
    }
    let Some(payload) = payload.as_verify_belief() else {
        return false;
    };
    if subject_place(payload.subject) != place {
        return false;
    }

    view.known_entity_beliefs(actor)
        .into_iter()
        .any(|(entity, state)| subject_matches_belief(entity, &state, payload.subject))
}

fn verify_belief_payload<'a>(
    def: &ActionDef,
    payload: &'a ActionPayload,
) -> Result<&'a VerifyBeliefPayload, ActionError> {
    payload.as_verify_belief().ok_or_else(|| {
        ActionError::PreconditionFailed(format!(
            "action def {} requires VerifyBelief payload",
            def.id
        ))
    })
}

fn validate_verify_belief_payload_authoritatively(
    def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let Some(place) = targets.first().copied() else {
        return Err(ActionError::InvalidTarget(actor));
    };
    if world
        .get_component_verification_disposition_profile(actor)
        .is_none()
    {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} lacks VerificationDispositionProfile"
        )));
    }

    let payload = verify_belief_payload(def, payload)?;
    if subject_place(payload.subject) != place {
        return Err(ActionError::PreconditionFailed(format!(
            "verify_belief payload place {} does not match bound target {}",
            subject_place(payload.subject),
            place
        )));
    }
    if !actor_has_verifiable_subject(world, actor, payload.subject) {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} has no verifiable belief for subject {:?}",
            payload.subject
        )));
    }
    Ok(())
}

fn validate_verify_belief_context(
    txn: &WorldTxn<'_>,
    instance: &ActionInstance,
    payload: &VerifyBeliefPayload,
) -> Result<(), ActionError> {
    let place = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    if subject_place(payload.subject) != place {
        return Err(ActionError::PreconditionFailed(format!(
            "verify_belief payload place {} does not match bound target {}",
            subject_place(payload.subject),
            place
        )));
    }
    let actor_place = txn.effective_place(instance.actor).ok_or_else(|| {
        ActionError::PreconditionFailed(format!("actor {} is not placed", instance.actor))
    })?;
    if actor_place != place {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {} is not at verification place {}",
            instance.actor, place
        )));
    }
    Ok(())
}

fn start_verify_belief(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let payload = verify_belief_payload(def, &instance.payload)?;
    validate_verify_belief_context(txn, instance, payload)?;
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_verify_belief(
    def: &ActionDef,
    instance: &mut ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    let payload = verify_belief_payload(def, &instance.payload)?;
    let place = subject_place(payload.subject);
    if txn.effective_place(instance.actor) != Some(place) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetNotColocated {
                actor: instance.actor,
                target: place,
            },
        ));
    }
    Ok(ActionProgress::Continue)
}

fn record_violation(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    kind: ViolationKind,
) -> Result<(), ActionError> {
    let retention = txn
        .get_component_violation_disposition_profile(actor)
        .map(|profile| profile.violation_memory_retention_ticks)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "actor {actor} lacks ViolationDispositionProfile"
            ))
        })?;
    let mut memory = txn
        .get_component_violation_memory(actor)
        .cloned()
        .unwrap_or_default();
    memory.record(kind, txn.tick(), retention);
    txn.set_component_violation_memory(actor, memory)
        .map_err(|err| ActionError::InternalError(err.to_string()))
}

fn commit_verify_belief(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = verify_belief_payload(def, &instance.payload)?;
    validate_verify_belief_context(txn, instance, payload)?;

    match payload.subject {
        VerificationSubject::EntityLocation { entity, place } => {
            if txn.effective_place(entity) == Some(place) {
                let state = build_believed_entity_state(
                    txn,
                    entity,
                    txn.tick(),
                    PerceptionSource::DirectObservation,
                )
                .ok_or_else(|| {
                    ActionError::PreconditionFailed(format!(
                        "subject {entity} is not observable at verification commit"
                    ))
                })?;
                let mut store = txn
                    .get_component_agent_belief_store(instance.actor)
                    .cloned()
                    .unwrap_or_default();
                store.update_entity(entity, state);
                txn.set_component_agent_belief_store(instance.actor, store)
                    .map_err(|err| ActionError::InternalError(err.to_string()))?;
            } else {
                record_violation(
                    txn,
                    instance.actor,
                    ViolationKind::EntityMissing {
                        entity,
                        expected_place: place,
                    },
                )?;
            }
        }
        VerificationSubject::SupplyAvailability {
            commodity,
            source,
            place,
        } => {
            let observed_state = (txn.effective_place(source) == Some(place))
                .then(|| {
                    build_believed_entity_state(
                        txn,
                        source,
                        txn.tick(),
                        PerceptionSource::DirectObservation,
                    )
                })
                .flatten();

            if let Some(state) = observed_state {
                let mut store = txn
                    .get_component_agent_belief_store(instance.actor)
                    .cloned()
                    .unwrap_or_default();
                store.update_entity(source, state.clone());
                txn.set_component_agent_belief_store(instance.actor, store)
                    .map_err(|err| ActionError::InternalError(err.to_string()))?;

                let productive = state.resource_source.as_ref().is_some_and(|resource| {
                    resource.commodity == commodity && resource.available_quantity > Quantity(0)
                });
                if !productive {
                    record_violation(
                        txn,
                        instance.actor,
                        ViolationKind::SupplyDepleted {
                            commodity,
                            source,
                            place,
                        },
                    )?;
                }
            } else {
                record_violation(
                    txn,
                    instance.actor,
                    ViolationKind::SupplyDepleted {
                        commodity,
                        source,
                        place,
                    },
                )?;
            }
        }
    }

    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_verify_belief(
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
    use super::*;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, BodyPart, CauseRef, CombatProfile, CombatWeaponRef,
        CommodityKind, ControlSource, DeadAt, EventLog, Permille, ResourceSource, Seed, Tick,
        VerificationDispositionProfile, ViolationDispositionProfile, ViolationMemory, WitnessData,
        Wound, WoundCause, WoundId, WoundList,
    };
    use worldwake_sim::{
        get_affordances, start_action, tick_action, ActionExecutionAuthority,
        ActionExecutionContext, ActionInstanceId, Affordance, PerAgentBeliefView, TickOutcome,
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

    fn spawn_resource_source(
        world: &mut World,
        place: EntityId,
        commodity: CommodityKind,
        available_quantity: u32,
    ) -> EntityId {
        let mut txn = new_txn(world, 1);
        let source = txn.create_entity(EntityKind::Facility);
        txn.set_ground_location(source, place).unwrap();
        txn.set_component_resource_source(
            source,
            ResourceSource {
                commodity,
                available_quantity: Quantity(available_quantity),
                max_quantity: Quantity(10),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        )
        .unwrap();
        commit_txn(txn);
        source
    }

    fn set_verification_profile(world: &mut World, actor: EntityId, duration: u32) {
        let mut txn = new_txn(world, 1);
        txn.set_component_verification_disposition_profile(
            actor,
            VerificationDispositionProfile {
                belief_verification_threshold: pm(400),
                verify_belief_duration_ticks: nz(duration),
                witness_query_duration_ticks: nz(2),
                verification_motive_weight: pm(200),
                ask_memory_retention_ticks: 12,
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    fn set_violation_tracking(world: &mut World, actor: EntityId, retention: u32) {
        let mut txn = new_txn(world, 1);
        txn.set_component_violation_disposition_profile(
            actor,
            ViolationDispositionProfile {
                investigation_duration_ticks: nz(2),
                violation_memory_retention_ticks: retention,
                investigation_motive_weight: pm(300),
                ownership_motive_bonus: pm(200),
            },
        )
        .unwrap();
        txn.set_component_violation_memory(actor, ViolationMemory::default())
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
        let state = build_believed_entity_state(
            world,
            subject,
            Tick(observed_tick),
            source,
        )
        .unwrap();
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

    fn set_source_available_quantity(world: &mut World, source: EntityId, qty: u32, tick: u64) {
        let mut txn = new_txn(world, tick);
        let mut resource = txn.get_component_resource_source(source).cloned().unwrap();
        resource.available_quantity = Quantity(qty);
        txn.set_component_resource_source(source, resource).unwrap();
        commit_txn(txn);
    }

    fn setup_registries() -> (ActionDefRegistry, ActionHandlerRegistry, ActionDefId) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let def_id = register_verify_belief_action(&mut defs, &mut handlers);
        (defs, handlers, def_id)
    }

    fn setup_registries_with_ask(
    ) -> (
        ActionDefRegistry,
        ActionHandlerRegistry,
        ActionDefId,
        ActionDefId,
    ) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let verify_id = register_verify_belief_action(&mut defs, &mut handlers);
        let ask_id = register_ask_witness_action(&mut defs, &mut handlers);
        (defs, handlers, verify_id, ask_id)
    }

    fn verify_affordance_for_subject(
        world: &World,
        actor: EntityId,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        subject: VerificationSubject,
    ) -> Affordance {
        let view = PerAgentBeliefView::from_world(actor, world);
        let effective_place = view.effective_place(actor);
        let known_beliefs = view.known_entity_beliefs(actor);
        let has_profile = view.verification_disposition_profile(actor).is_some();
        let affordances = get_affordances(&view, actor, defs, handlers);
        affordances
            .iter()
            .find(|affordance| {
                defs.get(affordance.def_id).unwrap().name == "verify_belief"
                    && affordance
                        .payload_override
                        .as_ref()
                        .and_then(ActionPayload::as_verify_belief)
                        .is_some_and(|payload| payload.subject == subject)
            })
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "verify_belief affordance should exist for {subject:?}; effective_place={effective_place:?}; has_profile={has_profile}; known_beliefs={known_beliefs:?}; available affordances: {affordances:?}"
                )
            })
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
            ActionExecutionContext {
                tick: Tick(start_tick),
                cause: CauseRef::Bootstrap,
            },
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
                ActionExecutionContext {
                    tick: Tick(tick),
                    cause: CauseRef::Bootstrap,
                },
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
    fn register_verify_belief_action_creates_expected_definition() {
        let (defs, handlers, def_id) = setup_registries();
        let def = defs.get(def_id).unwrap();

        assert!(handlers.get(def.handler).is_some());
        assert_eq!(def.name, "verify_belief");
        assert_eq!(def.domain, worldwake_sim::ActionDomain::Epistemic);
        assert_eq!(def.targets, vec![TargetSpec::ActorPlace]);
        assert_eq!(def.duration, DurationExpr::ActorVerificationDisposition);
        assert_eq!(def.interruptibility, Interruptibility::FreelyInterruptible);
        assert_eq!(def.visibility, VisibilitySpec::SamePlace);
        assert_eq!(def.causal_event_tags, BTreeSet::from([EventTag::Discovery]));
    }

    #[test]
    fn register_ask_witness_action_creates_expected_definition() {
        let (defs, handlers, _, def_id) = setup_registries_with_ask();
        let def = defs.get(def_id).unwrap();

        assert!(handlers.get(def.handler).is_some());
        assert_eq!(def.name, "ask_witness");
        assert_eq!(def.domain, worldwake_sim::ActionDomain::Epistemic);
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
        set_verification_profile(&mut world, actor, 2);
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
        let (defs, handlers, _, ask_id) = setup_registries_with_ask();
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
        set_verification_profile(&mut world, actor, 2);
        seed_entity_belief(&mut world, actor, subject, 1);

        let payload = AskWitnessPayload {
            target: witness,
            topic_entity: Some(subject),
            topic_commodity: None,
        };
        let (defs, handlers, _, ask_id) = setup_registries_with_ask();
        let affordance = manual_ask_affordance(actor, ask_id, witness, payload.clone());

        match run_action_to_completion(&mut world, &defs, &handlers, &affordance, [10; 32], 2, 4)
        {
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
        set_verification_profile(&mut world, actor, 2);
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

        let (defs, handlers, _, _) = setup_registries_with_ask();
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
        set_verification_profile(&mut world, actor, 2);

        let (defs, handlers, _, ask_id) = setup_registries_with_ask();
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
        set_verification_profile(&mut world, actor, 2);
        seed_entity_belief(&mut world, actor, subject, 1);

        let payload = AskWitnessPayload {
            target: witness,
            topic_entity: Some(subject),
            topic_commodity: None,
        };
        let (defs, handlers, _, ask_id) = setup_registries_with_ask();
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
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
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
                ActionExecutionContext {
                    tick: Tick(3),
                    cause: CauseRef::Bootstrap,
                },
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
            ActionExecutionContext {
                tick: Tick(4),
                cause: CauseRef::Bootstrap,
            },
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
        set_verification_profile(&mut world, actor, 2);
        seed_entity_belief(&mut world, actor, subject, 1);

        let payload = AskWitnessPayload {
            target: witness,
            topic_entity: Some(subject),
            topic_commodity: None,
        };
        let (defs, handlers, _, ask_id) = setup_registries_with_ask();
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
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
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
            ActionExecutionContext {
                tick: Tick(3),
                cause: CauseRef::Bootstrap,
            },
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
            ActionExecutionContext {
                tick: Tick(4),
                cause: CauseRef::Bootstrap,
            },
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
        set_verification_profile(&mut world, actor, 2);
        seed_entity_belief(&mut world, actor, subject, 1);

        let payload = AskWitnessPayload {
            target: witness,
            topic_entity: Some(subject),
            topic_commodity: None,
        };
        let (defs, handlers, _, ask_id) = setup_registries_with_ask();
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
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
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
            ActionExecutionContext {
                tick: Tick(3),
                cause: CauseRef::Bootstrap,
            },
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
            ActionExecutionContext {
                tick: Tick(4),
                cause: CauseRef::Bootstrap,
            },
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

    #[test]
    fn verify_belief_entity_location_refreshes_direct_observation() {
        let mut world = new_world();
        let (place, _) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place, "Aster");
        let subject = spawn_actor(&mut world, place, "Bram");
        set_verification_profile(&mut world, actor, 2);
        seed_entity_belief(&mut world, actor, subject, 1);

        let (defs, handlers, _) = setup_registries();
        let affordance = verify_affordance_for_subject(
            &world,
            actor,
            &defs,
            &handlers,
            VerificationSubject::EntityLocation {
                entity: subject,
                place,
            },
        );

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([1; 32]));
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
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
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
                ActionExecutionContext {
                    tick: Tick(3),
                    cause: CauseRef::Bootstrap,
                },
            )
            .unwrap(),
            TickOutcome::Continuing
        );

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
            ActionExecutionContext {
                tick: Tick(4),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap()
        {
            TickOutcome::Committed { .. } => {}
            other => panic!("expected verify_belief commit, got {other:?}"),
        }

        let belief = world
            .get_component_agent_belief_store(actor)
            .unwrap()
            .get_entity(&subject)
            .unwrap();
        assert_eq!(belief.last_known_place, Some(place));
        assert_eq!(belief.observed_tick, Tick(4));
        assert_eq!(belief.source, PerceptionSource::DirectObservation);
    }

    #[test]
    fn verify_belief_missing_entity_records_entity_missing_violation() {
        let mut world = new_world();
        let (place, other_place) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place, "Aster");
        let subject = spawn_actor(&mut world, place, "Bram");
        set_verification_profile(&mut world, actor, 2);
        set_violation_tracking(&mut world, actor, 50);
        seed_entity_belief(&mut world, actor, subject, 1);
        set_ground_location(&mut world, subject, other_place, 2);

        let (defs, handlers, _) = setup_registries();
        let affordance = verify_affordance_for_subject(
            &world,
            actor,
            &defs,
            &handlers,
            VerificationSubject::EntityLocation {
                entity: subject,
                place,
            },
        );

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([2; 32]));
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
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
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
            ActionExecutionContext {
                tick: Tick(3),
                cause: CauseRef::Bootstrap,
            },
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
            ActionExecutionContext {
                tick: Tick(4),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap();

        let memory = world.get_component_violation_memory(actor).unwrap();
        assert!(memory.violations.iter().any(|record| {
            record.kind
                == ViolationKind::EntityMissing {
                    entity: subject,
                    expected_place: place,
                }
                && record.observed_tick == Tick(4)
                && record.expires_tick == Tick(54)
        }));
    }

    #[test]
    fn verify_belief_productive_supply_refreshes_belief() {
        let mut world = new_world();
        let (place, _) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place, "Aster");
        let source = spawn_resource_source(&mut world, place, CommodityKind::Apple, 4);
        set_verification_profile(&mut world, actor, 2);
        seed_entity_belief(&mut world, actor, source, 1);

        let (defs, handlers, _) = setup_registries();
        let affordance = verify_affordance_for_subject(
            &world,
            actor,
            &defs,
            &handlers,
            VerificationSubject::SupplyAvailability {
                commodity: CommodityKind::Apple,
                source,
                place,
            },
        );

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([3; 32]));
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
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
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
            ActionExecutionContext {
                tick: Tick(3),
                cause: CauseRef::Bootstrap,
            },
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
            ActionExecutionContext {
                tick: Tick(4),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap();

        let belief = world
            .get_component_agent_belief_store(actor)
            .unwrap()
            .get_entity(&source)
            .unwrap();
        assert_eq!(belief.observed_tick, Tick(4));
        assert_eq!(belief.source, PerceptionSource::DirectObservation);
        assert_eq!(
            belief.resource_source.as_ref().unwrap().available_quantity,
            Quantity(4)
        );
    }

    #[test]
    fn verify_belief_depleted_supply_records_violation_and_refreshes_zero_supply() {
        let mut world = new_world();
        let (place, _) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place, "Aster");
        let source = spawn_resource_source(&mut world, place, CommodityKind::Apple, 4);
        set_verification_profile(&mut world, actor, 2);
        set_violation_tracking(&mut world, actor, 20);
        seed_entity_belief(&mut world, actor, source, 1);
        set_source_available_quantity(&mut world, source, 0, 2);

        let (defs, handlers, _) = setup_registries();
        let affordance = verify_affordance_for_subject(
            &world,
            actor,
            &defs,
            &handlers,
            VerificationSubject::SupplyAvailability {
                commodity: CommodityKind::Apple,
                source,
                place,
            },
        );

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([4; 32]));
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
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
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
            ActionExecutionContext {
                tick: Tick(3),
                cause: CauseRef::Bootstrap,
            },
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
            ActionExecutionContext {
                tick: Tick(4),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap();

        let belief = world
            .get_component_agent_belief_store(actor)
            .unwrap()
            .get_entity(&source)
            .unwrap();
        assert_eq!(
            belief.resource_source.as_ref().unwrap().available_quantity,
            Quantity(0)
        );
        assert_eq!(belief.observed_tick, Tick(4));

        let memory = world.get_component_violation_memory(actor).unwrap();
        assert!(memory.violations.iter().any(|record| {
            record.kind
                == ViolationKind::SupplyDepleted {
                    commodity: CommodityKind::Apple,
                    source,
                    place,
                }
                && record.observed_tick == Tick(4)
                && record.expires_tick == Tick(24)
        }));
    }

    #[test]
    fn verify_belief_requires_matching_payload_place() {
        let mut world = new_world();
        let (place, other_place) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place, "Aster");
        let subject = spawn_actor(&mut world, place, "Bram");
        set_verification_profile(&mut world, actor, 2);
        seed_entity_belief(&mut world, actor, subject, 1);

        let (defs, handlers, def_id) = setup_registries();
        let affordance = Affordance {
            def_id,
            actor,
            bound_targets: vec![place],
            payload_override: Some(ActionPayload::VerifyBelief(VerifyBeliefPayload {
                subject: VerificationSubject::EntityLocation {
                    entity: subject,
                    place: other_place,
                },
            })),
            explanation: None,
        };

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([5; 32]));
        let mut next_instance_id = ActionInstanceId(1);
        let err = start_action(
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
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::PreconditionFailed(format!(
                "verify_belief payload place {other_place} does not match bound target {place}"
            ))
        );
    }

    #[test]
    fn verify_belief_requires_verification_profile_for_affordance_enumeration() {
        let mut world = new_world();
        let (place, _) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place, "Aster");
        let subject = spawn_actor(&mut world, place, "Bram");
        seed_entity_belief(&mut world, actor, subject, 1);

        let (defs, handlers, _) = setup_registries();
        let affordances = get_affordances(
            &PerAgentBeliefView::from_world(actor, &world),
            actor,
            &defs,
            &handlers,
        );

        assert!(affordances.into_iter().all(|affordance| defs
            .get(affordance.def_id)
            .unwrap()
            .name
            != "verify_belief"));
    }

    #[test]
    fn verify_belief_abort_on_leaving_place_keeps_stale_belief_and_records_no_violation() {
        let mut world = new_world();
        let (place, other_place) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place, "Aster");
        let subject = spawn_actor(&mut world, place, "Bram");
        set_verification_profile(&mut world, actor, 2);
        set_violation_tracking(&mut world, actor, 20);
        seed_entity_belief(&mut world, actor, subject, 1);

        let (defs, handlers, _) = setup_registries();
        let affordance = verify_affordance_for_subject(
            &world,
            actor,
            &defs,
            &handlers,
            VerificationSubject::EntityLocation {
                entity: subject,
                place,
            },
        );

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([6; 32]));
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
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
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
                ActionExecutionContext {
                    tick: Tick(3),
                    cause: CauseRef::Bootstrap,
                },
            )
            .unwrap(),
            TickOutcome::Continuing
        );

        set_ground_location(&mut world, actor, other_place, 3);

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
            ActionExecutionContext {
                tick: Tick(4),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap()
        {
            TickOutcome::Aborted { reason, .. } => {
                assert_eq!(
                    reason,
                    AbortReason::external_abort(
                        worldwake_sim::ExternalAbortReason::HandlerRequested {
                            reason: ActionAbortRequestReason::TargetNotColocated {
                                actor,
                                target: place,
                            },
                        }
                    )
                );
            }
            other => panic!("expected verify_belief abort, got {other:?}"),
        }

        let belief = world
            .get_component_agent_belief_store(actor)
            .unwrap()
            .get_entity(&subject)
            .unwrap();
        assert_eq!(belief.observed_tick, Tick(1));
        assert_eq!(belief.source, PerceptionSource::Rumor { chain_len: 1 });
        assert!(world
            .get_component_violation_memory(actor)
            .unwrap()
            .violations
            .is_empty());
    }
}
