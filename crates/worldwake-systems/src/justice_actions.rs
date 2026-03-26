use std::collections::BTreeSet;
use std::num::NonZeroU32;

use worldwake_core::{
    ActionDefId, BodyCostPerTick, EntityId, EntityKind, EventTag, InstitutionalClaim, RecordData,
    RecordKind, SocialObservation, SocialObservationDetail, ViolationId, ViolationKind,
    VisibilitySpec, World, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionAbortRequestReason, ActionDef, ActionDefRegistry, ActionError,
    ActionHandler, ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload,
    ActionProgress, ActionState, AccuseActionPayload, CommitOutcome, Constraint,
    DeterministicRng, DurationExpr, Interruptibility, PerAgentBeliefView, Precondition,
    RuntimeBeliefView, TargetSpec,
};

pub fn register_accuse_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(start_accuse, tick_accuse, commit_accuse, abort_accuse)
            .with_affordance_payloads(enumerate_accuse_payloads)
            .with_payload_override_validator(validate_accuse_payload_override)
            .with_authoritative_payload_validator(validate_accuse_payload_authoritatively),
    );
    defs.register(accuse_action_def(ActionDefId(defs.len() as u32), handler))
}

fn accuse_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "accuse".to_string(),
        domain: worldwake_sim::ActionDomain::Social,
        actor_constraints: vec![
            Constraint::ActorAlive,
            Constraint::ActorHasControl,
            Constraint::ActorNotInTransit,
        ],
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
        ],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::Fixed(NonZeroU32::MIN),
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::NonInterruptible,
        commit_conditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetAtActorPlace(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Agent,
            },
        ],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([
            EventTag::Social,
            EventTag::Crime,
            EventTag::WorldMutation,
        ]),
        payload: ActionPayload::None,
        handler,
    }
}

fn accuse_payload<'a>(
    def: &ActionDef,
    payload: &'a ActionPayload,
) -> Result<&'a AccuseActionPayload, ActionError> {
    payload.as_accuse().ok_or_else(|| {
        ActionError::PreconditionFailed(format!("action def {} requires Accuse payload", def.id))
    })
}

fn validate_accuse_context(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    targets: &[EntityId],
    payload: &AccuseActionPayload,
) -> Result<(EntityId, EntityId, ViolationId), ActionError> {
    let accused = *targets.first().ok_or(ActionError::InvalidTarget(actor))?;
    if actor == accused {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} cannot accuse themselves"
        )));
    }
    let actor_place = txn
        .effective_place(actor)
        .ok_or(ActionError::AbortRequested(
            ActionAbortRequestReason::ActorNotPlaced { actor },
        ))?;
    if txn.effective_place(accused) != Some(actor_place) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetNotColocated { actor, target: accused },
        ));
    }
    Ok((accused, actor_place, payload.violation_id))
}

fn locate_unique_crime_register(world: &World, place: EntityId) -> Result<EntityId, ActionError> {
    let matching = world
        .query_record_data()
        .filter_map(|(record, data)| {
            (data.record_kind == RecordKind::CrimeRegister && world.effective_place(record) == Some(place))
                .then_some(record)
        })
        .collect::<Vec<_>>();

    match matching.as_slice() {
        [record] => Ok(*record),
        [] => Err(ActionError::PreconditionFailed(format!(
            "place {place} has no colocated CrimeRegister"
        ))),
        _ => Err(ActionError::PreconditionFailed(format!(
            "place {place} has multiple colocated CrimeRegisters"
        ))),
    }
}

fn unresolved_suspected_theft(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    violation_id: ViolationId,
) -> Option<(EntityId, EntityId, Option<EntityId>)> {
    view.active_violation_records(actor)
        .into_iter()
        .find(|record| record.id == violation_id)
        .and_then(|record| match record.kind {
            ViolationKind::SuspectedTheft {
                missing_entity,
                expected_place,
                suspect,
            } => Some((missing_entity, expected_place, suspect)),
            _ => None,
        })
}

fn social_observation_supports_case(
    observation: SocialObservation,
    accused: EntityId,
    missing_entity: EntityId,
    expected_place: EntityId,
) -> bool {
    matches!(
        observation.detail,
        SocialObservationDetail::SuspectedTheft {
            missing_entity: observed_missing,
            expected_place: observed_place,
            suspect: Some(observed_accused),
        } if observed_missing == missing_entity
            && observed_place == expected_place
            && observed_accused == accused
    )
}

fn actor_has_subjective_accusation_evidence(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    accused: EntityId,
    violation_id: ViolationId,
) -> bool {
    let Some((missing_entity, expected_place, suspect)) =
        unresolved_suspected_theft(view, actor, violation_id)
    else {
        return false;
    };

    if suspect == Some(accused) {
        return true;
    }

    view.known_social_observations(actor)
        .into_iter()
        .any(|observation| {
            social_observation_supports_case(observation, accused, missing_entity, expected_place)
        })
}

fn crime_case_already_recorded(
    record_data: &RecordData,
    accused: EntityId,
    violation_id: ViolationId,
) -> bool {
    record_data.active_entries().into_iter().any(|entry| {
        matches!(
            entry.claim,
            InstitutionalClaim::Accusation {
                accused: claim_accused,
                violation_id: claim_violation,
                ..
            } | InstitutionalClaim::Verdict {
                accused: claim_accused,
                violation_id: claim_violation,
                ..
            } if claim_accused == accused && claim_violation == violation_id
        )
    })
}

fn validate_accuse_subjective_context(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    accused: EntityId,
    violation_id: ViolationId,
) -> Result<(), ActionError> {
    if !view.is_alive(accused) {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} does not currently believe accused {accused} is alive"
        )));
    }
    if !actor_has_subjective_accusation_evidence(view, actor, accused, violation_id) {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} lacks subjective theft evidence for accused {accused} and violation {}",
            violation_id.0
        )));
    }
    Ok(())
}

fn enumerate_accuse_payloads(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    view: &dyn RuntimeBeliefView,
) -> Vec<ActionPayload> {
    let Some(accused) = targets.first().copied() else {
        return Vec::new();
    };
    if accused == actor || !view.is_alive(accused) {
        return Vec::new();
    }

    view.active_violation_records(actor)
        .into_iter()
        .filter_map(|record| {
            actor_has_subjective_accusation_evidence(view, actor, accused, record.id).then_some(
                ActionPayload::Accuse(AccuseActionPayload {
                    violation_id: record.id,
                }),
            )
        })
        .collect()
}

fn validate_accuse_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(payload) = payload.as_accuse() else {
        return false;
    };
    let Some(accused) = targets.first().copied() else {
        return false;
    };
    accused != actor && actor_has_subjective_accusation_evidence(view, actor, accused, payload.violation_id)
}

fn validate_accuse_payload_authoritatively(
    def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let payload = accuse_payload(def, payload)?;
    let view = worldwake_sim::PerAgentBeliefView::from_world(actor, world);
    let accused = *targets.first().ok_or(ActionError::InvalidTarget(actor))?;
    if accused == actor {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} cannot accuse themselves"
        )));
    }
    validate_accuse_subjective_context(&view, actor, accused, payload.violation_id)
}

fn start_accuse(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let payload = accuse_payload(def, &instance.payload)?;
    let (accused, actor_place, violation_id) =
        validate_accuse_context(txn, instance.actor, &instance.targets, payload)?;
    let view = PerAgentBeliefView::from_world(instance.actor, txn);
    validate_accuse_subjective_context(&view, instance.actor, accused, violation_id)?;
    let record = locate_unique_crime_register(txn, actor_place)?;
    let record_data = txn.get_component_record_data(record).ok_or_else(|| {
        ActionError::InternalError(format!("record {record} lacks RecordData"))
    })?;
    if crime_case_already_recorded(record_data, accused, violation_id) {
        return Err(ActionError::PreconditionFailed(format!(
            "crime case ({accused}, {}) is already recorded",
            violation_id.0
        )));
    }
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_accuse(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_accuse(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = accuse_payload(def, &instance.payload)?;
    let (accused, actor_place, violation_id) =
        validate_accuse_context(txn, instance.actor, &instance.targets, payload)?;
    let view = PerAgentBeliefView::from_world(instance.actor, txn);
    validate_accuse_subjective_context(&view, instance.actor, accused, violation_id)?;
    let record = locate_unique_crime_register(txn, actor_place)?;
    let record_data = txn.get_component_record_data(record).ok_or_else(|| {
        ActionError::InternalError(format!("record {record} lacks RecordData"))
    })?;
    if crime_case_already_recorded(record_data, accused, violation_id) {
        return Err(ActionError::PreconditionFailed(format!(
            "crime case ({accused}, {}) is already recorded",
            violation_id.0
        )));
    }
    txn.append_record_entry(
        record,
        InstitutionalClaim::Accusation {
            accuser: instance.actor,
            accused,
            violation_id,
            effective_tick: txn.tick(),
        },
    )
    .map_err(|err| ActionError::InternalError(err.to_string()))?;
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_accuse(
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
    use super::register_accuse_action;
    use std::collections::BTreeMap;
    use worldwake_core::{
        build_prototype_world, ActionDefId, AgentBeliefStore, BeliefConfidencePolicy,
        BelievedEntityState, CauseRef, EntityId, EventLog, EventTag, EventView,
        InstitutionalClaim, PerceptionProfile, PerceptionSource, PrototypePlace, Quantity,
        RecordData, RecordKind, Seed, SocialObservation, SocialObservationDetail, Tick,
        UtilityProfile,
        ViolationDispositionProfile, ViolationId, ViolationKind, ViolationMemory,
        VisibilitySpec, WitnessData, World, WorldTxn,
    };
    use worldwake_sim::{
        get_affordances, AbortReason, AccuseActionPayload, ActionDefRegistry,
        ActionError, ActionHandlerRegistry, ActionInstance, ActionInstanceId, ActionPayload,
        ActionStatus, DeterministicRng, ExternalAbortReason, PerAgentBeliefView,
    };

    fn pm(value: u16) -> worldwake_core::Permille {
        worldwake_core::Permille::new(value).unwrap()
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

    fn new_action_txn(world: &mut World, actor: EntityId, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            Some(actor),
            world.effective_place(actor),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        )
    }

    fn test_rng(seed: u8) -> DeterministicRng {
        DeterministicRng::new(Seed([seed; 32]))
    }

    fn setup_registries() -> (ActionDefRegistry, ActionHandlerRegistry, ActionDefId) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let id = register_accuse_action(&mut defs, &mut handlers);
        (defs, handlers, id)
    }

    fn commit_action(
        world: &mut World,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        def_id: ActionDefId,
        instance: &ActionInstance,
        seed: u8,
        tick: u64,
    ) -> EventLog {
        let def = defs.get(def_id).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let mut txn = new_action_txn(world, instance.actor, tick);
        let mut rng = test_rng(seed);
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
        log
    }

    fn create_record(
        txn: &mut WorldTxn<'_>,
        place: EntityId,
        issuer: EntityId,
        kind: RecordKind,
    ) -> EntityId {
        txn.create_record(RecordData {
            record_kind: kind,
            home_place: place,
            issuer,
            consultation_ticks: 4,
            max_entries_per_consult: 6,
            entries: Vec::new(),
            next_entry_id: 0,
        })
        .unwrap()
    }

    fn seed_known_entity(
        world: &mut World,
        agent: EntityId,
        entity: EntityId,
        place: EntityId,
        tick: u64,
        alive: bool,
    ) {
        let mut store = world
            .get_component_agent_belief_store(agent)
            .cloned()
            .unwrap_or_else(AgentBeliefStore::new);
        store.update_entity(
            entity,
            BelievedEntityState {
                last_known_place: Some(place),
                last_known_inventory: BTreeMap::new(),
                workstation_tag: None,
                resource_source: None,
                alive,
                wounds: Vec::new(),
                last_known_courage: None,
                observed_tick: Tick(tick),
                source: PerceptionSource::DirectObservation,
            },
        );
        let mut txn = new_txn(world, tick);
        txn.set_component_agent_belief_store(agent, store).unwrap();
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    struct JusticeFixture {
        world: World,
        place: EntityId,
        accuser: EntityId,
        accused: EntityId,
        witness: EntityId,
        crime_register: EntityId,
        violation_id: ViolationId,
        missing_item: EntityId,
    }

    impl JusticeFixture {
        fn new() -> Self {
            let mut world = World::new(build_prototype_world()).unwrap();
            let place = worldwake_core::prototype_place_entity(PrototypePlace::VillageSquare);
            let accuser;
            let suspect;
            let witness;
            let crime_register;
            let missing_item;
            let violation_id;

            {
                let mut txn = new_txn(&mut world, 1);
                accuser = txn
                    .create_agent("Accuser", worldwake_core::ControlSource::Ai)
                    .unwrap();
                suspect = txn
                    .create_agent("Accused", worldwake_core::ControlSource::Ai)
                    .unwrap();
                witness = txn
                    .create_agent("Witness", worldwake_core::ControlSource::Ai)
                    .unwrap();
                for agent in [accuser, suspect, witness] {
                    txn.set_ground_location(agent, place).unwrap();
                    txn.set_component_agent_belief_store(agent, AgentBeliefStore::new())
                        .unwrap();
                    txn.set_component_perception_profile(
                        agent,
                        PerceptionProfile {
                            memory_capacity: 16,
                            memory_retention_ticks: 100,
                            observation_fidelity: pm(1000),
                            confidence_policy: BeliefConfidencePolicy::default(),
                            institutional_memory_capacity: 16,
                            consultation_speed_factor: pm(1000),
                            contradiction_tolerance: pm(300),
                        },
                    )
                    .unwrap();
                    txn.set_component_violation_disposition_profile(
                        agent,
                        ViolationDispositionProfile {
                            investigation_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                            violation_memory_retention_ticks: 100,
                            investigation_motive_weight: pm(600),
                            ownership_motive_bonus: pm(300),
                        },
                    )
                    .unwrap();
                    txn.set_component_utility_profile(agent, UtilityProfile::default())
                        .unwrap();
                }
                crime_register = create_record(&mut txn, place, witness, RecordKind::CrimeRegister);
                missing_item = txn
                    .create_item_lot(worldwake_core::CommodityKind::Bread, Quantity(1))
                    .unwrap();
                txn.set_ground_location(missing_item, place).unwrap();
                txn.set_owner(missing_item, accuser).unwrap();
                let mut memory = ViolationMemory::default();
                violation_id = memory.record(
                    ViolationKind::SuspectedTheft {
                        missing_entity: missing_item,
                        expected_place: place,
                        suspect: None,
                    },
                    Tick(1),
                    100,
                );
                txn.set_component_violation_memory(accuser, memory).unwrap();
                let mut log = EventLog::new();
                let _ = txn.commit(&mut log);
            }

            for entity in [suspect, crime_register] {
                seed_known_entity(&mut world, accuser, entity, place, 2, true);
            }

            Self {
                world,
                place,
                accuser,
                accused: suspect,
                witness,
                crime_register,
                violation_id,
                missing_item,
            }
        }

        fn seed_social_observation(&mut self, suspect: EntityId, tick: u64) {
            let mut store = self
                .world
                .get_component_agent_belief_store(self.accuser)
                .cloned()
                .unwrap();
            store.record_social_observation(SocialObservation {
                detail: SocialObservationDetail::SuspectedTheft {
                    missing_entity: self.missing_item,
                    expected_place: self.place,
                    suspect: Some(suspect),
                },
                place: self.place,
                observed_tick: Tick(tick),
                source: PerceptionSource::Report {
                    from: self.witness,
                    chain_len: 1,
                },
            });
            let mut txn = new_txn(&mut self.world, tick);
            txn.set_component_agent_belief_store(self.accuser, store)
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        fn instance(&self, def_id: ActionDefId, accused: EntityId) -> ActionInstance {
            ActionInstance {
                instance_id: ActionInstanceId(0),
                def_id,
                payload: ActionPayload::Accuse(AccuseActionPayload {
                    violation_id: self.violation_id,
                }),
                actor: self.accuser,
                targets: vec![accused],
                start_tick: Tick(3),
                remaining_duration: worldwake_sim::ActionDuration::new(1),
                status: ActionStatus::Active,
                reservation_ids: Vec::new(),
                local_state: None,
            }
        }
    }

    #[test]
    fn register_accuse_action_creates_public_crime_definition() {
        let (defs, _handlers, id) = setup_registries();
        let def = defs.get(id).unwrap();

        assert_eq!(def.name, "accuse");
        assert_eq!(def.domain, worldwake_sim::ActionDomain::Social);
        assert_eq!(def.visibility, VisibilitySpec::SamePlace);
        assert!(def.causal_event_tags.contains(&EventTag::Crime));
        assert!(def.causal_event_tags.contains(&EventTag::Social));
    }

    #[test]
    fn accuse_affordance_emits_violation_bound_payload_for_matching_suspect_observation() {
        let (defs, handlers, id) = setup_registries();
        let mut fx = JusticeFixture::new();
        fx.seed_social_observation(fx.accused, 2);
        let view = PerAgentBeliefView::from_world(fx.accuser, &fx.world);

        let payloads = get_affordances(&view, fx.accuser, &defs, &handlers)
            .into_iter()
            .filter(|affordance| affordance.def_id == id && affordance.bound_targets == vec![fx.accused])
            .filter_map(|affordance| affordance.payload_override)
            .collect::<Vec<_>>();

        assert_eq!(
            payloads,
            vec![ActionPayload::Accuse(AccuseActionPayload {
                violation_id: fx.violation_id,
            })]
        );
    }

    #[test]
    fn accusation_appends_claim_to_crime_register_and_emits_commit_event() {
        let (defs, handlers, id) = setup_registries();
        let mut fx = JusticeFixture::new();
        fx.seed_social_observation(fx.accused, 2);
        let instance = fx.instance(id, fx.accused);

        let log = commit_action(&mut fx.world, &defs, &handlers, id, &instance, 7, 3);
        let record = fx.world.get_component_record_data(fx.crime_register).unwrap();
        let event = log
            .events_by_tag(EventTag::ActionCommitted)
            .iter()
            .map(|id| log.get(*id).unwrap())
            .find(|event| event.target_ids().contains(&fx.accused))
            .expect("commit should emit an accusation event");

        assert!(matches!(
            record.entries.last().map(|entry| entry.claim),
            Some(InstitutionalClaim::Accusation {
                accuser,
                accused,
                violation_id,
                effective_tick,
            }) if accuser == fx.accuser
                && accused == fx.accused
                && violation_id == fx.violation_id
                && effective_tick == Tick(3)
        ));
        assert!(event.tags().contains(&EventTag::Crime));
        assert_eq!(event.visibility(), VisibilitySpec::SamePlace);
    }

    #[test]
    fn duplicate_unresolved_accusation_rejects_at_start() {
        let (defs, handlers, id) = setup_registries();
        let mut fx = JusticeFixture::new();
        fx.seed_social_observation(fx.accused, 2);
        {
            let mut txn = new_txn(&mut fx.world, 2);
            txn.append_record_entry(
                fx.crime_register,
                InstitutionalClaim::Accusation {
                    accuser: fx.witness,
                    accused: fx.accused,
                    violation_id: fx.violation_id,
                    effective_tick: Tick(2),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let def = defs.get(id).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let instance = fx.instance(id, fx.accused);
        let mut txn = new_action_txn(&mut fx.world, fx.accuser, 3);
        let mut rng = test_rng(1);

        let err = (handler.on_start)(def, &instance, &mut rng, &mut txn).unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(message) if message.contains("already recorded")));
    }

    #[test]
    fn accusation_without_matching_subjective_evidence_rejects_at_start() {
        let (defs, handlers, id) = setup_registries();
        let mut fx = JusticeFixture::new();
        let def = defs.get(id).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let instance = fx.instance(id, fx.accused);
        let mut txn = new_action_txn(&mut fx.world, fx.accuser, 3);
        let mut rng = test_rng(2);

        let err = (handler.on_start)(def, &instance, &mut rng, &mut txn).unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(message) if message.contains("lacks subjective theft evidence")));
    }

    #[test]
    fn wrong_but_subjective_suspect_evidence_can_still_be_accused() {
        let (defs, handlers, id) = setup_registries();
        let mut fx = JusticeFixture::new();
        let wrong_accused;
        {
            let mut txn = new_txn(&mut fx.world, 2);
            wrong_accused = txn
                .create_agent("Wrong Suspect", worldwake_core::ControlSource::Ai)
                .unwrap();
            txn.set_ground_location(wrong_accused, fx.place).unwrap();
            txn.set_component_agent_belief_store(wrong_accused, AgentBeliefStore::new())
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        seed_known_entity(&mut fx.world, fx.accuser, wrong_accused, fx.place, 2, true);
        fx.seed_social_observation(wrong_accused, 2);
        let instance = fx.instance(id, wrong_accused);

        let _ = commit_action(&mut fx.world, &defs, &handlers, id, &instance, 9, 3);
        let record = fx.world.get_component_record_data(fx.crime_register).unwrap();

        assert!(record.entries.iter().any(|entry| {
            matches!(
                entry.claim,
                InstitutionalClaim::Accusation {
                    accused,
                    violation_id,
                    ..
                } if accused == wrong_accused && violation_id == fx.violation_id
            )
        }));
    }

    #[test]
    fn abort_is_noop() {
        let (defs, handlers, id) = setup_registries();
        let mut fx = JusticeFixture::new();
        fx.seed_social_observation(fx.accused, 2);
        let def = defs.get(id).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let instance = fx.instance(id, fx.accused);
        let before = fx.world.get_component_record_data(fx.crime_register).unwrap().clone();
        let mut txn = new_action_txn(&mut fx.world, fx.accuser, 3);
        let mut rng = test_rng(3);

        (handler.on_abort)(
            def,
            &instance,
            &AbortReason::external_abort(ExternalAbortReason::Other),
            &mut rng,
            &mut txn,
        )
        .unwrap();

        assert_eq!(
            fx.world.get_component_record_data(fx.crime_register),
            Some(&before)
        );
    }
}
