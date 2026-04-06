use std::collections::BTreeSet;
use std::num::NonZeroU32;

use worldwake_core::{
    ActionDefId, BodyCostPerTick, EntityId, EntityKind, EventLog, EventTag, ExpectationId,
    ExpectationRecord, ExpectationState, ViolationKind, VisibilitySpec, World, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerId,
    ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress, ActionState,
    CommitOutcome, Constraint, DeterministicRng, DurationExpr, Interruptibility,
    PerAgentBeliefView, Precondition, ReportMissingActionPayload, RuntimeBeliefView, TargetSpec,
};

pub fn register_report_missing_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(
            start_report_missing,
            tick_report_missing,
            commit_report_missing,
            abort_report_missing,
        )
        .with_affordance_payloads(enumerate_report_missing_payloads)
        .with_payload_override_validator(validate_report_missing_payload_override)
        .with_authoritative_payload_validator(validate_report_missing_payload_authoritatively),
    );
    let id = ActionDefId(defs.len() as u32);
    defs.register(report_missing_action_def(id, handler))
}

fn report_missing_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "report_missing".to_string(),
        domain: worldwake_core::ActionDomain::Social,
        actor_constraints: vec![Constraint::ActorAlive, Constraint::ActorNotIncapacitated],
        targets: vec![TargetSpec::ActorPlace],
        preconditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Place,
            },
        ],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::Fixed(NonZeroU32::new(2).unwrap()),
        body_cost_per_tick: BodyCostPerTick::zero(),
        attention_cost: worldwake_core::Permille::ZERO,
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Place,
            },
        ],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([
            EventTag::Social,
            EventTag::Discovery,
            EventTag::WorldMutation,
        ]),
        payload: ActionPayload::None,
        handler,
    }
}

fn report_missing_payload(payload: &ActionPayload) -> Result<&ReportMissingActionPayload, String> {
    payload
        .as_report_missing()
        .ok_or_else(|| "report_missing action requires report_missing payload".to_string())
}

fn reportable_expectation(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    expectation_id: ExpectationId,
) -> Option<ExpectationRecord> {
    let record = view
        .expectation_store(actor)?
        .records
        .get(&expectation_id)
        .copied()?;
    if record.owner != actor || record.state != ExpectationState::Overdue {
        return None;
    }

    let violation = ViolationKind::EntityMissing {
        entity: record.subject,
        expected_place: record.expected_place,
    };
    if view
        .active_violation_records(actor)
        .into_iter()
        .any(|existing| existing.kind == violation)
    {
        return None;
    }

    Some(record)
}

fn enumerate_report_missing_payloads(
    _def: &ActionDef,
    actor: EntityId,
    _targets: &[EntityId],
    view: &dyn RuntimeBeliefView,
) -> Vec<ActionPayload> {
    if view.violation_disposition_profile(actor).is_none() {
        return Vec::new();
    }
    let Some(store) = view.expectation_store(actor) else {
        return Vec::new();
    };

    store.records
        .values()
        .filter(|record| reportable_expectation(view, actor, record.id).is_some())
        .map(|record| {
            ActionPayload::ReportMissing(ReportMissingActionPayload {
                expectation_id: record.id,
            })
        })
        .collect()
}

fn validate_report_missing_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    _targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn RuntimeBeliefView,
) -> bool {
    if view.violation_disposition_profile(actor).is_none() {
        return false;
    }
    let Some(payload) = payload.as_report_missing() else {
        return false;
    };

    reportable_expectation(view, actor, payload.expectation_id).is_some()
}

fn validate_report_missing_payload_authoritatively(
    _def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    _targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    if world
        .get_component_violation_disposition_profile(actor)
        .is_none()
    {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} lacks ViolationDispositionProfile"
        )));
    }
    let payload = report_missing_payload(payload).map_err(ActionError::PreconditionFailed)?;
    let view = PerAgentBeliefView::from_world(actor, world);
    if reportable_expectation(&view, actor, payload.expectation_id).is_none() {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} cannot report overdue expectation {}",
            payload.expectation_id
        )));
    }
    Ok(())
}

fn start_report_missing(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let payload = report_missing_payload(&instance.payload).map_err(ActionError::PreconditionFailed)?;
    let view = PerAgentBeliefView::from_world(instance.actor, txn);
    if reportable_expectation(&view, instance.actor, payload.expectation_id).is_none() {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {} cannot report overdue expectation {}",
            instance.actor, payload.expectation_id
        )));
    }
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_report_missing(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_report_missing(
    _def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = report_missing_payload(&instance.payload).map_err(ActionError::PreconditionFailed)?;
    let actor = instance.actor;
    let profile = txn
        .get_component_violation_disposition_profile(actor)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "actor {actor} lacks ViolationDispositionProfile"
            ))
        })?;
    let view = PerAgentBeliefView::from_world(actor, txn);
    let record = reportable_expectation(&view, actor, payload.expectation_id).ok_or_else(|| {
        ActionError::PreconditionFailed(format!(
            "actor {actor} cannot report overdue expectation {} at commit",
            payload.expectation_id
        ))
    })?;

    let mut memory = txn.get_component_violation_memory(actor).cloned().unwrap_or_default();
    memory.record(
        ViolationKind::EntityMissing {
            entity: record.subject,
            expected_place: record.expected_place,
        },
        txn.tick(),
        profile.violation_memory_retention_ticks,
    );
    txn.set_component_violation_memory(actor, memory)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;

    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_report_missing(
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
    use worldwake_core::{
        CauseRef, ControlSource, ExpectationBasis, ExpectationStore, Seed, Tick,
        ViolationDispositionProfile, ViolationMemory, build_prototype_world,
    };
    use worldwake_sim::{
        ActionExecutionAuthority, ActionHandlerRegistry, ActionInstanceId, Affordance,
        TickOutcome, get_affordances, start_action, tick_action,
    };

    fn pm(value: u16) -> worldwake_core::Permille {
        worldwake_core::Permille::new(value).unwrap()
    }

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            worldwake_core::WitnessData::default(),
        )
    }

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    fn set_violation_profile(world: &mut World, actor: EntityId, retention: u32) {
        let mut txn = new_txn(world, 1);
        txn.set_component_violation_disposition_profile(
            actor,
            ViolationDispositionProfile {
                investigation_duration_ticks: nz(3),
                violation_memory_retention_ticks: retention,
                investigation_motive_weight: pm(400),
                ownership_motive_bonus: pm(200),
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    fn seed_expectation(
        world: &mut World,
        actor: EntityId,
        subject: EntityId,
        expected_place: EntityId,
        state: ExpectationState,
    ) -> ExpectationId {
        let mut txn = new_txn(world, 1);
        let mut store = txn
            .get_component_expectation_store(actor)
            .cloned()
            .unwrap_or_else(ExpectationStore::default);
        let id = ExpectationId(store.records.len() as u64 + 1);
        store.records.insert(
            id,
            ExpectationRecord {
                id,
                owner: actor,
                subject,
                expected_place,
                deadline_tick: Tick(5),
                grace_ticks: 1,
                basis: ExpectationBasis::RoutineReturn,
                state,
                created_tick: Tick(1),
            },
        );
        txn.set_component_expectation_store(actor, store).unwrap();
        commit_txn(txn);
        id
    }

    fn setup_fixture() -> (World, EntityId, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = worldwake_core::prototype_place_entity(worldwake_core::PrototypePlace::VillageSquare);
        let actor;
        let subject;
        {
            let mut txn = new_txn(&mut world, 1);
            actor = txn.create_agent("Reporter", ControlSource::Ai).unwrap();
            subject = txn.create_agent("Missing", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(subject, place).unwrap();
            commit_txn(txn);
        }
        set_violation_profile(&mut world, actor, 12);
        (world, actor, subject, place)
    }

    fn setup_registries() -> (ActionDefRegistry, ActionHandlerRegistry, ActionDefId) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let def_id = register_report_missing_action(&mut defs, &mut handlers);
        (defs, handlers, def_id)
    }

    fn run_action_to_completion(
        world: &mut World,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        affordance: &Affordance,
    ) -> TickOutcome {
        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([7; 32]));
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(1)),
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(2)),
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(3)),
        )
        .unwrap()
    }

    #[test]
    fn report_missing_affordance_enumerates_overdue_expectations() {
        let (mut world, actor, subject, place) = setup_fixture();
        let expectation_id =
            seed_expectation(&mut world, actor, subject, place, ExpectationState::Overdue);
        let (defs, handlers, def_id) = setup_registries();
        let view = PerAgentBeliefView::from_world(actor, &world);

        let affordances = get_affordances(&view, actor, &defs, &handlers);
        let affordance = affordances
            .iter()
            .find(|affordance| affordance.def_id == def_id)
            .expect("overdue expectation should produce report_missing affordance");

        assert_eq!(affordance.bound_targets, vec![place]);
        assert_eq!(
            affordance.payload_override,
            Some(ActionPayload::ReportMissing(ReportMissingActionPayload {
                expectation_id,
            }))
        );
    }

    #[test]
    fn report_missing_commit_records_entity_missing_violation() {
        let (mut world, actor, subject, place) = setup_fixture();
        let expectation_id =
            seed_expectation(&mut world, actor, subject, place, ExpectationState::Overdue);
        let (defs, handlers, def_id) = setup_registries();
        let affordance = Affordance {
            def_id,
            actor,
            bound_targets: vec![place],
            payload_override: Some(ActionPayload::ReportMissing(ReportMissingActionPayload {
                expectation_id,
            })),
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };

        let outcome = run_action_to_completion(&mut world, &defs, &handlers, &affordance);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));

        let memory = world
            .get_component_violation_memory(actor)
            .expect("commit should create ViolationMemory");
        assert!(memory.violations.iter().any(|record| {
            record.kind
                == ViolationKind::EntityMissing {
                    entity: subject,
                    expected_place: place,
                }
        }));
    }

    #[test]
    fn report_missing_skips_active_expectations() {
        let (mut world, actor, subject, place) = setup_fixture();
        seed_expectation(&mut world, actor, subject, place, ExpectationState::Active);
        let (defs, handlers, def_id) = setup_registries();
        let view = PerAgentBeliefView::from_world(actor, &world);

        let affordances = get_affordances(&view, actor, &defs, &handlers);
        assert!(
            !affordances.iter().any(|affordance| affordance.def_id == def_id),
            "active expectations should not produce report_missing affordances"
        );
    }

    #[test]
    fn report_missing_skips_already_recorded_missing_violation() {
        let (mut world, actor, subject, place) = setup_fixture();
        seed_expectation(&mut world, actor, subject, place, ExpectationState::Overdue);
        let mut txn = new_txn(&mut world, 2);
        let mut memory = ViolationMemory::default();
        memory.record(
            ViolationKind::EntityMissing {
                entity: subject,
                expected_place: place,
            },
            Tick(2),
            12,
        );
        txn.set_component_violation_memory(actor, memory).unwrap();
        commit_txn(txn);
        let (defs, handlers, def_id) = setup_registries();
        let view = PerAgentBeliefView::from_world(actor, &world);

        let affordances = get_affordances(&view, actor, &defs, &handlers);
        assert!(
            !affordances.iter().any(|affordance| affordance.def_id == def_id),
            "duplicate active EntityMissing records should suppress report_missing affordances"
        );
    }

    #[test]
    fn authoritative_validation_rejects_non_overdue_payload() {
        let (mut world, actor, subject, place) = setup_fixture();
        let expectation_id =
            seed_expectation(&mut world, actor, subject, place, ExpectationState::Active);
        let (defs, handlers, def_id) = setup_registries();
        let def = defs.get(def_id).unwrap();
        let handler = handlers.get(def.handler).unwrap();

        let validation = (handler.authoritative_payload_is_valid)(
            def,
            &defs,
            actor,
            &[place],
            &ActionPayload::ReportMissing(ReportMissingActionPayload { expectation_id }),
            &world,
        );

        assert!(validation.is_err());
    }
}
