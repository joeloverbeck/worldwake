use std::collections::BTreeSet;

use worldwake_core::{
    ActionDefId, AskWitnessMemory, AskWitnessMemoryKey, BodyCostPerTick, EntityId, EntityKind,
    EventLog, EventTag, ExpectationRecord, ExpectationState, LastSeenMemory,
    LastSeenProvenance, LastSeenRecord, VisibilitySpec, World, WorldTxn, is_incapacitated,
};
use worldwake_sim::{
    AbortReason, ActionAbortRequestReason, ActionDef, ActionDefRegistry, ActionError, ActionHandler,
    ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress,
    ActionState, AskAboutPersonActionPayload, CommitOutcome, Constraint, DeterministicRng,
    DurationExpr, Interruptibility, PerAgentBeliefView, Precondition, RuntimeBeliefView,
    TargetSpec,
};

pub fn register_ask_about_person_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(
            start_ask_about_person,
            tick_ask_about_person,
            commit_ask_about_person,
            abort_ask_about_person,
        )
        .with_affordance_payloads(enumerate_ask_about_person_payloads)
        .with_payload_override_validator(validate_ask_about_person_payload_override)
        .with_authoritative_payload_validator(validate_ask_about_person_payload_authoritatively),
    );
    let id = ActionDefId(defs.len() as u32);
    defs.register(ask_about_person_action_def(id, handler))
}

fn ask_about_person_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
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
        name: "ask_about_person".to_string(),
        domain: worldwake_core::ActionDomain::Epistemic,
        actor_constraints: vec![Constraint::ActorAlive, Constraint::ActorNotIncapacitated],
        targets: vec![TargetSpec::EntityAtActorPlace {
            kind: EntityKind::Agent,
        }],
        preconditions: preconditions.clone(),
        reservation_requirements: Vec::new(),
        duration: DurationExpr::ActorWitnessQueryDisposition,
        body_cost_per_tick: BodyCostPerTick::zero(),
        attention_cost: worldwake_core::Permille::ZERO,
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: preconditions,
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::Social, EventTag::Discovery]),
        payload: ActionPayload::None,
        handler,
    }
}

fn ask_about_person_payload<'a>(
    def: &ActionDef,
    payload: &'a ActionPayload,
) -> Result<&'a AskAboutPersonActionPayload, ActionError> {
    payload.as_ask_about_person().ok_or_else(|| {
        ActionError::PreconditionFailed(format!(
            "action def {} requires AskAboutPerson payload",
            def.id
        ))
    })
}

fn ask_about_person_memory_key(payload: &AskAboutPersonActionPayload) -> AskWitnessMemoryKey {
    AskWitnessMemoryKey {
        counterparty: payload.target,
        topic_entity: Some(payload.subject),
        topic_commodity: None,
    }
}

fn is_authoritatively_incapacitated(world: &World, entity: EntityId) -> bool {
    world
        .get_component_wound_list(entity)
        .zip(world.get_component_combat_profile(entity))
        .is_some_and(|(wounds, profile)| is_incapacitated(wounds, profile))
}

fn overdue_expectation_for_subject(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    subject: EntityId,
) -> Option<ExpectationRecord> {
    view.expectation_store(actor)?
        .records
        .values()
        .find(|record| {
            record.owner == actor
                && record.subject == subject
                && record.state == ExpectationState::Overdue
        })
        .copied()
}

fn enumerate_ask_about_person_payloads(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    view: &dyn RuntimeBeliefView,
) -> Vec<ActionPayload> {
    let Some(target) = targets.first().copied() else {
        return Vec::new();
    };
    if target == actor {
        return Vec::new();
    }
    let Some(profile) = view.epistemic_disposition_profile(actor) else {
        return Vec::new();
    };
    let Some(store) = view.expectation_store(actor) else {
        return Vec::new();
    };

    let mut subjects = BTreeSet::new();
    for record in store.records.values() {
        if record.owner == actor && record.state == ExpectationState::Overdue {
            subjects.insert(record.subject);
        }
    }

    subjects
        .into_iter()
        .filter(|subject| {
            view.ask_witness_memory(
                actor,
                &AskWitnessMemoryKey {
                    counterparty: target,
                    topic_entity: Some(*subject),
                    topic_commodity: None,
                },
            )
            .filter(|memory| {
                let _ = memory;
                profile.ask_memory_retention_ticks > 0
            })
            .is_none()
        })
        .map(|subject| {
            ActionPayload::AskAboutPerson(AskAboutPersonActionPayload { target, subject })
        })
        .collect()
}

fn validate_ask_about_person_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(target) = targets.first().copied() else {
        return false;
    };
    let Some(profile) = view.epistemic_disposition_profile(actor) else {
        return false;
    };
    let Some(payload) = payload.as_ask_about_person() else {
        return false;
    };
    if payload.target != target || payload.target == actor {
        return false;
    }
    if overdue_expectation_for_subject(view, actor, payload.subject).is_none() {
        return false;
    }

    view.ask_witness_memory(actor, &ask_about_person_memory_key(payload))
        .filter(|memory| {
            let _ = memory;
            profile.ask_memory_retention_ticks > 0
        })
        .is_none()
}

fn validate_ask_about_person_payload_authoritatively(
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
    let actor_profile = world
        .get_component_epistemic_disposition_profile(actor)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "actor {actor} lacks EpistemicDispositionProfile"
            ))
        })?;
    let payload = ask_about_person_payload(def, payload)?;
    if payload.target != target {
        return Err(ActionError::PreconditionFailed(format!(
            "ask_about_person payload target {} does not match bound target {}",
            payload.target, target
        )));
    }
    if payload.target == actor {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} cannot ask_about_person self"
        )));
    }

    let view = PerAgentBeliefView::from_world(actor, world);
    if overdue_expectation_for_subject(&view, actor, payload.subject).is_none() {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} lacks overdue expectation for subject {}",
            payload.subject
        )));
    }
    if view
        .ask_witness_memory(actor, &ask_about_person_memory_key(payload))
        .filter(|memory| {
            let _ = memory;
            actor_profile.ask_memory_retention_ticks > 0
        })
        .is_some()
    {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} recently queried target {} about subject {}",
            payload.target, payload.subject
        )));
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

fn validate_ask_about_person_context(
    txn: &WorldTxn<'_>,
    instance: &ActionInstance,
    payload: &AskAboutPersonActionPayload,
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

fn start_ask_about_person(
    def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let payload = ask_about_person_payload(def, &instance.payload)?;
    validate_ask_about_person_context(txn, instance, payload)?;
    let view = PerAgentBeliefView::from_world(instance.actor, txn);
    if overdue_expectation_for_subject(&view, instance.actor, payload.subject).is_none() {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {} lacks overdue expectation for subject {}",
            instance.actor, payload.subject
        )));
    }
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_ask_about_person(
    def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    let payload = ask_about_person_payload(def, &instance.payload)?;
    validate_ask_about_person_context(txn, instance, payload)?;
    Ok(ActionProgress::Continue)
}

fn relay_last_seen_record(record: LastSeenRecord, teller: EntityId) -> LastSeenRecord {
    let (original_observer, chain_depth) = match record.provenance {
        LastSeenProvenance::DirectObservation => (record.source, 1),
        LastSeenProvenance::Hearsay {
            original_observer,
            chain_depth,
        } => (original_observer, chain_depth.saturating_add(1)),
    };
    LastSeenRecord {
        subject: record.subject,
        place: record.place,
        observed_tick: record.observed_tick,
        source: teller,
        provenance: LastSeenProvenance::Hearsay {
            original_observer,
            chain_depth,
        },
    }
}

fn remember_last_seen(memory: &mut LastSeenMemory, record: LastSeenRecord) {
    if memory
        .records
        .get(&record.subject)
        .is_some_and(|existing| existing.observed_tick >= record.observed_tick)
    {
        return;
    }
    memory.records.insert(record.subject, record);
    let capacity = usize::from(memory.capacity);
    if memory.records.len() <= capacity {
        return;
    }

    let excess = memory.records.len() - capacity;
    let mut eviction_order = memory
        .records
        .iter()
        .map(|(subject, existing)| (existing.observed_tick, *subject))
        .collect::<Vec<_>>();
    eviction_order.sort_unstable();
    for (_, subject) in eviction_order.into_iter().take(excess) {
        memory.records.remove(&subject);
    }
}

fn commit_ask_about_person(
    def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = ask_about_person_payload(def, &instance.payload)?;
    let target = validate_ask_about_person_context(txn, instance, payload)?;
    let actor_profile = txn
        .get_component_epistemic_disposition_profile(instance.actor)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "actor {} lacks EpistemicDispositionProfile",
                instance.actor
            ))
        })?;

    let mut actor_store = txn
        .get_component_agent_belief_store(instance.actor)
        .cloned()
        .unwrap_or_default();
    actor_store.record_asked_witness(
        ask_about_person_memory_key(payload),
        AskWitnessMemory {
            asked_tick: txn.tick(),
        },
    );
    actor_store.enforce_ask_witness_memory(txn.tick(), actor_profile.ask_memory_retention_ticks);
    txn.set_component_agent_belief_store(instance.actor, actor_store)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;

    let Some(record) = txn
        .get_component_last_seen_memory(target)
        .and_then(|memory| memory.records.get(&payload.subject).copied())
    else {
        return Ok(CommitOutcome::empty());
    };

    let mut actor_memory = txn
        .get_component_last_seen_memory(instance.actor)
        .cloned()
        .unwrap_or_default();
    remember_last_seen(&mut actor_memory, relay_last_seen_record(record, target));
    txn.set_component_last_seen_memory(instance.actor, actor_memory)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;

    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_ask_about_person(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _reason: &AbortReason,
    _event_log: &EventLog,
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
        CauseRef, ControlSource, EpistemicDispositionProfile, EventLog, ExpectationBasis,
        ExpectationId, ExpectationStore, Permille, Seed, Tick, VisibilitySpec, WitnessData,
        build_believed_entity_state, build_prototype_world,
    };
    use worldwake_sim::{
        ActionExecutionAuthority, ActionInstanceId, Affordance, PerAgentBeliefView, TickOutcome,
        get_affordances, start_action, tick_action,
    };

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
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

    fn spawn_fixture() -> (World, EntityId, EntityId, EntityId, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let actor_place = places[0];
        let other_place = places[1];
        let actor;
        let witness;
        let subject;
        {
            let mut txn = new_txn(&mut world, 1);
            actor = txn.create_agent("Searcher", ControlSource::Ai).unwrap();
            witness = txn.create_agent("Witness", ControlSource::Ai).unwrap();
            subject = txn.create_agent("Missing", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, actor_place).unwrap();
            txn.set_ground_location(witness, actor_place).unwrap();
            txn.set_ground_location(subject, other_place).unwrap();
            commit_txn(txn);
        }
        set_epistemic_profile(&mut world, actor);
        (world, actor, witness, subject, actor_place, other_place)
    }

    fn set_epistemic_profile(world: &mut World, actor: EntityId) {
        let mut txn = new_txn(world, 1);
        txn.set_component_epistemic_disposition_profile(
            actor,
            EpistemicDispositionProfile {
                stale_evidence_barrier_threshold: pm(400),
                witness_query_duration_ticks: nz(2),
                ask_memory_retention_ticks: 12,
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    fn seed_overdue_expectation(
        world: &mut World,
        actor: EntityId,
        subject: EntityId,
        expected_place: EntityId,
    ) {
        let mut txn = new_txn(world, 2);
        let mut store = txn
            .get_component_expectation_store(actor)
            .cloned()
            .unwrap_or_else(ExpectationStore::default);
        store.records.insert(
            ExpectationId(1),
            ExpectationRecord {
                id: ExpectationId(1),
                owner: actor,
                subject,
                expected_place,
                deadline_tick: Tick(5),
                grace_ticks: 1,
                basis: ExpectationBasis::RoutineReturn,
                state: ExpectationState::Overdue,
                created_tick: Tick(1),
            },
        );
        txn.set_component_expectation_store(actor, store).unwrap();
        commit_txn(txn);
    }

    fn seed_last_seen_record(
        world: &mut World,
        holder: EntityId,
        record: LastSeenRecord,
        capacity: u16,
    ) {
        let mut txn = new_txn(world, 2);
        let mut memory = txn
            .get_component_last_seen_memory(holder)
            .cloned()
            .unwrap_or_default();
        memory.capacity = capacity;
        memory.records.insert(record.subject, record);
        txn.set_component_last_seen_memory(holder, memory).unwrap();
        commit_txn(txn);
    }

    fn seed_entity_belief(world: &mut World, actor: EntityId, subject: EntityId, tick: u64) {
        let state = build_believed_entity_state(
            world,
            subject,
            Tick(tick),
            worldwake_core::PerceptionSource::DirectObservation,
        )
        .unwrap();
        let mut txn = new_txn(world, tick);
        let mut store = txn
            .get_component_agent_belief_store(actor)
            .cloned()
            .unwrap_or_default();
        store.update_entity(subject, state);
        txn.set_component_agent_belief_store(actor, store).unwrap();
        commit_txn(txn);
    }

    fn seed_actor_ask_memory(world: &mut World, actor: EntityId, key: AskWitnessMemoryKey) {
        let mut txn = new_txn(world, 2);
        let mut store = txn
            .get_component_agent_belief_store(actor)
            .cloned()
            .unwrap_or_default();
        store.record_asked_witness(
            key,
            AskWitnessMemory {
                asked_tick: Tick(2),
            },
        );
        txn.set_component_agent_belief_store(actor, store).unwrap();
        commit_txn(txn);
    }

    fn setup_registries() -> (ActionDefRegistry, ActionHandlerRegistry, ActionDefId) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let def_id = register_ask_about_person_action(&mut defs, &mut handlers);
        (defs, handlers, def_id)
    }

    fn manual_affordance(
        actor: EntityId,
        def_id: ActionDefId,
        target: EntityId,
        subject: EntityId,
    ) -> Affordance {
        Affordance {
            def_id,
            actor,
            bound_targets: vec![target],
            payload_override: Some(ActionPayload::AskAboutPerson(AskAboutPersonActionPayload {
                target,
                subject,
            })),
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        }
    }

    fn run_action_to_completion(
        world: &mut World,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        affordance: &Affordance,
    ) -> TickOutcome {
        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([11; 32]));
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(2)),
        )
        .unwrap();

        let first = tick_action(
            instance_id,
            defs,
            handlers,
            ActionExecutionAuthority {
                world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(3)),
        )
        .unwrap();
        if !matches!(first, TickOutcome::Continuing) {
            return first;
        }

        tick_action(
            instance_id,
            defs,
            handlers,
            ActionExecutionAuthority {
                world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(4)),
        )
        .unwrap()
    }

    #[test]
    fn ask_about_person_affordance_enumerates_overdue_subjects_for_colocated_agents() {
        let (mut world, actor, witness, subject, _actor_place, other_place) = spawn_fixture();
        seed_overdue_expectation(&mut world, actor, subject, other_place);
        seed_entity_belief(&mut world, actor, witness, 2);
        let (defs, handlers, def_id) = setup_registries();
        let view = PerAgentBeliefView::from_world(actor, &world);

        let affordances = get_affordances(&view, actor, &defs, &handlers);
        assert!(affordances.iter().any(|affordance| {
            affordance.def_id == def_id
                && affordance.bound_targets == vec![witness]
                && affordance.payload_override
                    == Some(ActionPayload::AskAboutPerson(AskAboutPersonActionPayload {
                        target: witness,
                        subject,
                    }))
        }));
    }

    #[test]
    fn ask_about_person_reuses_existing_ask_memory_lane_for_duplicate_suppression() {
        let (mut world, actor, witness, subject, _actor_place, other_place) = spawn_fixture();
        seed_overdue_expectation(&mut world, actor, subject, other_place);
        seed_entity_belief(&mut world, actor, witness, 2);
        seed_actor_ask_memory(
            &mut world,
            actor,
            AskWitnessMemoryKey {
                counterparty: witness,
                topic_entity: Some(subject),
                topic_commodity: None,
            },
        );
        let (defs, handlers, def_id) = setup_registries();
        let view = PerAgentBeliefView::from_world(actor, &world);

        let affordances = get_affordances(&view, actor, &defs, &handlers);
        assert!(
            !affordances.iter().any(|affordance| affordance.def_id == def_id),
            "recently asked witness lane should suppress ask_about_person affordances"
        );
    }

    #[test]
    fn ask_about_person_commit_transfers_last_seen_with_hearsay_provenance() {
        let (mut world, actor, witness, subject, _actor_place, other_place) = spawn_fixture();
        seed_overdue_expectation(&mut world, actor, subject, other_place);
        seed_last_seen_record(
            &mut world,
            witness,
            LastSeenRecord {
                subject,
                place: other_place,
                observed_tick: Tick(7),
                source: witness,
                provenance: LastSeenProvenance::DirectObservation,
            },
            5,
        );
        let (defs, handlers, def_id) = setup_registries();
        let affordance = manual_affordance(actor, def_id, witness, subject);

        let outcome = run_action_to_completion(&mut world, &defs, &handlers, &affordance);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));

        let memory = world
            .get_component_last_seen_memory(actor)
            .expect("actor should keep last-seen memory");
        assert_eq!(
            memory.records.get(&subject),
            Some(&LastSeenRecord {
                subject,
                place: other_place,
                observed_tick: Tick(7),
                source: witness,
                provenance: LastSeenProvenance::Hearsay {
                    original_observer: witness,
                    chain_depth: 1,
                },
            })
        );
    }

    #[test]
    fn ask_about_person_increments_existing_hearsay_chain_and_respects_capacity() {
        let (mut world, actor, witness, subject, actor_place, other_place) = spawn_fixture();
        let stale_subject = EntityId {
            slot: 999,
            generation: 0,
        };
        seed_overdue_expectation(&mut world, actor, subject, other_place);
        seed_last_seen_record(
            &mut world,
            actor,
            LastSeenRecord {
                subject: stale_subject,
                place: actor_place,
                observed_tick: Tick(1),
                source: actor,
                provenance: LastSeenProvenance::DirectObservation,
            },
            1,
        );
        seed_last_seen_record(
            &mut world,
            witness,
            LastSeenRecord {
                subject,
                place: other_place,
                observed_tick: Tick(8),
                source: witness,
                provenance: LastSeenProvenance::Hearsay {
                    original_observer: subject,
                    chain_depth: 2,
                },
            },
            5,
        );
        let (defs, handlers, def_id) = setup_registries();
        let affordance = manual_affordance(actor, def_id, witness, subject);

        let outcome = run_action_to_completion(&mut world, &defs, &handlers, &affordance);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));

        let memory = world
            .get_component_last_seen_memory(actor)
            .expect("actor should keep last-seen memory");
        assert_eq!(memory.records.len(), 1);
        assert_eq!(
            memory.records.get(&subject),
            Some(&LastSeenRecord {
                subject,
                place: other_place,
                observed_tick: Tick(8),
                source: witness,
                provenance: LastSeenProvenance::Hearsay {
                    original_observer: subject,
                    chain_depth: 3,
                },
            })
        );
    }

    #[test]
    fn ask_about_person_without_record_leaves_last_seen_unchanged_but_records_query() {
        let (mut world, actor, witness, subject, actor_place, other_place) = spawn_fixture();
        seed_overdue_expectation(&mut world, actor, subject, other_place);
        seed_last_seen_record(
            &mut world,
            actor,
            LastSeenRecord {
                subject,
                place: actor_place,
                observed_tick: Tick(2),
                source: actor,
                provenance: LastSeenProvenance::DirectObservation,
            },
            5,
        );
        let (defs, handlers, def_id) = setup_registries();
        let affordance = manual_affordance(actor, def_id, witness, subject);

        let outcome = run_action_to_completion(&mut world, &defs, &handlers, &affordance);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));

        let memory = world
            .get_component_last_seen_memory(actor)
            .expect("actor should keep last-seen memory");
        assert_eq!(
            memory.records.get(&subject),
            Some(&LastSeenRecord {
                subject,
                place: actor_place,
                observed_tick: Tick(2),
                source: actor,
                provenance: LastSeenProvenance::DirectObservation,
            })
        );

        let store = world
            .get_component_agent_belief_store(actor)
            .expect("actor should keep agent belief store");
        assert_eq!(
            store.ask_witness_memory(
                &AskWitnessMemoryKey {
                    counterparty: witness,
                    topic_entity: Some(subject),
                    topic_commodity: None,
                },
                Tick(4),
                12,
            )
            .map(|memory| memory.asked_tick),
            Some(Tick(4))
        );
    }

    #[test]
    fn authoritative_validation_rejects_subject_without_overdue_expectation() {
        let (mut world, actor, witness, subject, _actor_place, _other_place) = spawn_fixture();
        let (defs, handlers, def_id) = setup_registries();
        let affordance = manual_affordance(actor, def_id, witness, subject);

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([13; 32]));
        let mut next_instance_id = ActionInstanceId(1);
        let error = start_action(
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
        .unwrap_err();

        assert!(matches!(error, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn authoritative_validation_rejects_non_colocated_target() {
        let (mut world, actor, witness, subject, _actor_place, other_place) = spawn_fixture();
        seed_overdue_expectation(&mut world, actor, subject, other_place);
        let mut txn = new_txn(&mut world, 2);
        txn.set_ground_location(witness, other_place).unwrap();
        commit_txn(txn);
        let (defs, handlers, def_id) = setup_registries();
        let affordance = manual_affordance(actor, def_id, witness, subject);

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([15; 32]));
        let mut next_instance_id = ActionInstanceId(1);
        let error = start_action(
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
        .unwrap_err();

        assert!(matches!(error, ActionError::PreconditionFailed(_)));
    }
}
