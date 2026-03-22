use std::collections::BTreeSet;
use worldwake_core::{
    ActionDefId, BelievedInstitutionalClaim, BodyCostPerTick, EntityId, EntityKind, EventTag,
    InstitutionalBeliefKey, InstitutionalClaim, InstitutionalKnowledgeSource, VisibilitySpec,
    World, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionAbortRequestReason, ActionDef, ActionDefRegistry, ActionError,
    ActionHandler, ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload,
    ActionProgress, ActionState, Constraint, ConsultRecordActionPayload, DeterministicRng,
    DurationExpr, Interruptibility, PayloadEntityRole, Precondition, RuntimeBeliefView, TargetSpec,
};

pub fn register_consult_record_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(
            start_consult_record,
            tick_consult_record,
            commit_consult_record,
            abort_consult_record,
        )
        .with_affordance_payloads(enumerate_consult_record_payloads)
        .with_payload_override_validator(validate_consult_record_payload_override)
        .with_authoritative_payload_validator(validate_consult_record_payload_authoritatively),
    );
    let id = ActionDefId(defs.len() as u32);
    defs.register(consult_record_action_def(id, handler))
}

fn consult_record_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "consult_record".to_string(),
        domain: worldwake_sim::ActionDomain::Social,
        actor_constraints: vec![
            Constraint::ActorAlive,
            Constraint::ActorHasControl,
            Constraint::ActorNotInTransit,
        ],
        targets: vec![TargetSpec::EntityAtActorPlace {
            kind: EntityKind::Record,
        }],
        preconditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetAtActorPlace(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Record,
            },
        ],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::ConsultRecord { target_index: 0 },
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetAtActorPlace(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Record,
            },
        ],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::Social, EventTag::WorldMutation]),
        payload: ActionPayload::None,
        handler,
    }
}

fn consult_record_payload<'a>(
    def: &ActionDef,
    payload: &'a ActionPayload,
) -> Result<&'a ConsultRecordActionPayload, ActionError> {
    payload.as_consult_record().ok_or_else(|| {
        ActionError::PreconditionFailed(format!(
            "action def {} requires ConsultRecord payload",
            def.id
        ))
    })
}

fn validate_consult_record_context(
    txn: &WorldTxn<'_>,
    instance: &ActionInstance,
    payload: &ConsultRecordActionPayload,
) -> Result<EntityId, ActionError> {
    let record = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    if payload.record != record {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::PayloadEntityMismatch {
                role: PayloadEntityRole::Target,
                expected: record,
                actual: payload.record,
            },
        ));
    }

    let actor_place = txn.effective_place(instance.actor).ok_or({
        ActionError::AbortRequested(ActionAbortRequestReason::ActorNotPlaced {
            actor: instance.actor,
        })
    })?;
    if txn.effective_place(record) != Some(actor_place) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetNotColocated {
                actor: instance.actor,
                target: record,
            },
        ));
    }

    Ok(record)
}

fn enumerate_consult_record_payloads(
    _def: &ActionDef,
    _actor: EntityId,
    targets: &[EntityId],
    _view: &dyn RuntimeBeliefView,
) -> Vec<ActionPayload> {
    let Some(record) = targets.first().copied() else {
        return Vec::new();
    };
    vec![ActionPayload::ConsultRecord(ConsultRecordActionPayload {
        record,
    })]
}

fn validate_consult_record_payload_override(
    _def: &ActionDef,
    _actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(payload) = payload.as_consult_record() else {
        return false;
    };
    targets.first().copied() == Some(payload.record)
        && view.entity_kind(payload.record) == Some(EntityKind::Record)
}

fn validate_consult_record_payload_authoritatively(
    def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let payload = consult_record_payload(def, payload)?;
    let record = *targets.first().ok_or(ActionError::InvalidTarget(actor))?;
    if payload.record != record {
        return Err(ActionError::PreconditionFailed(format!(
            "consult_record payload record {} does not match bound target {}",
            payload.record, record
        )));
    }
    if world.entity_kind(record) != Some(EntityKind::Record) {
        return Err(ActionError::PreconditionFailed(format!(
            "consult_record target {record} is not a record"
        )));
    }
    Ok(())
}

fn institutional_belief_key(claim: InstitutionalClaim) -> InstitutionalBeliefKey {
    match claim {
        InstitutionalClaim::OfficeHolder { office, .. } => {
            InstitutionalBeliefKey::OfficeHolderOf { office }
        }
        InstitutionalClaim::FactionMembership { faction, .. } => {
            InstitutionalBeliefKey::FactionMembersOf { faction }
        }
        InstitutionalClaim::SupportDeclaration {
            supporter, office, ..
        } => InstitutionalBeliefKey::SupportFor { supporter, office },
    }
}

fn start_consult_record(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let payload = consult_record_payload(def, &instance.payload)?;
    let _ = validate_consult_record_context(txn, instance, payload)?;
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_consult_record(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_consult_record(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<worldwake_sim::CommitOutcome, ActionError> {
    let payload = consult_record_payload(def, &instance.payload)?;
    let record = validate_consult_record_context(txn, instance, payload)?;
    let record_data = txn
        .get_component_record_data(record)
        .cloned()
        .ok_or_else(|| ActionError::InternalError(format!("record {record} lacks RecordData")))?;
    let learned_tick = txn.tick();
    let learned_at = txn.effective_place(instance.actor);

    for entry in record_data
        .entries_newest_first()
        .take(record_data.max_entries_per_consult as usize)
    {
        txn.project_institutional_belief(
            instance.actor,
            institutional_belief_key(entry.claim),
            BelievedInstitutionalClaim {
                claim: entry.claim,
                source: InstitutionalKnowledgeSource::RecordConsultation {
                    record,
                    entry_id: entry.entry_id,
                },
                learned_tick,
                learned_at,
            },
        )
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }

    Ok(worldwake_sim::CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_consult_record(
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
    use worldwake_core::{
        build_believed_entity_state, CauseRef, ControlSource, EventLog, PerceptionSource,
        RecordData, RecordEntryId, RecordKind, Seed, Tick, WitnessData,
    };
    use worldwake_sim::{
        abort_action, get_affordances, start_action, tick_action, ActionExecutionAuthority,
        ActionExecutionContext, ActionInstance, ActionInstanceId, ActionPayload, ActionState,
        ExternalAbortReason, PerAgentBeliefView, TickOutcome,
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

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    fn test_rng() -> DeterministicRng {
        DeterministicRng::new(Seed([0x72; 32]))
    }

    fn test_belief_store(world: &World, actor: EntityId) -> worldwake_core::AgentBeliefStore {
        let mut store = world
            .get_component_agent_belief_store(actor)
            .cloned()
            .unwrap_or_default();
        for entity in world.entities() {
            if entity == actor {
                continue;
            }
            if let Some(state) = build_believed_entity_state(
                world,
                entity,
                Tick(u64::MAX),
                PerceptionSource::DirectObservation,
            ) {
                store.update_entity(entity, state);
            }
        }
        store
    }

    fn affordances_for(
        world: &World,
        actor: EntityId,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
    ) -> Vec<worldwake_sim::Affordance> {
        let beliefs = test_belief_store(world, actor);
        let view = PerAgentBeliefView::new(actor, world, &beliefs);
        get_affordances(&view, actor, defs, handlers)
    }

    fn setup_registries() -> (ActionDefRegistry, ActionHandlerRegistry, ActionDefId) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let id = register_consult_record_action(&mut defs, &mut handlers);
        (defs, handlers, id)
    }

    fn setup_world(
        consultation_ticks: u32,
        max_entries_per_consult: u32,
        consultation_speed_factor: u16,
    ) -> (World, EntityId, EntityId, EntityId) {
        let mut world = World::new(worldwake_core::build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let actor_place = places[0];
        let other_place = places[1];
        let (actor, record) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, actor_place).unwrap();
            txn.set_component_perception_profile(
                actor,
                worldwake_core::PerceptionProfile {
                    consultation_speed_factor: worldwake_core::Permille::new(
                        consultation_speed_factor,
                    )
                    .unwrap(),
                    ..worldwake_core::PerceptionProfile::default()
                },
            )
            .unwrap();
            let record = txn
                .create_record(RecordData {
                    record_kind: RecordKind::OfficeRegister,
                    home_place: actor_place,
                    issuer: actor,
                    consultation_ticks,
                    max_entries_per_consult,
                    entries: Vec::new(),
                    next_entry_id: 0,
                })
                .unwrap();
            commit_txn(txn);
            (actor, record)
        };
        (world, actor, record, other_place)
    }

    fn append_three_entries(
        world: &mut World,
        record: EntityId,
    ) -> (RecordEntryId, RecordEntryId, RecordEntryId) {
        let office = entity(80);
        let faction = entity(81);
        let supporter = entity(82);
        let candidate = entity(83);
        let member = entity(84);
        let mut txn = new_txn(world, 2);
        let office_entry = txn
            .append_record_entry(
                record,
                InstitutionalClaim::OfficeHolder {
                    office,
                    holder: Some(entity(90)),
                    effective_tick: Tick(1),
                },
            )
            .unwrap();
        let faction_entry = txn
            .append_record_entry(
                record,
                InstitutionalClaim::FactionMembership {
                    faction,
                    member,
                    active: true,
                    effective_tick: Tick(2),
                },
            )
            .unwrap();
        let support_entry = txn
            .append_record_entry(
                record,
                InstitutionalClaim::SupportDeclaration {
                    office,
                    supporter,
                    candidate: Some(candidate),
                    effective_tick: Tick(3),
                },
            )
            .unwrap();
        commit_txn(txn);
        (office_entry, faction_entry, support_entry)
    }

    fn start_consult_action(
        world: &mut World,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        actor: EntityId,
        record: EntityId,
    ) -> (
        ActionInstanceId,
        BTreeMap<ActionInstanceId, ActionInstance>,
        EventLog,
        DeterministicRng,
    ) {
        let affordance = affordances_for(world, actor, defs, handlers)
            .into_iter()
            .find(|affordance| affordance.bound_targets == vec![record])
            .unwrap();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();
        let mut next_instance_id = ActionInstanceId(1);
        let instance_id = start_action(
            &affordance,
            defs,
            handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_instance_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(5),
            },
        )
        .unwrap();
        (instance_id, active_actions, log, rng)
    }

    #[test]
    fn register_consult_record_action_creates_expected_definition() {
        let (defs, handlers, id) = setup_registries();
        let def = defs.get(id).unwrap();

        assert_eq!(def.name, "consult_record");
        assert_eq!(def.domain, worldwake_sim::ActionDomain::Social);
        assert_eq!(
            def.actor_constraints,
            vec![
                Constraint::ActorAlive,
                Constraint::ActorHasControl,
                Constraint::ActorNotInTransit,
            ]
        );
        assert_eq!(
            def.targets,
            vec![TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Record,
            }]
        );
        assert_eq!(
            def.duration,
            DurationExpr::ConsultRecord { target_index: 0 }
        );
        assert_eq!(def.interruptibility, Interruptibility::FreelyInterruptible);
        assert_eq!(
            def.causal_event_tags,
            BTreeSet::from([EventTag::Social, EventTag::WorldMutation])
        );
        assert!(handlers.get(def.handler).is_some());
    }

    #[test]
    fn consult_record_start_resolves_scaled_duration_and_keeps_payload_binding() {
        let (mut world, actor, record, _) = setup_world(8, 3, 375);
        let (defs, handlers, _) = setup_registries();
        let (instance_id, active_actions, _log, _rng) =
            start_consult_action(&mut world, &defs, &handlers, actor, record);
        let instance = active_actions.get(&instance_id).unwrap();

        assert_eq!(
            instance.remaining_duration,
            worldwake_sim::ActionDuration::new(3)
        );
        assert_eq!(
            instance.payload,
            ActionPayload::ConsultRecord(ConsultRecordActionPayload { record })
        );
        assert_eq!(instance.local_state, Some(ActionState::Empty));
    }

    #[test]
    fn consult_record_fails_to_start_when_record_is_not_colocated() {
        let (mut world, actor, record, other_place) = setup_world(4, 2, 500);
        let (defs, handlers, id) = setup_registries();
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_ground_location(record, other_place).unwrap();
            commit_txn(txn);
        }

        let affordance = worldwake_sim::Affordance {
            def_id: id,
            actor,
            bound_targets: vec![record],
            payload_override: Some(ActionPayload::ConsultRecord(ConsultRecordActionPayload {
                record,
            })),
            explanation: None,
        };
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();
        let mut next_instance_id = ActionInstanceId(1);

        let err = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_instance_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(5),
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::PreconditionFailed("TargetAtActorPlace(0)".to_string())
        );
    }

    #[test]
    fn consult_record_commit_projects_newest_entries_with_limit_and_preserves_record_data() {
        let (mut world, actor, record, _) = setup_world(4, 2, 500);
        let (office_entry, faction_entry, support_entry) = append_three_entries(&mut world, record);
        let before = world.get_component_record_data(record).cloned().unwrap();
        let (defs, handlers, _) = setup_registries();
        let (instance_id, mut active_actions, mut log, mut rng) =
            start_consult_action(&mut world, &defs, &handlers, actor, record);

        let first = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(6),
            },
        )
        .unwrap();
        assert_eq!(first, TickOutcome::Continuing);

        let second = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(7),
            },
        )
        .unwrap();
        assert!(matches!(second, TickOutcome::Committed { .. }));

        let store = world.get_component_agent_belief_store(actor).unwrap();
        assert!(!store
            .institutional_beliefs
            .contains_key(&InstitutionalBeliefKey::OfficeHolderOf { office: entity(80) }));

        let faction_beliefs = store
            .institutional_beliefs
            .get(&InstitutionalBeliefKey::FactionMembersOf {
                faction: entity(81),
            })
            .unwrap();
        assert_eq!(faction_beliefs.len(), 1);
        assert_eq!(
            faction_beliefs[0].source,
            InstitutionalKnowledgeSource::RecordConsultation {
                record,
                entry_id: faction_entry,
            }
        );

        let support_beliefs = store
            .institutional_beliefs
            .get(&InstitutionalBeliefKey::SupportFor {
                supporter: entity(82),
                office: entity(80),
            })
            .unwrap();
        assert_eq!(support_beliefs.len(), 1);
        assert_eq!(
            support_beliefs[0].source,
            InstitutionalKnowledgeSource::RecordConsultation {
                record,
                entry_id: support_entry,
            }
        );
        assert_eq!(support_beliefs[0].learned_tick, Tick(7));
        assert_eq!(support_beliefs[0].learned_at, world.effective_place(actor));

        let after = world.get_component_record_data(record).cloned().unwrap();
        assert_eq!(after, before);
        assert_ne!(office_entry, faction_entry);
    }

    #[test]
    fn consult_record_abort_before_commit_projects_no_beliefs() {
        let (mut world, actor, record, _) = setup_world(4, 3, 500);
        let _ = append_three_entries(&mut world, record);
        let (defs, handlers, _) = setup_registries();
        let (instance_id, mut active_actions, mut log, mut rng) =
            start_consult_action(&mut world, &defs, &handlers, actor, record);

        abort_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(6),
            },
            ExternalAbortReason::Other,
        )
        .unwrap();

        let store = world.get_component_agent_belief_store(actor).unwrap();
        assert!(store.institutional_beliefs.is_empty());
    }
}
