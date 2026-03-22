use crate::offices::candidate_is_eligible;
use std::collections::BTreeSet;
use std::num::NonZeroU32;

use worldwake_core::{
    ActionDefId, BodyCostPerTick, CombatProfile, CommodityKind, EntityId, EntityKind, EventTag,
    Permille, Quantity, SuccessionLaw, VisibilitySpec, World, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionAbortRequestReason, ActionDef, ActionDefRegistry, ActionError,
    ActionHandler, ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload,
    ActionProgress, ActionState, BribeActionPayload, CommitOutcome, DeclareSupportActionPayload,
    DeterministicRng, DurationExpr, Interruptibility, PayloadEntityRole, Precondition,
    RuntimeBeliefView, TargetSpec, ThreatenActionPayload,
};

pub fn register_office_actions(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> [ActionDefId; 3] {
    let bribe_id = register_bribe_action(defs, handlers);
    let threaten_id = register_threaten_action(defs, handlers);
    let declare_support_id = register_declare_support_action(defs, handlers);
    [bribe_id, threaten_id, declare_support_id]
}

fn register_bribe_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(start_bribe, tick_bribe, commit_bribe, abort_bribe)
            .with_affordance_payloads(enumerate_bribe_payloads)
            .with_payload_override_validator(validate_bribe_payload_override)
            .with_authoritative_payload_validator(validate_bribe_payload_authoritatively),
    );
    defs.register(bribe_action_def(ActionDefId(defs.len() as u32), handler))
}

fn register_threaten_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(
            start_threaten,
            tick_threaten,
            commit_threaten,
            abort_threaten,
        )
        .with_affordance_payloads(enumerate_threaten_payloads)
        .with_payload_override_validator(validate_threaten_payload_override)
        .with_authoritative_payload_validator(validate_threaten_payload_authoritatively),
    );
    defs.register(threaten_action_def(ActionDefId(defs.len() as u32), handler))
}

fn register_declare_support_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(
            start_declare_support,
            tick_declare_support,
            commit_declare_support,
            abort_declare_support,
        )
        .with_payload_override_validator(validate_declare_support_payload_override)
        .with_authoritative_payload_validator(validate_declare_support_payload_authoritatively),
    );
    defs.register(declare_support_action_def(
        ActionDefId(defs.len() as u32),
        handler,
    ))
}

fn bribe_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "bribe".to_string(),
        domain: worldwake_sim::ActionDomain::Social,
        actor_constraints: vec![worldwake_sim::Constraint::ActorAlive],
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
        causal_event_tags: BTreeSet::from([
            EventTag::Social,
            EventTag::Transfer,
            EventTag::WorldMutation,
        ]),
        payload: ActionPayload::None,
        handler,
    }
}

fn threaten_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "threaten".to_string(),
        domain: worldwake_sim::ActionDomain::Social,
        actor_constraints: vec![worldwake_sim::Constraint::ActorAlive],
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
        causal_event_tags: BTreeSet::from([
            EventTag::Social,
            EventTag::Coercion,
            EventTag::WorldMutation,
        ]),
        payload: ActionPayload::None,
        handler,
    }
}

fn declare_support_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "declare_support".to_string(),
        domain: worldwake_sim::ActionDomain::Social,
        actor_constraints: vec![worldwake_sim::Constraint::ActorAlive],
        targets: Vec::new(),
        preconditions: vec![Precondition::ActorAlive],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::Fixed(NonZeroU32::MIN),
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::NonInterruptible,
        commit_conditions: vec![Precondition::ActorAlive],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::Political, EventTag::WorldMutation]),
        payload: ActionPayload::None,
        handler,
    }
}

fn bribe_payload<'a>(
    def: &ActionDef,
    payload: &'a ActionPayload,
) -> Result<&'a BribeActionPayload, ActionError> {
    payload.as_bribe().ok_or_else(|| {
        ActionError::PreconditionFailed(format!("action def {} requires Bribe payload", def.id))
    })
}

fn threaten_payload<'a>(
    def: &ActionDef,
    payload: &'a ActionPayload,
) -> Result<&'a ThreatenActionPayload, ActionError> {
    payload.as_threaten().ok_or_else(|| {
        ActionError::PreconditionFailed(format!("action def {} requires Threaten payload", def.id))
    })
}

fn declare_support_payload<'a>(
    def: &ActionDef,
    payload: &'a ActionPayload,
) -> Result<&'a DeclareSupportActionPayload, ActionError> {
    payload.as_declare_support().ok_or_else(|| {
        ActionError::PreconditionFailed(format!(
            "action def {} requires DeclareSupport payload",
            def.id
        ))
    })
}

fn enumerate_bribe_payloads(
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

    CommodityKind::ALL
        .into_iter()
        .filter_map(|commodity| {
            let quantity = view.commodity_quantity(actor, commodity);
            (quantity > Quantity(0)).then_some(ActionPayload::Bribe(BribeActionPayload {
                target,
                offered_commodity: commodity,
                offered_quantity: quantity,
            }))
        })
        .collect()
}

fn validate_bribe_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    _view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(payload) = payload.as_bribe() else {
        return false;
    };
    targets.first().copied() == Some(payload.target)
        && payload.target != actor
        && payload.offered_quantity > Quantity(0)
}

fn validate_bribe_payload_authoritatively(
    def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let payload = bribe_payload(def, payload)?;
    let target = *targets.first().ok_or(ActionError::InvalidTarget(actor))?;
    if payload.target != target {
        return Err(ActionError::PreconditionFailed(format!(
            "bribe payload target {} does not match bound target {}",
            payload.target, target
        )));
    }
    if target == actor {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} cannot bribe themselves"
        )));
    }
    if payload.offered_quantity == Quantity(0) {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} cannot bribe with zero quantity"
        )));
    }
    let available = world.controlled_commodity_quantity(actor, payload.offered_commodity);
    if available < payload.offered_quantity {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} controls only {:?} {:?}, but payload requires {:?}",
            available, payload.offered_commodity, payload.offered_quantity
        )));
    }
    Ok(())
}

fn enumerate_threaten_payloads(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    view: &dyn RuntimeBeliefView,
) -> Vec<ActionPayload> {
    let Some(target) = targets.first().copied() else {
        return Vec::new();
    };
    if target == actor || view.combat_profile(actor).is_none() {
        return Vec::new();
    }
    vec![ActionPayload::Threaten(ThreatenActionPayload { target })]
}

fn validate_threaten_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    _view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(payload) = payload.as_threaten() else {
        return false;
    };
    targets.first().copied() == Some(payload.target) && payload.target != actor
}

fn validate_threaten_payload_authoritatively(
    def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let payload = threaten_payload(def, payload)?;
    let target = *targets.first().ok_or(ActionError::InvalidTarget(actor))?;
    if payload.target != target {
        return Err(ActionError::PreconditionFailed(format!(
            "threaten payload target {} does not match bound target {}",
            payload.target, target
        )));
    }
    if target == actor {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} cannot threaten themselves"
        )));
    }
    if world.get_component_combat_profile(actor).is_none() {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::ActorMissingCombatProfile { actor },
        ));
    }
    if world.get_component_utility_profile(target).is_none() {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} lacks UtilityProfile"
        )));
    }
    Ok(())
}

fn validate_declare_support_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    _targets: &[EntityId],
    payload: &ActionPayload,
    _view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(payload) = payload.as_declare_support() else {
        return false;
    };
    payload.candidate != actor || payload.office != actor
}

fn validate_declare_support_payload_authoritatively(
    def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    _targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let payload = declare_support_payload(def, payload)?;
    validate_declare_support_context_in_world(world, actor, payload)
}

#[allow(clippy::unnecessary_wraps)]
fn start_bribe(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let _ = bribe_payload(def, &instance.payload)?;
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_bribe(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_bribe(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = bribe_payload(def, &instance.payload)?;
    let (target, place) =
        validate_agent_target_context(txn, instance.actor, &instance.targets, payload.target)?;
    transfer_controlled_commodity(
        txn,
        instance.actor,
        target,
        payload.offered_commodity,
        payload.offered_quantity,
        place,
    )?;
    increase_loyalty(
        txn,
        target,
        instance.actor,
        quantity_to_permille(payload.offered_quantity),
    )?;
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_bribe(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _reason: &AbortReason,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn start_threaten(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let _ = threaten_payload(def, &instance.payload)?;
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_threaten(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_threaten(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = threaten_payload(def, &instance.payload)?;
    let (target, _) =
        validate_agent_target_context(txn, instance.actor, &instance.targets, payload.target)?;
    let actor_profile = required_combat_profile(txn, instance.actor)?;
    let courage = txn
        .get_component_utility_profile(target)
        .cloned()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("target {target} lacks UtilityProfile"))
        })?
        .courage;
    let pressure = threat_pressure(actor_profile);
    if pressure > courage {
        increase_loyalty(txn, target, instance.actor, pressure)?;
    } else {
        txn.add_hostility(target, instance.actor)
            .map_err(|error| ActionError::InternalError(error.to_string()))?;
    }
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_threaten(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _reason: &AbortReason,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn start_declare_support(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let _ = declare_support_payload(def, &instance.payload)?;
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_declare_support(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_declare_support(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = declare_support_payload(def, &instance.payload)?;
    validate_declare_support_context_in_world(txn, instance.actor, payload)?;
    txn.declare_support(instance.actor, payload.office, payload.candidate)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    txn.add_target(payload.office);
    txn.add_target(payload.candidate);
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_declare_support(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _reason: &AbortReason,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

fn validate_agent_target_context(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    targets: &[EntityId],
    payload_target: EntityId,
) -> Result<(EntityId, EntityId), ActionError> {
    let target = *targets.first().ok_or(ActionError::InvalidTarget(actor))?;
    if target != payload_target {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::PayloadEntityMismatch {
                role: PayloadEntityRole::Target,
                expected: target,
                actual: payload_target,
            },
        ));
    }
    if actor == target {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} cannot target themselves"
        )));
    }
    let place = txn
        .effective_place(actor)
        .ok_or(ActionError::AbortRequested(
            ActionAbortRequestReason::ActorNotPlaced { actor },
        ))?;
    if txn.effective_place(target) != Some(place) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetNotColocated { actor, target },
        ));
    }
    Ok((target, place))
}

fn validate_declare_support_context_in_world(
    world: &World,
    actor: EntityId,
    payload: &DeclareSupportActionPayload,
) -> Result<(), ActionError> {
    let office_data = world
        .get_component_office_data(payload.office)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("office {} lacks OfficeData", payload.office))
        })?;
    if office_data.succession_law != SuccessionLaw::Support {
        return Err(ActionError::PreconditionFailed(format!(
            "office {} does not use support-based succession",
            payload.office
        )));
    }
    if world.entity_kind(payload.candidate) != Some(EntityKind::Agent)
        || !world.is_alive(payload.candidate)
    {
        return Err(ActionError::PreconditionFailed(format!(
            "candidate {} must be a live agent",
            payload.candidate
        )));
    }
    if world.effective_place(actor) != Some(office_data.jurisdiction) {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} is not at office jurisdiction {}",
            office_data.jurisdiction
        )));
    }
    if office_data.vacancy_since.is_none() || world.office_holder(payload.office).is_some() {
        return Err(ActionError::PreconditionFailed(format!(
            "office {} is not vacant",
            payload.office
        )));
    }
    if !candidate_is_eligible(world, office_data, payload.candidate) {
        return Err(ActionError::PreconditionFailed(format!(
            "candidate {} is not eligible for office {}",
            payload.candidate, payload.office
        )));
    }
    Ok(())
}

fn required_combat_profile(
    world: &WorldTxn<'_>,
    actor: EntityId,
) -> Result<CombatProfile, ActionError> {
    world
        .get_component_combat_profile(actor)
        .copied()
        .ok_or(ActionError::AbortRequested(
            ActionAbortRequestReason::ActorMissingCombatProfile { actor },
        ))
}

fn threat_pressure(profile: CombatProfile) -> Permille {
    profile.attack_skill
}

fn quantity_to_permille(quantity: Quantity) -> Permille {
    Permille::new(quantity.0.min(1000) as u16)
        .expect("clamped quantity always fits within permille bounds")
}

fn increase_loyalty(
    txn: &mut WorldTxn<'_>,
    subject: EntityId,
    target: EntityId,
    delta: Permille,
) -> Result<(), ActionError> {
    let next = txn
        .loyalty_to(subject, target)
        .unwrap_or(Permille::new(0).unwrap())
        .saturating_add(delta);
    txn.set_loyalty(subject, target, next)
        .map_err(|error| ActionError::InternalError(error.to_string()))
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
    for (lot_id, moved_quantity) in
        resolve_controlled_lots(txn, holder, commodity, quantity, place)?
    {
        transfer_lot(txn, lot_id, new_holder, place, moved_quantity)?;
    }
    Ok(())
}

fn ensure_accessible_quantity(
    txn: &WorldTxn<'_>,
    holder: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
) -> Result<(), ActionError> {
    let available = txn.controlled_commodity_quantity(holder, commodity);
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

fn transfer_lot(
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

#[cfg(test)]
mod tests {
    use super::register_office_actions;
    use crate::perception::perception_system;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, verify_live_lot_conservation, ActionDefId, AgentBeliefStore,
        CauseRef, CombatProfile, CommodityKind, ControlSource, EligibilityRule, EntityId, EventLog,
        EventTag, EventView, OfficeData, Permille, Quantity, RecordData, RecordKind, Seed,
        SuccessionLaw, Tick, UtilityProfile, VisibilitySpec, WitnessData, World, WorldTxn,
    };
    use worldwake_sim::{
        AbortReason, ActionAbortRequestReason, ActionDefRegistry, ActionError,
        ActionHandlerRegistry, ActionInstance, ActionInstanceId, ActionPayload, ActionStatus,
        BribeActionPayload, DeclareSupportActionPayload, DeterministicRng, ExternalAbortReason,
        PerAgentBeliefView, SystemExecutionContext, SystemId, ThreatenActionPayload,
    };

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

    fn setup_registries() -> (ActionDefRegistry, ActionHandlerRegistry, [ActionDefId; 3]) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let ids = register_office_actions(&mut defs, &mut handlers);
        (defs, handlers, ids)
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

    fn record_at_place(world: &World, place: EntityId, kind: RecordKind) -> RecordData {
        world
            .query_record_data()
            .find_map(|(_, record)| {
                (record.home_place == place && record.record_kind == kind).then_some(record.clone())
            })
            .expect("fixture should provision the requested record")
    }

    struct SocialFixture {
        world: World,
        place: EntityId,
        actor: EntityId,
        target: EntityId,
        observer: EntityId,
        office: EntityId,
        candidate: EntityId,
        faction: EntityId,
    }

    impl SocialFixture {
        fn new() -> Self {
            let mut world = World::new(build_prototype_world()).unwrap();
            let place = world.topology().place_ids().next().unwrap();
            let (actor, target, observer, office, candidate, faction) = {
                let mut txn = new_txn(&mut world, 1);
                let actor = txn.create_agent("Actor", ControlSource::Ai).unwrap();
                let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
                let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
                let candidate = txn.create_agent("Candidate", ControlSource::Ai).unwrap();
                let office = txn.create_office("Chair").unwrap();
                let faction = txn.create_faction("Ward").unwrap();
                let lot = txn
                    .create_item_lot(CommodityKind::Coin, Quantity(300))
                    .unwrap();

                for entity in [actor, target, observer, candidate] {
                    txn.set_ground_location(entity, place).unwrap();
                }
                txn.set_ground_location(lot, place).unwrap();
                txn.set_possessor(lot, actor).unwrap();
                txn.set_owner(lot, actor).unwrap();
                txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                    .unwrap();
                txn.set_component_perception_profile(
                    observer,
                    worldwake_core::PerceptionProfile {
                        observation_fidelity: pm(1000),
                        ..worldwake_core::PerceptionProfile::default()
                    },
                )
                .unwrap();
                txn.set_component_office_data(
                    office,
                    OfficeData {
                        title: "Chair".to_string(),
                        jurisdiction: place,
                        succession_law: SuccessionLaw::Support,
                        eligibility_rules: vec![EligibilityRule::FactionMember(faction)],
                        succession_period_ticks: 12,
                        vacancy_since: Some(Tick(1)),
                    },
                )
                .unwrap();
                let _ = create_record(&mut txn, place, actor, RecordKind::OfficeRegister);
                let _ = create_record(&mut txn, place, actor, RecordKind::SupportLedger);
                txn.add_member(candidate, faction).unwrap();
                txn.set_component_utility_profile(
                    target,
                    UtilityProfile {
                        courage: pm(400),
                        ..UtilityProfile::default()
                    },
                )
                .unwrap();
                txn.set_component_combat_profile(
                    actor,
                    CombatProfile::new(
                        pm(1000),
                        pm(700),
                        pm(650),
                        pm(500),
                        pm(50),
                        pm(25),
                        pm(20),
                        pm(100),
                        pm(25),
                        NonZeroU32::new(4).unwrap(),
                        NonZeroU32::new(10).unwrap(),
                    ),
                )
                .unwrap();
                let mut log = EventLog::new();
                let _ = txn.commit(&mut log);
                (actor, target, observer, office, candidate, faction)
            };

            Self {
                world,
                place,
                actor,
                target,
                observer,
                office,
                candidate,
                faction,
            }
        }
    }

    struct PayloadFixture {
        world: World,
        place: EntityId,
        actor: EntityId,
        target: EntityId,
    }

    impl PayloadFixture {
        fn new(with_combat_profile: bool) -> Self {
            let mut world = World::new(build_prototype_world()).unwrap();
            let place = world.topology().place_ids().next().unwrap();
            let (actor, target) = {
                let mut txn = new_txn(&mut world, 1);
                let actor = txn
                    .create_agent("Payload Actor", ControlSource::Ai)
                    .unwrap();
                let target = txn
                    .create_agent("Payload Target", ControlSource::Ai)
                    .unwrap();
                txn.set_ground_location(actor, place).unwrap();
                txn.set_ground_location(target, place).unwrap();
                txn.set_component_agent_belief_store(actor, AgentBeliefStore::new())
                    .unwrap();
                if with_combat_profile {
                    txn.set_component_combat_profile(
                        actor,
                        CombatProfile::new(
                            pm(1000),
                            pm(700),
                            pm(650),
                            pm(500),
                            pm(50),
                            pm(25),
                            pm(20),
                            pm(100),
                            pm(25),
                            NonZeroU32::new(4).unwrap(),
                            NonZeroU32::new(10).unwrap(),
                        ),
                    )
                    .unwrap();
                }
                let mut log = EventLog::new();
                let _ = txn.commit(&mut log);
                (actor, target)
            };

            Self {
                world,
                place,
                actor,
                target,
            }
        }

        fn view(&self) -> PerAgentBeliefView<'_> {
            PerAgentBeliefView::from_world(self.actor, &self.world)
        }

        fn give_actor_commodity(&mut self, commodity: CommodityKind, quantity: Quantity) {
            let mut txn = new_txn(&mut self.world, 2);
            let lot = txn.create_item_lot(commodity, quantity).unwrap();
            txn.set_ground_location(lot, self.place).unwrap();
            txn.set_owner(lot, self.actor).unwrap();
            txn.set_possessor(lot, self.actor).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        fn clear_actor_combat_profile(&mut self) {
            let mut txn = new_txn(&mut self.world, 2);
            txn.clear_component_combat_profile(self.actor).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
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

    fn run_perception(world: &mut World, event_log: &mut EventLog, tick: u64) {
        let mut rng = test_rng(0x44);
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();
        perception_system(SystemExecutionContext {
            world,
            event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            tick: Tick(tick),
            system_id: SystemId::Perception,
        })
        .unwrap();
    }

    #[test]
    fn register_office_actions_creates_social_defs() {
        let (defs, handlers, ids) = setup_registries();

        assert_eq!(handlers.len(), 3);
        assert_eq!(defs.get(ids[0]).unwrap().name, "bribe");
        assert_eq!(defs.get(ids[1]).unwrap().name, "threaten");
        assert_eq!(defs.get(ids[2]).unwrap().name, "declare_support");
        assert_eq!(
            defs.iter().map(|def| def.domain).collect::<Vec<_>>(),
            vec![
                worldwake_sim::ActionDomain::Social,
                worldwake_sim::ActionDomain::Social,
                worldwake_sim::ActionDomain::Social,
            ]
        );
    }

    #[test]
    fn bribe_payload_offers_full_stock() {
        let (defs, _handlers, ids) = setup_registries();
        let def = defs.get(ids[0]).unwrap();
        let mut fx = PayloadFixture::new(true);
        fx.give_actor_commodity(CommodityKind::Bread, Quantity(5));
        let view = fx.view();

        let payloads = super::enumerate_bribe_payloads(def, fx.actor, &[fx.target], &view);

        assert_eq!(payloads.len(), 1);
        assert_eq!(
            payloads[0],
            ActionPayload::Bribe(BribeActionPayload {
                target: fx.target,
                offered_commodity: CommodityKind::Bread,
                offered_quantity: Quantity(5),
            })
        );
    }

    #[test]
    fn bribe_payload_no_self_bribe() {
        let (defs, _handlers, ids) = setup_registries();
        let def = defs.get(ids[0]).unwrap();
        let mut fx = PayloadFixture::new(true);
        fx.give_actor_commodity(CommodityKind::Bread, Quantity(5));
        let view = fx.view();

        let payloads = super::enumerate_bribe_payloads(def, fx.actor, &[fx.actor], &view);

        assert!(payloads.is_empty());
    }

    #[test]
    fn bribe_payload_empty_without_commodities() {
        let (defs, _handlers, ids) = setup_registries();
        let def = defs.get(ids[0]).unwrap();
        let fx = PayloadFixture::new(true);
        let view = fx.view();

        let payloads = super::enumerate_bribe_payloads(def, fx.actor, &[fx.target], &view);

        assert!(payloads.is_empty());
    }

    #[test]
    fn bribe_payload_multiple_commodity_types() {
        let (defs, _handlers, ids) = setup_registries();
        let def = defs.get(ids[0]).unwrap();
        let mut fx = PayloadFixture::new(true);
        fx.give_actor_commodity(CommodityKind::Bread, Quantity(5));
        fx.give_actor_commodity(CommodityKind::Coin, Quantity(3));
        let view = fx.view();

        let payloads = super::enumerate_bribe_payloads(def, fx.actor, &[fx.target], &view);

        assert_eq!(payloads.len(), 2);
        assert!(payloads.contains(&ActionPayload::Bribe(BribeActionPayload {
            target: fx.target,
            offered_commodity: CommodityKind::Bread,
            offered_quantity: Quantity(5),
        })));
        assert!(payloads.contains(&ActionPayload::Bribe(BribeActionPayload {
            target: fx.target,
            offered_commodity: CommodityKind::Coin,
            offered_quantity: Quantity(3),
        })));
    }

    #[test]
    fn threaten_payload_emits_for_bound_target_with_combat_profile() {
        let (defs, _handlers, ids) = setup_registries();
        let def = defs.get(ids[1]).unwrap();
        let fx = PayloadFixture::new(true);
        let view = fx.view();

        let payloads = super::enumerate_threaten_payloads(def, fx.actor, &[fx.target], &view);

        assert_eq!(
            payloads,
            vec![ActionPayload::Threaten(ThreatenActionPayload {
                target: fx.target,
            })]
        );
    }

    #[test]
    fn threaten_payload_no_self_threaten() {
        let (defs, _handlers, ids) = setup_registries();
        let def = defs.get(ids[1]).unwrap();
        let fx = PayloadFixture::new(true);
        let view = fx.view();

        let payloads = super::enumerate_threaten_payloads(def, fx.actor, &[fx.actor], &view);

        assert!(payloads.is_empty());
    }

    #[test]
    fn threaten_payload_empty_without_targets() {
        let (defs, _handlers, ids) = setup_registries();
        let def = defs.get(ids[1]).unwrap();
        let fx = PayloadFixture::new(true);
        let view = fx.view();

        let payloads = super::enumerate_threaten_payloads(def, fx.actor, &[], &view);

        assert!(payloads.is_empty());
    }

    #[test]
    fn threaten_payload_requires_combat_profile() {
        let (defs, _handlers, ids) = setup_registries();
        let def = defs.get(ids[1]).unwrap();
        let mut fx = PayloadFixture::new(true);
        fx.clear_actor_combat_profile();
        let view = fx.view();

        let payloads = super::enumerate_threaten_payloads(def, fx.actor, &[fx.target], &view);

        assert!(payloads.is_empty());
    }

    #[test]
    fn bribe_commit_transfers_goods_increases_loyalty_and_preserves_conservation() {
        let (defs, handlers, ids) = setup_registries();
        let mut fx = SocialFixture::new();
        let instance = ActionInstance {
            instance_id: ActionInstanceId(0),
            def_id: ids[0],
            payload: ActionPayload::Bribe(BribeActionPayload {
                target: fx.target,
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(300),
            }),
            actor: fx.actor,
            targets: vec![fx.target],
            start_tick: Tick(3),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };

        let log = commit_action(&mut fx.world, &defs, &handlers, ids[0], &instance, 1, 3);
        let bribe_event_id = log.events_by_tag(EventTag::ActionCommitted)[0];
        let transferred_lot = fx
            .world
            .entities_with_item_lot()
            .find(|lot_id| {
                fx.world.owner_of(*lot_id) == Some(fx.target)
                    && fx
                        .world
                        .get_component_item_lot(*lot_id)
                        .is_some_and(|lot| lot.commodity == CommodityKind::Coin)
            })
            .unwrap();
        let provenance = fx
            .world
            .get_component_item_lot(transferred_lot)
            .unwrap()
            .provenance
            .last()
            .unwrap();

        assert_eq!(
            fx.world
                .controlled_commodity_quantity(fx.actor, CommodityKind::Coin),
            Quantity(0)
        );
        assert_eq!(
            fx.world
                .controlled_commodity_quantity(fx.target, CommodityKind::Coin),
            Quantity(300)
        );
        assert_eq!(fx.world.loyalty_to(fx.target, fx.actor), Some(pm(300)));
        assert_eq!(
            provenance.operation,
            worldwake_core::LotOperation::Transferred
        );
        assert_eq!(provenance.amount, Quantity(300));
        assert_eq!(provenance.event_id, Some(bribe_event_id));
        verify_live_lot_conservation(&fx.world, CommodityKind::Coin, 300).unwrap();
    }

    #[test]
    fn bribe_commit_rejects_when_actor_lacks_goods() {
        let (defs, handlers, ids) = setup_registries();
        let mut fx = SocialFixture::new();
        let def = defs.get(ids[0]).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let instance = ActionInstance {
            instance_id: ActionInstanceId(0),
            def_id: ids[0],
            payload: ActionPayload::Bribe(BribeActionPayload {
                target: fx.target,
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(301),
            }),
            actor: fx.actor,
            targets: vec![fx.target],
            start_tick: Tick(3),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };
        let mut txn = new_action_txn(&mut fx.world, fx.actor, 3);
        let mut rng = test_rng(2);

        let err = (handler.on_commit)(def, &instance, &mut rng, &mut txn).unwrap_err();
        assert!(matches!(
            err,
            ActionError::AbortRequested(
                ActionAbortRequestReason::HolderLacksAccessibleCommodity { .. }
            )
        ));
    }

    #[test]
    fn bribe_abort_has_no_side_effects() {
        let (defs, handlers, ids) = setup_registries();
        let mut fx = SocialFixture::new();
        let before_actor = fx
            .world
            .controlled_commodity_quantity(fx.actor, CommodityKind::Coin);
        let def = defs.get(ids[0]).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let instance = ActionInstance {
            instance_id: ActionInstanceId(0),
            def_id: ids[0],
            payload: ActionPayload::Bribe(BribeActionPayload {
                target: fx.target,
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(300),
            }),
            actor: fx.actor,
            targets: vec![fx.target],
            start_tick: Tick(3),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };
        let mut txn = new_action_txn(&mut fx.world, fx.actor, 3);
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
            fx.world
                .controlled_commodity_quantity(fx.actor, CommodityKind::Coin),
            before_actor
        );
        assert_eq!(fx.world.loyalty_to(fx.target, fx.actor), None);
    }

    #[test]
    fn threaten_commit_yield_increases_loyalty() {
        let (defs, handlers, ids) = setup_registries();
        let mut fx = SocialFixture::new();
        let instance = ActionInstance {
            instance_id: ActionInstanceId(1),
            def_id: ids[1],
            payload: ActionPayload::Threaten(ThreatenActionPayload { target: fx.target }),
            actor: fx.actor,
            targets: vec![fx.target],
            start_tick: Tick(3),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };

        let _ = commit_action(&mut fx.world, &defs, &handlers, ids[1], &instance, 4, 3);

        assert_eq!(fx.world.loyalty_to(fx.target, fx.actor), Some(pm(650)));
        assert_eq!(fx.world.hostile_towards(fx.actor), Vec::<EntityId>::new());
    }

    #[test]
    fn threaten_commit_resist_adds_hostility() {
        let (defs, handlers, ids) = setup_registries();
        let mut fx = SocialFixture::new();
        {
            let mut txn = new_txn(&mut fx.world, 2);
            txn.set_component_utility_profile(
                fx.target,
                UtilityProfile {
                    courage: pm(900),
                    ..UtilityProfile::default()
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = ActionInstance {
            instance_id: ActionInstanceId(1),
            def_id: ids[1],
            payload: ActionPayload::Threaten(ThreatenActionPayload { target: fx.target }),
            actor: fx.actor,
            targets: vec![fx.target],
            start_tick: Tick(3),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };

        let _ = commit_action(&mut fx.world, &defs, &handlers, ids[1], &instance, 5, 3);

        assert_eq!(fx.world.loyalty_to(fx.target, fx.actor), None);
        assert_eq!(fx.world.hostile_towards(fx.actor), vec![fx.target]);
    }

    #[test]
    fn declare_support_commit_sets_and_overwrites_support_declaration() {
        let (defs, handlers, ids) = setup_registries();
        let mut fx = SocialFixture::new();
        let first = ActionInstance {
            instance_id: ActionInstanceId(2),
            def_id: ids[2],
            payload: ActionPayload::DeclareSupport(DeclareSupportActionPayload {
                office: fx.office,
                candidate: fx.candidate,
            }),
            actor: fx.actor,
            targets: Vec::new(),
            start_tick: Tick(3),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };

        let second_candidate;
        {
            let mut txn = new_txn(&mut fx.world, 2);
            second_candidate = txn.create_agent("Second", ControlSource::Ai).unwrap();
            txn.set_ground_location(second_candidate, fx.place).unwrap();
            txn.add_member(second_candidate, fx.faction).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let second = ActionInstance {
            payload: ActionPayload::DeclareSupport(DeclareSupportActionPayload {
                office: fx.office,
                candidate: second_candidate,
            }),
            ..first.clone()
        };

        let _ = commit_action(&mut fx.world, &defs, &handlers, ids[2], &first, 6, 3);
        assert_eq!(
            fx.world.support_declaration(fx.actor, fx.office),
            Some(fx.candidate)
        );
        let after_first = record_at_place(&fx.world, fx.place, RecordKind::SupportLedger);
        assert_eq!(after_first.entries.len(), 1);

        let _ = commit_action(&mut fx.world, &defs, &handlers, ids[2], &second, 7, 4);
        assert_eq!(
            fx.world.support_declaration(fx.actor, fx.office),
            Some(second_candidate)
        );
        let after_second = record_at_place(&fx.world, fx.place, RecordKind::SupportLedger);
        assert_eq!(after_second.entries.len(), 2);
        assert_eq!(after_second.entries[1].supersedes, Some(after_second.entries[0].entry_id));
    }

    #[test]
    fn declare_support_requires_jurisdiction_place_and_vacancy() {
        let (defs, handlers, ids) = setup_registries();
        let mut fx = SocialFixture::new();
        let def = defs.get(ids[2]).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let payload = ActionPayload::DeclareSupport(DeclareSupportActionPayload {
            office: fx.office,
            candidate: fx.candidate,
        });

        {
            let other_place = fx
                .world
                .topology()
                .place_ids()
                .find(|place| *place != fx.place)
                .unwrap();
            let mut txn = new_txn(&mut fx.world, 2);
            txn.set_ground_location(fx.actor, other_place).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let err = (handler.authoritative_payload_is_valid)(
            def,
            &defs,
            fx.actor,
            &[],
            &payload,
            &fx.world,
        )
        .unwrap_err();
        assert!(matches!(err, ActionError::PreconditionFailed(_)));

        {
            let mut txn = new_txn(&mut fx.world, 3);
            txn.set_ground_location(fx.actor, fx.place).unwrap();
            let mut office = txn.get_component_office_data(fx.office).cloned().unwrap();
            office.vacancy_since = None;
            txn.set_component_office_data(fx.office, office).unwrap();
            txn.assign_office(fx.office, fx.candidate).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let err = (handler.authoritative_payload_is_valid)(
            def,
            &defs,
            fx.actor,
            &[],
            &payload,
            &fx.world,
        )
        .unwrap_err();
        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn declare_support_rejects_force_law_offices() {
        let (defs, handlers, ids) = setup_registries();
        let mut fx = SocialFixture::new();
        {
            let mut txn = new_txn(&mut fx.world, 2);
            let mut office = txn.get_component_office_data(fx.office).cloned().unwrap();
            office.succession_law = SuccessionLaw::Force;
            txn.set_component_office_data(fx.office, office).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        let declare_payload = ActionPayload::DeclareSupport(DeclareSupportActionPayload {
            office: fx.office,
            candidate: fx.candidate,
        });
        let def = defs.get(ids[2]).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let err = (handler.authoritative_payload_is_valid)(
            def,
            &defs,
            fx.actor,
            &[],
            &declare_payload,
            &fx.world,
        )
        .unwrap_err();
        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn bribe_event_records_witnessed_obligation() {
        let (defs, handlers, ids) = setup_registries();
        let mut fx = SocialFixture::new();
        let instance = ActionInstance {
            instance_id: ActionInstanceId(3),
            def_id: ids[0],
            payload: ActionPayload::Bribe(BribeActionPayload {
                target: fx.target,
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(300),
            }),
            actor: fx.actor,
            targets: vec![fx.target],
            start_tick: Tick(3),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };

        let mut log = commit_action(&mut fx.world, &defs, &handlers, ids[0], &instance, 8, 3);
        run_perception(&mut fx.world, &mut log, 3);

        assert!(fx
            .world
            .get_component_agent_belief_store(fx.observer)
            .unwrap()
            .social_observations
            .iter()
            .any(|observation| observation.kind
                == worldwake_core::SocialObservationKind::WitnessedObligation
                && observation.subjects == (fx.actor, fx.target)));
    }

    #[test]
    fn threaten_event_records_witnessed_conflict() {
        let (defs, handlers, ids) = setup_registries();
        let mut fx = SocialFixture::new();
        let instance = ActionInstance {
            instance_id: ActionInstanceId(4),
            def_id: ids[1],
            payload: ActionPayload::Threaten(ThreatenActionPayload { target: fx.target }),
            actor: fx.actor,
            targets: vec![fx.target],
            start_tick: Tick(3),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };

        let mut log = commit_action(&mut fx.world, &defs, &handlers, ids[1], &instance, 9, 3);
        run_perception(&mut fx.world, &mut log, 3);

        assert!(fx
            .world
            .get_component_agent_belief_store(fx.observer)
            .unwrap()
            .social_observations
            .iter()
            .any(|observation| observation.kind
                == worldwake_core::SocialObservationKind::WitnessedConflict
                && observation.subjects == (fx.actor, fx.target)));
    }

    #[test]
    fn declare_support_event_records_witnessed_cooperation() {
        let (defs, handlers, ids) = setup_registries();
        let mut fx = SocialFixture::new();
        let instance = ActionInstance {
            instance_id: ActionInstanceId(5),
            def_id: ids[2],
            payload: ActionPayload::DeclareSupport(DeclareSupportActionPayload {
                office: fx.office,
                candidate: fx.candidate,
            }),
            actor: fx.actor,
            targets: Vec::new(),
            start_tick: Tick(3),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };

        let mut log = commit_action(&mut fx.world, &defs, &handlers, ids[2], &instance, 10, 3);
        run_perception(&mut fx.world, &mut log, 3);

        assert!(fx
            .world
            .get_component_agent_belief_store(fx.observer)
            .unwrap()
            .social_observations
            .iter()
            .any(|observation| observation.kind
                == worldwake_core::SocialObservationKind::WitnessedCooperation
                && observation.subjects == (fx.actor, fx.candidate)));
    }

    #[test]
    fn committed_events_include_expected_tags() {
        let (defs, handlers, ids) = setup_registries();
        let mut fx = SocialFixture::new();
        let bribe = ActionInstance {
            instance_id: ActionInstanceId(6),
            def_id: ids[0],
            payload: ActionPayload::Bribe(BribeActionPayload {
                target: fx.target,
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(300),
            }),
            actor: fx.actor,
            targets: vec![fx.target],
            start_tick: Tick(3),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };
        let threaten = ActionInstance {
            instance_id: ActionInstanceId(7),
            def_id: ids[1],
            payload: ActionPayload::Threaten(ThreatenActionPayload { target: fx.target }),
            actor: fx.actor,
            targets: vec![fx.target],
            start_tick: Tick(4),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };
        let declare_support = ActionInstance {
            instance_id: ActionInstanceId(8),
            def_id: ids[2],
            payload: ActionPayload::DeclareSupport(DeclareSupportActionPayload {
                office: fx.office,
                candidate: fx.candidate,
            }),
            actor: fx.actor,
            targets: Vec::new(),
            start_tick: Tick(5),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };

        let bribe_log = commit_action(&mut fx.world, &defs, &handlers, ids[0], &bribe, 11, 3);
        let threaten_log = commit_action(&mut fx.world, &defs, &handlers, ids[1], &threaten, 12, 4);
        let support_log = commit_action(
            &mut fx.world,
            &defs,
            &handlers,
            ids[2],
            &declare_support,
            13,
            5,
        );

        let bribe_record = bribe_log
            .get(bribe_log.events_by_tag(EventTag::ActionCommitted)[0])
            .unwrap();
        assert!(bribe_record.tags().contains(&EventTag::Social));
        assert!(bribe_record.tags().contains(&EventTag::Transfer));

        let threaten_record = threaten_log
            .get(threaten_log.events_by_tag(EventTag::ActionCommitted)[0])
            .unwrap();
        assert!(threaten_record.tags().contains(&EventTag::Social));
        assert!(threaten_record.tags().contains(&EventTag::Coercion));

        let support_record = support_log
            .get(support_log.events_by_tag(EventTag::ActionCommitted)[0])
            .unwrap();
        assert!(support_record.tags().contains(&EventTag::Political));
    }
}
