use std::collections::BTreeSet;
use std::num::NonZeroU32;

use worldwake_core::{
    ActionDefId, BodyCostPerTick, Discrepancy, EntityId, EntityKind, EventLog, EventTag,
    ExpectationId, ExpectationOutcome, ExpectationRecord, ExpectationState, ExpectationStore,
    InstitutionalClaim, LastSeenMemory, LastSeenProvenance, LastSeenRecord, RecordData,
    RecordEntryId, RecordKind, ViolationKind, VisibilitySpec, World, WorldTxn,
    institutional::MissingPersonReportStatus, is_incapacitated,
};
use worldwake_sim::{
    AbortReason, ActionAbortRequestReason, ActionDef, ActionDefRegistry, ActionError,
    ActionHandler, ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload,
    ActionProgress, ActionState, CommitOutcome, Constraint, DeterministicRng, DurationExpr,
    EffectEvaluationContext, EffectMode, EffectSink, EffectStep, Interruptibility,
    PayloadEntityRole, PerAgentBeliefView, Precondition, ReportFoundActionPayload,
    ReportMissingActionPayload, RuntimeBeliefView, TargetSpec, apply_effects_with_context,
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

pub fn register_report_found_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(
            start_report_found,
            tick_report_found,
            commit_report_found,
            abort_report_found,
        )
        .with_affordance_payloads(enumerate_report_found_payloads)
        .with_payload_override_validator(validate_report_found_payload_override)
        .with_authoritative_payload_validator(validate_report_found_payload_authoritatively),
    );
    let id = ActionDefId(defs.len() as u32);
    defs.register(report_found_action_def(id, handler))
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
        binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
        guard_template: None,
        expectation_template: vec![],
        effect_schema: worldwake_sim::EffectSchema {
            preconditions: vec![],
            steps: vec![EffectStep::ReportMissing],
        },
    }
}

fn report_found_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    let preconditions = vec![
        Precondition::ActorAlive,
        Precondition::TargetExists(0),
        Precondition::TargetAtActorPlace(0),
    ];

    ActionDef {
        id,
        name: "report_found".to_string(),
        domain: worldwake_core::ActionDomain::Social,
        actor_constraints: vec![Constraint::ActorAlive, Constraint::ActorNotIncapacitated],
        targets: vec![TargetSpec::EntityAtActorPlaceAnyOf {
            kinds: [EntityKind::Agent, EntityKind::Record],
        }],
        preconditions: preconditions.clone(),
        reservation_requirements: Vec::new(),
        duration: DurationExpr::Fixed(NonZeroU32::new(2).unwrap()),
        body_cost_per_tick: BodyCostPerTick::zero(),
        attention_cost: worldwake_core::Permille::ZERO,
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: preconditions,
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([
            EventTag::Social,
            EventTag::Discovery,
            EventTag::WorldMutation,
        ]),
        payload: ActionPayload::None,
        handler,
        binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
        guard_template: None,
        expectation_template: vec![],
        effect_schema: worldwake_sim::EffectSchema {
            preconditions: vec![],
            steps: vec![EffectStep::ReportFound],
        },
    }
}

fn report_missing_payload(payload: &ActionPayload) -> Result<&ReportMissingActionPayload, String> {
    payload
        .as_report_missing()
        .ok_or_else(|| "report_missing action requires report_missing payload".to_string())
}

fn report_found_payload(payload: &ActionPayload) -> Result<&ReportFoundActionPayload, String> {
    payload
        .as_report_found()
        .ok_or_else(|| "report_found action requires report_found payload".to_string())
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

#[derive(Clone, Copy)]
struct ReportableFoundExpectation {
    subject: EntityId,
    outcome: ExpectationOutcome,
    last_seen: LastSeenRecord,
}

#[derive(Clone, Copy)]
enum ReportFoundTarget {
    Agent(EntityId),
    OfficeRegister(EntityId),
}

fn target_has_overdue_expectation_for_subject(
    store: Option<&ExpectationStore>,
    target: EntityId,
    subject: EntityId,
) -> bool {
    store.is_some_and(|store| {
        store.records.values().any(|record| {
            record.owner == target
                && record.subject == subject
                && record.state == ExpectationState::Overdue
        })
    })
}

fn reportable_found_expectation(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    expectation_id: ExpectationId,
) -> Option<ReportableFoundExpectation> {
    let record = view
        .expectation_store(actor)?
        .records
        .get(&expectation_id)
        .copied()?;
    if record.owner != actor {
        return None;
    }
    let outcome = match record.state {
        ExpectationState::Resolved {
            outcome:
                outcome @ (ExpectationOutcome::FoundSafe { .. }
                | ExpectationOutcome::FoundWounded { .. }
                | ExpectationOutcome::FoundDead { .. }),
        } => outcome,
        ExpectationState::Active
        | ExpectationState::Overdue
        | ExpectationState::Expired
        | ExpectationState::Resolved { .. } => return None,
    };
    let last_seen = view
        .last_seen_memory(actor)?
        .records
        .get(&record.subject)
        .copied()?;
    let found_place = match outcome {
        ExpectationOutcome::FoundSafe { at_place }
        | ExpectationOutcome::FoundWounded { at_place }
        | ExpectationOutcome::FoundDead { at_place } => at_place,
        ExpectationOutcome::Fulfilled
        | ExpectationOutcome::NotFound
        | ExpectationOutcome::ReturnedLate => unreachable!("filtered above"),
    };
    (last_seen.place == found_place).then_some(ReportableFoundExpectation {
        subject: record.subject,
        outcome,
        last_seen,
    })
}

fn office_register_at_place(
    txn: &WorldTxn<'_>,
    place: EntityId,
) -> Result<Option<EntityId>, ActionError> {
    let matches = txn
        .query_record_data()
        .filter_map(|(entity, record)| {
            (record.home_place == place && record.record_kind == RecordKind::OfficeRegister)
                .then_some(entity)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [record] => Ok(Some(*record)),
        _ => Err(ActionError::InternalError(format!(
            "multiple OfficeRegister records at place {place}"
        ))),
    }
}

fn active_missing_person_entry(
    record_data: &RecordData,
    subject: EntityId,
) -> Option<RecordEntryId> {
    record_data
        .active_entries()
        .into_iter()
        .find_map(|entry| match entry.claim {
            InstitutionalClaim::MissingPersonStatus {
                subject: claim_subject,
                ..
            } if claim_subject == subject => Some(entry.entry_id),
            _ => None,
        })
}

fn missing_person_status_for_missing(record: &ExpectationRecord) -> MissingPersonReportStatus {
    MissingPersonReportStatus::Missing {
        expected_place: record.expected_place,
    }
}

fn missing_person_status_for_found(outcome: ExpectationOutcome) -> MissingPersonReportStatus {
    match outcome {
        ExpectationOutcome::FoundSafe { at_place } => {
            MissingPersonReportStatus::FoundSafe { at_place }
        }
        ExpectationOutcome::FoundWounded { at_place } => {
            MissingPersonReportStatus::FoundWounded { at_place }
        }
        ExpectationOutcome::FoundDead { at_place } => {
            MissingPersonReportStatus::FoundDead { at_place }
        }
        ExpectationOutcome::Fulfilled
        | ExpectationOutcome::NotFound
        | ExpectationOutcome::ReturnedLate => unreachable!("report_found only uses found outcomes"),
    }
}

fn write_missing_person_status_claim(
    txn: &mut WorldTxn<'_>,
    office_register: EntityId,
    reporter: EntityId,
    subject: EntityId,
    status: MissingPersonReportStatus,
) -> Result<(), ActionError> {
    let record_data = txn
        .get_component_record_data(office_register)
        .cloned()
        .ok_or_else(|| {
            ActionError::InternalError(format!("record {office_register} lacks RecordData"))
        })?;
    if record_data.record_kind != RecordKind::OfficeRegister {
        return Err(ActionError::PreconditionFailed(format!(
            "record {office_register} is not an OfficeRegister"
        )));
    }

    let claim = InstitutionalClaim::MissingPersonStatus {
        subject,
        reporter,
        status,
        effective_tick: txn.tick(),
    };
    if active_missing_person_entry(&record_data, subject)
        .and_then(|entry_id| {
            record_data
                .entries
                .iter()
                .find(|entry| entry.entry_id == entry_id)
                .map(|entry| entry.claim)
        })
        .is_some_and(|existing| existing == claim)
    {
        return Ok(());
    }

    match active_missing_person_entry(&record_data, subject) {
        Some(entry_id) => txn
            .supersede_record_entry(office_register, entry_id, claim)
            .map_err(|error| ActionError::InternalError(error.to_string()))?,
        None => txn
            .append_record_entry(office_register, claim)
            .map_err(|error| ActionError::InternalError(error.to_string()))?,
    };
    Ok(())
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

    store
        .records
        .values()
        .filter(|record| reportable_expectation(view, actor, record.id).is_some())
        .map(|record| {
            ActionPayload::ReportMissing(ReportMissingActionPayload {
                expectation_id: record.id,
            })
        })
        .collect()
}

fn enumerate_report_found_payloads(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    view: &dyn RuntimeBeliefView,
) -> Vec<ActionPayload> {
    let Some(target) = targets.first().copied() else {
        return Vec::new();
    };
    let Some(record_data) = view.record_data(target) else {
        return Vec::new();
    };
    if record_data.record_kind != RecordKind::OfficeRegister {
        return Vec::new();
    }
    let Some(store) = view.expectation_store(actor) else {
        return Vec::new();
    };

    store
        .records
        .values()
        .filter(|record| {
            target != actor && reportable_found_expectation(view, actor, record.id).is_some()
        })
        .map(|record| {
            ActionPayload::ReportFound(ReportFoundActionPayload {
                target,
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

fn validate_report_found_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(target) = targets.first().copied() else {
        return false;
    };
    let Some(record_data) = view.record_data(target) else {
        return false;
    };
    let Some(payload) = payload.as_report_found() else {
        return false;
    };
    if payload.target != target {
        return false;
    }
    if record_data.record_kind != RecordKind::OfficeRegister {
        return false;
    }

    target != actor && reportable_found_expectation(view, actor, payload.expectation_id).is_some()
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

fn validate_report_found_payload_authoritatively(
    _def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let Some(target) = targets.first().copied() else {
        return Err(ActionError::InvalidTarget(actor));
    };
    let payload = report_found_payload(payload).map_err(ActionError::PreconditionFailed)?;
    if payload.target != target {
        return Err(ActionError::PreconditionFailed(format!(
            "report_found payload target {} does not match bound target {}",
            payload.target, target
        )));
    }
    let report_target = validate_report_found_authoritative_context(world, actor, target)?;
    let view = PerAgentBeliefView::from_world(actor, world);
    let Some(report) = reportable_found_expectation(&view, actor, payload.expectation_id) else {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} cannot report found expectation {} to target {target}",
            payload.expectation_id
        )));
    };
    if let ReportFoundTarget::Agent(target_agent) = report_target
        && !target_has_overdue_expectation_for_subject(
            world.get_component_expectation_store(target_agent),
            target_agent,
            report.subject,
        )
    {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target_agent} lacks overdue expectation for subject {}",
            report.subject
        )));
    }
    Ok(())
}

fn is_authoritatively_incapacitated(world: &World, entity: EntityId) -> bool {
    world
        .get_component_wound_list(entity)
        .zip(world.get_component_combat_profile(entity))
        .is_some_and(|(wounds, profile)| is_incapacitated(wounds, profile))
}

fn validate_report_found_authoritative_context(
    world: &World,
    actor: EntityId,
    target: EntityId,
) -> Result<ReportFoundTarget, ActionError> {
    if world.effective_place(actor) != world.effective_place(target) {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} is not colocated with actor {actor}"
        )));
    }
    match world.entity_kind(target) {
        Some(EntityKind::Agent) => {
            if target == actor {
                return Err(ActionError::PreconditionFailed(format!(
                    "actor {actor} cannot report_found to self"
                )));
            }
            if world.get_component_dead_at(target).is_some() {
                return Err(ActionError::PreconditionFailed(format!(
                    "target {target} is not alive"
                )));
            }
            if is_authoritatively_incapacitated(world, target) {
                return Err(ActionError::PreconditionFailed(format!(
                    "target {target} is incapacitated"
                )));
            }
            Ok(ReportFoundTarget::Agent(target))
        }
        Some(EntityKind::Record) => {
            let Some(record_data) = world.get_component_record_data(target) else {
                return Err(ActionError::InternalError(format!(
                    "record {target} lacks RecordData"
                )));
            };
            if record_data.record_kind != RecordKind::OfficeRegister {
                return Err(ActionError::PreconditionFailed(format!(
                    "record {target} is not an OfficeRegister"
                )));
            }
            Ok(ReportFoundTarget::OfficeRegister(target))
        }
        Some(kind) => Err(ActionError::PreconditionFailed(format!(
            "target {target} has unsupported kind {kind:?}"
        ))),
        None => Err(ActionError::InvalidTarget(target)),
    }
}

fn validate_report_found_context(
    txn: &WorldTxn<'_>,
    instance: &ActionInstance,
    payload: &ReportFoundActionPayload,
) -> Result<ReportFoundTarget, ActionError> {
    let target = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    if payload.target != target {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::PayloadEntityMismatch {
                role: PayloadEntityRole::Counterparty,
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
    match txn.entity_kind(target) {
        Some(EntityKind::Agent) => {
            if target == instance.actor {
                return Err(ActionError::PreconditionFailed(format!(
                    "actor {} cannot report_found to self",
                    instance.actor
                )));
            }
            if txn.get_component_dead_at(target).is_some() {
                return Err(ActionError::AbortRequested(
                    ActionAbortRequestReason::TargetNotAlive { target },
                ));
            }
            if is_authoritatively_incapacitated(txn, target) {
                return Err(ActionError::AbortRequested(
                    ActionAbortRequestReason::TargetIncapacitated { target },
                ));
            }
            Ok(ReportFoundTarget::Agent(target))
        }
        Some(EntityKind::Record) => {
            let Some(record_data) = txn.get_component_record_data(target) else {
                return Err(ActionError::InternalError(format!(
                    "record {target} lacks RecordData"
                )));
            };
            if record_data.record_kind != RecordKind::OfficeRegister {
                return Err(ActionError::PreconditionFailed(format!(
                    "record {target} is not an OfficeRegister"
                )));
            }
            Ok(ReportFoundTarget::OfficeRegister(target))
        }
        Some(kind) => Err(ActionError::PreconditionFailed(format!(
            "target {target} has unsupported kind {kind:?}"
        ))),
        None => Err(ActionError::InvalidTarget(target)),
    }
}

fn ensure_agent_target_has_overdue_expectation(
    txn: &WorldTxn<'_>,
    target: EntityId,
    subject: EntityId,
    at_commit: bool,
) -> Result<(), ActionError> {
    if target_has_overdue_expectation_for_subject(
        txn.get_component_expectation_store(target),
        target,
        subject,
    ) {
        return Ok(());
    }

    let suffix = if at_commit { " at commit" } else { "" };
    Err(ActionError::PreconditionFailed(format!(
        "target {target} lacks overdue expectation for subject {subject}{suffix}"
    )))
}

fn start_report_missing(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let payload =
        report_missing_payload(&instance.payload).map_err(ActionError::PreconditionFailed)?;
    let view = PerAgentBeliefView::from_world(instance.actor, txn);
    if reportable_expectation(&view, instance.actor, payload.expectation_id).is_none() {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {} cannot report overdue expectation {}",
            instance.actor, payload.expectation_id
        )));
    }
    Ok(Some(ActionState::Empty))
}

fn start_report_found(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let payload =
        report_found_payload(&instance.payload).map_err(ActionError::PreconditionFailed)?;
    let target = validate_report_found_context(txn, instance, payload)?;
    let view = PerAgentBeliefView::from_world(instance.actor, txn);
    let Some(report) = reportable_found_expectation(&view, instance.actor, payload.expectation_id)
    else {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {} cannot report found expectation {} to target {}",
            instance.actor, payload.expectation_id, payload.target
        )));
    };
    if let ReportFoundTarget::Agent(target_agent) = target {
        ensure_agent_target_has_overdue_expectation(txn, target_agent, report.subject, false)?;
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

#[allow(clippy::unnecessary_wraps)]
fn tick_report_found(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    let payload =
        report_found_payload(&instance.payload).map_err(ActionError::PreconditionFailed)?;
    validate_report_found_context(txn, instance, payload)?;
    Ok(ActionProgress::Continue)
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

fn resolve_overdue_expectations_for_subject(
    store: &mut ExpectationStore,
    owner: EntityId,
    subject: EntityId,
    outcome: ExpectationOutcome,
) {
    for record in store.records.values_mut() {
        if record.owner == owner
            && record.subject == subject
            && record.state == ExpectationState::Overdue
        {
            record.state = ExpectationState::Resolved { outcome };
        }
    }
}

fn resolve_missing_violations_for_subject(
    txn: &mut WorldTxn<'_>,
    target: EntityId,
    subject: EntityId,
) -> Result<(), ActionError> {
    let Some(profile) = txn.get_component_violation_disposition_profile(target) else {
        return Ok(());
    };
    let Some(mut memory) = txn.get_component_violation_memory(target).cloned() else {
        return Ok(());
    };
    let matching_ids = memory
        .unresolved_records(txn.tick())
        .into_iter()
        .filter_map(|record| match record.kind {
            ViolationKind::EntityMissing { entity, .. } if entity == subject => Some(record.id),
            ViolationKind::EntityMissing { .. }
            | ViolationKind::SupplyDepleted { .. }
            | ViolationKind::EntityDead { .. }
            | ViolationKind::SuspectedTheft { .. } => None,
        })
        .collect::<Vec<_>>();
    if matching_ids.is_empty() {
        return Ok(());
    }

    for id in matching_ids {
        let _ = memory.resolve_id(id, txn.tick(), profile.violation_memory_retention_ticks);
    }
    txn.set_component_violation_memory(target, memory)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    Ok(())
}

fn commit_report_missing(
    def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    apply_report_effect_schema(def, instance, txn)
}

fn commit_report_found(
    def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    apply_report_effect_schema(def, instance, txn)
}

struct ReportEffectSink<'txn, 'world, 'instance> {
    txn: &'txn mut WorldTxn<'world>,
    instance: &'instance ActionInstance,
    action_error: Option<ActionError>,
}

impl ReportEffectSink<'_, '_, '_> {
    fn record_error(&mut self, error: ActionError) -> Discrepancy {
        self.action_error = Some(error);
        Discrepancy::PartialExecutionDrift
    }

    fn take_error(self, discrepancy: Discrepancy) -> ActionError {
        self.action_error.unwrap_or_else(|| {
            ActionError::PreconditionFailed(format!("effect schema failed: {discrepancy:?}"))
        })
    }
}

impl EffectSink for ReportEffectSink<'_, '_, '_> {
    fn check_precondition(
        &self,
        _precondition: &worldwake_sim::EffectPrecondition,
        _actor: EntityId,
        _targets: &[EntityId],
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn checkpoint(&mut self) -> usize {
        0
    }

    fn restore(&mut self, _checkpoint: usize) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn report_missing(
        &mut self,
        actor: EntityId,
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        if actor != self.instance.actor {
            return Err(self.record_error(ActionError::InvalidTarget(actor)));
        }
        apply_report_missing_commit(self.instance, self.txn)
            .map(|_| ())
            .map_err(|err| self.record_error(err))
    }

    fn report_found(
        &mut self,
        actor: EntityId,
        _payload: &ActionPayload,
    ) -> Result<(), Discrepancy> {
        if actor != self.instance.actor {
            return Err(self.record_error(ActionError::InvalidTarget(actor)));
        }
        apply_report_found_commit(self.instance, self.txn)
            .map(|_| ())
            .map_err(|err| self.record_error(err))
    }
}

fn apply_report_effect_schema(
    def: &ActionDef,
    instance: &ActionInstance,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let mut sink = ReportEffectSink {
        txn,
        instance,
        action_error: None,
    };
    match apply_effects_with_context(
        &def.effect_schema,
        EffectEvaluationContext {
            actor: instance.actor,
            targets: &instance.targets,
            payload: &instance.payload,
            action_def_id: def.id,
        },
        &mut sink,
        EffectMode::Authoritative,
    ) {
        Ok(_) => Ok(CommitOutcome::empty()),
        Err(discrepancy) => Err(sink.take_error(discrepancy)),
    }
}

fn apply_report_missing_commit(
    instance: &ActionInstance,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload =
        report_missing_payload(&instance.payload).map_err(ActionError::PreconditionFailed)?;
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

    let mut memory = txn
        .get_component_violation_memory(actor)
        .cloned()
        .unwrap_or_default();
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
    if let Some(office_register) = office_register_at_place(txn, record.expected_place)? {
        write_missing_person_status_claim(
            txn,
            office_register,
            actor,
            record.subject,
            missing_person_status_for_missing(&record),
        )?;
    }

    Ok(CommitOutcome::empty())
}

fn apply_report_found_commit(
    instance: &ActionInstance,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload =
        report_found_payload(&instance.payload).map_err(ActionError::PreconditionFailed)?;
    let target = validate_report_found_context(txn, instance, payload)?;
    let view = PerAgentBeliefView::from_world(instance.actor, txn);
    let report = reportable_found_expectation(&view, instance.actor, payload.expectation_id)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "actor {} cannot report found expectation {} to target {} at commit",
                instance.actor, payload.expectation_id, payload.target
            ))
        })?;
    match target {
        ReportFoundTarget::Agent(target_agent) => {
            ensure_agent_target_has_overdue_expectation(txn, target_agent, report.subject, true)?;

            let mut target_memory = txn
                .get_component_last_seen_memory(target_agent)
                .cloned()
                .unwrap_or_default();
            remember_last_seen(
                &mut target_memory,
                relay_last_seen_record(report.last_seen, instance.actor),
            );
            txn.set_component_last_seen_memory(target_agent, target_memory)
                .map_err(|error| ActionError::InternalError(error.to_string()))?;

            let mut target_store = txn
                .get_component_expectation_store(target_agent)
                .cloned()
                .unwrap_or_default();
            resolve_overdue_expectations_for_subject(
                &mut target_store,
                target_agent,
                report.subject,
                report.outcome,
            );
            txn.set_component_expectation_store(target_agent, target_store)
                .map_err(|error| ActionError::InternalError(error.to_string()))?;

            resolve_missing_violations_for_subject(txn, target_agent, report.subject)?;
        }
        ReportFoundTarget::OfficeRegister(office_register) => {
            write_missing_person_status_claim(
                txn,
                office_register,
                instance.actor,
                report.subject,
                missing_person_status_for_found(report.outcome),
            )?;
        }
    }

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

#[allow(clippy::unnecessary_wraps)]
fn abort_report_found(
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
        ViolationDispositionProfile, ViolationMemory, build_believed_entity_state,
        build_prototype_world,
    };
    use worldwake_sim::{
        ActionExecutionAuthority, ActionHandlerRegistry, ActionInstanceId, Affordance, TickOutcome,
        get_affordances, start_action, tick_action,
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
        let id = store.allocate_record(|id| ExpectationRecord {
            id,
            owner: actor,
            subject,
            expected_place,
            deadline_tick: Tick(5),
            grace_ticks: 1,
            basis: ExpectationBasis::RoutineReturn,
            state,
            created_tick: Tick(1),
        });
        txn.set_component_expectation_store(actor, store).unwrap();
        commit_txn(txn);
        id
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

    fn create_record(
        world: &mut World,
        place: EntityId,
        issuer: EntityId,
        kind: RecordKind,
        tick: u64,
    ) -> EntityId {
        let mut txn = new_txn(world, tick);
        let record = txn
            .create_record(RecordData {
                record_kind: kind,
                home_place: place,
                issuer,
                consultation_ticks: 4,
                max_entries_per_consult: 6,
                entries: Vec::new(),
                next_entry_id: 0,
            })
            .unwrap();
        commit_txn(txn);
        record
    }

    fn setup_fixture() -> (World, EntityId, EntityId, EntityId, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place =
            worldwake_core::prototype_place_entity(worldwake_core::PrototypePlace::VillageSquare);
        let other_place =
            worldwake_core::prototype_place_entity(worldwake_core::PrototypePlace::OrchardFarm);
        let actor;
        let listener;
        let subject;
        {
            let mut txn = new_txn(&mut world, 1);
            actor = txn.create_agent("Reporter", ControlSource::Ai).unwrap();
            listener = txn.create_agent("Listener", ControlSource::Ai).unwrap();
            subject = txn.create_agent("Missing", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(listener, place).unwrap();
            txn.set_ground_location(subject, place).unwrap();
            commit_txn(txn);
        }
        set_violation_profile(&mut world, actor, 12);
        set_violation_profile(&mut world, listener, 12);
        seed_entity_belief(&mut world, actor, listener, 2);
        (world, actor, listener, subject, place, other_place)
    }

    fn setup_registries() -> (
        ActionDefRegistry,
        ActionHandlerRegistry,
        ActionDefId,
        ActionDefId,
    ) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let report_missing_id = register_report_missing_action(&mut defs, &mut handlers);
        let report_found_id = register_report_found_action(&mut defs, &mut handlers);
        (defs, handlers, report_missing_id, report_found_id)
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
        let (mut world, actor, _listener, subject, place, _other_place) = setup_fixture();
        let expectation_id =
            seed_expectation(&mut world, actor, subject, place, ExpectationState::Overdue);
        let (defs, handlers, report_missing_id, _report_found_id) = setup_registries();
        let view = PerAgentBeliefView::from_world(actor, &world);

        let affordances = get_affordances(&view, actor, &defs, &handlers);
        let affordance = affordances
            .iter()
            .find(|affordance| affordance.def_id == report_missing_id)
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
        let (mut world, actor, _listener, subject, place, _other_place) = setup_fixture();
        let expectation_id =
            seed_expectation(&mut world, actor, subject, place, ExpectationState::Overdue);
        let (defs, handlers, report_missing_id, _report_found_id) = setup_registries();
        let affordance = Affordance {
            def_id: report_missing_id,
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
    fn report_missing_commit_writes_missing_person_claim_to_local_office_register() {
        let (mut world, actor, _listener, subject, place, _other_place) = setup_fixture();
        let office_register =
            create_record(&mut world, place, actor, RecordKind::OfficeRegister, 2);
        let expectation_id =
            seed_expectation(&mut world, actor, subject, place, ExpectationState::Overdue);
        let (defs, handlers, report_missing_id, _report_found_id) = setup_registries();
        let affordance = Affordance {
            def_id: report_missing_id,
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

        let record_data = world.get_component_record_data(office_register).unwrap();
        let entry = record_data
            .active_entries()
            .into_iter()
            .find(|entry| {
                matches!(
                    entry.claim,
                    InstitutionalClaim::MissingPersonStatus { subject: claim_subject, .. }
                        if claim_subject == subject
                )
            })
            .expect("office register should hold missing-person claim");
        assert_eq!(
            entry.claim,
            InstitutionalClaim::MissingPersonStatus {
                subject,
                reporter: actor,
                status: MissingPersonReportStatus::Missing {
                    expected_place: place,
                },
                effective_tick: Tick(3),
            }
        );
    }

    #[test]
    fn report_missing_commit_supersedes_existing_missing_person_claim_for_same_subject() {
        let (mut world, actor, _listener, subject, place, other_place) = setup_fixture();
        let office_register =
            create_record(&mut world, place, actor, RecordKind::OfficeRegister, 2);
        {
            let mut txn = new_txn(&mut world, 2);
            let _ = txn
                .append_record_entry(
                    office_register,
                    InstitutionalClaim::MissingPersonStatus {
                        subject,
                        reporter: actor,
                        status: MissingPersonReportStatus::FoundSafe {
                            at_place: other_place,
                        },
                        effective_tick: Tick(2),
                    },
                )
                .unwrap();
            commit_txn(txn);
        }
        let expectation_id =
            seed_expectation(&mut world, actor, subject, place, ExpectationState::Overdue);
        let (defs, handlers, report_missing_id, _report_found_id) = setup_registries();
        let affordance = Affordance {
            def_id: report_missing_id,
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

        let record_data = world.get_component_record_data(office_register).unwrap();
        let active_claims = record_data
            .active_entries()
            .into_iter()
            .filter(|entry| {
                matches!(
                    entry.claim,
                    InstitutionalClaim::MissingPersonStatus { subject: claim_subject, .. }
                        if claim_subject == subject
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(active_claims.len(), 1);
        assert_eq!(
            active_claims[0].claim,
            InstitutionalClaim::MissingPersonStatus {
                subject,
                reporter: actor,
                status: MissingPersonReportStatus::Missing {
                    expected_place: place,
                },
                effective_tick: Tick(3),
            }
        );
    }

    #[test]
    fn report_missing_skips_active_expectations() {
        let (mut world, actor, _listener, subject, place, _other_place) = setup_fixture();
        seed_expectation(&mut world, actor, subject, place, ExpectationState::Active);
        let (defs, handlers, report_missing_id, _report_found_id) = setup_registries();
        let view = PerAgentBeliefView::from_world(actor, &world);

        let affordances = get_affordances(&view, actor, &defs, &handlers);
        assert!(
            !affordances
                .iter()
                .any(|affordance| affordance.def_id == report_missing_id),
            "active expectations should not produce report_missing affordances"
        );
    }

    #[test]
    fn report_missing_skips_already_recorded_missing_violation() {
        let (mut world, actor, _listener, subject, place, _other_place) = setup_fixture();
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
        let (defs, handlers, report_missing_id, _report_found_id) = setup_registries();
        let view = PerAgentBeliefView::from_world(actor, &world);

        let affordances = get_affordances(&view, actor, &defs, &handlers);
        assert!(
            !affordances
                .iter()
                .any(|affordance| affordance.def_id == report_missing_id),
            "duplicate active EntityMissing records should suppress report_missing affordances"
        );
    }

    #[test]
    fn authoritative_validation_rejects_non_overdue_missing_payload() {
        let (mut world, actor, _listener, subject, place, _other_place) = setup_fixture();
        let expectation_id =
            seed_expectation(&mut world, actor, subject, place, ExpectationState::Active);
        let (defs, handlers, report_missing_id, _report_found_id) = setup_registries();
        let def = defs.get(report_missing_id).unwrap();
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

    #[test]
    fn report_found_affordance_enumerates_resolved_found_expectations_for_local_office_register() {
        let (mut world, actor, _listener, subject, place, other_place) = setup_fixture();
        let office_register =
            create_record(&mut world, place, actor, RecordKind::OfficeRegister, 2);
        seed_entity_belief(&mut world, actor, office_register, 3);
        let expectation_id = seed_expectation(
            &mut world,
            actor,
            subject,
            place,
            ExpectationState::Resolved {
                outcome: ExpectationOutcome::FoundSafe {
                    at_place: other_place,
                },
            },
        );
        seed_last_seen_record(
            &mut world,
            actor,
            LastSeenRecord {
                subject,
                place: other_place,
                observed_tick: Tick(6),
                source: actor,
                provenance: LastSeenProvenance::DirectObservation,
            },
            5,
        );
        let (defs, handlers, _report_missing_id, report_found_id) = setup_registries();
        let view = PerAgentBeliefView::from_world(actor, &world);

        let affordances = get_affordances(&view, actor, &defs, &handlers);
        assert!(affordances.iter().any(|affordance| {
            affordance.def_id == report_found_id
                && affordance.bound_targets == vec![office_register]
                && affordance.payload_override
                    == Some(ActionPayload::ReportFound(ReportFoundActionPayload {
                        target: office_register,
                        expectation_id,
                    }))
        }));
    }

    #[test]
    fn report_found_commit_relays_last_seen_and_resolves_listener_state() {
        let (mut world, actor, listener, subject, place, other_place) = setup_fixture();
        let expectation_id = seed_expectation(
            &mut world,
            actor,
            subject,
            place,
            ExpectationState::Resolved {
                outcome: ExpectationOutcome::FoundWounded {
                    at_place: other_place,
                },
            },
        );
        seed_expectation(
            &mut world,
            listener,
            subject,
            place,
            ExpectationState::Overdue,
        );
        seed_last_seen_record(
            &mut world,
            actor,
            LastSeenRecord {
                subject,
                place: other_place,
                observed_tick: Tick(7),
                source: actor,
                provenance: LastSeenProvenance::DirectObservation,
            },
            5,
        );
        let mut txn = new_txn(&mut world, 2);
        let mut violation_memory = ViolationMemory::default();
        let missing_violation_id = violation_memory.record(
            ViolationKind::EntityMissing {
                entity: subject,
                expected_place: place,
            },
            Tick(2),
            12,
        );
        txn.set_component_violation_memory(listener, violation_memory)
            .unwrap();
        commit_txn(txn);
        let (defs, handlers, _report_missing_id, report_found_id) = setup_registries();
        let affordance = Affordance {
            def_id: report_found_id,
            actor,
            bound_targets: vec![listener],
            payload_override: Some(ActionPayload::ReportFound(ReportFoundActionPayload {
                target: listener,
                expectation_id,
            })),
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };

        let outcome = run_action_to_completion(&mut world, &defs, &handlers, &affordance);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));

        let memory = world
            .get_component_last_seen_memory(listener)
            .expect("listener should keep last-seen memory");
        assert_eq!(
            memory.records.get(&subject),
            Some(&LastSeenRecord {
                subject,
                place: other_place,
                observed_tick: Tick(7),
                source: actor,
                provenance: LastSeenProvenance::Hearsay {
                    original_observer: actor,
                    chain_depth: 1,
                },
            })
        );

        let store = world
            .get_component_expectation_store(listener)
            .expect("listener should keep expectation store");
        assert!(store.records.values().any(|record| {
            record.subject == subject
                && record.state
                    == ExpectationState::Resolved {
                        outcome: ExpectationOutcome::FoundWounded {
                            at_place: other_place,
                        },
                    }
        }));

        let violation_memory = world
            .get_component_violation_memory(listener)
            .expect("listener should keep violation memory");
        assert!(
            violation_memory
                .unresolved_by_id(missing_violation_id, Tick(3))
                .is_none(),
            "listener missing violation should be resolved after report_found"
        );
    }

    #[test]
    fn report_found_commit_writes_found_status_to_office_register() {
        let (mut world, actor, _listener, subject, place, other_place) = setup_fixture();
        let office_register =
            create_record(&mut world, place, actor, RecordKind::OfficeRegister, 2);
        let expectation_id = seed_expectation(
            &mut world,
            actor,
            subject,
            place,
            ExpectationState::Resolved {
                outcome: ExpectationOutcome::FoundDead {
                    at_place: other_place,
                },
            },
        );
        seed_last_seen_record(
            &mut world,
            actor,
            LastSeenRecord {
                subject,
                place: other_place,
                observed_tick: Tick(7),
                source: actor,
                provenance: LastSeenProvenance::DirectObservation,
            },
            5,
        );
        let (defs, handlers, _report_missing_id, report_found_id) = setup_registries();
        let affordance = Affordance {
            def_id: report_found_id,
            actor,
            bound_targets: vec![office_register],
            payload_override: Some(ActionPayload::ReportFound(ReportFoundActionPayload {
                target: office_register,
                expectation_id,
            })),
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };

        let outcome = run_action_to_completion(&mut world, &defs, &handlers, &affordance);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));

        let record_data = world.get_component_record_data(office_register).unwrap();
        let entry = record_data
            .active_entries()
            .into_iter()
            .find(|entry| {
                matches!(
                    entry.claim,
                    InstitutionalClaim::MissingPersonStatus { subject: claim_subject, .. }
                        if claim_subject == subject
                )
            })
            .expect("office register should hold found-person claim");
        assert_eq!(
            entry.claim,
            InstitutionalClaim::MissingPersonStatus {
                subject,
                reporter: actor,
                status: MissingPersonReportStatus::FoundDead {
                    at_place: other_place,
                },
                effective_tick: Tick(3),
            }
        );
    }

    #[test]
    fn report_found_does_not_surface_listener_targets_without_local_office_register() {
        let (mut world, actor, listener, subject, place, other_place) = setup_fixture();
        let expectation_id = seed_expectation(
            &mut world,
            actor,
            subject,
            place,
            ExpectationState::Resolved {
                outcome: ExpectationOutcome::FoundDead {
                    at_place: other_place,
                },
            },
        );
        seed_last_seen_record(
            &mut world,
            actor,
            LastSeenRecord {
                subject,
                place: other_place,
                observed_tick: Tick(8),
                source: actor,
                provenance: LastSeenProvenance::DirectObservation,
            },
            5,
        );
        let (defs, handlers, _report_missing_id, report_found_id) = setup_registries();
        let view = PerAgentBeliefView::from_world(actor, &world);

        let affordances = get_affordances(&view, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .all(|affordance| affordance.def_id != report_found_id),
            "report_found should stay absent without a planner-visible OfficeRegister target"
        );
        assert!(!validate_report_found_payload_override(
            defs.get(report_found_id).unwrap(),
            actor,
            &[listener],
            &ActionPayload::ReportFound(ReportFoundActionPayload {
                target: listener,
                expectation_id,
            }),
            &view,
        ));
        let def = defs.get(report_found_id).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let validation = (handler.authoritative_payload_is_valid)(
            def,
            &defs,
            actor,
            &[listener],
            &ActionPayload::ReportFound(ReportFoundActionPayload {
                target: listener,
                expectation_id,
            }),
            &world,
        );
        assert!(validation.is_err());
    }

    #[test]
    fn authoritative_validation_rejects_found_payload_without_matching_last_seen_record() {
        let (mut world, actor, listener, subject, place, other_place) = setup_fixture();
        let expectation_id = seed_expectation(
            &mut world,
            actor,
            subject,
            place,
            ExpectationState::Resolved {
                outcome: ExpectationOutcome::FoundSafe {
                    at_place: other_place,
                },
            },
        );
        seed_expectation(
            &mut world,
            listener,
            subject,
            place,
            ExpectationState::Overdue,
        );
        seed_last_seen_record(
            &mut world,
            actor,
            LastSeenRecord {
                subject,
                place,
                observed_tick: Tick(4),
                source: actor,
                provenance: LastSeenProvenance::DirectObservation,
            },
            5,
        );
        let (defs, handlers, _report_missing_id, report_found_id) = setup_registries();
        let def = defs.get(report_found_id).unwrap();
        let handler = handlers.get(def.handler).unwrap();

        let validation = (handler.authoritative_payload_is_valid)(
            def,
            &defs,
            actor,
            &[listener],
            &ActionPayload::ReportFound(ReportFoundActionPayload {
                target: listener,
                expectation_id,
            }),
            &world,
        );

        assert!(validation.is_err());
    }
}
