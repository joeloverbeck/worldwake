use std::collections::BTreeSet;
use std::num::NonZeroU32;

use worldwake_core::{
    ActionDefId, BelievedInstitutionalClaim, BodyCostPerTick, CommodityKind, EligibilityRule,
    EntityId, EntityKind, EventTag, InstitutionalBeliefKey, InstitutionalClaim,
    InstitutionalKnowledgeSource, JusticeDispositionProfile, PunishmentKind, Quantity, RecordData,
    RecordEntryId, RecordKind, SocialObservation, SocialObservationDetail, TheftFacts, ViolationId,
    ViolationKind, VisibilitySpec, World, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionAbortRequestReason, ActionDef, ActionDefRegistry, ActionError,
    ActionHandler, ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload,
    ActionProgress, ActionState, AccuseActionPayload, CommitOutcome, Constraint,
    DeterministicRng, DurationExpr, Interruptibility, PerAgentBeliefView, Precondition,
    RuntimeBeliefView, TargetSpec,
};
use worldwake_sim::action_payload::PunishActionPayload;

pub fn register_accuse_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(start_accuse, tick_accuse, commit_accuse, abort_accuse)
            .with_affordance_targets(enumerate_accuse_targets)
            .with_affordance_payloads(enumerate_accuse_payloads)
            .with_payload_override_validator(validate_accuse_payload_override)
            .with_authoritative_payload_validator(validate_accuse_payload_authoritatively),
    );
    defs.register(accuse_action_def(ActionDefId(defs.len() as u32), handler))
}

pub fn register_fine_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(start_fine, tick_punishment, commit_fine, abort_punishment)
            .with_payload_override_validator(validate_fine_payload_override)
            .with_authoritative_payload_validator(validate_fine_payload_authoritatively),
    );
    defs.register(fine_action_def(ActionDefId(defs.len() as u32), handler))
}

pub fn register_exile_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(start_exile, tick_punishment, commit_exile, abort_punishment)
            .with_payload_override_validator(validate_exile_payload_override)
            .with_authoritative_payload_validator(validate_exile_payload_authoritatively),
    );
    defs.register(exile_action_def(ActionDefId(defs.len() as u32), handler))
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
        targets: vec![TargetSpec::SpecificEntity(EntityId {
            slot: 0,
            generation: 0,
        })],
        preconditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
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

fn fine_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    punishment_action_def(
        id,
        handler,
        "fine",
        BTreeSet::from([
            EventTag::Social,
            EventTag::Crime,
            EventTag::Transfer,
            EventTag::WorldMutation,
        ]),
    )
}

fn exile_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    punishment_action_def(
        id,
        handler,
        "exile",
        BTreeSet::from([
            EventTag::Social,
            EventTag::Crime,
            EventTag::Political,
            EventTag::WorldMutation,
        ]),
    )
}

fn punishment_action_def(
    id: ActionDefId,
    handler: ActionHandlerId,
    name: &str,
    tags: BTreeSet<EventTag>,
) -> ActionDef {
    ActionDef {
        id,
        name: name.to_string(),
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
            Precondition::TargetAlive(0),
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
            Precondition::TargetAlive(0),
        ],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: tags,
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
) -> Option<(TheftFacts, Option<EntityId>)> {
    view.active_violation_records(actor)
        .into_iter()
        .find(|record| record.id == violation_id)
        .and_then(|record| match record.kind {
            ViolationKind::SuspectedTheft { theft, suspect } => Some((theft, suspect)),
            _ => None,
        })
}

fn social_observation_supports_case(
    observation: SocialObservation,
    accused: EntityId,
    theft: TheftFacts,
) -> bool {
    matches!(
        observation.detail,
        SocialObservationDetail::SuspectedTheft {
            theft: observed_theft,
            suspect: Some(observed_accused),
        } if observed_theft == theft
            && observed_accused == accused
    )
}

fn actor_has_subjective_accusation_evidence(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    accused: EntityId,
    violation_id: ViolationId,
) -> bool {
    let Some((theft, suspect)) = unresolved_suspected_theft(view, actor, violation_id) else {
        return false;
    };

    if suspect == Some(accused) {
        return true;
    }

    view.known_social_observations(actor)
        .into_iter()
        .any(|observation| social_observation_supports_case(observation, accused, theft))
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

fn subjective_theft_facts_for_accusation(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    accused: EntityId,
    violation_id: ViolationId,
) -> Result<TheftFacts, ActionError> {
    let Some((theft, suspect)) = unresolved_suspected_theft(view, actor, violation_id) else {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} lacks active theft facts for violation {}",
            violation_id.0
        )));
    };
    if suspect == Some(accused)
        || view
            .known_social_observations(actor)
            .into_iter()
            .any(|observation| social_observation_supports_case(observation, accused, theft))
    {
        return Ok(theft);
    }

    Err(ActionError::PreconditionFailed(format!(
        "actor {actor} lacks theft facts linked to accused {accused} for violation {}",
        violation_id.0
    )))
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

fn enumerate_accuse_targets(
    _def: &ActionDef,
    actor: EntityId,
    view: &dyn RuntimeBeliefView,
) -> Vec<Vec<EntityId>> {
    let mut targets = BTreeSet::new();

    for record in view.active_violation_records(actor) {
        let ViolationKind::SuspectedTheft { theft, suspect } = record.kind else {
            continue;
        };

        if let Some(accused) = suspect.filter(|accused| *accused != actor && view.is_alive(*accused)) {
            targets.insert(accused);
        }

        targets.extend(
            view.known_social_observations(actor)
                .into_iter()
                .filter(|observation| {
                    matches!(
                        observation.detail,
                        SocialObservationDetail::SuspectedTheft {
                            theft: observed_theft,
                            suspect: Some(accused),
                        } if observed_theft == theft
                            && accused != actor
                            && view.is_alive(accused)
                    )
                })
                .map(|observation| match observation.detail {
                        SocialObservationDetail::SuspectedTheft {
                            suspect: Some(accused),
                            ..
                        } => accused,
                        _ => unreachable!(),
                }),
        );
    }

    targets.into_iter().map(|target| vec![target]).collect()
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
    let theft =
        subjective_theft_facts_for_accusation(&view, instance.actor, accused, violation_id)?;
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
    let entry_id = txn
        .append_record_entry(
        record,
        InstitutionalClaim::Accusation {
            accuser: instance.actor,
            accused,
            violation_id,
            theft,
            effective_tick: txn.tick(),
        },
    )
    .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.replace_institutional_belief(
        instance.actor,
        InstitutionalBeliefKey::CrimeCase {
            accused,
            violation_id,
        },
        BelievedInstitutionalClaim {
            claim: InstitutionalClaim::Accusation {
                accuser: instance.actor,
                accused,
                violation_id,
                theft,
                effective_tick: txn.tick(),
            },
            source: InstitutionalKnowledgeSource::RecordConsultation {
                record,
                entry_id,
            },
            learned_tick: txn.tick(),
            learned_at: Some(actor_place),
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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct ActiveAccusationCase {
    accused: EntityId,
    violation_id: ViolationId,
    theft: TheftFacts,
}

fn punish_payload<'a>(
    def: &ActionDef,
    payload: &'a ActionPayload,
) -> Result<&'a PunishActionPayload, ActionError> {
    payload.as_punish().ok_or_else(|| {
        ActionError::PreconditionFailed(format!("action def {} requires Punish payload", def.id))
    })
}

fn punishment_actor_place(world: &World, actor: EntityId) -> Result<EntityId, ActionError> {
    world.effective_place(actor).ok_or(ActionError::AbortRequested(
        ActionAbortRequestReason::ActorNotPlaced { actor },
    ))
}

fn validate_same_place_target(
    world: &World,
    actor: EntityId,
    targets: &[EntityId],
) -> Result<(EntityId, EntityId), ActionError> {
    let accused = *targets.first().ok_or(ActionError::InvalidTarget(actor))?;
    if actor == accused {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} cannot punish themselves"
        )));
    }
    let actor_place = punishment_actor_place(world, actor)?;
    if world.effective_place(accused) != Some(actor_place) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetNotColocated { actor, target: accused },
        ));
    }
    Ok((accused, actor_place))
}

fn validate_office_authority_at_place(
    world: &World,
    actor: EntityId,
    office: EntityId,
    place: EntityId,
) -> Result<(), ActionError> {
    let office_data = world
        .get_component_office_data(office)
        .ok_or_else(|| ActionError::PreconditionFailed(format!("office {office} lacks OfficeData")))?;
    if office_data.jurisdiction != place {
        return Err(ActionError::PreconditionFailed(format!(
            "office {office} lacks jurisdiction at place {place}"
        )));
    }
    if world.office_holder(office) != Some(actor) {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} does not hold office {office}"
        )));
    }
    Ok(())
}

fn active_accusation_case(
    record_data: &RecordData,
    accusation_entry: RecordEntryId,
) -> Option<ActiveAccusationCase> {
    record_data.active_entries().into_iter().find_map(|entry| match entry.claim {
        InstitutionalClaim::Accusation {
            accused,
            violation_id,
            theft,
            ..
        } if entry.entry_id == accusation_entry => Some(ActiveAccusationCase {
            accused,
            violation_id,
            theft,
        }),
        _ => None,
    })
}

fn punishment_profile(
    world: &World,
    actor: EntityId,
) -> Result<JusticeDispositionProfile, ActionError> {
    world
        .get_component_justice_disposition_profile(actor)
        .cloned()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "actor {actor} lacks JusticeDispositionProfile"
            ))
        })
}

fn fine_amount(profile: &JusticeDispositionProfile, theft: TheftFacts) -> Quantity {
    Quantity((u64::from(theft.quantity.0) * u64::from(profile.fine_severity.value()) / 1000) as u32)
}

fn ensure_accessible_quantity(
    world: &World,
    holder: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
) -> Result<(), ActionError> {
    let available = world.controlled_commodity_quantity(holder, commodity);
    if available < quantity {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::HolderLacksAccessibleCommodity {
                holder,
                commodity,
                quantity,
            },
        ));
    }
    Ok(())
}

fn resolve_controlled_lots(
    txn: &mut WorldTxn<'_>,
    holder: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
    place: EntityId,
) -> Result<Vec<(EntityId, Quantity)>, ActionError> {
    let mut remaining = quantity;
    let mut selected = Vec::new();
    let mut lots = txn
        .query_item_lot()
        .filter_map(|(entity, lot)| {
            (lot.commodity == commodity
                && txn.can_exercise_control(holder, entity).is_ok()
                && txn.effective_place(entity) == Some(place))
            .then_some((entity, lot.quantity))
        })
        .collect::<Vec<_>>();
    lots.sort_by_key(|(entity, _)| *entity);

    for (lot_id, available) in lots {
        if remaining == Quantity(0) {
            break;
        }
        if available > remaining {
            let (_, split_off) = txn
                .split_lot(lot_id, remaining)
                .map_err(|error| ActionError::InternalError(error.to_string()))?;
            selected.push((split_off, remaining));
            remaining = Quantity(0);
            break;
        }

        selected.push((lot_id, available));
        remaining = remaining.checked_sub(available).ok_or_else(|| {
            ActionError::InternalError("controlled lot accounting underflowed".to_string())
        })?;
    }

    if remaining != Quantity(0) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::HolderLacksAccessibleCommodity {
                holder,
                commodity,
                quantity,
            },
        ));
    }

    Ok(selected)
}

fn transfer_lot_to_holder(
    txn: &mut WorldTxn<'_>,
    lot_id: EntityId,
    new_holder: EntityId,
    place: EntityId,
    quantity: Quantity,
) -> Result<(), ActionError> {
    if txn.direct_container(lot_id).is_some() {
        txn.remove_from_container(lot_id)
            .map_err(|error| ActionError::InternalError(error.to_string()))?;
    }
    if txn.possessor_of(lot_id).is_some() {
        txn.clear_possessor(lot_id)
            .map_err(|error| ActionError::InternalError(error.to_string()))?;
    }
    if txn.effective_place(lot_id) != Some(place) {
        txn.set_ground_location(lot_id, place)
            .map_err(|error| ActionError::InternalError(error.to_string()))?;
    }
    txn.set_owner(lot_id, new_holder)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    txn.set_possessor(lot_id, new_holder)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    txn.append_transfer_provenance(lot_id, quantity)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    txn.add_target(lot_id);
    Ok(())
}

fn transfer_controlled_commodity(
    txn: &mut WorldTxn<'_>,
    holder: EntityId,
    new_holder: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
    place: EntityId,
) -> Result<(), ActionError> {
    ensure_accessible_quantity(txn, holder, commodity, quantity)?;
    for (lot_id, moved_quantity) in resolve_controlled_lots(txn, holder, commodity, quantity, place)?
    {
        transfer_lot_to_holder(txn, lot_id, new_holder, place, moved_quantity)?;
    }
    Ok(())
}

fn validate_punishment_case(
    world: &World,
    actor: EntityId,
    targets: &[EntityId],
    payload: &PunishActionPayload,
) -> Result<(EntityId, EntityId, EntityId, ActiveAccusationCase), ActionError> {
    let (accused, place) = validate_same_place_target(world, actor, targets)?;
    validate_office_authority_at_place(world, actor, payload.office, place)?;
    let record = locate_unique_crime_register(world, place)?;
    let record_data = world.get_component_record_data(record).ok_or_else(|| {
        ActionError::InternalError(format!("record {record} lacks RecordData"))
    })?;
    let accusation =
        active_accusation_case(record_data, payload.accusation_entry).ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "crime register {record} has no active accusation entry {}",
                payload.accusation_entry.0
            ))
        })?;
    if accusation.accused != accused {
        return Err(ActionError::PreconditionFailed(format!(
            "accusation entry {} targets {}, not {accused}",
            payload.accusation_entry.0, accusation.accused
        )));
    }
    Ok((accused, place, record, accusation))
}

fn validate_fine_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    _view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(payload) = payload.as_punish() else {
        return false;
    };
    matches!(payload.punishment, PunishmentKind::Fine { .. })
        && targets.first().copied().is_some_and(|accused| accused != actor)
}

fn validate_fine_payload_authoritatively(
    def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let payload = punish_payload(def, payload)?;
    let (_accused, _place, _record, accusation) = validate_punishment_case(world, actor, targets, payload)?;
    let profile = punishment_profile(world, actor)?;
    let expected_amount = fine_amount(&profile, accusation.theft);
    if expected_amount == Quantity(0) {
        return Err(ActionError::PreconditionFailed(format!(
            "accusation entry {} resolves to zero fine",
            payload.accusation_entry.0
        )));
    }
    let expected = PunishmentKind::Fine {
        commodity: accusation.theft.commodity,
        amount: expected_amount,
    };
    if payload.punishment != expected {
        return Err(ActionError::PreconditionFailed(format!(
            "fine payload {:?} does not match authoritative {:?}",
            payload.punishment, expected
        )));
    }
    ensure_accessible_quantity(world, accusation.accused, accusation.theft.commodity, expected_amount)
}

fn validate_exile_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    _view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(payload) = payload.as_punish() else {
        return false;
    };
    matches!(payload.punishment, PunishmentKind::Exile { .. })
        && targets.first().copied().is_some_and(|accused| accused != actor)
}

fn validate_exile_payload_authoritatively(
    def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let payload = punish_payload(def, payload)?;
    let (accused, _place, _record, _accusation) = validate_punishment_case(world, actor, targets, payload)?;
    let PunishmentKind::Exile { from_faction } = payload.punishment else {
        return Err(ActionError::PreconditionFailed(format!(
            "payload for exile action must be Exile, got {:?}",
            payload.punishment
        )));
    };
    let office_data = world
        .get_component_office_data(payload.office)
        .ok_or_else(|| ActionError::PreconditionFailed(format!("office {} lacks OfficeData", payload.office)))?;
    if !office_data
        .eligibility_rules
        .iter()
        .any(|rule| matches!(rule, EligibilityRule::FactionMember(faction) if *faction == from_faction))
    {
        return Err(ActionError::PreconditionFailed(format!(
            "office {} does not govern faction {from_faction}",
            payload.office
        )));
    }
    if !world.factions_of(accused).contains(&from_faction) {
        return Err(ActionError::PreconditionFailed(format!(
            "accused {accused} is not a member of faction {from_faction}"
        )));
    }
    Ok(())
}

fn validate_fine_start(
    def: &ActionDef,
    instance: &ActionInstance,
    txn: &WorldTxn<'_>,
) -> Result<(), ActionError> {
    let payload = punish_payload(def, &instance.payload)?;
    let (_accused, _place, _record, accusation) =
        validate_punishment_case(txn, instance.actor, &instance.targets, payload)?;
    let profile = punishment_profile(txn, instance.actor)?;
    let expected_amount = fine_amount(&profile, accusation.theft);
    if expected_amount == Quantity(0) {
        return Err(ActionError::PreconditionFailed(format!(
            "accusation entry {} resolves to zero fine",
            payload.accusation_entry.0
        )));
    }
    let expected = PunishmentKind::Fine {
        commodity: accusation.theft.commodity,
        amount: expected_amount,
    };
    if payload.punishment != expected {
        return Err(ActionError::PreconditionFailed(format!(
            "fine payload {:?} does not match authoritative {:?}",
            payload.punishment, expected
        )));
    }
    ensure_accessible_quantity(
        txn,
        accusation.accused,
        accusation.theft.commodity,
        expected_amount,
    )
}

fn validate_exile_start(
    def: &ActionDef,
    instance: &ActionInstance,
    txn: &WorldTxn<'_>,
) -> Result<(), ActionError> {
    let payload = punish_payload(def, &instance.payload)?;
    let (accused, _place, _record, _accusation) =
        validate_punishment_case(txn, instance.actor, &instance.targets, payload)?;
    let PunishmentKind::Exile { from_faction } = payload.punishment else {
        return Err(ActionError::PreconditionFailed(format!(
            "payload for exile action must be Exile, got {:?}",
            payload.punishment
        )));
    };
    let office_data = txn.get_component_office_data(payload.office).ok_or_else(|| {
        ActionError::PreconditionFailed(format!("office {} lacks OfficeData", payload.office))
    })?;
    if !office_data
        .eligibility_rules
        .iter()
        .any(|rule| matches!(rule, EligibilityRule::FactionMember(faction) if *faction == from_faction))
    {
        return Err(ActionError::PreconditionFailed(format!(
            "office {} does not govern faction {from_faction}",
            payload.office
        )));
    }
    if !txn.factions_of(accused).contains(&from_faction) {
        return Err(ActionError::PreconditionFailed(format!(
            "accused {accused} is not a member of faction {from_faction}"
        )));
    }
    Ok(())
}

fn start_fine(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    validate_fine_start(def, instance, txn)?;
    Ok(Some(ActionState::Empty))
}

fn start_exile(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    validate_exile_start(def, instance, txn)?;
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_punishment(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_fine(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = punish_payload(def, &instance.payload)?;
    let (_accused, place, record, accusation) =
        validate_punishment_case(txn, instance.actor, &instance.targets, payload)?;
    let profile = punishment_profile(txn, instance.actor)?;
    let expected_amount = fine_amount(&profile, accusation.theft);
    let expected = PunishmentKind::Fine {
        commodity: accusation.theft.commodity,
        amount: expected_amount,
    };
    if payload.punishment != expected {
        return Err(ActionError::PreconditionFailed(format!(
            "fine payload {:?} does not match authoritative {:?}",
            payload.punishment, expected
        )));
    }
    if expected_amount == Quantity(0) {
        return Err(ActionError::PreconditionFailed(format!(
            "accusation entry {} resolves to zero fine",
            payload.accusation_entry.0
        )));
    }
    transfer_controlled_commodity(
        txn,
        accusation.accused,
        payload.office,
        accusation.theft.commodity,
        expected_amount,
        place,
    )?;
    txn.supersede_record_entry(
        record,
        payload.accusation_entry,
        InstitutionalClaim::Verdict {
            accused: accusation.accused,
            violation_id: accusation.violation_id,
            punishment: expected,
            effective_tick: txn.tick(),
        },
    )
    .map_err(|error| ActionError::InternalError(error.to_string()))?;
    Ok(CommitOutcome::empty())
}

fn commit_exile(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = punish_payload(def, &instance.payload)?;
    let (accused, _place, record, accusation) =
        validate_punishment_case(txn, instance.actor, &instance.targets, payload)?;
    let PunishmentKind::Exile { from_faction } = payload.punishment else {
        return Err(ActionError::PreconditionFailed(format!(
            "payload for exile action must be Exile, got {:?}",
            payload.punishment
        )));
    };
    let office_data = txn
        .get_component_office_data(payload.office)
        .ok_or_else(|| ActionError::PreconditionFailed(format!("office {} lacks OfficeData", payload.office)))?;
    if !office_data
        .eligibility_rules
        .iter()
        .any(|rule| matches!(rule, EligibilityRule::FactionMember(faction) if *faction == from_faction))
    {
        return Err(ActionError::PreconditionFailed(format!(
            "office {} does not govern faction {from_faction}",
            payload.office
        )));
    }
    if !txn.factions_of(accused).contains(&from_faction) {
        return Err(ActionError::PreconditionFailed(format!(
            "accused {accused} is not a member of faction {from_faction}"
        )));
    }
    txn.remove_member(accused, from_faction)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    txn.add_hostility(from_faction, accused)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    txn.supersede_record_entry(
        record,
        payload.accusation_entry,
        InstitutionalClaim::Verdict {
            accused: accusation.accused,
            violation_id: accusation.violation_id,
            punishment: payload.punishment,
            effective_tick: txn.tick(),
        },
    )
    .map_err(|error| ActionError::InternalError(error.to_string()))?;
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_punishment(
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
    use super::{register_accuse_action, register_exile_action, register_fine_action};
    use std::collections::BTreeMap;
    use worldwake_core::{
        build_prototype_world, verify_live_lot_conservation, ActionDefId, AgentBeliefStore,
        BeliefConfidencePolicy,
        BelievedEntityState, CauseRef, EntityId, EventLog, EventTag, EventView,
        EligibilityRule, InstitutionalClaim, JusticeDispositionProfile, OfficeData,
        PerceptionProfile, PerceptionSource, PrototypePlace, PunishmentKind, Quantity, RecordData,
        RecordEntryId, RecordKind, Seed, SocialObservation, SocialObservationDetail, SuccessionLaw,
        TheftFacts, Tick, UtilityProfile,
        ViolationDispositionProfile, ViolationId, ViolationKind, ViolationMemory,
        VisibilitySpec, WitnessData, World, WorldTxn,
    };
    use worldwake_sim::{
        get_affordances, AbortReason, AccuseActionPayload, ActionDefRegistry,
        ActionAbortRequestReason, ActionError, ActionHandlerRegistry, ActionInstance,
        ActionInstanceId, ActionPayload, ActionStatus, DeterministicRng, ExternalAbortReason,
        PerAgentBeliefView, PunishActionPayload,
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

    fn setup_punishment_registries(
    ) -> (
        ActionDefRegistry,
        ActionHandlerRegistry,
        ActionDefId,
        ActionDefId,
    ) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let fine_id = register_fine_action(&mut defs, &mut handlers);
        let exile_id = register_exile_action(&mut defs, &mut handlers);
        (defs, handlers, fine_id, exile_id)
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

    struct PunishmentFixture {
        world: World,
        office: EntityId,
        actor: EntityId,
        accused: EntityId,
        faction: EntityId,
        crime_register: EntityId,
        accusation_entry: RecordEntryId,
    }

    impl PunishmentFixture {
        fn new() -> Self {
            let mut world = World::new(build_prototype_world()).unwrap();
            let place = worldwake_core::prototype_place_entity(PrototypePlace::VillageSquare);
            let office;
            let actor;
            let accused;
            let faction;
            let crime_register;
            let accusation_entry;
            {
                let mut txn = new_txn(&mut world, 1);
                actor = txn
                    .create_agent("Punisher", worldwake_core::ControlSource::Ai)
                    .unwrap();
                accused = txn
                    .create_agent("Accused", worldwake_core::ControlSource::Ai)
                    .unwrap();
                office = txn.create_office("Magistrate").unwrap();
                faction = txn.create_faction("Ward").unwrap();
                for agent in [actor, accused] {
                    txn.set_ground_location(agent, place).unwrap();
                    txn.set_component_agent_belief_store(agent, AgentBeliefStore::new())
                        .unwrap();
                }
                txn.set_component_justice_disposition_profile(
                    actor,
                    JusticeDispositionProfile {
                        accusation_motive_weight: pm(700),
                        fine_severity: pm(500),
                    },
                )
                .unwrap();
                txn.set_component_office_data(
                    office,
                    OfficeData {
                        title: "Magistrate".to_string(),
                        jurisdiction: place,
                        succession_law: SuccessionLaw::Support,
                        eligibility_rules: vec![EligibilityRule::FactionMember(faction)],
                        succession_period_ticks: 12,
                        vacancy_since: None,
                    },
                )
                .unwrap();
                let _ = create_record(&mut txn, place, actor, RecordKind::OfficeRegister);
                txn.assign_office(office, actor).unwrap();
                txn.add_member(accused, faction).unwrap();
                crime_register = create_record(&mut txn, place, actor, RecordKind::CrimeRegister);
                let missing_entity = txn
                    .create_item_lot(worldwake_core::CommodityKind::Bread, Quantity(4))
                    .unwrap();
                accusation_entry = txn
                    .append_record_entry(
                        crime_register,
                        InstitutionalClaim::Accusation {
                            accuser: actor,
                            accused,
                            violation_id: ViolationId(1),
                            theft: TheftFacts {
                                missing_entity,
                                expected_place: place,
                                commodity: worldwake_core::CommodityKind::Bread,
                                quantity: Quantity(4),
                            },
                            effective_tick: Tick(1),
                        },
                    )
                    .unwrap();
                let lot = txn
                    .create_item_lot(worldwake_core::CommodityKind::Bread, Quantity(4))
                    .unwrap();
                txn.set_ground_location(lot, place).unwrap();
                txn.set_owner(lot, accused).unwrap();
                txn.set_possessor(lot, accused).unwrap();
                let mut log = EventLog::new();
                let _ = txn.commit(&mut log);
            }
            Self {
                world,
                office,
                actor,
                accused,
                faction,
                crime_register,
                accusation_entry,
            }
        }

        fn punishment_instance(
            &self,
            def_id: ActionDefId,
            punishment: PunishmentKind,
        ) -> ActionInstance {
            ActionInstance {
                instance_id: ActionInstanceId(0),
                def_id,
                payload: ActionPayload::Punish(PunishActionPayload {
                    office: self.office,
                    accusation_entry: self.accusation_entry,
                    punishment,
                }),
                actor: self.actor,
                targets: vec![self.accused],
                start_tick: Tick(3),
                remaining_duration: worldwake_sim::ActionDuration::new(1),
                status: ActionStatus::Active,
                reservation_ids: Vec::new(),
                local_state: None,
            }
        }
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
                        theft: TheftFacts {
                            missing_entity: missing_item,
                            expected_place: place,
                            commodity: worldwake_core::CommodityKind::Bread,
                            quantity: Quantity(1),
                        },
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

        fn move_accused_to(&mut self, place: EntityId, tick: u64) {
            let mut txn = new_txn(&mut self.world, tick);
            txn.set_ground_location(self.accused, place).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        fn seed_social_observation(&mut self, suspect: EntityId, tick: u64) {
            let mut store = self
                .world
                .get_component_agent_belief_store(self.accuser)
                .cloned()
                .unwrap();
            store.record_social_observation(SocialObservation {
                detail: SocialObservationDetail::SuspectedTheft {
                    theft: TheftFacts {
                        missing_entity: self.missing_item,
                        expected_place: self.place,
                        commodity: worldwake_core::CommodityKind::Bread,
                        quantity: Quantity(1),
                    },
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
    fn accuse_affordance_emits_payload_for_known_remote_suspect_observation() {
        let (defs, handlers, id) = setup_registries();
        let mut fx = JusticeFixture::new();
        fx.seed_social_observation(fx.accused, 2);
        fx.move_accused_to(
            worldwake_core::prototype_place_entity(PrototypePlace::CommonHouse),
            2,
        );
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
            })],
            "accuse affordances should still target a known remote suspect while filing at the local crime register"
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
                ..
            }) if accuser == fx.accuser
                && accused == fx.accused
                && violation_id == fx.violation_id
                && effective_tick == Tick(3)
        ));
        assert!(event.tags().contains(&EventTag::Crime));
        assert_eq!(event.visibility(), VisibilitySpec::SamePlace);
    }

    #[test]
    fn accusation_can_file_against_remote_known_suspect() {
        let (defs, handlers, id) = setup_registries();
        let mut fx = JusticeFixture::new();
        fx.seed_social_observation(fx.accused, 2);
        fx.move_accused_to(
            worldwake_core::prototype_place_entity(PrototypePlace::CommonHouse),
            2,
        );
        let instance = fx.instance(id, fx.accused);

        let _ = commit_action(&mut fx.world, &defs, &handlers, id, &instance, 7, 3);
        let record = fx.world.get_component_record_data(fx.crime_register).unwrap();

        assert!(record.entries.iter().any(|entry| {
            matches!(
                entry.claim,
                InstitutionalClaim::Accusation {
                    accuser,
                    accused,
                    violation_id,
                    effective_tick,
                    ..
                } if accuser == fx.accuser
                    && accused == fx.accused
                    && violation_id == fx.violation_id
                    && effective_tick == Tick(3)
            )
        }));
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
                    theft: TheftFacts {
                        missing_entity: fx.missing_item,
                        expected_place: fx.place,
                        commodity: worldwake_core::CommodityKind::Bread,
                        quantity: Quantity(1),
                    },
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

    #[test]
    fn fine_transfers_goods_to_office_and_supersedes_exact_accusation() {
        let (defs, handlers, fine_id, _exile_id) = setup_punishment_registries();
        let mut fx = PunishmentFixture::new();
        let instance = fx.punishment_instance(
            fine_id,
            PunishmentKind::Fine {
                commodity: worldwake_core::CommodityKind::Bread,
                amount: Quantity(2),
            },
        );

        let _ = commit_action(&mut fx.world, &defs, &handlers, fine_id, &instance, 7, 3);
        let record = fx.world.get_component_record_data(fx.crime_register).unwrap();

        assert_eq!(
            fx.world
                .controlled_commodity_quantity(fx.accused, worldwake_core::CommodityKind::Bread),
            Quantity(2)
        );
        assert_eq!(
            fx.world
                .controlled_commodity_quantity(fx.office, worldwake_core::CommodityKind::Bread),
            Quantity(2)
        );
        verify_live_lot_conservation(&fx.world, worldwake_core::CommodityKind::Bread, 8).unwrap();
        assert!(record.active_entries().iter().any(|entry| {
            matches!(
                entry.claim,
                InstitutionalClaim::Verdict {
                    accused,
                    violation_id,
                    punishment: PunishmentKind::Fine { commodity, amount },
                    effective_tick,
                } if accused == fx.accused
                    && violation_id == ViolationId(1)
                    && commodity == worldwake_core::CommodityKind::Bread
                    && amount == Quantity(2)
                    && effective_tick == Tick(3)
            ) && entry.supersedes == Some(fx.accusation_entry)
        }));
    }

    #[test]
    fn fine_rejects_when_accused_lacks_sufficient_goods() {
        let (defs, handlers, fine_id, _exile_id) = setup_punishment_registries();
        let mut fx = PunishmentFixture::new();
        let def = defs.get(fine_id).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let instance = fx.punishment_instance(
            fine_id,
            PunishmentKind::Fine {
                commodity: worldwake_core::CommodityKind::Bread,
                amount: Quantity(2),
            },
        );
        {
            let bread = fx
                .world
                .possessions_of(fx.accused)
                .into_iter()
                .find(|entity| {
                    fx.world
                        .get_component_item_lot(*entity)
                        .is_some_and(|lot| lot.commodity == worldwake_core::CommodityKind::Bread)
                })
                .unwrap();
            let mut txn = new_txn(&mut fx.world, 2);
            txn.archive_entity(bread).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let mut txn = new_action_txn(&mut fx.world, fx.actor, 3);
        let mut rng = test_rng(1);

        let err = (handler.on_start)(def, &instance, &mut rng, &mut txn).unwrap_err();

        assert!(matches!(
            err,
            ActionError::AbortRequested(ActionAbortRequestReason::HolderLacksAccessibleCommodity { .. })
        ));
    }

    #[test]
    fn fine_accepts_unpossessed_owned_ground_stock() {
        let (defs, handlers, fine_id, _exile_id) = setup_punishment_registries();
        let mut fx = PunishmentFixture::new();
        let instance = fx.punishment_instance(
            fine_id,
            PunishmentKind::Fine {
                commodity: worldwake_core::CommodityKind::Bread,
                amount: Quantity(2),
            },
        );
        {
            let bread = fx
                .world
                .possessions_of(fx.accused)
                .into_iter()
                .find(|entity| {
                    fx.world
                        .get_component_item_lot(*entity)
                        .is_some_and(|lot| lot.commodity == worldwake_core::CommodityKind::Bread)
                })
                .unwrap();
            let mut txn = new_txn(&mut fx.world, 2);
            txn.clear_possessor(bread).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        let def = defs.get(fine_id).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let mut start_txn = new_action_txn(&mut fx.world, fx.actor, 3);
        let mut start_rng = test_rng(5);
        (handler.on_start)(def, &instance, &mut start_rng, &mut start_txn).unwrap();

        let _ = commit_action(&mut fx.world, &defs, &handlers, fine_id, &instance, 7, 3);

        assert_eq!(
            fx.world
                .controlled_commodity_quantity(fx.accused, worldwake_core::CommodityKind::Bread),
            Quantity(2)
        );
        assert_eq!(
            fx.world
                .controlled_commodity_quantity(fx.office, worldwake_core::CommodityKind::Bread),
            Quantity(2)
        );
    }

    #[test]
    fn exile_removes_membership_adds_hostility_and_supersedes_exact_accusation() {
        let (defs, handlers, _fine_id, exile_id) = setup_punishment_registries();
        let mut fx = PunishmentFixture::new();
        let instance = fx.punishment_instance(
            exile_id,
            PunishmentKind::Exile {
                from_faction: fx.faction,
            },
        );

        let _ = commit_action(&mut fx.world, &defs, &handlers, exile_id, &instance, 8, 3);
        let record = fx.world.get_component_record_data(fx.crime_register).unwrap();

        assert!(!fx.world.factions_of(fx.accused).contains(&fx.faction));
        assert!(fx.world.hostile_towards(fx.accused).contains(&fx.faction));
        assert!(record.active_entries().iter().any(|entry| {
            matches!(
                entry.claim,
                InstitutionalClaim::Verdict {
                    accused,
                    violation_id,
                    punishment: PunishmentKind::Exile { from_faction },
                    effective_tick,
                } if accused == fx.accused
                    && violation_id == ViolationId(1)
                    && from_faction == fx.faction
                    && effective_tick == Tick(3)
            ) && entry.supersedes == Some(fx.accusation_entry)
        }));
    }

    #[test]
    fn exile_rejects_when_actor_lacks_office_authority() {
        let (defs, handlers, _fine_id, exile_id) = setup_punishment_registries();
        let mut fx = PunishmentFixture::new();
        let def = defs.get(exile_id).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let instance = fx.punishment_instance(
            exile_id,
            PunishmentKind::Exile {
                from_faction: fx.faction,
            },
        );
        {
            let mut txn = new_txn(&mut fx.world, 2);
            txn.vacate_office(fx.office).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let mut txn = new_action_txn(&mut fx.world, fx.actor, 3);
        let mut rng = test_rng(2);

        let err = (handler.on_start)(def, &instance, &mut rng, &mut txn).unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(message) if message.contains("does not hold office")));
    }
}
