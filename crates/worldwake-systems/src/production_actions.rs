use crate::experience_recording::{
    record_failed_source_attempt, record_successful_source_acquisition,
};
use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    ActionDefId, CommodityKind, Container, ContentionGrant, EntityId, EntityKind, EventTag,
    HarvestTraceEntry, LoadUnits, ProductionOutputOwner, Quantity, SourceKey, VisibilitySpec,
    WorkstationMarker, World, WorldTxn, load_per_unit,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerId,
    ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress, ActionState,
    CommitOutcome, CommitTraceData, Constraint, CraftActionPayload, DeterministicRng, DurationExpr,
    HarvestActionPayload, HarvestCommitTrace, Interruptibility, Precondition, RecipeDefinition,
    RecipeRegistry, ReservationReq, RuntimeBeliefView, TargetSpec,
};

/// Sentinel `PreconditionFailed` message emitted by `start_harvest` when no
/// extraction slot is free for the actor. `record_harvest_start_failure`
/// matches on this string to commit the enqueue write on a fresh
/// transaction (the start-handler txn is dropped on `Err`).
const HARVEST_START_FAILURE_SLOTS_FULL: &str = "extraction_slots_full";

pub fn register_harvest_actions(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
    recipes: &RecipeRegistry,
) -> Vec<ActionDefId> {
    let handler = handlers.register(
        ActionHandler::new(start_harvest, tick_harvest, commit_harvest, abort_harvest)
            .with_start_failure(record_harvest_start_failure)
            .with_payload_override_validator(validate_harvest_payload_override)
            .with_authoritative_payload_validator(validate_exclusive_facility_grant),
    );

    let mut ids = Vec::new();
    for (recipe_id, recipe) in recipes.iter() {
        let Some(def) =
            harvest_action_def(ActionDefId(defs.len() as u32), handler, recipe_id, recipe)
        else {
            continue;
        };
        ids.push(defs.register(def));
    }
    ids
}

pub fn register_craft_actions(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
    recipes: &RecipeRegistry,
) -> Vec<ActionDefId> {
    let handler = handlers.register(
        ActionHandler::new(start_craft, tick_craft, commit_craft, abort_craft)
            .with_authoritative_payload_validator(validate_exclusive_facility_grant),
    );

    let mut ids = Vec::new();
    for (recipe_id, recipe) in recipes.iter() {
        let Some(def) =
            craft_action_def(ActionDefId(defs.len() as u32), handler, recipe_id, recipe)
        else {
            continue;
        };
        ids.push(defs.register(def));
    }
    ids
}

fn harvest_action_def(
    id: ActionDefId,
    handler: ActionHandlerId,
    recipe_id: worldwake_core::RecipeId,
    recipe: &RecipeDefinition,
) -> Option<ActionDef> {
    if !recipe.inputs.is_empty() {
        return None;
    }
    let [(output_commodity, output_quantity)] = recipe.outputs.as_slice() else {
        return None;
    };
    let workstation_tag = recipe.required_workstation_tag?;
    let mut actor_constraints = vec![
        Constraint::ActorAlive,
        Constraint::ActorKnowsRecipe(recipe_id),
    ];
    actor_constraints.extend(
        recipe
            .required_tool_kinds
            .iter()
            .copied()
            .map(|kind| Constraint::ActorHasUniqueItemKind { kind, min_count: 1 }),
    );
    let preconditions = vec![
        Precondition::TargetExists(0),
        Precondition::TargetAtActorPlace(0),
        Precondition::TargetKind {
            target_index: 0,
            kind: EntityKind::Facility,
        },
        Precondition::TargetHasWorkstationTag {
            target_index: 0,
            tag: workstation_tag,
        },
        Precondition::TargetHasResourceSource {
            target_index: 0,
            commodity: *output_commodity,
            min_available: *output_quantity,
        },
    ];

    Some(ActionDef {
        id,
        name: format!("harvest:{}", recipe.name),
        domain: worldwake_core::ActionDomain::Production,
        actor_constraints,
        targets: vec![TargetSpec::EntityAtActorPlace {
            kind: EntityKind::Facility,
        }],
        preconditions: preconditions.clone(),
        // Slot occupancy lives in `ResourceExtractionQueues` (per FND-26),
        // not in the temporal `try_reserve` substrate. Exclusive reservation
        // would block parallel multi-slot harvests on the same source.
        reservation_requirements: Vec::new(),
        duration: DurationExpr::Fixed(recipe.work_ticks),
        body_cost_per_tick: recipe.body_cost_per_tick,
        attention_cost: worldwake_core::Permille::new_unchecked(200),
        interruptibility: Interruptibility::InterruptibleWithPenalty,
        // Source-availability is owned by `commit_harvest` so that the partial-success
        // path (`available < requested && available >= 1`) and the depleted-failure
        // path (`available == 0`) can both surface their aftermath via
        // `LastHarvestTrace` and `CommitTraceData::Harvest`. A duplicate
        // `TargetHasResourceSource` here would short-circuit those outcomes.
        commit_conditions: preconditions
            .into_iter()
            .filter(|p| !matches!(p, Precondition::TargetHasResourceSource { .. }))
            .collect(),
        visibility: VisibilitySpec::ParticipantsOnly,
        causal_event_tags: BTreeSet::from([EventTag::WorldMutation]),
        payload: ActionPayload::Harvest(HarvestActionPayload {
            recipe_id,
            required_workstation_tag: workstation_tag,
            output_commodity: *output_commodity,
            requested_quantity: *output_quantity,
            required_tool_kinds: recipe.required_tool_kinds.clone(),
        }),
        handler,
        binding_strictness: worldwake_sim::BindingStrictness::EquivalentWorkstationTagAtSamePlace,
        guard_template: None,
        expectation_template: vec![],
    })
}

fn craft_action_def(
    id: ActionDefId,
    handler: ActionHandlerId,
    recipe_id: worldwake_core::RecipeId,
    recipe: &RecipeDefinition,
) -> Option<ActionDef> {
    if recipe.inputs.is_empty() || recipe.outputs.is_empty() {
        return None;
    }
    let workstation_tag = recipe.required_workstation_tag?;
    let mut actor_constraints = vec![
        Constraint::ActorAlive,
        Constraint::ActorKnowsRecipe(recipe_id),
    ];
    actor_constraints.extend(
        aggregate_recipe_entries(&recipe.inputs)
            .into_iter()
            .map(|(kind, min_qty)| Constraint::ActorHasCommodityAtActorPlace { kind, min_qty }),
    );
    actor_constraints.extend(
        recipe
            .required_tool_kinds
            .iter()
            .copied()
            .map(|kind| Constraint::ActorHasUniqueItemKind { kind, min_count: 1 }),
    );
    let preconditions = vec![
        Precondition::TargetExists(0),
        Precondition::TargetAtActorPlace(0),
        Precondition::TargetKind {
            target_index: 0,
            kind: EntityKind::Facility,
        },
        Precondition::TargetHasWorkstationTag {
            target_index: 0,
            tag: workstation_tag,
        },
        Precondition::TargetLacksProductionJob(0),
    ];

    Some(ActionDef {
        id,
        name: format!("craft:{}", recipe.name),
        domain: worldwake_core::ActionDomain::Production,
        actor_constraints,
        targets: vec![TargetSpec::EntityAtActorPlace {
            kind: EntityKind::Facility,
        }],
        preconditions: preconditions.clone(),
        reservation_requirements: vec![ReservationReq { target_index: 0 }],
        duration: DurationExpr::Fixed(recipe.work_ticks),
        body_cost_per_tick: recipe.body_cost_per_tick,
        attention_cost: worldwake_core::Permille::new_unchecked(200),
        interruptibility: Interruptibility::InterruptibleWithPenalty,
        commit_conditions: preconditions
            .into_iter()
            .filter(|precondition| {
                !matches!(precondition, Precondition::TargetLacksProductionJob(_))
            })
            .collect(),
        visibility: VisibilitySpec::ParticipantsOnly,
        causal_event_tags: BTreeSet::from([EventTag::WorldMutation]),
        payload: ActionPayload::Craft(CraftActionPayload {
            recipe_id,
            required_workstation_tag: workstation_tag,
            inputs: recipe.inputs.clone(),
            outputs: recipe.outputs.clone(),
            required_tool_kinds: recipe.required_tool_kinds.clone(),
        }),
        handler,
        binding_strictness: worldwake_sim::BindingStrictness::EquivalentWorkstationTagAtSamePlace,
        guard_template: None,
        expectation_template: vec![],
    })
}

fn harvest_payload<'a>(
    def: &ActionDef,
    instance: &'a ActionInstance,
) -> Result<&'a HarvestActionPayload, ActionError> {
    instance.payload.as_harvest().ok_or_else(|| {
        ActionError::InternalError(format!("action def {} is missing harvest payload", def.id))
    })
}

fn craft_payload<'a>(
    def: &ActionDef,
    instance: &'a ActionInstance,
) -> Result<&'a CraftActionPayload, ActionError> {
    instance.payload.as_craft().ok_or_else(|| {
        ActionError::InternalError(format!("action def {} is missing craft payload", def.id))
    })
}

fn aggregate_recipe_entries(
    entries: &[(CommodityKind, Quantity)],
) -> BTreeMap<CommodityKind, Quantity> {
    let mut aggregated = BTreeMap::new();
    for (kind, quantity) in entries {
        aggregated
            .entry(*kind)
            .and_modify(|existing: &mut Quantity| *existing = *existing + *quantity)
            .or_insert(*quantity);
    }
    aggregated
}

fn staging_container(payload: &CraftActionPayload) -> Container {
    let capacity = payload
        .inputs
        .iter()
        .fold(0_u32, |total, (commodity, quantity)| {
            total + commodity.spec().physical_profile.load_per_unit.0 * quantity.0
        })
        .max(1);
    Container {
        capacity: LoadUnits(capacity),
        allowed_commodities: Some(payload.inputs.iter().map(|(kind, _)| *kind).collect()),
        allows_unique_items: false,
        allows_nested_containers: false,
    }
}

fn controlled_lots_for(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    commodity: CommodityKind,
    place: EntityId,
) -> Vec<(EntityId, Quantity)> {
    let mut lots = txn
        .query_item_lot()
        .filter_map(|(entity, lot)| {
            (lot.commodity == commodity
                && txn.can_exercise_control(actor, entity).is_ok()
                && txn.effective_place(entity) == Some(place))
            .then_some((entity, lot.quantity))
        })
        .collect::<Vec<_>>();
    lots.sort_by_key(|(entity, _)| *entity);
    lots
}

fn move_lot_into_container(
    txn: &mut WorldTxn<'_>,
    lot: EntityId,
    container: EntityId,
) -> Result<(), ActionError> {
    if txn.direct_container(lot).is_some() {
        txn.remove_from_container(lot)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    if txn.possessor_of(lot).is_some() {
        txn.clear_possessor(lot)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    txn.put_into_container(lot, container)
        .map_err(|err| ActionError::InternalError(err.to_string()))
}

fn stage_inputs(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    place: EntityId,
    container: EntityId,
    payload: &CraftActionPayload,
) -> Result<(), ActionError> {
    for (commodity, required_quantity) in aggregate_recipe_entries(&payload.inputs) {
        let mut remaining = required_quantity;
        for (lot_id, lot_quantity) in controlled_lots_for(txn, actor, commodity, place) {
            if remaining == Quantity(0) {
                break;
            }
            if lot_quantity > remaining {
                let (_, split_off) = txn
                    .split_lot(lot_id, remaining)
                    .map_err(|err| ActionError::InternalError(err.to_string()))?;
                move_lot_into_container(txn, split_off, container)?;
                remaining = Quantity(0);
                break;
            }

            move_lot_into_container(txn, lot_id, container)?;
            remaining = remaining.checked_sub(lot_quantity).ok_or_else(|| {
                ActionError::InternalError("staged input accounting underflowed".to_string())
            })?;
        }

        if remaining != Quantity(0) {
            return Err(ActionError::PreconditionFailed(format!(
                "actor {actor} lacks accessible {required_quantity:?} units of {commodity:?}"
            )));
        }
    }
    Ok(())
}

fn consume_staged_inputs(txn: &mut WorldTxn<'_>, container: EntityId) -> Result<(), ActionError> {
    for entity in txn.recursive_contents_of(container) {
        txn.archive_entity(entity)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    Ok(())
}

pub(crate) fn ensure_matching_contention_grant(
    world: &World,
    actor: EntityId,
    entity: EntityId,
    action_def: ActionDefId,
) -> Result<(), ActionError> {
    let policy = world.get_component_contention_policy(entity);
    let queue = world.get_component_contention_queue(entity);
    let queue = match (policy, queue) {
        (None, None) => return Ok(()),
        (Some(_), Some(queue)) => queue,
        (Some(_), None) => {
            return Err(ActionError::PreconditionFailed(format!(
                "entity {entity} is contention-managed but lacks ContentionQueue grant state"
            )));
        }
        (None, Some(_)) => {
            return Err(ActionError::PreconditionFailed(format!(
                "entity {entity} has ContentionQueue grant state without ContentionPolicy"
            )));
        }
    };
    match queue.granted.as_ref() {
        Some(granted) if granted.actor == actor && granted.intended_action == action_def => Ok(()),
        Some(granted) => Err(ActionError::PreconditionFailed(format!(
            "entity {entity} grant belongs to actor {} action {:?}, not actor {actor} action {:?}",
            granted.actor, granted.intended_action, action_def
        ))),
        None => Err(ActionError::PreconditionFailed(format!(
            "entity {entity} has no matching grant for actor {actor} action {action_def:?}"
        ))),
    }
}

fn consume_matching_facility_grant(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    facility: EntityId,
    action_def: ActionDefId,
) -> Result<(), ActionError> {
    ensure_matching_contention_grant(txn, actor, facility, action_def)?;
    if txn.get_component_contention_policy(facility).is_none()
        && txn.get_component_contention_queue(facility).is_none()
    {
        return Ok(());
    }
    let mut queue = txn
        .get_component_contention_queue(facility)
        .cloned()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "facility {facility} lacks ContentionQueue grant state"
            ))
        })?;
    queue.clear_grant();
    txn.set_component_contention_queue(facility, queue)
        .map_err(|err| ActionError::InternalError(err.to_string()))
}

fn validate_exclusive_facility_grant(
    def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    _payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let facility = *targets.first().ok_or(ActionError::InvalidTarget(actor))?;
    ensure_matching_contention_grant(world, actor, facility, def.id)
}

fn start_harvest(
    def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let _ = harvest_payload(def, instance)?;
    let workstation = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    grant_or_signal_full(txn, instance.actor, workstation, def.id)?;
    Ok(None)
}

/// Find a slot in `ResourceExtractionQueues` that `actor` may legally
/// claim and grant it, writing back. Slots are eligible iff the actor
/// already holds the grant, or the slot is free **and** the actor is
/// the head of the slot's waiting list (or the slot has no waiters).
///
/// The head-of-waiting precedence is what makes the queue substrate
/// FIFO-fair: when a slot becomes free, only the queued head can claim
/// it on the next harvest start, even if a fresh agent (with no prior
/// queue position) tries simultaneously. Without this rule a previously
/// granted actor could re-grab the slot tick after tick and starve out
/// every queued waiter (FND-26: contention drives extraction state, not
/// first-call wins).
///
/// Returns `Err(PreconditionFailed("extraction_slots_full"))` when no
/// slot is eligible. The failure handler enqueues the actor on the
/// shortest-waitlist slot using a fresh transaction (the start-handler
/// txn is dropped on `Err`).
fn grant_or_signal_full(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    workstation: EntityId,
    action_def: ActionDefId,
) -> Result<(), ActionError> {
    let Some(mut queues) = txn
        .get_component_resource_extraction_queues(workstation)
        .cloned()
    else {
        return Err(ActionError::PreconditionFailed(format!(
            "workstation {workstation} lacks ResourceExtractionQueues"
        )));
    };
    if queues.queues.is_empty() {
        return Err(ActionError::PreconditionFailed(format!(
            "workstation {workstation} has zero extraction slots"
        )));
    }

    let chosen_slot = queues.queues.iter().position(|queue| match &queue.granted {
        Some(grant) => grant.actor == actor,
        None => match queue.waiting.values().next() {
            Some(head) => head.actor == actor,
            None => true,
        },
    });

    if let Some(slot) = chosen_slot {
        let queue = &mut queues.queues[slot];
        if queue.granted.is_none() {
            // Promote the head waiter (or grab a free slot). Removing from
            // waiting before granting keeps the queue invariant that an
            // actor never appears in both `granted` and `waiting`.
            // `expires_at` mirrors the facility-queue grant lifecycle;
            // commit/abort clear the grant so the expiry value only
            // matters as a sentinel for stale grants if cleanup is missed.
            queue.remove_actor(actor);
            queue.granted = Some(ContentionGrant {
                actor,
                intended_action: action_def,
                granted_at: txn.tick(),
                expires_at: txn.tick(),
            });
            txn.set_component_resource_extraction_queues(workstation, queues)
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
        }
        Ok(())
    } else {
        Err(ActionError::PreconditionFailed(
            HARVEST_START_FAILURE_SLOTS_FULL.to_string(),
        ))
    }
}

/// Enqueue `actor` at the slot with the shortest waitlist. Called from
/// `record_harvest_start_failure` against a fresh transaction so the write
/// persists. No-op if `actor` is already enqueued or granted.
fn enqueue_at_shortest_slot(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    workstation: EntityId,
    action_def: ActionDefId,
) -> Result<(), ActionError> {
    let Some(mut queues) = txn
        .get_component_resource_extraction_queues(workstation)
        .cloned()
    else {
        return Ok(());
    };
    if queues.queues.is_empty() {
        return Ok(());
    }
    if queues.queues.iter().any(|queue| queue.has_actor(actor)) {
        return Ok(());
    }

    let chosen_slot = queues
        .queues
        .iter()
        .enumerate()
        .min_by_key(|(_, queue)| queue.waiting.len())
        .map_or(0, |(slot, _)| slot);

    queues.queues[chosen_slot]
        .enqueue(actor, action_def, txn.tick(), None)
        .map_err(|err| ActionError::PreconditionFailed(format!("{err:?}")))?;
    txn.set_component_resource_extraction_queues(workstation, queues)
        .map_err(|err| ActionError::InternalError(err.to_string()))
}

/// Clear `actor`'s grant or queued entry from any slot in the workstation's
/// `ResourceExtractionQueues`. Called from `commit_harvest`, `abort_harvest`,
/// and the depleted-source abort path so a held slot is released for the
/// next agent. No-op if the source has no queues registered.
fn release_extraction_slot(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    workstation: EntityId,
) -> Result<(), ActionError> {
    let Some(mut queues) = txn
        .get_component_resource_extraction_queues(workstation)
        .cloned()
    else {
        return Ok(());
    };
    let mut changed = false;
    for queue in &mut queues.queues {
        if queue.remove_actor(actor) {
            changed = true;
        }
    }
    if changed {
        txn.set_component_resource_extraction_queues(workstation, queues)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn tick_harvest(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn start_craft(
    def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let payload = craft_payload(def, instance)?;
    let workstation = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    consume_matching_facility_grant(txn, instance.actor, workstation, def.id)?;
    if txn.has_component_production_job(workstation) {
        return Err(ActionError::PreconditionFailed(format!(
            "workstation {workstation} already has production job"
        )));
    }
    let place = txn.effective_place(workstation).ok_or_else(|| {
        ActionError::InternalError(format!("workstation {workstation} has no effective place"))
    })?;
    let marker = txn
        .get_component_workstation_marker(workstation)
        .copied()
        .ok_or(ActionError::InvalidTarget(workstation))?;
    if marker != WorkstationMarker(payload.required_workstation_tag) {
        return Err(ActionError::PreconditionFailed(format!(
            "workstation {workstation} tag {:?} does not match {:?}",
            marker.0, payload.required_workstation_tag
        )));
    }

    let staged_inputs_container = txn
        .create_container(staging_container(payload))
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.set_ground_location(staged_inputs_container, place)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    stage_inputs(txn, instance.actor, place, staged_inputs_container, payload)?;
    txn.set_component_production_job(
        workstation,
        worldwake_core::ProductionJob {
            recipe_id: payload.recipe_id,
            worker: instance.actor,
            staged_inputs_container,
            progress_ticks: 0,
        },
    )
    .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.add_target(staged_inputs_container);
    Ok(None)
}

fn tick_craft(
    def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    let payload = craft_payload(def, instance)?;
    let workstation = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    let mut job = txn
        .get_component_production_job(workstation)
        .cloned()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "workstation {workstation} lacks craft job for recipe {:?}",
                payload.recipe_id
            ))
        })?;
    if job.recipe_id != payload.recipe_id {
        return Err(ActionError::PreconditionFailed(format!(
            "workstation {workstation} job recipe {:?} does not match {:?}",
            job.recipe_id, payload.recipe_id
        )));
    }
    job.progress_ticks = job
        .progress_ticks
        .checked_add(1)
        .ok_or_else(|| ActionError::InternalError("craft progress overflowed".to_string()))?;
    txn.set_component_production_job(workstation, job)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    Ok(ActionProgress::Continue)
}

/// Resolves the `ProductionOutputOwnershipPolicy` on a producer entity to determine
/// who should own the output lots.
fn resolve_output_owner(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    producer: EntityId,
) -> Result<Option<EntityId>, ActionError> {
    let policy = txn
        .get_component_production_output_ownership_policy(producer)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "producer {producer} has no ProductionOutputOwnershipPolicy"
            ))
        })?;
    match policy.output_owner {
        ProductionOutputOwner::Actor => Ok(Some(actor)),
        ProductionOutputOwner::ProducerOwner => {
            let owner = txn.owner_of(producer).ok_or_else(|| {
                ActionError::PreconditionFailed(format!(
                    "producer {producer} has ProducerOwner policy but no owner"
                ))
            })?;
            Ok(Some(owner))
        }
        ProductionOutputOwner::Unowned => Ok(None),
    }
}

fn commit_harvest(
    def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = harvest_payload(def, instance)?;
    let workstation = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    let place = txn.effective_place(workstation).ok_or_else(|| {
        ActionError::InternalError(format!("workstation {workstation} has no effective place"))
    })?;
    let marker = txn
        .get_component_workstation_marker(workstation)
        .copied()
        .ok_or(ActionError::InvalidTarget(workstation))?;
    if marker != WorkstationMarker(payload.required_workstation_tag) {
        return Err(ActionError::PreconditionFailed(format!(
            "workstation {workstation} tag {:?} does not match {:?}",
            marker.0, payload.required_workstation_tag
        )));
    }
    let mut source = txn
        .get_component_resource_source(workstation)
        .cloned()
        .ok_or(ActionError::InvalidTarget(workstation))?;
    if source.commodity != payload.output_commodity {
        return Err(ActionError::PreconditionFailed(format!(
            "resource source {workstation} commodity {:?} does not match {:?}",
            source.commodity, payload.output_commodity
        )));
    }
    let available = source.available_quantity.0;
    let requested = payload.requested_quantity.0;
    let actual = available.min(requested);

    if actual == 0 {
        let mut trace = txn
            .get_component_last_harvest_trace(workstation)
            .cloned()
            .unwrap_or_default();
        trace.push(HarvestTraceEntry {
            harvester: instance.actor,
            tick: txn.tick(),
            quantity: 0,
            partial: true,
        });
        txn.set_component_last_harvest_trace(workstation, trace)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
        release_extraction_slot(txn, instance.actor, workstation)?;
        // AbortRequested routes through `finalize_failed_action`, which commits
        // the WorldTxn (including the failed-harvest trace append above) before
        // returning. PreconditionFailed would drop the txn — see ticket
        // reassessment item 19.
        return Err(ActionError::AbortRequested(
            worldwake_sim::ActionAbortRequestReason::HarvestSourceDepleted { workstation },
        ));
    }

    source.available_quantity = Quantity(available - actual);
    txn.set_component_resource_source(workstation, source)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;

    let owner = resolve_output_owner(txn, instance.actor, workstation)?;
    let lot = txn
        .create_item_lot_with_owner(payload.output_commodity, Quantity(actual), place, owner)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.add_target(lot);

    let mut trace = txn
        .get_component_last_harvest_trace(workstation)
        .cloned()
        .unwrap_or_default();
    let actual_u16 = u16::try_from(actual).map_err(|err| {
        ActionError::InternalError(format!(
            "harvest commit: actual quantity {actual} exceeds u16: {err}"
        ))
    })?;
    let is_partial = actual < requested;
    trace.push(HarvestTraceEntry {
        harvester: instance.actor,
        tick: txn.tick(),
        quantity: actual_u16,
        partial: is_partial,
    });
    txn.set_component_last_harvest_trace(workstation, trace)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;

    record_successful_source_acquisition(
        txn,
        instance.actor,
        harvest_source_key(payload, workstation),
        txn.tick(),
    )?;

    release_extraction_slot(txn, instance.actor, workstation)?;

    if is_partial {
        Ok(
            CommitOutcome::empty().with_trace(CommitTraceData::Harvest(HarvestCommitTrace {
                requested_quantity: payload.requested_quantity,
                partial_quantity: Some(Quantity(actual)),
            })),
        )
    } else {
        Ok(CommitOutcome::empty())
    }
}

fn commit_craft(
    def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = craft_payload(def, instance)?;
    let workstation = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    let place = txn.effective_place(workstation).ok_or_else(|| {
        ActionError::InternalError(format!("workstation {workstation} has no effective place"))
    })?;
    let job = txn
        .get_component_production_job(workstation)
        .cloned()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "workstation {workstation} lacks craft job on commit"
            ))
        })?;
    if job.recipe_id != payload.recipe_id {
        return Err(ActionError::PreconditionFailed(format!(
            "workstation {workstation} job recipe {:?} does not match {:?}",
            job.recipe_id, payload.recipe_id
        )));
    }

    consume_staged_inputs(txn, job.staged_inputs_container)?;
    txn.archive_entity(job.staged_inputs_container)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.clear_component_production_job(workstation)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;

    let owner = resolve_output_owner(txn, instance.actor, workstation)?;
    for (commodity, quantity) in &payload.outputs {
        let lot = txn
            .create_item_lot_with_owner(*commodity, *quantity, place, owner)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
        txn.add_target(lot);
    }
    Ok(CommitOutcome::empty())
}

fn harvest_source_key(payload: &HarvestActionPayload, workstation: EntityId) -> SourceKey {
    SourceKey {
        entity: workstation,
        commodity: payload.output_commodity,
    }
}

fn harvest_source_failed_intrinsically(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    workstation: EntityId,
    payload: &HarvestActionPayload,
    error: &ActionError,
) -> bool {
    match error {
        ActionError::PreconditionFailed(_) | ActionError::InvalidTarget(_) => {}
        ActionError::ReservationUnavailable(_)
        | ActionError::ConstraintFailed(_)
        | ActionError::AbortRequested(_)
        | ActionError::InternalError(_)
        | ActionError::UnknownActionInstance(_)
        | ActionError::UnknownActionDef(_)
        | ActionError::UnknownActionHandler(_)
        | ActionError::InvalidActionStatus { .. }
        | ActionError::InterruptBlocked { .. } => return false,
    }

    let actor_place = txn.effective_place(actor);
    let workstation_place = txn.effective_place(workstation);
    if workstation_place.is_none() {
        return true;
    }
    if actor_place != workstation_place {
        return false;
    }
    if txn.get_component_workstation_marker(workstation)
        != Some(&WorkstationMarker(payload.required_workstation_tag))
    {
        return false;
    }
    let Some(source) = txn.get_component_resource_source(workstation) else {
        return true;
    };
    source.commodity != payload.output_commodity
        || source.available_quantity < payload.requested_quantity
}

fn validate_harvest_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    _targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(harvest) = payload.as_harvest() else {
        return false;
    };
    let requested = harvest.requested_quantity.0;
    if requested == 0 {
        return false;
    }
    let Some(carry_capacity) = view.carry_capacity(actor) else {
        return false;
    };
    let Some(load) = view.load_of_entity(actor) else {
        return false;
    };
    let per_unit = load_per_unit(harvest.output_commodity).0;
    if per_unit == 0 {
        return false;
    }
    let headroom_units = carry_capacity.0.saturating_sub(load.0) / per_unit;
    requested <= headroom_units
}

#[allow(clippy::too_many_arguments, clippy::unnecessary_wraps)]
fn record_harvest_start_failure(
    def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    error: &ActionError,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    let Some(payload) = payload.as_harvest() else {
        return Ok(());
    };
    let Some(workstation) = targets.first().copied() else {
        return Ok(());
    };
    if matches!(
        error,
        ActionError::PreconditionFailed(message) if message == HARVEST_START_FAILURE_SLOTS_FULL,
    ) {
        // Slots-full failure: enqueue the actor on the shortest-waitlist
        // slot. The start-handler txn was dropped on `Err`; this fresh
        // failure-handler txn commits the queue write.
        enqueue_at_shortest_slot(txn, actor, workstation, def.id)?;
        return Ok(());
    }
    if harvest_source_failed_intrinsically(txn, actor, workstation, payload, error) {
        record_failed_source_attempt(
            txn,
            actor,
            harvest_source_key(payload, workstation),
            txn.tick(),
        )?;
    }
    Ok(())
}

fn abort_harvest(
    _def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _reason: &AbortReason,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    // External interruptions do not penalize source reliability. The source
    // did not fail; the actor was interrupted. Release any held extraction
    // slot so the next agent can promote into it.
    if let Some(workstation) = instance.targets.first().copied() {
        release_extraction_slot(txn, instance.actor, workstation)?;
    }
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_craft(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _reason: &AbortReason,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{register_craft_actions, register_harvest_actions};
    use crate::needs::needs_system;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        AgentBeliefStore, BodyCostPerTick, CauseRef, CommodityKind, Container, ContentionGrant,
        ContentionPolicy, ContentionQueue, ControlSource, DeprivationExposure, DriveThresholds,
        EntityId, EventId, EventLog, EventView, HomeostaticNeeds, LoadUnits, MetabolismProfile,
        PerceptionSource, Permille, PreferenceProfile, ProductionOutputOwner,
        ProductionOutputOwnershipPolicy, Quantity, RelationDelta, RelationKind, RelationValue,
        ReliabilityRecord, ResourceSource, Seed, SourceKey, SourceReliability, StateDelta, Tick,
        VisibilitySpec, WitnessData, WorkstationMarker, WorkstationTag, World, WorldTxn,
        build_believed_entity_state, build_prototype_world,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionExecutionAuthority, ActionHandlerRegistry, ActionInstance,
        ActionInstanceId, ActionPayload, DeterministicRng, ExternalAbortReason, PerAgentBeliefView,
        RecipeRegistry, SystemExecutionContext, SystemId, TickOutcome, TradeActionPayload,
        abort_action, get_affordances, start_action, tick_action,
    };

    use super::*;

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
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
            WitnessData::default(),
        )
    }

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    fn test_rng(byte: u8) -> DeterministicRng {
        DeterministicRng::new(Seed([byte; 32]))
    }

    fn harvest_recipe_registry(
        body_cost_per_tick: BodyCostPerTick,
    ) -> (RecipeRegistry, worldwake_core::RecipeId) {
        harvest_recipe_registry_with_tools(body_cost_per_tick, Vec::new())
    }

    fn harvest_recipe_registry_with_tools(
        body_cost_per_tick: BodyCostPerTick,
        required_tool_kinds: Vec<worldwake_core::UniqueItemKind>,
    ) -> (RecipeRegistry, worldwake_core::RecipeId) {
        let mut recipes = RecipeRegistry::new();
        let recipe_id = recipes.register(RecipeDefinition {
            name: "Harvest Apples".to_string(),
            inputs: Vec::new(),
            outputs: vec![(CommodityKind::Apple, Quantity(2))],
            work_ticks: nz(2),
            required_workstation_tag: Some(WorkstationTag::OrchardRow),
            required_tool_kinds,
            body_cost_per_tick,
        });
        (recipes, recipe_id)
    }

    fn setup_world(
        known_recipe: bool,
        workstation_tag: WorkstationTag,
        available_quantity: u32,
    ) -> (World, EntityId, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let mut txn = new_txn(&mut world, 1);
        let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        let workstation = txn.create_entity(EntityKind::Facility);
        txn.set_ground_location(actor, place).unwrap();
        txn.set_ground_location(workstation, place).unwrap();
        txn.set_component_workstation_marker(workstation, WorkstationMarker(workstation_tag))
            .unwrap();
        txn.set_component_resource_source(
            workstation,
            ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(available_quantity),
                max_quantity: Quantity(10),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        )
        .unwrap();
        // Mirror the scenario translator: every resource source has
        // `ResourceExtractionQueues` with one queue per slot. `start_harvest`
        // grants the actor a slot here.
        txn.set_component_resource_extraction_queues(
            workstation,
            worldwake_core::ResourceExtractionQueues {
                queues: vec![ContentionQueue::default()],
            },
        )
        .unwrap();
        txn.set_component_production_output_ownership_policy(
            workstation,
            ProductionOutputOwnershipPolicy {
                output_owner: ProductionOutputOwner::Actor,
            },
        )
        .unwrap();
        txn.set_component_homeostatic_needs(actor, HomeostaticNeeds::new_sated())
            .unwrap();
        txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
            .unwrap();
        txn.set_component_drive_thresholds(actor, DriveThresholds::default())
            .unwrap();
        txn.set_component_metabolism_profile(
            actor,
            MetabolismProfile::new(
                pm(1),
                pm(1),
                pm(1),
                pm(1),
                pm(1),
                pm(20),
                nz(10),
                nz(10),
                nz(10),
                nz(10),
                nz(2),
                nz(3),
                nz(8),
                pm(0),
                pm(0),
                pm(0),
                pm(0),
            ),
        )
        .unwrap();
        if known_recipe {
            txn.set_component_known_recipes(actor, worldwake_core::KnownRecipes::new())
                .unwrap();
        }
        let _ = txn.commit(&mut EventLog::new());
        (world, actor, workstation, place)
    }

    fn grant_recipe(world: &mut World, actor: EntityId, recipe_id: worldwake_core::RecipeId) {
        let mut txn = new_txn(world, 2);
        txn.set_component_known_recipes(actor, worldwake_core::KnownRecipes::with([recipe_id]))
            .unwrap();
        commit_txn(txn);
    }

    fn set_preference_profile(world: &mut World, actor: EntityId, source_memory_capacity: u32) {
        let mut txn = new_txn(world, 2);
        txn.set_component_preference_profile(
            actor,
            PreferenceProfile {
                route_caution_weight: pm(0),
                source_trust_weight: pm(0),
                route_memory_capacity: 8,
                source_memory_capacity,
                memory_retention_ticks: 100,
                wait_sensitivity_weight: pm(150),
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    fn set_source_reliability(world: &mut World, actor: EntityId, reliability: SourceReliability) {
        let mut txn = new_txn(world, 2);
        txn.set_component_source_reliability(actor, reliability)
            .unwrap();
        commit_txn(txn);
    }

    fn setup_registries(
        recipes: &RecipeRegistry,
    ) -> (ActionDefRegistry, ActionHandlerRegistry, Vec<ActionDefId>) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let ids = register_harvest_actions(&mut defs, &mut handlers, recipes);
        (defs, handlers, ids)
    }

    fn setup_craft_registries(
        recipes: &RecipeRegistry,
    ) -> (ActionDefRegistry, ActionHandlerRegistry, Vec<ActionDefId>) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let ids = register_craft_actions(&mut defs, &mut handlers, recipes);
        (defs, handlers, ids)
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

    fn single_harvest_affordance(
        world: &World,
        actor: EntityId,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
    ) -> worldwake_sim::Affordance {
        let affordances = affordances_for(world, actor, defs, handlers);
        assert_eq!(affordances.len(), 1);
        affordances.into_iter().next().unwrap()
    }

    fn single_craft_affordance(
        world: &World,
        actor: EntityId,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
    ) -> worldwake_sim::Affordance {
        let affordances = affordances_for(world, actor, defs, handlers);
        assert_eq!(affordances.len(), 1);
        affordances.into_iter().next().unwrap()
    }

    fn craft_recipe_registry(
        body_cost_per_tick: BodyCostPerTick,
        required_tool_kinds: Vec<worldwake_core::UniqueItemKind>,
    ) -> (RecipeRegistry, worldwake_core::RecipeId) {
        let mut recipes = RecipeRegistry::new();
        let recipe_id = recipes.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: nz(2),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds,
            body_cost_per_tick,
        });
        (recipes, recipe_id)
    }

    fn craft_fixture(with_recipe: bool) -> (World, EntityId, EntityId, EntityId) {
        let (mut world, actor, workstation, place) = setup_world(false, WorkstationTag::Mill, 0);
        let mut txn = new_txn(&mut world, 2);
        txn.set_component_resource_source(
            workstation,
            ResourceSource {
                commodity: CommodityKind::Grain,
                available_quantity: Quantity(0),
                max_quantity: Quantity(0),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        )
        .unwrap();
        if with_recipe {
            txn.set_component_known_recipes(actor, worldwake_core::KnownRecipes::new())
                .unwrap();
        }
        commit_txn(txn);
        (world, actor, workstation, place)
    }

    fn add_possessed_lot(
        world: &mut World,
        actor: EntityId,
        place: EntityId,
        commodity: CommodityKind,
        quantity: u32,
    ) -> EntityId {
        let mut txn = new_txn(world, 3);
        let lot = txn.create_item_lot(commodity, Quantity(quantity)).unwrap();
        txn.set_ground_location(lot, place).unwrap();
        txn.set_possessor(lot, actor).unwrap();
        commit_txn(txn);
        lot
    }

    fn add_possessed_container_with_lot(
        world: &mut World,
        actor: EntityId,
        place: EntityId,
        commodity: CommodityKind,
        quantity: u32,
    ) -> EntityId {
        let mut txn = new_txn(world, 3);
        let container = txn
            .create_container(Container {
                capacity: LoadUnits(20),
                allowed_commodities: None,
                allows_unique_items: true,
                allows_nested_containers: true,
            })
            .unwrap();
        let lot = txn.create_item_lot(commodity, Quantity(quantity)).unwrap();
        txn.set_ground_location(container, place).unwrap();
        txn.set_possessor(container, actor).unwrap();
        txn.put_into_container(lot, container).unwrap();
        commit_txn(txn);
        lot
    }

    fn add_tool(world: &mut World, actor: EntityId, place: EntityId) {
        let mut txn = new_txn(world, 3);
        let tool = txn
            .create_unique_item(
                worldwake_core::UniqueItemKind::SimpleTool,
                Some("Mill Paddle"),
                std::collections::BTreeMap::new(),
            )
            .unwrap();
        txn.set_ground_location(tool, place).unwrap();
        txn.set_possessor(tool, actor).unwrap();
        commit_txn(txn);
    }

    fn grant_facility_use(
        world: &mut World,
        facility: EntityId,
        actor: EntityId,
        intended_action: ActionDefId,
        granted_at: u64,
    ) {
        let mut txn = new_txn(world, granted_at);
        let mut queue = ensure_facility_queue_components(&mut txn, facility);
        queue.granted = Some(ContentionGrant {
            actor,
            intended_action,
            granted_at: Tick(granted_at),
            expires_at: Tick(granted_at + 3),
        });
        txn.set_component_contention_queue(facility, queue).unwrap();
        commit_txn(txn);
    }

    fn provision_facility_queue(world: &mut World, facility: EntityId, tick: u64) {
        let mut txn = new_txn(world, tick);
        let queue = ensure_facility_queue_components(&mut txn, facility);
        txn.set_component_contention_queue(facility, queue).unwrap();
        commit_txn(txn);
    }

    fn ensure_facility_queue_components(
        txn: &mut WorldTxn<'_>,
        facility: EntityId,
    ) -> ContentionQueue {
        if txn.get_component_contention_policy(facility).is_none() {
            txn.set_component_contention_policy(
                facility,
                ContentionPolicy {
                    grant_hold_ticks: nz(3),
                    auto_promote: true,
                    max_waiters: None,
                },
            )
            .unwrap();
        }
        txn.get_component_contention_queue(facility)
            .cloned()
            .unwrap_or_else(ContentionQueue::default)
    }

    #[allow(clippy::too_many_arguments)]
    fn run_to_completion(
        world: &mut World,
        event_log: &mut EventLog,
        rng: &mut DeterministicRng,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        instance_id: ActionInstanceId,
        active: &mut BTreeMap<ActionInstanceId, ActionInstance>,
        start_tick: u64,
    ) {
        for tick in start_tick..start_tick + 4 {
            match tick_action(
                instance_id,
                defs,
                handlers,
                ActionExecutionAuthority {
                    active_actions: active,
                    world,
                    event_log,
                    rng,
                },
                worldwake_sim::ActionExecutionContext::without_recipes(
                    CauseRef::SystemTick(Tick(tick)),
                    Tick(tick),
                ),
            )
            .unwrap()
            {
                TickOutcome::Continuing => {}
                TickOutcome::Committed { .. } => return,
                TickOutcome::Aborted { reason, .. } => panic!("unexpected abort: {reason:?}"),
            }
        }
        panic!("harvest did not commit in expected tick window");
    }

    #[test]
    fn register_harvest_actions_creates_recipe_backed_action_defs() {
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, ids) = setup_registries(&recipes);

        assert_eq!(handlers.len(), 1);
        assert_eq!(ids.len(), 1);
        let def = defs.get(ids[0]).unwrap();
        assert_eq!(def.name, "harvest:Harvest Apples");
        assert_eq!(
            def.actor_constraints,
            vec![
                Constraint::ActorAlive,
                Constraint::ActorKnowsRecipe(recipe_id),
            ]
        );
        assert_eq!(
            def.preconditions,
            vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
                Precondition::TargetKind {
                    target_index: 0,
                    kind: EntityKind::Facility,
                },
                Precondition::TargetHasWorkstationTag {
                    target_index: 0,
                    tag: WorkstationTag::OrchardRow,
                },
                Precondition::TargetHasResourceSource {
                    target_index: 0,
                    commodity: CommodityKind::Apple,
                    min_available: Quantity(2),
                },
            ]
        );
        assert_eq!(
            def.payload,
            ActionPayload::Harvest(HarvestActionPayload {
                recipe_id,
                required_workstation_tag: WorkstationTag::OrchardRow,
                output_commodity: CommodityKind::Apple,
                requested_quantity: Quantity(2),
                required_tool_kinds: Vec::new(),
            })
        );
        // Commit conditions filter out `TargetHasResourceSource` so the
        // partial-success and depleted-failure paths inside `commit_harvest`
        // can surface their LastHarvestTrace and CommitTraceData aftermath.
        assert!(
            !def.commit_conditions
                .iter()
                .any(|p| matches!(p, Precondition::TargetHasResourceSource { .. })),
            "harvest commit_conditions must not duplicate TargetHasResourceSource"
        );
    }

    #[test]
    fn harvest_payload_rejects_trade_payloads() {
        let def = ActionDef {
            id: ActionDefId(77),
            name: "trade:test".to_string(),
            domain: worldwake_core::ActionDomain::Trade,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(nz(1)),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::Trade(TradeActionPayload {
                counterparty: EntityId {
                    slot: 9,
                    generation: 0,
                },
                sale_lot: EntityId {
                    slot: 50,
                    generation: 0,
                },
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(3),
                requested_quantity: Quantity(1),
            }),
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
        };

        let instance = ActionInstance {
            instance_id: ActionInstanceId(0),
            def_id: def.id,
            payload: def.payload.clone(),
            actor: EntityId {
                slot: 1,
                generation: 0,
            },
            targets: Vec::new(),
            start_tick: Tick(0),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: worldwake_sim::ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
            body_cost_override: None,
        };

        let err = harvest_payload(&def, &instance).unwrap_err();
        assert_eq!(
            err,
            ActionError::InternalError(format!("action def {} is missing harvest payload", def.id))
        );
    }

    #[test]
    fn harvest_happy_path_reduces_source_and_creates_output_lot() {
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, ids) = setup_registries(&recipes);
        let (mut world, actor, workstation, place) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        grant_recipe(&mut world, actor, recipe_id);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);
        let affordance = single_harvest_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0x81);
        let mut next_id = ActionInstanceId(0);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();

        run_to_completion(
            &mut world,
            &mut event_log,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            &mut active,
            11,
        );

        assert_eq!(
            world
                .get_component_resource_source(workstation)
                .unwrap()
                .available_quantity,
            Quantity(3)
        );
        let apple_lots = world
            .query_item_lot()
            .filter(|(entity, lot)| {
                lot.commodity == CommodityKind::Apple
                    && world.effective_place(*entity) == Some(place)
            })
            .collect::<Vec<_>>();
        assert_eq!(apple_lots.len(), 1);
        assert_eq!(apple_lots[0].1.quantity, Quantity(2));
        let record = event_log.get(EventId(event_log.len() as u64 - 1)).unwrap();
        assert!(record.tags().contains(&EventTag::ActionCommitted));
        assert!(record.tags().contains(&EventTag::WorldMutation));
    }

    #[test]
    fn harvest_commit_records_successful_source_reliability_and_enforces_capacity() {
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, ids) = setup_registries(&recipes);
        let (mut world, actor, workstation, _place) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        set_preference_profile(&mut world, actor, 1);
        set_source_reliability(
            &mut world,
            actor,
            SourceReliability {
                sources: BTreeMap::from([(
                    SourceKey {
                        entity: EntityId {
                            slot: 400,
                            generation: 1,
                        },
                        commodity: CommodityKind::Apple,
                    },
                    ReliabilityRecord {
                        successful_acquisitions: 7,
                        last_attempt_tick: Tick(1),
                        ..ReliabilityRecord::default()
                    },
                )]),
            },
        );
        grant_recipe(&mut world, actor, recipe_id);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);
        let affordance = single_harvest_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0x82);
        let mut next_id = ActionInstanceId(0);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();

        run_to_completion(
            &mut world,
            &mut event_log,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            &mut active,
            11,
        );

        let reliability = world.get_component_source_reliability(actor).unwrap();
        assert_eq!(reliability.sources.len(), 1);
        assert_eq!(
            reliability.sources.get(&SourceKey {
                entity: workstation,
                commodity: CommodityKind::Apple,
            }),
            Some(&ReliabilityRecord {
                successful_acquisitions: 1,
                last_attempt_tick: Tick(12),
                ..ReliabilityRecord::default()
            })
        );
    }

    #[test]
    fn harvest_start_failure_records_source_intrinsic_reliability_failure() {
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, ids) = setup_registries(&recipes);
        let (mut world, actor, workstation, _place) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        grant_recipe(&mut world, actor, recipe_id);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);
        let affordance = single_harvest_affordance(&world, actor, &defs, &handlers);
        {
            let mut txn = new_txn(&mut world, 10);
            txn.set_component_resource_source(
                workstation,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(0),
                    max_quantity: Quantity(5),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                    extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                    extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0x83);
        let mut next_id = ActionInstanceId(0);
        let err = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .expect_err("depleted harvest source should fail on authoritative start");

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
        assert_eq!(
            world
                .get_component_source_reliability(actor)
                .unwrap()
                .sources
                .get(&SourceKey {
                    entity: workstation,
                    commodity: CommodityKind::Apple,
                }),
            Some(&ReliabilityRecord {
                failed_attempts: 1,
                last_attempt_tick: Tick(10),
                ..ReliabilityRecord::default()
            })
        );
    }

    #[test]
    fn harvest_external_abort_does_not_update_source_reliability() {
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, ids) = setup_registries(&recipes);
        let (mut world, actor, workstation, _place) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        grant_recipe(&mut world, actor, recipe_id);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);
        let affordance = single_harvest_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0x84);
        let mut next_id = ActionInstanceId(0);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();

        abort_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(11)),
            ExternalAbortReason::Other,
        )
        .unwrap();

        assert!(
            world.get_component_source_reliability(actor).is_none(),
            "external harvest abort should not create source reliability history"
        );
    }

    #[test]
    fn harvest_affordance_requires_recipe_stock_and_matching_workstation() {
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, _) = setup_registries(&recipes);

        let (mut world_missing_recipe, actor_missing_recipe, _, _) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        assert!(
            affordances_for(
                &world_missing_recipe,
                actor_missing_recipe,
                &defs,
                &handlers
            )
            .is_empty()
        );

        let (mut world_wrong_tag, actor_wrong_tag, _, _) =
            setup_world(false, WorkstationTag::Mill, 5);
        grant_recipe(&mut world_wrong_tag, actor_wrong_tag, recipe_id);
        assert!(affordances_for(&world_wrong_tag, actor_wrong_tag, &defs, &handlers).is_empty());

        let (mut world_empty, actor_empty, _, _) =
            setup_world(false, WorkstationTag::OrchardRow, 1);
        grant_recipe(&mut world_empty, actor_empty, recipe_id);
        assert!(affordances_for(&world_empty, actor_empty, &defs, &handlers).is_empty());

        let _ = &mut world_missing_recipe;
    }

    #[test]
    fn harvest_affordance_requires_possessed_unique_tool_kind() {
        let (recipes, recipe_id) = harvest_recipe_registry_with_tools(
            BodyCostPerTick::zero(),
            vec![worldwake_core::UniqueItemKind::SimpleTool],
        );
        let (defs, handlers, _) = setup_registries(&recipes);
        let (mut world, actor, _workstation, place) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        grant_recipe(&mut world, actor, recipe_id);

        assert!(affordances_for(&world, actor, &defs, &handlers).is_empty());

        let mut txn = new_txn(&mut world, 3);
        let tool = txn
            .create_unique_item(
                worldwake_core::UniqueItemKind::SimpleTool,
                Some("Basket Hook"),
                std::collections::BTreeMap::new(),
            )
            .unwrap();
        txn.set_ground_location(tool, place).unwrap();
        txn.set_possessor(tool, actor).unwrap();
        commit_txn(txn);

        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert_eq!(affordances.len(), 1);
    }

    #[test]
    fn harvest_start_grants_extraction_slot_and_releases_on_commit() {
        // After ticket 007 the harvest start handler grants a free slot in
        // the source's `ResourceExtractionQueues` rather than consuming a
        // singleton `ContentionQueue` grant. The grant carries the actor
        // identity for the duration of the action and is cleared at commit.
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, ids) = setup_registries(&recipes);
        let (mut world, actor, workstation, _place) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        grant_recipe(&mut world, actor, recipe_id);
        let affordance = single_harvest_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0x91);
        let mut next_id = ActionInstanceId(0);

        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();

        let slot = world
            .get_component_resource_extraction_queues(workstation)
            .expect("queues registered at scenario spawn");
        assert_eq!(slot.queues.len(), 1);
        assert_eq!(
            slot.queues[0]
                .granted
                .as_ref()
                .map(|g| (g.actor, g.intended_action)),
            Some((actor, ids[0])),
        );

        run_to_completion(
            &mut world,
            &mut event_log,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            &mut active,
            11,
        );

        // Slot grant must be released after commit so the next agent can
        // claim it.
        let post_commit = world
            .get_component_resource_extraction_queues(workstation)
            .expect("queues remain registered after commit");
        assert!(
            post_commit.queues[0].granted.is_none(),
            "slot grant must be cleared on commit, got {:?}",
            post_commit.queues[0].granted
        );
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn harvest_single_slot_blocks_second_actor_and_abort_releases_slot() {
        // Single-slot source: the second actor's start fails with
        // `extraction_slots_full`, the failure handler enqueues them on
        // slot 0 (the only slot), and aborting the first actor's harvest
        // releases the slot without consuming source stock.
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, _ids) = setup_registries(&recipes);
        let (mut world, actor_a, workstation, _place) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        grant_recipe(&mut world, actor_a, recipe_id);
        let actor_b = {
            let place = world.topology().place_ids().next().unwrap();
            let mut txn = new_txn(&mut world, 3);
            let actor = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_component_known_recipes(actor, worldwake_core::KnownRecipes::with([recipe_id]))
                .unwrap();
            let _ = txn.commit(&mut EventLog::new());
            actor
        };

        let affordance_a = single_harvest_affordance(&world, actor_a, &defs, &handlers);
        let affordance_b = single_harvest_affordance(&world, actor_b, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0x82);
        let mut next_id = ActionInstanceId(0);
        let first_id = start_action(
            &affordance_a,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();

        let second_start = start_action(
            &affordance_b,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap_err();
        assert_eq!(
            second_start,
            ActionError::PreconditionFailed("extraction_slots_full".to_string()),
        );
        // Failure handler should have enqueued actor_b on the only slot.
        let queues_after_b = world
            .get_component_resource_extraction_queues(workstation)
            .expect("queues registered at spawn");
        assert_eq!(queues_after_b.queues[0].position_of(actor_b), Some(0));
        assert_eq!(
            queues_after_b.queues[0].granted.as_ref().map(|g| g.actor),
            Some(actor_a)
        );

        abort_action(
            first_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(11)),
            worldwake_sim::ExternalAbortReason::Other,
        )
        .unwrap();

        // Source stock unchanged — abort never harvested.
        assert_eq!(
            world
                .get_component_resource_source(workstation)
                .unwrap()
                .available_quantity,
            Quantity(5)
        );
        // Slot 0's grant was released by abort_harvest.
        let post_abort = world
            .get_component_resource_extraction_queues(workstation)
            .unwrap();
        assert!(post_abort.queues[0].granted.is_none());
        // actor_b is still queued (manual promotion is owned by a separate
        // ticket — no auto-promote here).
        assert_eq!(post_abort.queues[0].position_of(actor_b), Some(0));
    }

    #[test]
    fn harvest_second_start_failure_preserves_source_until_winner_commit() {
        // The losing start (slots full → enqueue) must not consume source
        // stock; only the granted actor's commit drains the source.
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (mut world, actor_a, workstation, _place) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        grant_recipe(&mut world, actor_a, recipe_id);
        let actor_b = {
            let place = world.topology().place_ids().next().unwrap();
            let mut txn = new_txn(&mut world, 3);
            let actor = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_component_known_recipes(actor, worldwake_core::KnownRecipes::with([recipe_id]))
                .unwrap();
            let _ = txn.commit(&mut EventLog::new());
            actor
        };
        let (defs, handlers, _ids) = setup_registries(&recipes);

        let affordance_a = single_harvest_affordance(&world, actor_a, &defs, &handlers);
        let affordance_b = single_harvest_affordance(&world, actor_b, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0x84);
        let mut next_id = ActionInstanceId(0);

        let first_id = start_action(
            &affordance_a,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();

        let second_start = start_action(
            &affordance_b,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap_err();
        assert_eq!(
            second_start,
            ActionError::PreconditionFailed("extraction_slots_full".to_string())
        );
        assert_eq!(
            world
                .get_component_resource_source(workstation)
                .unwrap()
                .available_quantity,
            Quantity(5),
            "losing the contested start must not consume orchard stock"
        );

        run_to_completion(
            &mut world,
            &mut event_log,
            &mut rng,
            &defs,
            &handlers,
            first_id,
            &mut active,
            11,
        );

        assert_eq!(
            world
                .get_component_resource_source(workstation)
                .unwrap()
                .available_quantity,
            Quantity(3),
            "orchard stock should drop only when the winning harvest commits"
        );
    }

    #[test]
    fn harvest_body_cost_flows_through_needs_system() {
        let body_cost = BodyCostPerTick::new(pm(2), pm(3), pm(5), pm(0), pm(7));
        let (recipes, recipe_id) = harvest_recipe_registry(body_cost);
        let (defs, handlers, ids) = setup_registries(&recipes);
        let (mut world, actor, workstation, _) = setup_world(false, WorkstationTag::OrchardRow, 5);
        grant_recipe(&mut world, actor, recipe_id);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);
        let affordance = single_harvest_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0x83);
        let mut next_id = ActionInstanceId(0);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();

        for tick in [10_u64, 11_u64] {
            let _ = tick_action(
                instance_id,
                &defs,
                &handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world: &mut world,
                    event_log: &mut event_log,
                    rng: &mut rng,
                },
                worldwake_sim::ActionExecutionContext::without_recipes(
                    CauseRef::SystemTick(Tick(tick)),
                    Tick(tick),
                ),
            )
            .unwrap();

            needs_system(SystemExecutionContext {
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
                active_actions: &active,
                action_defs: &defs,
                politics_trace: None,
                perception_trace: None,
                tick: Tick(tick),
                system_id: SystemId::Needs,
            })
            .unwrap();
        }

        let needs = world.get_component_homeostatic_needs(actor).unwrap();
        assert_eq!(
            *needs,
            HomeostaticNeeds::new(pm(4), pm(5), pm(7), pm(2), pm(9))
        );
    }

    #[test]
    fn register_craft_actions_creates_recipe_backed_defs_and_filters_invalid_shapes() {
        let (mut recipes, recipe_id) = craft_recipe_registry(BodyCostPerTick::zero(), Vec::new());
        recipes.register(RecipeDefinition {
            name: "Bad Harvest".to_string(),
            inputs: Vec::new(),
            outputs: vec![(CommodityKind::Apple, Quantity(1))],
            work_ticks: nz(1),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::zero(),
        });
        recipes.register(RecipeDefinition {
            name: "Bad Disposal".to_string(),
            inputs: vec![(CommodityKind::Waste, Quantity(1))],
            outputs: Vec::new(),
            work_ticks: nz(1),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::zero(),
        });

        let (defs, handlers, ids) = setup_craft_registries(&recipes);

        assert_eq!(handlers.len(), 1);
        assert_eq!(ids.len(), 1);
        let def = defs.get(ids[0]).unwrap();
        assert_eq!(def.name, "craft:Bake Bread");
        assert_eq!(
            def.actor_constraints,
            vec![
                Constraint::ActorAlive,
                Constraint::ActorKnowsRecipe(recipe_id),
                Constraint::ActorHasCommodityAtActorPlace {
                    kind: CommodityKind::Grain,
                    min_qty: Quantity(2),
                },
            ]
        );
        assert!(
            def.preconditions
                .contains(&Precondition::TargetLacksProductionJob(0))
        );
        assert_eq!(
            def.payload,
            ActionPayload::Craft(CraftActionPayload {
                recipe_id,
                required_workstation_tag: WorkstationTag::Mill,
                inputs: vec![(CommodityKind::Grain, Quantity(2))],
                outputs: vec![(CommodityKind::Bread, Quantity(1))],
                required_tool_kinds: Vec::new(),
            })
        );
    }

    #[test]
    fn craft_payload_rejects_trade_payloads() {
        let def = ActionDef {
            id: ActionDefId(88),
            name: "trade:test".to_string(),
            domain: worldwake_core::ActionDomain::Trade,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(nz(1)),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::Trade(TradeActionPayload {
                counterparty: EntityId {
                    slot: 10,
                    generation: 0,
                },
                sale_lot: EntityId {
                    slot: 50,
                    generation: 0,
                },
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(4),
                requested_quantity: Quantity(2),
            }),
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
        };

        let instance = ActionInstance {
            instance_id: ActionInstanceId(0),
            def_id: def.id,
            payload: def.payload.clone(),
            actor: EntityId {
                slot: 1,
                generation: 0,
            },
            targets: Vec::new(),
            start_tick: Tick(0),
            remaining_duration: worldwake_sim::ActionDuration::new(1),
            status: worldwake_sim::ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
            body_cost_override: None,
        };

        let err = craft_payload(&def, &instance).unwrap_err();
        assert_eq!(
            err,
            ActionError::InternalError(format!("action def {} is missing craft payload", def.id))
        );
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn craft_stages_inputs_tracks_wip_and_produces_outputs() {
        let (recipes, recipe_id) = craft_recipe_registry(BodyCostPerTick::zero(), Vec::new());
        let (defs, handlers, ids) = setup_craft_registries(&recipes);
        let (mut world, actor, workstation, place) = craft_fixture(false);
        grant_recipe(&mut world, actor, recipe_id);
        let source_lot = add_possessed_lot(&mut world, actor, place, CommodityKind::Grain, 3);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);
        let affordance = single_craft_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0x84);
        let mut next_id = ActionInstanceId(0);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();

        let job = world
            .get_component_production_job(workstation)
            .unwrap()
            .clone();
        assert_eq!(job.recipe_id, recipe_id);
        assert_eq!(job.worker, actor);
        assert_eq!(job.progress_ticks, 0);
        assert_eq!(
            world.get_component_item_lot(source_lot).unwrap().quantity,
            Quantity(1)
        );
        let staged_lots = world
            .recursive_contents_of(job.staged_inputs_container)
            .into_iter()
            .filter_map(|entity| {
                world
                    .get_component_item_lot(entity)
                    .map(|lot| (entity, lot.clone()))
            })
            .collect::<Vec<_>>();
        assert_eq!(staged_lots.len(), 1);
        assert_eq!(staged_lots[0].1.commodity, CommodityKind::Grain);
        assert_eq!(staged_lots[0].1.quantity, Quantity(2));

        let first_tick = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(
                CauseRef::SystemTick(Tick(11)),
                Tick(11),
            ),
        )
        .unwrap();
        assert_eq!(first_tick, TickOutcome::Continuing);
        assert_eq!(
            world
                .get_component_production_job(workstation)
                .unwrap()
                .progress_ticks,
            1
        );

        let second_tick = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(
                CauseRef::SystemTick(Tick(12)),
                Tick(12),
            ),
        )
        .unwrap();
        assert!(matches!(second_tick, TickOutcome::Committed { .. }));
        assert!(world.get_component_production_job(workstation).is_none());
        assert!(world.is_archived(job.staged_inputs_container));
        assert!(world.get_component_item_lot(staged_lots[0].0).is_none());
        let bread_lots = world
            .query_item_lot()
            .filter(|(entity, lot)| {
                lot.commodity == CommodityKind::Bread
                    && world.effective_place(*entity) == Some(place)
            })
            .collect::<Vec<_>>();
        assert_eq!(bread_lots.len(), 1);
        assert_eq!(bread_lots[0].1.quantity, Quantity(1));
        let record = event_log.get(EventId(event_log.len() as u64 - 1)).unwrap();
        assert!(record.tags().contains(&EventTag::ActionCommitted));
        assert!(record.tags().contains(&EventTag::WorldMutation));
    }

    #[test]
    fn craft_affordance_requires_recipe_tools_inputs_and_open_workstation() {
        let (recipes, recipe_id) = craft_recipe_registry(
            BodyCostPerTick::zero(),
            vec![worldwake_core::UniqueItemKind::SimpleTool],
        );
        let (defs, handlers, _) = setup_craft_registries(&recipes);

        let (mut world_missing_recipe, actor_missing_recipe, _, place_missing_recipe) =
            craft_fixture(false);
        add_possessed_lot(
            &mut world_missing_recipe,
            actor_missing_recipe,
            place_missing_recipe,
            CommodityKind::Grain,
            2,
        );
        assert!(
            affordances_for(
                &world_missing_recipe,
                actor_missing_recipe,
                &defs,
                &handlers
            )
            .is_empty()
        );

        let (mut world_missing_tool, actor_missing_tool, _, place_missing_tool) =
            craft_fixture(false);
        grant_recipe(&mut world_missing_tool, actor_missing_tool, recipe_id);
        add_possessed_lot(
            &mut world_missing_tool,
            actor_missing_tool,
            place_missing_tool,
            CommodityKind::Grain,
            2,
        );
        assert!(
            affordances_for(&world_missing_tool, actor_missing_tool, &defs, &handlers).is_empty()
        );

        let (mut world_ready, actor_ready, workstation_ready, place_ready) = craft_fixture(false);
        grant_recipe(&mut world_ready, actor_ready, recipe_id);
        add_possessed_container_with_lot(
            &mut world_ready,
            actor_ready,
            place_ready,
            CommodityKind::Grain,
            2,
        );
        add_tool(&mut world_ready, actor_ready, place_ready);
        assert_eq!(
            affordances_for(&world_ready, actor_ready, &defs, &handlers).len(),
            1
        );

        let mut txn = new_txn(&mut world_ready, 4);
        txn.set_component_production_job(
            workstation_ready,
            worldwake_core::ProductionJob {
                recipe_id,
                worker: actor_ready,
                staged_inputs_container: workstation_ready,
                progress_ticks: 1,
            },
        )
        .unwrap();
        commit_txn(txn);
        assert!(affordances_for(&world_ready, actor_ready, &defs, &handlers).is_empty());
    }

    #[test]
    fn craft_start_requires_matching_grant_and_consumes_it() {
        let (recipes, recipe_id) = craft_recipe_registry(BodyCostPerTick::zero(), Vec::new());
        let (defs, handlers, ids) = setup_craft_registries(&recipes);
        let (mut world, actor, workstation, place) = craft_fixture(false);
        grant_recipe(&mut world, actor, recipe_id);
        add_possessed_lot(&mut world, actor, place, CommodityKind::Grain, 2);
        provision_facility_queue(&mut world, workstation, 9);
        let affordance = single_craft_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0x92);
        let mut next_id = ActionInstanceId(0);

        let missing_grant_err = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap_err();
        assert!(matches!(
            missing_grant_err,
            ActionError::PreconditionFailed(message)
                if message.contains("no matching grant")
        ));

        grant_facility_use(&mut world, workstation, actor, ids[0], 11);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(11)),
        )
        .unwrap();

        assert!(
            world
                .get_component_contention_queue(workstation)
                .unwrap()
                .granted
                .is_none()
        );

        abort_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(12)),
            worldwake_sim::ExternalAbortReason::Other,
        )
        .unwrap();
    }

    #[test]
    fn interrupted_craft_preserves_job_and_staged_inputs() {
        let (recipes, recipe_id) = craft_recipe_registry(BodyCostPerTick::zero(), Vec::new());
        let (defs, handlers, ids) = setup_craft_registries(&recipes);
        let (mut world, actor, workstation, place) = craft_fixture(false);
        grant_recipe(&mut world, actor, recipe_id);
        add_possessed_lot(&mut world, actor, place, CommodityKind::Grain, 2);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);
        let affordance = single_craft_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0x85);
        let mut next_id = ActionInstanceId(0);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();

        let staged_container = world
            .get_component_production_job(workstation)
            .unwrap()
            .staged_inputs_container;
        abort_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(11)),
            worldwake_sim::ExternalAbortReason::Other,
        )
        .unwrap();

        let job = world.get_component_production_job(workstation).unwrap();
        assert_eq!(job.recipe_id, recipe_id);
        assert_eq!(job.staged_inputs_container, staged_container);
        let staged_lots = world
            .recursive_contents_of(staged_container)
            .into_iter()
            .filter(|entity| world.get_component_item_lot(*entity).is_some())
            .collect::<Vec<_>>();
        assert_eq!(staged_lots.len(), 1);
        assert_eq!(
            world
                .get_component_item_lot(staged_lots[0])
                .unwrap()
                .quantity,
            Quantity(2)
        );
    }

    #[test]
    fn craft_body_cost_flows_through_needs_system() {
        let body_cost = BodyCostPerTick::new(pm(2), pm(3), pm(5), pm(0), pm(7));
        let (recipes, recipe_id) = craft_recipe_registry(body_cost, Vec::new());
        let (defs, handlers, ids) = setup_craft_registries(&recipes);
        let (mut world, actor, workstation, place) = craft_fixture(false);
        grant_recipe(&mut world, actor, recipe_id);
        add_possessed_lot(&mut world, actor, place, CommodityKind::Grain, 2);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);
        let affordance = single_craft_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0x86);
        let mut next_id = ActionInstanceId(0);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();

        for tick in [10_u64, 11_u64] {
            let _ = tick_action(
                instance_id,
                &defs,
                &handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world: &mut world,
                    event_log: &mut event_log,
                    rng: &mut rng,
                },
                worldwake_sim::ActionExecutionContext::without_recipes(
                    CauseRef::SystemTick(Tick(tick)),
                    Tick(tick),
                ),
            )
            .unwrap();

            needs_system(SystemExecutionContext {
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
                active_actions: &active,
                action_defs: &defs,
                politics_trace: None,
                perception_trace: None,
                tick: Tick(tick),
                system_id: SystemId::Needs,
            })
            .unwrap();
        }

        let needs = world.get_component_homeostatic_needs(actor).unwrap();
        assert_eq!(
            *needs,
            HomeostaticNeeds::new(pm(4), pm(5), pm(7), pm(2), pm(9))
        );
    }

    // ── Harvest ownership tests (S01PROOUTOWNCLA-004) ──────────────────────

    /// Helper: set up a harvest, run to completion, and return the world + event log.
    fn run_harvest_to_completion_with_policy(
        policy: ProductionOutputOwnershipPolicy,
    ) -> (World, EntityId, EntityId, EntityId, EventLog) {
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, ids) = setup_registries(&recipes);
        let (mut world, actor, workstation, place) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        // Override the default Actor policy with the requested one.
        {
            let mut txn = new_txn(&mut world, 5);
            txn.set_component_production_output_ownership_policy(workstation, policy)
                .unwrap();
            commit_txn(txn);
        }
        grant_recipe(&mut world, actor, recipe_id);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);
        let affordance = single_harvest_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0xA0);
        let mut next_id = ActionInstanceId(0);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();
        run_to_completion(
            &mut world,
            &mut event_log,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            &mut active,
            11,
        );
        (world, actor, workstation, place, event_log)
    }

    /// Find the single apple lot at `place`.
    fn find_apple_lot(world: &World, place: EntityId) -> EntityId {
        let lots: Vec<_> = world
            .query_item_lot()
            .filter(|(entity, lot)| {
                lot.commodity == CommodityKind::Apple
                    && world.effective_place(*entity) == Some(place)
            })
            .collect();
        assert_eq!(lots.len(), 1, "expected exactly one apple lot at place");
        lots[0].0
    }

    #[test]
    fn harvest_actor_policy_creates_actor_owned_unpossessed_ground_lot() {
        let policy = ProductionOutputOwnershipPolicy {
            output_owner: ProductionOutputOwner::Actor,
        };
        let (world, actor, _ws, place, _log) = run_harvest_to_completion_with_policy(policy);
        let lot = find_apple_lot(&world, place);

        // Owned by the actor.
        assert_eq!(world.owner_of(lot), Some(actor));
        // Unpossessed (on ground, not in inventory).
        assert_eq!(world.possessor_of(lot), None);
        // At the workstation's place.
        assert_eq!(world.effective_place(lot), Some(place));
    }

    #[test]
    fn harvest_producer_owner_policy_creates_producer_owner_owned_output() {
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, ids) = setup_registries(&recipes);
        let (mut world, actor, workstation, place) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        // Create a facility owner and assign ownership.
        let facility_owner = {
            let mut txn = new_txn(&mut world, 5);
            let owner = txn.create_agent("Lord", ControlSource::None).unwrap();
            txn.set_ground_location(owner, place).unwrap();
            txn.set_owner(workstation, owner).unwrap();
            txn.set_component_production_output_ownership_policy(
                workstation,
                ProductionOutputOwnershipPolicy {
                    output_owner: ProductionOutputOwner::ProducerOwner,
                },
            )
            .unwrap();
            commit_txn(txn);
            owner
        };
        grant_recipe(&mut world, actor, recipe_id);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);
        let affordance = single_harvest_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0xA1);
        let mut next_id = ActionInstanceId(0);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();
        run_to_completion(
            &mut world,
            &mut event_log,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            &mut active,
            11,
        );

        let lot = find_apple_lot(&world, place);
        assert_eq!(world.owner_of(lot), Some(facility_owner));
        assert_eq!(world.possessor_of(lot), None);
    }

    #[test]
    fn harvest_unowned_policy_creates_unowned_output() {
        let policy = ProductionOutputOwnershipPolicy {
            output_owner: ProductionOutputOwner::Unowned,
        };
        let (world, _actor, _ws, place, _log) = run_harvest_to_completion_with_policy(policy);
        let lot = find_apple_lot(&world, place);

        assert_eq!(world.owner_of(lot), None);
        assert_eq!(world.possessor_of(lot), None);
    }

    #[test]
    fn harvest_producer_owner_on_ownerless_producer_fails_commit() {
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, ids) = setup_registries(&recipes);
        let (mut world, actor, workstation, _place) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        // Set ProducerOwner policy but do NOT assign an owner to the workstation.
        {
            let mut txn = new_txn(&mut world, 5);
            txn.set_component_production_output_ownership_policy(
                workstation,
                ProductionOutputOwnershipPolicy {
                    output_owner: ProductionOutputOwner::ProducerOwner,
                },
            )
            .unwrap();
            commit_txn(txn);
        }
        assert_eq!(
            world.owner_of(workstation),
            None,
            "workstation must be ownerless"
        );

        grant_recipe(&mut world, actor, recipe_id);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);
        let affordance = single_harvest_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0xA2);
        let mut next_id = ActionInstanceId(0);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();

        // run_to_completion panics on error; tick manually and expect commit failure.
        let mut committed = false;
        let mut errored = false;
        for tick in 11..15 {
            match tick_action(
                instance_id,
                &defs,
                &handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world: &mut world,
                    event_log: &mut event_log,
                    rng: &mut rng,
                },
                worldwake_sim::ActionExecutionContext::without_recipes(
                    CauseRef::SystemTick(Tick(tick)),
                    Tick(tick),
                ),
            ) {
                Ok(TickOutcome::Continuing) => {}
                Ok(TickOutcome::Committed { .. }) => {
                    committed = true;
                    break;
                }
                Err(_) => {
                    errored = true;
                    break;
                }
                _ => break,
            }
        }
        assert!(
            errored || !committed,
            "ProducerOwner on ownerless producer must fail commit"
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn harvest_missing_policy_fails_commit() {
        // Build world manually without setting ProductionOutputOwnershipPolicy.
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, ids) = setup_registries(&recipes);
        let (actor, workstation) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let ws = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(ws, place).unwrap();
            txn.set_component_workstation_marker(ws, WorkstationMarker(WorkstationTag::OrchardRow))
                .unwrap();
            txn.set_component_resource_source(
                ws,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(5),
                    max_quantity: Quantity(10),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                    extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                    extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                },
            )
            .unwrap();
            txn.set_component_resource_extraction_queues(
                ws,
                worldwake_core::ResourceExtractionQueues {
                    queues: vec![ContentionQueue::default()],
                },
            )
            .unwrap();
            // Deliberately NOT setting ProductionOutputOwnershipPolicy.
            txn.set_component_homeostatic_needs(actor, HomeostaticNeeds::new_sated())
                .unwrap();
            txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
                .unwrap();
            txn.set_component_drive_thresholds(actor, DriveThresholds::default())
                .unwrap();
            txn.set_component_metabolism_profile(
                actor,
                MetabolismProfile::new(
                    pm(1),
                    pm(1),
                    pm(1),
                    pm(1),
                    pm(1),
                    pm(20),
                    nz(10),
                    nz(10),
                    nz(10),
                    nz(10),
                    nz(2),
                    nz(3),
                    nz(8),
                    pm(0),
                    pm(0),
                    pm(0),
                    pm(0),
                ),
            )
            .unwrap();
            commit_txn(txn);
            (actor, ws)
        };
        grant_recipe(&mut world, actor, recipe_id);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);

        // Affordance filtering should still produce the harvest (policy is not a precondition
        // at affordance level — it's checked at commit time).
        let affordance = single_harvest_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0xA3);
        let mut next_id = ActionInstanceId(0);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();

        let mut committed = false;
        let mut errored = false;
        for tick in 11..15 {
            match tick_action(
                instance_id,
                &defs,
                &handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world: &mut world,
                    event_log: &mut event_log,
                    rng: &mut rng,
                },
                worldwake_sim::ActionExecutionContext::without_recipes(
                    CauseRef::SystemTick(Tick(tick)),
                    Tick(tick),
                ),
            ) {
                Ok(TickOutcome::Continuing) => {}
                Ok(TickOutcome::Committed { .. }) => {
                    committed = true;
                    break;
                }
                Err(_) => {
                    errored = true;
                    break;
                }
                _ => break,
            }
        }
        assert!(
            errored || !committed,
            "missing policy on producer must fail commit"
        );
    }

    #[test]
    fn harvest_ownership_produces_relation_delta_in_committed_event() {
        let policy = ProductionOutputOwnershipPolicy {
            output_owner: ProductionOutputOwner::Actor,
        };
        let (world, actor, _ws, place, event_log) = run_harvest_to_completion_with_policy(policy);
        let lot = find_apple_lot(&world, place);

        // Find the commit event (last event with ActionCommitted tag).
        let commit_event = (0..event_log.len())
            .rev()
            .map(|i| event_log.get(EventId(i as u64)).unwrap())
            .find(|r| r.tags().contains(&EventTag::ActionCommitted))
            .expect("expected an ActionCommitted event");

        let has_ownership_delta = commit_event.state_deltas().iter().any(|d| {
            matches!(
                d,
                StateDelta::Relation(RelationDelta::Added {
                    relation_kind: RelationKind::OwnedBy,
                    relation: RelationValue::OwnedBy {
                        entity: lot_id,
                        owner: owner_id,
                    },
                }) if *lot_id == lot && *owner_id == actor
            )
        });
        assert!(
            has_ownership_delta,
            "expected OwnedBy relation delta in committed event for lot {lot}"
        );
    }

    #[test]
    fn harvest_output_is_at_workstation_place_and_unpossessed() {
        let policy = ProductionOutputOwnershipPolicy {
            output_owner: ProductionOutputOwner::Actor,
        };
        let (world, _actor, _ws, place, _log) = run_harvest_to_completion_with_policy(policy);
        let lot = find_apple_lot(&world, place);

        assert_eq!(world.effective_place(lot), Some(place));
        assert_eq!(world.possessor_of(lot), None);
    }

    // ── Craft ownership tests (S01PROOUTOWNCLA-005) ──────────────────────

    /// Helper: set up a craft, run to completion, and return the world + event log.
    /// Uses the standard Grain→Bread recipe (2 Grain → 1 Bread, Mill workstation).
    fn run_craft_to_completion_with_policy(
        policy: ProductionOutputOwnershipPolicy,
    ) -> (World, EntityId, EntityId, EntityId, EventLog) {
        let (recipes, recipe_id) = craft_recipe_registry(BodyCostPerTick::zero(), Vec::new());
        let (defs, handlers, ids) = setup_craft_registries(&recipes);
        let (mut world, actor, workstation, place) = craft_fixture(false);
        // Override the default Actor policy with the requested one.
        {
            let mut txn = new_txn(&mut world, 5);
            txn.set_component_production_output_ownership_policy(workstation, policy)
                .unwrap();
            commit_txn(txn);
        }
        grant_recipe(&mut world, actor, recipe_id);
        // Craft requires possessed input lots (2 Grain).
        add_possessed_lot(&mut world, actor, place, CommodityKind::Grain, 3);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);
        let affordance = single_craft_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0xB0);
        let mut next_id = ActionInstanceId(0);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();
        run_to_completion(
            &mut world,
            &mut event_log,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            &mut active,
            11,
        );
        (world, actor, workstation, place, event_log)
    }

    /// Find the single bread lot at `place`.
    fn find_bread_lot(world: &World, place: EntityId) -> EntityId {
        let lots: Vec<_> = world
            .query_item_lot()
            .filter(|(entity, lot)| {
                lot.commodity == CommodityKind::Bread
                    && world.effective_place(*entity) == Some(place)
            })
            .collect();
        assert_eq!(lots.len(), 1, "expected exactly one bread lot at place");
        lots[0].0
    }

    #[test]
    fn craft_actor_policy_creates_actor_owned_unpossessed_ground_lot() {
        let policy = ProductionOutputOwnershipPolicy {
            output_owner: ProductionOutputOwner::Actor,
        };
        let (world, actor, _ws, place, _log) = run_craft_to_completion_with_policy(policy);
        let lot = find_bread_lot(&world, place);

        assert_eq!(world.owner_of(lot), Some(actor));
        assert_eq!(world.possessor_of(lot), None);
        assert_eq!(world.effective_place(lot), Some(place));
    }

    #[test]
    fn craft_producer_owner_policy_creates_producer_owner_owned_output() {
        let (recipes, recipe_id) = craft_recipe_registry(BodyCostPerTick::zero(), Vec::new());
        let (defs, handlers, ids) = setup_craft_registries(&recipes);
        let (mut world, actor, workstation, place) = craft_fixture(false);
        // Create a facility owner and assign ownership.
        let facility_owner = {
            let mut txn = new_txn(&mut world, 5);
            let owner = txn
                .create_agent("GuildMaster", ControlSource::None)
                .unwrap();
            txn.set_ground_location(owner, place).unwrap();
            txn.set_owner(workstation, owner).unwrap();
            txn.set_component_production_output_ownership_policy(
                workstation,
                ProductionOutputOwnershipPolicy {
                    output_owner: ProductionOutputOwner::ProducerOwner,
                },
            )
            .unwrap();
            commit_txn(txn);
            owner
        };
        grant_recipe(&mut world, actor, recipe_id);
        add_possessed_lot(&mut world, actor, place, CommodityKind::Grain, 3);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);
        let affordance = single_craft_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0xB1);
        let mut next_id = ActionInstanceId(0);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();
        run_to_completion(
            &mut world,
            &mut event_log,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            &mut active,
            11,
        );

        let lot = find_bread_lot(&world, place);
        assert_eq!(world.owner_of(lot), Some(facility_owner));
        assert_eq!(world.possessor_of(lot), None);
    }

    #[test]
    fn craft_unowned_policy_creates_unowned_output() {
        let policy = ProductionOutputOwnershipPolicy {
            output_owner: ProductionOutputOwner::Unowned,
        };
        let (world, _actor, _ws, place, _log) = run_craft_to_completion_with_policy(policy);
        let lot = find_bread_lot(&world, place);

        assert_eq!(world.owner_of(lot), None);
        assert_eq!(world.possessor_of(lot), None);
    }

    #[test]
    fn craft_producer_owner_on_ownerless_producer_fails_commit() {
        let (recipes, recipe_id) = craft_recipe_registry(BodyCostPerTick::zero(), Vec::new());
        let (defs, handlers, ids) = setup_craft_registries(&recipes);
        let (mut world, actor, workstation, place) = craft_fixture(false);
        // Set ProducerOwner policy but do NOT assign an owner to the workstation.
        {
            let mut txn = new_txn(&mut world, 5);
            txn.set_component_production_output_ownership_policy(
                workstation,
                ProductionOutputOwnershipPolicy {
                    output_owner: ProductionOutputOwner::ProducerOwner,
                },
            )
            .unwrap();
            commit_txn(txn);
        }
        assert_eq!(
            world.owner_of(workstation),
            None,
            "workstation must be ownerless"
        );

        grant_recipe(&mut world, actor, recipe_id);
        add_possessed_lot(&mut world, actor, place, CommodityKind::Grain, 3);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);
        let affordance = single_craft_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0xB2);
        let mut next_id = ActionInstanceId(0);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();

        // Tick manually — expect commit failure, not panic.
        let mut committed = false;
        let mut errored = false;
        for tick in 11..15 {
            match tick_action(
                instance_id,
                &defs,
                &handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world: &mut world,
                    event_log: &mut event_log,
                    rng: &mut rng,
                },
                worldwake_sim::ActionExecutionContext::without_recipes(
                    CauseRef::SystemTick(Tick(tick)),
                    Tick(tick),
                ),
            ) {
                Ok(TickOutcome::Continuing) => {}
                Ok(TickOutcome::Committed { .. }) => {
                    committed = true;
                    break;
                }
                Err(_) => {
                    errored = true;
                    break;
                }
                _ => break,
            }
        }
        assert!(
            errored || !committed,
            "ProducerOwner on ownerless producer must fail craft commit"
        );
    }

    #[test]
    fn craft_all_outputs_share_same_ownership() {
        // Use a recipe with multiple outputs to verify all get the same owner.
        let mut recipes = RecipeRegistry::new();
        let recipe_id = recipes.register(RecipeDefinition {
            name: "Multi-Output".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![
                (CommodityKind::Bread, Quantity(1)),
                (CommodityKind::Firewood, Quantity(1)),
            ],
            work_ticks: nz(2),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::zero(),
        });
        let (defs, handlers, ids) = setup_craft_registries(&recipes);
        let (mut world, actor, workstation, place) = craft_fixture(false);
        {
            let mut txn = new_txn(&mut world, 5);
            txn.set_component_production_output_ownership_policy(
                workstation,
                ProductionOutputOwnershipPolicy {
                    output_owner: ProductionOutputOwner::Actor,
                },
            )
            .unwrap();
            commit_txn(txn);
        }
        grant_recipe(&mut world, actor, recipe_id);
        add_possessed_lot(&mut world, actor, place, CommodityKind::Grain, 3);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);
        let affordance = single_craft_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0xB3);
        let mut next_id = ActionInstanceId(0);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();
        run_to_completion(
            &mut world,
            &mut event_log,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            &mut active,
            11,
        );

        // All output lots at place should be owned by the actor.
        let output_lots: Vec<_> = world
            .query_item_lot()
            .filter(|(entity, lot)| {
                (lot.commodity == CommodityKind::Bread || lot.commodity == CommodityKind::Firewood)
                    && world.effective_place(*entity) == Some(place)
            })
            .collect();
        assert_eq!(output_lots.len(), 2, "expected two output lots");
        for (lot_id, _lot) in &output_lots {
            assert_eq!(
                world.owner_of(*lot_id),
                Some(actor),
                "all outputs must share actor ownership"
            );
            assert_eq!(
                world.possessor_of(*lot_id),
                None,
                "outputs must be unpossessed"
            );
        }
    }

    #[test]
    fn craft_golden_scenario_works_with_actor_owned_output() {
        // This is the standard craft test with explicit Actor policy — regression guard.
        let policy = ProductionOutputOwnershipPolicy {
            output_owner: ProductionOutputOwner::Actor,
        };
        let (world, actor, _ws, place, event_log) = run_craft_to_completion_with_policy(policy);
        let lot = find_bread_lot(&world, place);

        // Output exists, is owned, unpossessed, at place.
        assert_eq!(world.owner_of(lot), Some(actor));
        assert_eq!(world.possessor_of(lot), None);
        assert_eq!(world.effective_place(lot), Some(place));
        // Commit event was recorded.
        let commit_event = (0..event_log.len())
            .rev()
            .map(|i| event_log.get(EventId(i as u64)).unwrap())
            .find(|r| r.tags().contains(&EventTag::ActionCommitted))
            .expect("expected an ActionCommitted event");
        assert!(commit_event.tags().contains(&EventTag::WorldMutation));
    }

    fn make_harvest_instance(
        def: &ActionDef,
        actor: EntityId,
        workstation: EntityId,
    ) -> ActionInstance {
        ActionInstance {
            instance_id: ActionInstanceId(1),
            def_id: def.id,
            payload: def.payload.clone(),
            actor,
            targets: vec![workstation],
            start_tick: Tick(10),
            remaining_duration: worldwake_sim::ActionDuration::new(0),
            status: worldwake_sim::ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
            body_cost_override: None,
        }
    }

    fn invoke_commit_harvest(
        world: &mut World,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        def_id: ActionDefId,
        actor: EntityId,
        workstation: EntityId,
        commit_tick: u64,
    ) -> Result<CommitOutcome, ActionError> {
        let def = defs.get(def_id).unwrap().clone();
        let handler = handlers.get(def.handler).unwrap();
        let instance = make_harvest_instance(&def, actor, workstation);
        let event_log = EventLog::new();
        let mut rng = test_rng(0xCC);
        let recipes = RecipeRegistry::new();
        let context = worldwake_sim::ActionExecutionContext {
            cause: CauseRef::SystemTick(Tick(commit_tick)),
            tick: Tick(commit_tick),
            recipe_registry: &recipes,
            action_defs: defs,
        };
        let mut txn = new_txn(world, commit_tick);
        let result = (handler.on_commit)(&def, &instance, &context, &event_log, &mut rng, &mut txn);
        // Mirror the runtime: commit the txn on Ok and on AbortRequested
        // (the latter routes through `finalize_failed_action` which commits).
        // For other errors the runtime drops the txn.
        let should_commit = matches!(&result, Ok(_) | Err(ActionError::AbortRequested(_)));
        if should_commit {
            commit_txn(txn);
        }
        result
    }

    fn drain_source_to(world: &mut World, workstation: EntityId, available: u32, tick: u64) {
        let mut txn = new_txn(world, tick);
        let mut source = txn
            .get_component_resource_source(workstation)
            .cloned()
            .unwrap();
        source.available_quantity = Quantity(available);
        txn.set_component_resource_source(workstation, source)
            .unwrap();
        commit_txn(txn);
    }

    #[test]
    fn commit_harvest_full_success_emits_no_partial_trace() {
        // Recipe outputs 2 apples; source has 5 — full success.
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, ids) = setup_registries(&recipes);
        let (mut world, actor, workstation, place) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        grant_recipe(&mut world, actor, recipe_id);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);

        let outcome =
            invoke_commit_harvest(&mut world, &defs, &handlers, ids[0], actor, workstation, 12)
                .expect("full-quantity harvest commit succeeds");

        // Trace should be absent for full-success (no partial path triggered).
        assert!(
            outcome.trace.is_none(),
            "full-success commit must not emit CommitTraceData::Harvest, got {:?}",
            outcome.trace
        );

        // Source drained by exactly the requested 2.
        let source = world
            .get_component_resource_source(workstation)
            .expect("source still exists");
        assert_eq!(source.available_quantity, Quantity(3));

        // Item lot of 2 apples appears at the workstation's place.
        let apple_lots: Vec<_> = world
            .query_item_lot()
            .filter(|(entity, lot)| {
                lot.commodity == CommodityKind::Apple
                    && world.effective_place(*entity) == Some(place)
            })
            .collect();
        assert_eq!(apple_lots.len(), 1);
        assert_eq!(apple_lots[0].1.quantity, Quantity(2));

        // LastHarvestTrace records a non-partial entry.
        let trace = world
            .get_component_last_harvest_trace(workstation)
            .expect("LastHarvestTrace appended on commit");
        assert_eq!(trace.entries.len(), 1);
        let entry = trace.entries[0];
        assert_eq!(entry.harvester, actor);
        assert_eq!(entry.quantity, 2);
        assert!(!entry.partial);
    }

    #[test]
    fn commit_harvest_partial_success_emits_partial_trace_and_drains_source() {
        // Recipe outputs 2 apples; setup with 2 to pass start preconditions, then
        // drain to 1 to simulate concurrent depletion before commit fires.
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, ids) = setup_registries(&recipes);
        let (mut world, actor, workstation, place) =
            setup_world(false, WorkstationTag::OrchardRow, 2);
        grant_recipe(&mut world, actor, recipe_id);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);

        // Concurrent draw simulation: drain to 1 before commit.
        drain_source_to(&mut world, workstation, 1, 11);

        let outcome =
            invoke_commit_harvest(&mut world, &defs, &handlers, ids[0], actor, workstation, 12)
                .expect("partial-success commit succeeds with actual >= 1");

        let trace = match outcome.trace {
            Some(CommitTraceData::Harvest(harvest)) => harvest,
            other => panic!("expected CommitTraceData::Harvest, got {other:?}"),
        };
        assert_eq!(trace.requested_quantity, Quantity(2));
        assert_eq!(trace.partial_quantity, Some(Quantity(1)));

        // Source drained to 0 (took the remaining 1).
        let source = world
            .get_component_resource_source(workstation)
            .expect("source still exists");
        assert_eq!(source.available_quantity, Quantity(0));

        // Item lot of exactly the actual quantity (1 apple) is created.
        let apple_lots: Vec<_> = world
            .query_item_lot()
            .filter(|(entity, lot)| {
                lot.commodity == CommodityKind::Apple
                    && world.effective_place(*entity) == Some(place)
            })
            .collect();
        assert_eq!(apple_lots.len(), 1);
        assert_eq!(apple_lots[0].1.quantity, Quantity(1));

        // LastHarvestTrace records the partial harvest.
        let last_trace = world
            .get_component_last_harvest_trace(workstation)
            .expect("LastHarvestTrace appended on partial commit");
        assert_eq!(last_trace.entries.len(), 1);
        let entry = last_trace.entries[0];
        assert_eq!(entry.harvester, actor);
        assert_eq!(entry.quantity, 1);
        assert!(entry.partial);
    }

    #[test]
    fn commit_harvest_depleted_failure_records_zero_quantity_partial_trace() {
        // Recipe outputs 2; pass start with 2; drain to 0 before commit.
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, ids) = setup_registries(&recipes);
        let (mut world, actor, workstation, place) =
            setup_world(false, WorkstationTag::OrchardRow, 2);
        grant_recipe(&mut world, actor, recipe_id);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);

        drain_source_to(&mut world, workstation, 0, 11);

        let err =
            invoke_commit_harvest(&mut world, &defs, &handlers, ids[0], actor, workstation, 12)
                .expect_err("depleted source must fail commit");

        // The depleted-failure path returns AbortRequested so that
        // `finalize_failed_action` commits the LastHarvestTrace append
        // (PreconditionFailed would drop the txn — see ticket reassessment 19).
        match err {
            ActionError::AbortRequested(
                worldwake_sim::ActionAbortRequestReason::HarvestSourceDepleted { workstation: ws },
            ) if ws == workstation => {}
            other => panic!("expected AbortRequested(HarvestSourceDepleted), got {other:?}"),
        }

        // Source remains at 0; no item lot is created at the place.
        let source = world
            .get_component_resource_source(workstation)
            .expect("source still exists");
        assert_eq!(source.available_quantity, Quantity(0));

        let apple_lots: Vec<_> = world
            .query_item_lot()
            .filter(|(entity, lot)| {
                lot.commodity == CommodityKind::Apple
                    && world.effective_place(*entity) == Some(place)
            })
            .collect();
        assert!(
            apple_lots.is_empty(),
            "depleted commit must not create any item lot"
        );

        // LastHarvestTrace records the failed harvest with quantity 0 and partial=true.
        let last_trace = world
            .get_component_last_harvest_trace(workstation)
            .expect("LastHarvestTrace appended even on depleted-failure commit");
        assert_eq!(last_trace.entries.len(), 1);
        let entry = last_trace.entries[0];
        assert_eq!(entry.harvester, actor);
        assert_eq!(entry.quantity, 0);
        assert!(entry.partial);
    }

    #[test]
    fn ai_tick_records_partial_inventory_delta() {
        // Verifies the spec contract: after a partial harvest commit, the agent's
        // believed inventory of the commodity reflects the *actual* harvested
        // quantity (1), not the *requested* quantity (2). The mechanism is
        // perception of the actual ItemLot the commit creates — no new
        // CommitTraceData consumer is required (FND-14A).
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, ids) = setup_registries(&recipes);
        let (mut world, actor, workstation, _place) =
            setup_world(false, WorkstationTag::OrchardRow, 2);
        grant_recipe(&mut world, actor, recipe_id);
        grant_facility_use(&mut world, workstation, actor, ids[0], 9);

        drain_source_to(&mut world, workstation, 1, 11);

        let _ = invoke_commit_harvest(&mut world, &defs, &handlers, ids[0], actor, workstation, 12)
            .expect("partial-success commit succeeds");

        // Build a belief view as the actor would observe their possessions
        // through perception of co-located lots; carry-quantity must reflect
        // the actual lot (1), not the requested quantity (2).
        let store = test_belief_store(&world, actor);
        let belief_view = PerAgentBeliefView::new(actor, &world, &store);
        let believed_apples =
            <PerAgentBeliefView<'_> as worldwake_sim::InventoryBeliefView>::commodity_quantity(
                &belief_view,
                actor,
                CommodityKind::Apple,
            );
        assert_eq!(
            believed_apples,
            Quantity(1),
            "agent's believed apple inventory must reflect partial-harvest actual quantity"
        );
    }

    #[test]
    fn harvest_payload_validator_rejects_overcarry() {
        // Validator at registration time accepts only requested_quantity within
        // believed carry headroom. Drives the actor's headroom near zero by
        // adding an oversized possessed lot, then asserts the override fails.
        let (recipes, _recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, ids) = setup_registries(&recipes);
        let (mut world, actor, workstation, place) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        // Saturate the actor's load so any harvest of >= 1 apple exceeds headroom.
        // CarryCapacity default is 100 LoadUnits; load_per_unit(Apple) is 1.
        // Possess 100 apples to drive headroom to 0.
        let _heavy_lot = add_possessed_lot(&mut world, actor, place, CommodityKind::Apple, 100);

        let def = defs.get(ids[0]).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let store = test_belief_store(&world, actor);
        let view = PerAgentBeliefView::new(actor, &world, &store);

        // Overrided payload requesting 2 apples — exceeds carry headroom.
        let override_payload = ActionPayload::Harvest(HarvestActionPayload {
            recipe_id: worldwake_core::RecipeId(0),
            required_workstation_tag: WorkstationTag::OrchardRow,
            output_commodity: CommodityKind::Apple,
            requested_quantity: Quantity(2),
            required_tool_kinds: Vec::new(),
        });
        assert!(
            !(handler.payload_override_is_valid)(
                def,
                actor,
                std::slice::from_ref(&workstation),
                &override_payload,
                &view,
            ),
            "validator must reject overcarry"
        );

        // A valid override with quantity = 0 also fails (>= 1 floor).
        let zero_payload = ActionPayload::Harvest(HarvestActionPayload {
            recipe_id: worldwake_core::RecipeId(0),
            required_workstation_tag: WorkstationTag::OrchardRow,
            output_commodity: CommodityKind::Apple,
            requested_quantity: Quantity(0),
            required_tool_kinds: Vec::new(),
        });
        assert!(
            !(handler.payload_override_is_valid)(
                def,
                actor,
                std::slice::from_ref(&workstation),
                &zero_payload,
                &view,
            ),
            "validator must reject zero-quantity request"
        );
    }

    /// Helper: spawn a second agent at the same place as `existing_actor`
    /// with the given name and recipe knowledge.
    fn spawn_co_located_agent(
        world: &mut World,
        existing_actor: EntityId,
        name: &str,
        recipe_id: worldwake_core::RecipeId,
        tick: u64,
    ) -> EntityId {
        let place = world.effective_place(existing_actor).unwrap();
        let mut txn = new_txn(world, tick);
        let actor = txn.create_agent(name, ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, place).unwrap();
        txn.set_component_known_recipes(actor, worldwake_core::KnownRecipes::with([recipe_id]))
            .unwrap();
        commit_txn(txn);
        actor
    }

    /// Helper: replace the source's `extraction_slots` and reinitialize
    /// `ResourceExtractionQueues` to the matching length.
    fn set_extraction_slots(world: &mut World, workstation: EntityId, slots: u8) {
        let mut txn = new_txn(world, 4);
        let mut source = txn
            .get_component_resource_source(workstation)
            .cloned()
            .unwrap();
        source.extraction_slots = std::num::NonZeroU8::new(slots).unwrap();
        txn.set_component_resource_source(workstation, source)
            .unwrap();
        txn.set_component_resource_extraction_queues(
            workstation,
            worldwake_core::ResourceExtractionQueues {
                queues: vec![ContentionQueue::default(); usize::from(slots)],
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    #[test]
    fn harvest_start_picks_free_slot_for_three_concurrent_agents() {
        // 3-slot source; three agents start harvest concurrently. Each should
        // claim a different slot via the lowest-free-slot policy.
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, _ids) = setup_registries(&recipes);
        let (mut world, actor_a, workstation, _place) =
            setup_world(false, WorkstationTag::OrchardRow, 9);
        grant_recipe(&mut world, actor_a, recipe_id);
        set_extraction_slots(&mut world, workstation, 3);
        let actor_b = spawn_co_located_agent(&mut world, actor_a, "Bram", recipe_id, 5);
        let actor_c = spawn_co_located_agent(&mut world, actor_a, "Cael", recipe_id, 5);

        let affordance_a = single_harvest_affordance(&world, actor_a, &defs, &handlers);
        let affordance_b = single_harvest_affordance(&world, actor_b, &defs, &handlers);
        let affordance_c = single_harvest_affordance(&world, actor_c, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0xA1);
        let mut next_id = ActionInstanceId(0);

        for affordance in [affordance_a, affordance_b, affordance_c] {
            start_action(
                &affordance,
                &defs,
                &handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world: &mut world,
                    event_log: &mut event_log,
                    rng: &mut rng,
                },
                &mut next_id,
                worldwake_sim::ActionExecutionContext::without_recipes(
                    CauseRef::Bootstrap,
                    Tick(10),
                ),
            )
            .unwrap();
        }

        let queues = world
            .get_component_resource_extraction_queues(workstation)
            .expect("queues registered at spawn");
        assert_eq!(queues.queues.len(), 3);
        // Each slot has a distinct grant; together they cover all three actors.
        let granted: Vec<EntityId> = queues
            .queues
            .iter()
            .map(|q| q.granted.as_ref().expect("each slot granted").actor)
            .collect();
        assert_eq!(granted.len(), 3);
        assert!(granted.contains(&actor_a));
        assert!(granted.contains(&actor_b));
        assert!(granted.contains(&actor_c));
        // No actor is in any waiting queue.
        for queue in &queues.queues {
            assert!(queue.waiting.is_empty(), "no actor should be waiting");
        }
    }

    #[test]
    fn harvest_start_enqueues_third_actor_when_single_slot_is_full() {
        // 1-slot source; first start grants slot 0, second start fails with
        // `extraction_slots_full` and the failure handler enqueues actor_b
        // on slot 0. A third actor enqueues behind actor_b on the same slot
        // (only slot available has the shortest waitlist by definition).
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, _ids) = setup_registries(&recipes);
        let (mut world, actor_a, workstation, _place) =
            setup_world(false, WorkstationTag::OrchardRow, 9);
        grant_recipe(&mut world, actor_a, recipe_id);
        let actor_b = spawn_co_located_agent(&mut world, actor_a, "Bram", recipe_id, 5);
        let actor_c = spawn_co_located_agent(&mut world, actor_a, "Cael", recipe_id, 5);

        let affordance_a = single_harvest_affordance(&world, actor_a, &defs, &handlers);
        let affordance_b = single_harvest_affordance(&world, actor_b, &defs, &handlers);
        let affordance_c = single_harvest_affordance(&world, actor_c, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0xA2);
        let mut next_id = ActionInstanceId(0);

        // First start: grants slot 0.
        start_action(
            &affordance_a,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap();

        // Second start: fails with extraction_slots_full; failure handler enqueues actor_b.
        let err_b = start_action(
            &affordance_b,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap_err();
        assert_eq!(
            err_b,
            ActionError::PreconditionFailed("extraction_slots_full".to_string())
        );

        // Third start: same fate, enqueues actor_c behind actor_b.
        let err_c = start_action(
            &affordance_c,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
            },
            &mut next_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
        )
        .unwrap_err();
        assert_eq!(
            err_c,
            ActionError::PreconditionFailed("extraction_slots_full".to_string())
        );

        let queues = world
            .get_component_resource_extraction_queues(workstation)
            .expect("queues registered at spawn");
        assert_eq!(queues.queues.len(), 1);
        let slot = &queues.queues[0];
        assert_eq!(slot.granted.as_ref().map(|g| g.actor), Some(actor_a));
        assert_eq!(slot.position_of(actor_b), Some(0));
        assert_eq!(slot.position_of(actor_c), Some(1));
    }
}
