use std::collections::BTreeSet;
use std::num::NonZeroU32;
use worldwake_core::{
    load_of_entity, load_per_unit, ActionDefId, BodyCostPerTick, ContentionGrant, ContentionPolicy,
    ContentionQueue, EntityId, EntityKind, EventTag, Quantity, VisibilitySpec, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerRegistry,
    ActionInstance, ActionPayload, ActionProgress, CommitOutcome, Constraint, DeterministicRng,
    DurationExpr, Interruptibility, Materialization, MaterializationTag, Precondition, TargetSpec,
    TransportActionPayload,
};

use crate::evidence_support::emit_evidence;
use crate::inventory::{move_entity_to_direct_possession, remaining_capacity};

#[allow(clippy::too_many_lines)]
pub fn register_transport_actions(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> Vec<ActionDefId> {
    let pick_up_handler = handlers.register(
        ActionHandler::new(
            start_pick_up,
            tick_transport,
            commit_pick_up,
            abort_transport,
        )
        .with_payload_override_validator(validate_pick_up_payload_override),
    );
    let put_down_handler = handlers.register(ActionHandler::new(
        start_put_down,
        tick_transport,
        commit_put_down,
        abort_transport,
    ));
    let steal_handler = handlers.register(ActionHandler::new(
        start_steal,
        tick_transport,
        commit_steal,
        abort_transport,
    ));

    let pick_up_id = ActionDefId(defs.len() as u32);
    let put_down_id = ActionDefId(pick_up_id.0 + 1);
    let steal_id = ActionDefId(put_down_id.0 + 1);

    vec![
        defs.register(ActionDef {
            id: pick_up_id,
            name: "pick_up".to_string(),
            domain: worldwake_core::ActionDomain::Transport,
            actor_constraints: vec![Constraint::ActorAlive, Constraint::ActorHasControl],
            targets: vec![TargetSpec::EntityAtActorPlaceAnyOf {
                kinds: [EntityKind::ItemLot, EntityKind::UniqueItem],
            }],
            preconditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
                Precondition::TargetNotInContainer(0),
                Precondition::TargetUnpossessed(0),
                Precondition::TargetUnownedOrActorControls(0),
            ],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::MIN),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::InterruptibleWithPenalty,
            commit_conditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
                Precondition::TargetNotInContainer(0),
                Precondition::TargetUnpossessed(0),
                Precondition::TargetUnownedOrActorControls(0),
            ],
            visibility: VisibilitySpec::ParticipantsOnly,
            causal_event_tags: BTreeSet::from([
                EventTag::WorldMutation,
                EventTag::Inventory,
                EventTag::Transfer,
            ]),
            payload: ActionPayload::None,
            handler: pick_up_handler,
        }),
        defs.register(ActionDef {
            id: put_down_id,
            name: "put_down".to_string(),
            domain: worldwake_core::ActionDomain::Transport,
            actor_constraints: vec![Constraint::ActorAlive, Constraint::ActorHasControl],
            targets: vec![TargetSpec::EntityDirectlyPossessedByActorAnyOf {
                kinds: [EntityKind::ItemLot, EntityKind::UniqueItem],
            }],
            preconditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
                Precondition::TargetDirectlyPossessedByActor(0),
            ],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::MIN),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::InterruptibleWithPenalty,
            commit_conditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
                Precondition::TargetDirectlyPossessedByActor(0),
            ],
            visibility: VisibilitySpec::ParticipantsOnly,
            causal_event_tags: BTreeSet::from([
                EventTag::WorldMutation,
                EventTag::Inventory,
                EventTag::Transfer,
            ]),
            payload: ActionPayload::None,
            handler: put_down_handler,
        }),
        defs.register(ActionDef {
            id: steal_id,
            name: "steal".to_string(),
            domain: worldwake_core::ActionDomain::Transport,
            actor_constraints: vec![Constraint::ActorAlive, Constraint::ActorNotIncapacitated],
            targets: vec![TargetSpec::EntityAtActorPlace {
                kind: EntityKind::ItemLot,
            }],
            preconditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
                Precondition::TargetKind {
                    target_index: 0,
                    kind: EntityKind::ItemLot,
                },
                Precondition::TargetUnpossessed(0),
            ],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::ActorTheftDisposition,
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
                Precondition::TargetKind {
                    target_index: 0,
                    kind: EntityKind::ItemLot,
                },
                Precondition::TargetUnpossessed(0),
            ],
            visibility: VisibilitySpec::Hidden,
            causal_event_tags: BTreeSet::from([EventTag::Crime, EventTag::Transfer]),
            payload: ActionPayload::None,
            handler: steal_handler,
        }),
    ]
}

fn require_transport_target(instance: &ActionInstance) -> Result<EntityId, ActionError> {
    instance
        .targets
        .first()
        .copied()
        .ok_or(ActionError::InvalidTarget(instance.actor))
}

fn unique_item_pickup_contention_policy() -> ContentionPolicy {
    ContentionPolicy {
        grant_hold_ticks: NonZeroU32::new(3).unwrap(),
        auto_promote: false,
        max_waiters: Some(0),
    }
}

fn is_transport_ground_pickup_kind(kind: EntityKind) -> bool {
    matches!(kind, EntityKind::ItemLot | EntityKind::UniqueItem)
}

fn is_transport_direct_possession_kind(kind: EntityKind) -> bool {
    matches!(kind, EntityKind::ItemLot | EntityKind::UniqueItem)
}

fn unique_item_pickup_contention_eligible(txn: &WorldTxn<'_>, target: EntityId) -> bool {
    txn.entity_kind(target) == Some(EntityKind::UniqueItem)
        && txn.effective_place(target).is_some()
        && txn.direct_container(target).is_none()
        && txn.possessor_of(target).is_none()
        && txn.owner_of(target).is_none()
}

fn ensure_unique_item_pickup_contention_components(
    txn: &mut WorldTxn<'_>,
    target: EntityId,
) -> Result<(), ActionError> {
    if txn.entity_kind(target) != Some(EntityKind::UniqueItem) {
        return Ok(());
    }
    match (
        txn.get_component_contention_policy(target),
        txn.get_component_contention_queue(target),
    ) {
        (None, None) => {
            if !unique_item_pickup_contention_eligible(txn, target) {
                return Ok(());
            }
            txn.set_component_contention_policy(target, unique_item_pickup_contention_policy())
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
            txn.set_component_contention_queue(target, ContentionQueue::default())
                .map_err(|err| ActionError::InternalError(err.to_string()))
        }
        (Some(_), Some(_)) => Ok(()),
        (Some(_), None) => Err(ActionError::PreconditionFailed(format!(
            "unique item {target} lacks ContentionQueue grant state"
        ))),
        (None, Some(_)) => Err(ActionError::PreconditionFailed(format!(
            "unique item {target} lacks ContentionPolicy"
        ))),
    }
}

fn claim_or_require_unique_item_pickup_grant(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    target: EntityId,
    action_def: ActionDefId,
    claim_if_absent: bool,
) -> Result<(), ActionError> {
    if txn.entity_kind(target) != Some(EntityKind::UniqueItem) {
        return Ok(());
    }
    ensure_unique_item_pickup_contention_components(txn, target)?;
    let policy = txn
        .get_component_contention_policy(target)
        .cloned()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("unique item {target} lacks ContentionPolicy"))
        })?;
    let mut queue = txn
        .get_component_contention_queue(target)
        .cloned()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("unique item {target} lacks ContentionQueue"))
        })?;

    match queue.granted.as_ref() {
        Some(granted) if granted.actor == actor && granted.intended_action == action_def => {
            return Ok(());
        }
        Some(_) => {
            return Err(ActionError::PreconditionFailed(
                "contention_rejected".to_string(),
            ));
        }
        None if !claim_if_absent => {
            return Err(ActionError::PreconditionFailed(
                "contention_rejected".to_string(),
            ));
        }
        None => {}
    }

    queue.granted = Some(ContentionGrant {
        actor,
        intended_action: action_def,
        granted_at: txn.tick(),
        expires_at: txn.tick() + u64::from(policy.grant_hold_ticks.get()),
    });
    txn.set_component_contention_queue(target, queue)
        .map_err(|err| ActionError::InternalError(err.to_string()))
}

fn clear_unique_item_pickup_grant(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    target: EntityId,
    action_def: ActionDefId,
    detach_when_ineligible: bool,
) -> Result<(), ActionError> {
    if txn.entity_kind(target) != Some(EntityKind::UniqueItem) {
        return Ok(());
    }
    if let Some(mut queue) = txn.get_component_contention_queue(target).cloned() {
        if queue
            .granted
            .as_ref()
            .is_some_and(|granted| granted.actor == actor && granted.intended_action == action_def)
        {
            queue.clear_grant();
            txn.set_component_contention_queue(target, queue)
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
        }
    }
    if detach_when_ineligible && !unique_item_pickup_contention_eligible(txn, target) {
        if txn.get_component_contention_queue(target).is_some() {
            txn.clear_component_contention_queue(target)
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
        }
        if txn.get_component_contention_policy(target).is_some() {
            txn.clear_component_contention_policy(target)
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
        }
    }
    Ok(())
}

fn validate_pick_up(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    target: EntityId,
    requested_quantity: Option<Quantity>,
) -> Result<(), ActionError> {
    let actor_place = txn
        .effective_place(actor)
        .ok_or_else(|| ActionError::PreconditionFailed(format!("actor {actor} has no place")))?;
    if txn.effective_place(target) != Some(actor_place) {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} is not at actor {actor} place {actor_place}"
        )));
    }
    let kind = txn
        .entity_kind(target)
        .ok_or(ActionError::InvalidTarget(target))?;
    if !is_transport_ground_pickup_kind(kind) {
        return Err(ActionError::InvalidTarget(target));
    }
    if txn.direct_container(target).is_some() {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} is inside a container"
        )));
    }
    if txn.possessor_of(target).is_some() {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} is already possessed"
        )));
    }
    // Ownership check: actor can pick up only if unowned or actor can exercise control
    if txn.owner_of(target).is_some() {
        txn.can_exercise_control(actor, target).map_err(|e| {
            ActionError::PreconditionFailed(format!(
                "actor {actor} cannot lawfully pick up owned entity {target}: {e}"
            ))
        })?;
    }
    match kind {
        EntityKind::ItemLot => {
            let lot = txn
                .get_component_item_lot(target)
                .ok_or(ActionError::InvalidTarget(target))?;
            let per_unit = load_per_unit(lot.commodity).0;
            let remaining = remaining_capacity(txn, actor)?.0;
            if remaining < per_unit {
                return Err(ActionError::PreconditionFailed(format!(
                    "actor {actor} has insufficient carry capacity for any {:?}",
                    lot.commodity
                )));
            }
            if let Some(quantity) = requested_quantity {
                let max_quantity = Quantity((remaining / per_unit).min(lot.quantity.0));
                if quantity == Quantity(0) || quantity > max_quantity {
                    return Err(ActionError::PreconditionFailed(format!(
                        "requested pickup quantity {quantity:?} exceeds available movable quantity {max_quantity:?}",
                    )));
                }
            }
        }
        EntityKind::UniqueItem => {
            if requested_quantity.is_some() {
                return Err(ActionError::PreconditionFailed(
                    "unique item pick_up does not accept quantity override".to_string(),
                ));
            }
            let remaining = remaining_capacity(txn, actor)?.0;
            let load = load_of_entity(txn, target)
                .map_err(|err| ActionError::InternalError(err.to_string()))?
                .0;
            if remaining < load {
                return Err(ActionError::PreconditionFailed(format!(
                    "actor {actor} has insufficient carry capacity for target {target}"
                )));
            }
        }
        _ => return Err(ActionError::InvalidTarget(target)),
    }
    Ok(())
}

fn execute_pick_up(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    target: EntityId,
    requested_quantity: Option<Quantity>,
) -> Result<EntityId, ActionError> {
    let actor_place = txn
        .effective_place(actor)
        .ok_or_else(|| ActionError::PreconditionFailed(format!("actor {actor} has no place")))?;
    match txn.entity_kind(target) {
        Some(EntityKind::ItemLot) => {
            let lot = txn
                .get_component_item_lot(target)
                .cloned()
                .ok_or(ActionError::InvalidTarget(target))?;
            let remaining = remaining_capacity(txn, actor)?.0;
            let per_unit = load_per_unit(lot.commodity).0;
            let requested_quantity =
                requested_quantity.unwrap_or(Quantity((remaining / per_unit).min(lot.quantity.0)));
            let moved_entity = if load_of_entity(txn, target)
                .map_err(|err| ActionError::InternalError(err.to_string()))?
                .0
                <= remaining
                && requested_quantity == lot.quantity
            {
                target
            } else {
                let (_, split_off) = txn
                    .split_lot(target, requested_quantity)
                    .map_err(|err| ActionError::InternalError(err.to_string()))?;
                split_off
            };

            move_entity_to_direct_possession(txn, moved_entity, actor, actor_place)?;
            Ok(moved_entity)
        }
        Some(EntityKind::UniqueItem) => {
            move_entity_to_direct_possession(txn, target, actor, actor_place)?;
            Ok(target)
        }
        _ => Err(ActionError::InvalidTarget(target)),
    }
}

fn validate_put_down(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    target: EntityId,
) -> Result<EntityId, ActionError> {
    let actor_place = txn
        .effective_place(actor)
        .ok_or_else(|| ActionError::PreconditionFailed(format!("actor {actor} has no place")))?;
    if txn.effective_place(target) != Some(actor_place) {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} is not at actor {actor} place {actor_place}"
        )));
    }
    let kind = txn
        .entity_kind(target)
        .ok_or(ActionError::InvalidTarget(target))?;
    if !is_transport_direct_possession_kind(kind) {
        return Err(ActionError::InvalidTarget(target));
    }
    if txn.possessor_of(target) != Some(actor) {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} does not directly possess target {target}"
        )));
    }
    Ok(actor_place)
}

fn validate_steal(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    target: EntityId,
) -> Result<(), ActionError> {
    let actor_place = txn
        .effective_place(actor)
        .ok_or_else(|| ActionError::PreconditionFailed(format!("actor {actor} has no place")))?;
    if txn.effective_place(target) != Some(actor_place) {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} is not at actor {actor} place {actor_place}"
        )));
    }
    if txn.entity_kind(target) != Some(EntityKind::ItemLot) {
        return Err(ActionError::InvalidTarget(target));
    }
    let owner = txn.owner_of(target).ok_or_else(|| {
        ActionError::PreconditionFailed(format!("target {target} is unowned and not stealable"))
    })?;
    if owner == actor {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} already owns target {target}"
        )));
    }
    if txn.can_exercise_control(actor, target).is_ok() {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} can lawfully control target {target}; use pick_up instead"
        )));
    }
    if txn.get_component_theft_disposition_profile(actor).is_none() {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} lacks TheftDispositionProfile"
        )));
    }
    if txn.possessor_of(target).is_some() {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} is already possessed"
        )));
    }
    if !txn.reservations_for(target).is_empty() {
        return Err(ActionError::PreconditionFailed(format!(
            "target {target} is reserved"
        )));
    }
    let remaining = remaining_capacity(txn, actor)?.0;
    let load = load_of_entity(txn, target)
        .map_err(|err| ActionError::InternalError(err.to_string()))?
        .0;
    if remaining < load {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} has insufficient carry capacity for target {target}"
        )));
    }
    Ok(())
}

fn start_pick_up(
    def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<worldwake_sim::ActionState>, ActionError> {
    let target = require_transport_target(instance)?;
    validate_pick_up(
        txn,
        instance.actor,
        target,
        requested_pick_up_quantity(&instance.payload)?,
    )?;
    claim_or_require_unique_item_pickup_grant(txn, instance.actor, target, def.id, true)?;
    Ok(None)
}

fn commit_pick_up(
    def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let target = require_transport_target(instance)?;
    validate_pick_up(
        txn,
        instance.actor,
        target,
        requested_pick_up_quantity(&instance.payload)?,
    )?;
    claim_or_require_unique_item_pickup_grant(txn, instance.actor, target, def.id, false)?;
    let moved_entity = execute_pick_up(
        txn,
        instance.actor,
        target,
        requested_pick_up_quantity(&instance.payload)?,
    )?;
    clear_unique_item_pickup_grant(txn, instance.actor, target, def.id, true)?;
    if moved_entity == target {
        Ok(CommitOutcome::empty())
    } else {
        Ok(CommitOutcome {
            materializations: vec![Materialization {
                tag: MaterializationTag::SplitOffLot,
                entity: moved_entity,
            }],
            trace: None,
        })
    }
}

fn start_put_down(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<worldwake_sim::ActionState>, ActionError> {
    validate_put_down(txn, instance.actor, require_transport_target(instance)?)?;
    Ok(None)
}

fn commit_put_down(
    _def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let target = require_transport_target(instance)?;
    let actor_place = validate_put_down(txn, instance.actor, target)?;
    txn.clear_possessor(target)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.set_ground_location(target, actor_place)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    ensure_unique_item_pickup_contention_components(txn, target)?;
    txn.add_target(target);
    Ok(CommitOutcome::empty())
}

fn start_steal(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<worldwake_sim::ActionState>, ActionError> {
    validate_steal(txn, instance.actor, require_transport_target(instance)?)?;
    Ok(None)
}

fn commit_steal(
    _def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let target = require_transport_target(instance)?;
    validate_steal(txn, instance.actor, target)?;
    let target_container = txn.direct_container(target);
    let actor_place = txn.effective_place(instance.actor).ok_or_else(|| {
        ActionError::PreconditionFailed(format!("actor {} has no place", instance.actor))
    })?;
    if txn.get_component_stock_assignment(target).is_some() {
        let _ = txn.clear_component_stock_assignment(target);
        let _ = txn.clear_component_sale_listing(target);
    }
    move_entity_to_direct_possession(txn, target, instance.actor, actor_place)?;
    if let Some(container) = target_container {
        let current_tick = txn.tick();
        emit_evidence(
            txn,
            actor_place,
            worldwake_core::EvidenceKind::ContainerTampered {
                container,
                tampered_at: current_tick,
            },
            200,
        )
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
        emit_evidence(
            txn,
            actor_place,
            worldwake_core::EvidenceKind::DisturbanceMarker {
                place: actor_place,
                kind: worldwake_core::DisturbanceKind::ForcedEntry,
                created_at: current_tick,
            },
            50,
        )
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn tick_transport(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

#[allow(clippy::unnecessary_wraps)]
fn abort_transport(
    def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _reason: &AbortReason,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    if def.name == "pick_up" {
        if let Some(target) = instance.targets.first().copied() {
            clear_unique_item_pickup_grant(txn, instance.actor, target, def.id, false)?;
        }
    }
    Ok(())
}

fn requested_pick_up_quantity(payload: &ActionPayload) -> Result<Option<Quantity>, ActionError> {
    match payload {
        ActionPayload::None => Ok(None),
        ActionPayload::Transport(TransportActionPayload { quantity }) => Ok(Some(*quantity)),
        _ => Err(ActionError::PreconditionFailed(
            "pick_up received non-transport payload".to_string(),
        )),
    }
}

fn validate_pick_up_payload_override(
    def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn worldwake_sim::RuntimeBeliefView,
) -> bool {
    if def.name != "pick_up" {
        return false;
    }
    let Some(TransportActionPayload { quantity }) = payload.as_transport() else {
        return false;
    };
    if *quantity == Quantity(0) {
        return false;
    }
    let Some(target) = targets.first().copied() else {
        return false;
    };
    if view.entity_kind(target) == Some(EntityKind::UniqueItem) {
        return false;
    }
    let Some(commodity) = view.item_lot_commodity(target) else {
        return false;
    };
    let lot_quantity = view.commodity_quantity(target, commodity);
    let Some(carry_capacity) = view.carry_capacity(actor) else {
        return false;
    };
    let Some(load) = view.load_of_entity(actor) else {
        return false;
    };
    let per_unit = load_per_unit(commodity).0;
    let max_quantity =
        Quantity((carry_capacity.0.saturating_sub(load.0) / per_unit).min(lot_quantity.0));
    *quantity <= max_quantity
}

#[cfg(test)]
mod tests {
    use super::register_transport_actions;
    use std::collections::BTreeMap;
    use worldwake_core::{
        build_believed_entity_state, build_prototype_world, verify_live_lot_conservation,
        AgentBeliefStore, CarryCapacity, CauseRef, CommodityKind, Container, ControlSource,
        DisturbanceKind, EventLog, EventView, EvidenceEntry, EvidenceEntryId, EvidenceKind,
        LoadUnits, PerceptionSource, Place, Quantity, SaleListing, Seed, StockAssignment,
        StockAssignmentKind, Tick, Topology, TravelEdge, TravelEdgeId, UniqueItemKind,
        VisibilitySpec, WitnessData, World, WorldTxn,
    };
    use worldwake_sim::{
        get_affordances, start_action, tick_action, ActionDefRegistry, ActionExecutionAuthority,
        ActionHandlerRegistry, ActionInstance, ActionInstanceId, DeterministicRng,
        PerAgentBeliefView, TickOutcome,
    };

    use super::*;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn transport_topology() -> Topology {
        let mut topology = Topology::new();
        for (slot, name) in [(1, "Square"), (2, "Storehouse"), (3, "Field")] {
            topology
                .add_place(
                    entity(slot),
                    Place {
                        name: name.to_string(),
                        capacity: None,
                        tags: BTreeSet::new(),
                    },
                )
                .unwrap();
        }
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(10), entity(1), entity(2), 2, None).unwrap())
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(11), entity(2), entity(1), 2, None).unwrap())
            .unwrap();
        topology
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
        DeterministicRng::new(Seed([0x73; 32]))
    }

    fn test_belief_store(world: &World, actor: EntityId) -> AgentBeliefStore {
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

    fn setup_world() -> (World, EntityId, EntityId, EntityId, EntityId) {
        let mut world = World::new(transport_topology()).unwrap();
        let place = entity(1);
        let other_place = entity(2);
        let (actor, lot) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(3))
                .unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(lot, place).unwrap();
            txn.set_component_carry_capacity(actor, CarryCapacity(LoadUnits(4)))
                .unwrap();
            commit_txn(txn);
            (actor, lot)
        };
        (world, actor, lot, place, other_place)
    }

    fn setup_registries() -> (
        ActionDefRegistry,
        ActionHandlerRegistry,
        ActionDefId,
        ActionDefId,
        ActionDefId,
    ) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let ids = register_transport_actions(&mut defs, &mut handlers);
        (defs, handlers, ids[0], ids[1], ids[2])
    }

    #[allow(clippy::too_many_arguments)]
    fn start_action_for_target(
        world: &mut World,
        log: &mut EventLog,
        active_actions: &mut BTreeMap<ActionInstanceId, ActionInstance>,
        rng: &mut DeterministicRng,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        actor: EntityId,
        target: EntityId,
    ) -> ActionInstanceId {
        let affordance = affordances_for(world, actor, defs, handlers)
            .into_iter()
            .find(|affordance| affordance.bound_targets == vec![target])
            .unwrap();
        let mut next_instance_id = ActionInstanceId(1);
        start_action(
            &affordance,
            defs,
            handlers,
            ActionExecutionAuthority {
                active_actions,
                world,
                event_log: log,
                rng,
            },
            &mut next_instance_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap()
    }

    #[test]
    fn register_transport_actions_creates_pick_up_put_down_and_steal_defs() {
        let (defs, _, pick_up_id, put_down_id, steal_id) = setup_registries();
        let pick_up = defs.get(pick_up_id).unwrap();
        let put_down = defs.get(put_down_id).unwrap();
        let steal = defs.get(steal_id).unwrap();

        assert_eq!(pick_up.name, "pick_up");
        assert_eq!(put_down.name, "put_down");
        assert_eq!(steal.name, "steal");
        assert_eq!(
            pick_up.targets,
            vec![TargetSpec::EntityAtActorPlaceAnyOf {
                kinds: [EntityKind::ItemLot, EntityKind::UniqueItem],
            }]
        );
        assert_eq!(
            put_down.targets,
            vec![TargetSpec::EntityDirectlyPossessedByActorAnyOf {
                kinds: [EntityKind::ItemLot, EntityKind::UniqueItem],
            }]
        );
        assert!(pick_up
            .preconditions
            .contains(&Precondition::TargetNotInContainer(0)));
        assert!(pick_up
            .preconditions
            .contains(&Precondition::TargetUnpossessed(0)));
        assert!(put_down
            .preconditions
            .contains(&Precondition::TargetDirectlyPossessedByActor(0)));
        assert!(!steal
            .preconditions
            .contains(&Precondition::TargetNotInContainer(0)));
        assert_eq!(steal.duration, DurationExpr::ActorTheftDisposition);
        assert_eq!(steal.visibility, VisibilitySpec::Hidden);
        assert!(steal.causal_event_tags.contains(&EventTag::Crime));
    }

    #[test]
    fn pick_up_unique_item_claims_race_grant_and_rejects_second_claimant() {
        let mut world = World::new(transport_topology()).unwrap();
        let place = entity(1);
        let (actor_a, actor_b, item) = {
            let mut txn = new_txn(&mut world, 1);
            let actor_a = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let actor_b = txn.create_agent("Briar", ControlSource::Ai).unwrap();
            let item = txn
                .create_unique_item(UniqueItemKind::Artifact, Some("Seal"), BTreeMap::new())
                .unwrap();
            for actor in [actor_a, actor_b] {
                txn.set_ground_location(actor, place).unwrap();
                txn.set_component_carry_capacity(actor, CarryCapacity(LoadUnits(10)))
                    .unwrap();
            }
            txn.set_ground_location(item, place).unwrap();
            commit_txn(txn);
            (actor_a, actor_b, item)
        };
        let (defs, handlers, pick_up_id, _, _) = setup_registries();
        let affordance_a = affordances_for(&world, actor_a, &defs, &handlers)
            .into_iter()
            .find(|affordance| {
                affordance.def_id == pick_up_id && affordance.bound_targets == vec![item]
            })
            .expect("ground unique item should expose pick_up");
        let affordance_b = affordances_for(&world, actor_b, &defs, &handlers)
            .into_iter()
            .find(|affordance| {
                affordance.def_id == pick_up_id && affordance.bound_targets == vec![item]
            })
            .expect("ground unique item should expose pick_up for second actor");
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng();

        let instance_id = start_action(
            &affordance_a,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_instance_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap();

        let policy = world
            .get_component_contention_policy(item)
            .expect("starting unique-item pickup should materialize contention policy");
        assert!(!policy.auto_promote);
        assert_eq!(policy.max_waiters, Some(0));
        let grant = world
            .get_component_contention_queue(item)
            .and_then(|queue| queue.granted.as_ref())
            .expect("starting unique-item pickup should claim a race grant");
        assert_eq!(grant.actor, actor_a);
        assert_eq!(grant.intended_action, pick_up_id);

        let err = start_action(
            &affordance_b,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_instance_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap_err();
        assert!(
            matches!(err, ActionError::PreconditionFailed(message) if message == "contention_rejected")
        );

        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(world.possessor_of(item), Some(actor_a));
        assert_eq!(world.get_component_contention_policy(item), None);
        assert_eq!(world.get_component_contention_queue(item), None);
    }

    #[test]
    fn put_down_unique_item_attaches_race_mode_contention_state() {
        let mut world = World::new(transport_topology()).unwrap();
        let place = entity(1);
        let (actor, item) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let item = txn
                .create_unique_item(UniqueItemKind::Misc, Some("Token"), BTreeMap::new())
                .unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(item, place).unwrap();
            txn.set_component_carry_capacity(actor, CarryCapacity(LoadUnits(10)))
                .unwrap();
            txn.set_possessor(item, actor).unwrap();
            commit_txn(txn);
            (actor, item)
        };
        let (defs, handlers, _, put_down_id, _) = setup_registries();
        let affordance = affordances_for(&world, actor, &defs, &handlers)
            .into_iter()
            .find(|affordance| {
                affordance.def_id == put_down_id && affordance.bound_targets == vec![item]
            })
            .expect("possessed unique item should expose put_down");
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng();

        let instance_id = start_action(
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap();
        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(world.possessor_of(item), None);
        let policy = world
            .get_component_contention_policy(item)
            .expect("put_down should attach contention policy for unowned ground unique item");
        assert_eq!(policy.max_waiters, Some(0));
        assert!(!policy.auto_promote);
        assert_eq!(
            world.get_component_contention_queue(item),
            Some(&ContentionQueue::default())
        );
    }

    #[test]
    fn pick_up_happy_path_moves_lot_into_actor_possession_and_emits_tags() {
        let (mut world, actor, lot, place, _) = setup_world();
        let (defs, handlers, _, _, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();

        let instance_id = start_action_for_target(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            lot,
        );

        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();

        assert_eq!(
            outcome,
            TickOutcome::Committed {
                outcome: CommitOutcome::empty(),
            }
        );
        assert_eq!(world.possessor_of(lot), Some(actor));
        assert_eq!(world.owner_of(lot), None);
        assert_eq!(world.effective_place(lot), Some(place));

        let record = log
            .get(log.events_by_tag(EventTag::ActionCommitted)[0])
            .unwrap();
        assert!(record.tags().contains(&EventTag::Inventory));
        assert!(record.tags().contains(&EventTag::Transfer));
    }

    #[test]
    fn pick_up_fails_when_target_not_colocated() {
        let (mut world, actor, lot, _, other_place) = setup_world();
        let (defs, handlers, pick_up_id, _, _) = setup_registries();
        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_ground_location(lot, other_place).unwrap();
            commit_txn(txn);
        }

        let affordance = worldwake_sim::Affordance {
            def_id: pick_up_id,
            actor,
            bound_targets: vec![lot],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng();
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::PreconditionFailed("TargetAtActorPlace(0)".to_string())
        );
    }

    #[test]
    fn pick_up_fails_when_actor_has_no_remaining_capacity() {
        let (mut world, actor, lot, place, _) = setup_world();
        let (defs, handlers, pick_up_id, _, _) = setup_registries();
        {
            let mut txn = new_txn(&mut world, 2);
            let load_filler = txn
                .create_item_lot(CommodityKind::Water, Quantity(2))
                .unwrap();
            txn.set_ground_location(load_filler, place).unwrap();
            txn.set_possessor(load_filler, actor).unwrap();
            commit_txn(txn);
        }

        let affordance = worldwake_sim::Affordance {
            def_id: pick_up_id,
            actor,
            bound_targets: vec![lot],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng();
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap_err();

        assert!(
            matches!(err, ActionError::PreconditionFailed(message) if message.contains("insufficient carry capacity"))
        );
    }

    #[test]
    fn pick_up_splits_lot_when_only_partial_quantity_fits() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (actor, lot) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let lot = txn
                .create_item_lot(CommodityKind::Water, Quantity(3))
                .unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(lot, place).unwrap();
            txn.set_component_carry_capacity(actor, CarryCapacity(LoadUnits(4)))
                .unwrap();
            commit_txn(txn);
            (actor, lot)
        };
        let (defs, handlers, _, _, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();

        let instance_id = start_action_for_target(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            lot,
        );
        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();

        let direct_possessions = world.possessions_of(actor);
        assert_eq!(direct_possessions.len(), 1);
        let picked_up = direct_possessions[0];
        assert_eq!(
            outcome,
            TickOutcome::Committed {
                outcome: CommitOutcome {
                    materializations: vec![Materialization {
                        tag: MaterializationTag::SplitOffLot,
                        entity: picked_up,
                    }],
                    trace: None,
                },
            }
        );
        let carried_lot = world.get_component_item_lot(picked_up).unwrap();
        let remaining_lot = world.get_component_item_lot(lot).unwrap();
        assert_eq!(carried_lot.quantity, Quantity(2));
        assert_eq!(remaining_lot.quantity, Quantity(1));
        assert_eq!(world.possessor_of(picked_up), Some(actor));
        assert_eq!(world.owner_of(picked_up), None);
        assert_eq!(world.effective_place(picked_up), Some(place));
        assert_eq!(world.effective_place(lot), Some(place));
    }

    #[test]
    fn pick_up_transport_payload_moves_exact_requested_quantity() {
        let (mut world, actor, lot, place, _) = setup_world();
        let (defs, handlers, pick_up_id, _, _) = setup_registries();
        let affordance = worldwake_sim::Affordance {
            def_id: pick_up_id,
            actor,
            bound_targets: vec![lot],
            payload_override: Some(ActionPayload::Transport(TransportActionPayload {
                quantity: Quantity(1),
            })),
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng();

        let instance_id = start_action(
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap();

        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();

        let direct_possessions = world.possessions_of(actor);
        assert_eq!(direct_possessions.len(), 1);
        let picked_up = direct_possessions[0];
        assert_eq!(
            outcome,
            TickOutcome::Committed {
                outcome: CommitOutcome {
                    materializations: vec![Materialization {
                        tag: MaterializationTag::SplitOffLot,
                        entity: picked_up,
                    }],
                    trace: None,
                },
            }
        );
        assert_eq!(
            world.get_component_item_lot(picked_up).unwrap().quantity,
            Quantity(1)
        );
        assert_eq!(
            world.get_component_item_lot(lot).unwrap().quantity,
            Quantity(2)
        );
        assert_eq!(world.possessor_of(picked_up), Some(actor));
        assert_eq!(world.owner_of(picked_up), None);
        assert_eq!(world.effective_place(picked_up), Some(place));
        assert_eq!(world.effective_place(lot), Some(place));
    }

    #[test]
    fn put_down_happy_path_clears_possession_without_changing_owner() {
        let (mut world, actor, lot, place, _) = setup_world();
        let owner = {
            let mut txn = new_txn(&mut world, 2);
            let owner = txn.create_faction("Granary Guild").unwrap();
            txn.set_owner(lot, owner).unwrap();
            txn.set_possessor(lot, actor).unwrap();
            commit_txn(txn);
            owner
        };
        let (defs, handlers, _, _, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();

        let instance_id = start_action_for_target(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            lot,
        );
        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(world.possessor_of(lot), None);
        assert_eq!(world.owner_of(lot), Some(owner));
        assert_eq!(world.effective_place(lot), Some(place));
    }

    #[test]
    fn put_down_affordance_excludes_ground_and_nested_lots() {
        let (mut world, actor, ground_lot, place, _) = setup_world();
        let carried_lot = {
            let mut txn = new_txn(&mut world, 2);
            let carried_lot = txn
                .create_item_lot(CommodityKind::Apple, Quantity(1))
                .unwrap();
            let bag = txn
                .create_container(Container {
                    capacity: LoadUnits(10),
                    allowed_commodities: None,
                    allows_unique_items: true,
                    allows_nested_containers: true,
                })
                .unwrap();
            let nested_lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(bag, place).unwrap();
            txn.set_possessor(bag, actor).unwrap();
            txn.set_possessor(carried_lot, actor).unwrap();
            txn.set_ground_location(carried_lot, place).unwrap();
            txn.put_into_container(nested_lot, bag).unwrap();
            commit_txn(txn);
            carried_lot
        };
        let (defs, handlers, _, put_down_id, _) = setup_registries();

        let affordances = affordances_for(&world, actor, &defs, &handlers)
            .into_iter()
            .filter(|affordance| affordance.def_id == put_down_id)
            .collect::<Vec<_>>();

        assert_eq!(affordances.len(), 1);
        assert_eq!(affordances[0].bound_targets, vec![carried_lot]);
        assert_ne!(affordances[0].bound_targets, vec![ground_lot]);
    }

    #[test]
    fn put_down_fails_for_non_possessed_lot() {
        let (mut world, actor, lot, _, _) = setup_world();
        let (defs, handlers, _, put_down_id, _) = setup_registries();
        let affordance = worldwake_sim::Affordance {
            def_id: put_down_id,
            actor,
            bound_targets: vec![lot],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng();
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::PreconditionFailed("TargetDirectlyPossessedByActor(0)".to_string())
        );
    }

    #[test]
    fn picked_up_lot_moves_with_travel_via_existing_possession_architecture() {
        let (mut world, actor, lot, _, destination) = setup_world();
        let (defs, handlers, _, _, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();

        let pick_up_instance = start_action_for_target(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            lot,
        );
        let _ = tick_action(
            pick_up_instance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();

        let mut travel_defs = ActionDefRegistry::new();
        let mut travel_handlers = ActionHandlerRegistry::new();
        let travel_id =
            crate::travel_actions::register_travel_actions(&mut travel_defs, &mut travel_handlers);
        let travel_affordance = affordances_for(&world, actor, &travel_defs, &travel_handlers)
            .into_iter()
            .find(|affordance| {
                affordance.def_id == travel_id && affordance.bound_targets == vec![destination]
            })
            .unwrap();
        let mut next_instance_id = ActionInstanceId(2);
        let travel_instance = start_action(
            &travel_affordance,
            &travel_defs,
            &travel_handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_instance_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(7)),
        )
        .unwrap();

        let _ = tick_action(
            travel_instance,
            &travel_defs,
            &travel_handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(8)),
        )
        .unwrap();
        let outcome = tick_action(
            travel_instance,
            &travel_defs,
            &travel_handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(9)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(world.possessor_of(lot), Some(actor));
        assert_eq!(world.effective_place(lot), Some(destination));
    }

    #[test]
    fn pick_up_affordance_excludes_contained_lots() {
        let (mut world, actor, ground_lot, place, _) = setup_world();
        let contained_lot = {
            let mut txn = new_txn(&mut world, 2);
            let bag = txn
                .create_container(Container {
                    capacity: LoadUnits(10),
                    allowed_commodities: None,
                    allows_unique_items: true,
                    allows_nested_containers: true,
                })
                .unwrap();
            let contained_lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(bag, place).unwrap();
            txn.put_into_container(contained_lot, bag).unwrap();
            commit_txn(txn);
            contained_lot
        };
        let (defs, handlers, pick_up_id, _, _) = setup_registries();

        let affordances = affordances_for(&world, actor, &defs, &handlers)
            .into_iter()
            .filter(|affordance| affordance.def_id == pick_up_id)
            .collect::<Vec<_>>();

        assert!(affordances
            .iter()
            .any(|affordance| affordance.bound_targets == vec![ground_lot]));
        assert!(!affordances
            .iter()
            .any(|affordance| affordance.bound_targets == vec![contained_lot]));
    }

    #[test]
    fn pick_up_succeeds_for_actor_owned_lot() {
        let (mut world, actor, lot, place, _) = setup_world();
        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_owner(lot, actor).unwrap();
            commit_txn(txn);
        }
        let (defs, handlers, _, _, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();

        let instance_id = start_action_for_target(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            lot,
        );
        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(world.possessor_of(lot), Some(actor));
        assert_eq!(world.owner_of(lot), Some(actor));
        assert_eq!(world.effective_place(lot), Some(place));
    }

    #[test]
    fn pick_up_succeeds_for_unowned_lot() {
        let (mut world, actor, lot, _place, _) = setup_world();
        assert_eq!(world.owner_of(lot), None);
        let (defs, handlers, _, _, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();

        let instance_id = start_action_for_target(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            lot,
        );
        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(world.possessor_of(lot), Some(actor));
    }

    #[test]
    fn pick_up_rejects_owned_lot_when_actor_lacks_control() {
        let (mut world, actor, lot, _, _) = setup_world();
        let other_owner = {
            let mut txn = new_txn(&mut world, 2);
            let other = txn.create_agent("Briar", ControlSource::Ai).unwrap();
            txn.set_owner(lot, other).unwrap();
            commit_txn(txn);
            other
        };
        let (defs, handlers, pick_up_id, _, _) = setup_registries();
        let affordance = worldwake_sim::Affordance {
            def_id: pick_up_id,
            actor,
            bound_targets: vec![lot],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng();
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(msg) if
            msg.contains("TargetUnownedOrActorControls") || msg.contains("cannot lawfully pick up")));
        let _ = other_owner;
    }

    #[test]
    fn pick_up_succeeds_for_faction_member_on_faction_owned_lot() {
        let (mut world, actor, lot, place, _) = setup_world();
        {
            let mut txn = new_txn(&mut world, 2);
            let faction = txn.create_faction("Bakers Guild").unwrap();
            txn.set_owner(lot, faction).unwrap();
            txn.add_member(actor, faction).unwrap();
            commit_txn(txn);
        }
        let (defs, handlers, _, _, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();

        let instance_id = start_action_for_target(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            lot,
        );
        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(world.possessor_of(lot), Some(actor));
        assert_eq!(world.effective_place(lot), Some(place));
    }

    #[test]
    fn pick_up_succeeds_for_office_holder_on_office_owned_lot() {
        let (mut world, actor, lot, place, _) = setup_world();
        {
            let mut txn = new_txn(&mut world, 2);
            let office = txn.create_office("Lord of the Granary").unwrap();
            txn.set_owner(lot, office).unwrap();
            txn.assign_office(office, actor).unwrap();
            commit_txn(txn);
        }
        let (defs, handlers, _, _, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();

        let instance_id = start_action_for_target(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            lot,
        );
        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(world.possessor_of(lot), Some(actor));
        assert_eq!(world.effective_place(lot), Some(place));
    }

    #[test]
    fn steal_happy_path_transfers_possession_without_transferring_ownership() {
        let (mut world, actor, lot, place, _) = setup_world();
        let owner = {
            let mut txn = new_txn(&mut world, 2);
            let owner = txn.create_agent("Briar", ControlSource::Ai).unwrap();
            txn.set_owner(lot, owner).unwrap();
            txn.set_component_theft_disposition_profile(
                actor,
                worldwake_core::TheftDispositionProfile {
                    steal_duration_ticks: NonZeroU32::new(2).unwrap(),
                    theft_motive_weight: worldwake_core::Permille::new(500).unwrap(),
                    witness_risk_penalty: worldwake_core::Permille::new(100).unwrap(),
                },
            )
            .unwrap();
            commit_txn(txn);
            owner
        };
        let (defs, handlers, _, _, steal_id) = setup_registries();
        let affordance = affordances_for(&world, actor, &defs, &handlers)
            .into_iter()
            .find(|affordance| {
                affordance.def_id == steal_id && affordance.bound_targets == vec![lot]
            })
            .expect("contained displayed lot should expose a steal affordance");
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng();

        let instance_id = start_action(
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap();
        assert_eq!(
            active_actions
                .get(&instance_id)
                .unwrap()
                .remaining_duration
                .ticks(),
            2
        );

        let first_tick = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();
        assert_eq!(first_tick, TickOutcome::Continuing);
        assert_eq!(world.possessor_of(lot), None);

        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(7)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(world.possessor_of(lot), Some(actor));
        assert_eq!(world.owner_of(lot), Some(owner));
        assert_eq!(world.effective_place(lot), Some(place));
        verify_live_lot_conservation(&world, CommodityKind::Bread, 3).unwrap();

        let record = log
            .get(log.events_by_tag(EventTag::ActionCommitted)[0])
            .unwrap();
        assert_eq!(record.visibility(), VisibilitySpec::Hidden);
        assert!(record.tags().contains(&EventTag::Crime));
        assert!(record.tags().contains(&EventTag::Transfer));
    }

    #[test]
    fn steal_happy_path_removes_facility_stock_markers_from_displayed_lot() {
        let (mut world, actor, lot, place, _) = setup_world();
        let owner = {
            let mut txn = new_txn(&mut world, 2);
            let owner = txn.create_agent("Briar", ControlSource::Ai).unwrap();
            let (facility, _stock_container, display_container) = txn
                .create_merchant_facility(place, owner, LoadUnits(200), Some(LoadUnits(100)))
                .unwrap();
            let display_container = display_container.expect("display container should exist");
            txn.set_owner(lot, owner).unwrap();
            txn.put_into_container(lot, display_container).unwrap();
            txn.set_component_stock_assignment(
                lot,
                StockAssignment {
                    facility,
                    kind: StockAssignmentKind::Displayed,
                },
            )
            .unwrap();
            txn.set_component_sale_listing(lot, SaleListing { listed_at: Tick(2) })
                .unwrap();
            txn.set_component_theft_disposition_profile(
                actor,
                worldwake_core::TheftDispositionProfile {
                    steal_duration_ticks: NonZeroU32::new(2).unwrap(),
                    theft_motive_weight: worldwake_core::Permille::new(500).unwrap(),
                    witness_risk_penalty: worldwake_core::Permille::new(100).unwrap(),
                },
            )
            .unwrap();
            commit_txn(txn);
            owner
        };
        let (defs, handlers, _, _, steal_id) = setup_registries();
        let affordance = affordances_for(&world, actor, &defs, &handlers)
            .into_iter()
            .find(|affordance| {
                affordance.def_id == steal_id && affordance.bound_targets == vec![lot]
            })
            .expect("contained displayed lot should expose a steal affordance");
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng();

        let instance_id = start_action(
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap();

        let first_tick = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();
        assert_eq!(first_tick, TickOutcome::Continuing);

        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(7)),
        )
        .unwrap();

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(world.possessor_of(lot), Some(actor));
        assert_eq!(world.owner_of(lot), Some(owner));
        assert_eq!(world.effective_place(lot), Some(place));
        assert_eq!(world.direct_container(lot), None);
        assert_eq!(world.get_component_stock_assignment(lot), None);
        assert_eq!(world.get_component_sale_listing(lot), None);
    }

    #[test]
    fn contained_steal_emits_container_tamper_and_forced_entry_evidence_without_overwrite() {
        let (mut world, actor, lot, place, _) = setup_world();
        let display_container = {
            let mut txn = new_txn(&mut world, 2);
            let owner = txn.create_agent("Briar", ControlSource::Ai).unwrap();
            let (_facility, _stock_container, display_container) = txn
                .create_merchant_facility(place, owner, LoadUnits(200), Some(LoadUnits(100)))
                .unwrap();
            let display_container = display_container.expect("display container should exist");
            txn.set_owner(lot, owner).unwrap();
            txn.put_into_container(lot, display_container).unwrap();
            txn.set_component_theft_disposition_profile(
                actor,
                worldwake_core::TheftDispositionProfile {
                    steal_duration_ticks: NonZeroU32::new(2).unwrap(),
                    theft_motive_weight: worldwake_core::Permille::new(500).unwrap(),
                    witness_risk_penalty: worldwake_core::Permille::new(100).unwrap(),
                },
            )
            .unwrap();
            txn.set_component_scene_evidence(
                place,
                worldwake_core::SceneEvidence {
                    evidence: vec![EvidenceEntry {
                        id: EvidenceEntryId(0),
                        kind: EvidenceKind::MovementTrace {
                            entity: actor,
                            departed_from: place,
                            direction: place,
                            observed_at: Tick(2),
                        },
                        created_at: Tick(2),
                        decay_ticks: 30,
                    }],
                    next_entry_id: 1,
                },
            )
            .unwrap();
            commit_txn(txn);
            display_container
        };
        let (defs, handlers, _, _, steal_id) = setup_registries();
        let affordance = affordances_for(&world, actor, &defs, &handlers)
            .into_iter()
            .find(|affordance| {
                affordance.def_id == steal_id && affordance.bound_targets == vec![lot]
            })
            .expect("contained displayed lot should expose a steal affordance");
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng();

        let instance_id = start_action(
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap();

        let _ = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();
        let _ = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(7)),
        )
        .unwrap();

        let scene = world
            .get_component_scene_evidence(place)
            .expect("steal should leave scene evidence");
        assert_eq!(scene.evidence.len(), 3);
        assert_eq!(
            scene
                .evidence
                .iter()
                .map(|entry| entry.id)
                .collect::<Vec<_>>(),
            vec![EvidenceEntryId(0), EvidenceEntryId(1), EvidenceEntryId(2)]
        );
        assert!(scene.evidence.iter().any(|entry| {
            entry.kind
                == EvidenceKind::ContainerTampered {
                    container: display_container,
                    tampered_at: Tick(7),
                }
        }));
        assert!(scene.evidence.iter().any(|entry| {
            entry.kind
                == EvidenceKind::DisturbanceMarker {
                    place,
                    kind: DisturbanceKind::ForcedEntry,
                    created_at: Tick(7),
                }
        }));
    }

    #[test]
    fn steal_rejects_lawfully_controllable_displayed_lot() {
        let (mut world, actor, lot, place, _) = setup_world();
        {
            let mut txn = new_txn(&mut world, 2);
            let (facility, _stock_container, display_container) = txn
                .create_merchant_facility(place, actor, LoadUnits(200), Some(LoadUnits(100)))
                .unwrap();
            let display_container = display_container.expect("display container should exist");
            txn.set_owner(lot, actor).unwrap();
            txn.put_into_container(lot, display_container).unwrap();
            txn.set_component_stock_assignment(
                lot,
                StockAssignment {
                    facility,
                    kind: StockAssignmentKind::Displayed,
                },
            )
            .unwrap();
            txn.set_component_sale_listing(lot, SaleListing { listed_at: Tick(2) })
                .unwrap();
            txn.set_component_theft_disposition_profile(
                actor,
                worldwake_core::TheftDispositionProfile {
                    steal_duration_ticks: NonZeroU32::new(2).unwrap(),
                    theft_motive_weight: worldwake_core::Permille::new(500).unwrap(),
                    witness_risk_penalty: worldwake_core::Permille::new(100).unwrap(),
                },
            )
            .unwrap();
            commit_txn(txn);
        }
        let (defs, handlers, _, _, steal_id) = setup_registries();
        let affordance = worldwake_sim::Affordance {
            def_id: steal_id,
            actor,
            bound_targets: vec![lot],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng();
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap_err();

        assert!(
            matches!(err, ActionError::PreconditionFailed(message) if message.contains("already owns") || message.contains("use pick_up"))
        );
    }

    #[test]
    fn steal_requires_theft_profile() {
        let (mut world, actor, lot, _, _) = setup_world();
        {
            let mut txn = new_txn(&mut world, 2);
            let owner = txn.create_agent("Briar", ControlSource::Ai).unwrap();
            txn.set_owner(lot, owner).unwrap();
            commit_txn(txn);
        }
        let (defs, handlers, _, _, steal_id) = setup_registries();
        let affordance = worldwake_sim::Affordance {
            def_id: steal_id,
            actor,
            bound_targets: vec![lot],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng();
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(message) if
                message.contains("TheftDispositionProfile")
                    || message.contains("theft disposition profile")));
    }

    #[test]
    fn steal_rejects_lawfully_controllable_lot() {
        let (mut world, actor, lot, _, _) = setup_world();
        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_owner(lot, actor).unwrap();
            txn.set_component_theft_disposition_profile(
                actor,
                worldwake_core::TheftDispositionProfile {
                    steal_duration_ticks: NonZeroU32::new(2).unwrap(),
                    theft_motive_weight: worldwake_core::Permille::new(500).unwrap(),
                    witness_risk_penalty: worldwake_core::Permille::new(100).unwrap(),
                },
            )
            .unwrap();
            commit_txn(txn);
        }
        let (defs, handlers, _, _, steal_id) = setup_registries();
        let affordance = worldwake_sim::Affordance {
            def_id: steal_id,
            actor,
            bound_targets: vec![lot],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng();
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap_err();

        assert!(
            matches!(err, ActionError::PreconditionFailed(message) if message.contains("already owns") || message.contains("use pick_up"))
        );
    }
}
