use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    ActionDefId, CommodityKind, Container, EntityId, EntityKind, EventTag, LoadUnits, Quantity,
    VisibilitySpec, WorkstationMarker, World, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerId,
    ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress, ActionState,
    CommitOutcome, Constraint, CraftActionPayload, DeterministicRng, DurationExpr,
    HarvestActionPayload, Interruptibility, Precondition, RecipeDefinition, RecipeRegistry,
    ReservationReq, TargetSpec,
};

pub fn register_harvest_actions(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
    recipes: &RecipeRegistry,
) -> Vec<ActionDefId> {
    let handler = handlers.register(
        ActionHandler::new(start_harvest, tick_harvest, commit_harvest, abort_harvest)
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
        domain: worldwake_sim::ActionDomain::Production,
        actor_constraints,
        targets: vec![TargetSpec::EntityAtActorPlace {
            kind: EntityKind::Facility,
        }],
        preconditions: preconditions.clone(),
        reservation_requirements: vec![ReservationReq { target_index: 0 }],
        duration: DurationExpr::Fixed(recipe.work_ticks),
        body_cost_per_tick: recipe.body_cost_per_tick,
        interruptibility: Interruptibility::InterruptibleWithPenalty,
        commit_conditions: preconditions,
        visibility: VisibilitySpec::ParticipantsOnly,
        causal_event_tags: BTreeSet::from([EventTag::WorldMutation]),
        payload: ActionPayload::Harvest(HarvestActionPayload {
            recipe_id,
            required_workstation_tag: workstation_tag,
            output_commodity: *output_commodity,
            output_quantity: *output_quantity,
            required_tool_kinds: recipe.required_tool_kinds.clone(),
        }),
        handler,
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
            .map(|(kind, min_qty)| Constraint::ActorHasCommodity { kind, min_qty }),
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
        domain: worldwake_sim::ActionDomain::Production,
        actor_constraints,
        targets: vec![TargetSpec::EntityAtActorPlace {
            kind: EntityKind::Facility,
        }],
        preconditions: preconditions.clone(),
        reservation_requirements: vec![ReservationReq { target_index: 0 }],
        duration: DurationExpr::Fixed(recipe.work_ticks),
        body_cost_per_tick: recipe.body_cost_per_tick,
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

fn ensure_matching_facility_grant(
    world: &World,
    actor: EntityId,
    facility: EntityId,
    action_def: ActionDefId,
) -> Result<(), ActionError> {
    let policy = world.get_component_exclusive_facility_policy(facility);
    let queue = world.get_component_facility_use_queue(facility);
    let queue = match (policy, queue) {
        (None, None) => return Ok(()),
        (Some(_), Some(queue)) => queue,
        (Some(_), None) => {
            return Err(ActionError::PreconditionFailed(format!(
                "facility {facility} is exclusive but lacks FacilityUseQueue grant state"
            )))
        }
        (None, Some(_)) => {
            return Err(ActionError::PreconditionFailed(format!(
            "facility {facility} has FacilityUseQueue grant state without ExclusiveFacilityPolicy"
        )))
        }
    };
    match queue.granted.as_ref() {
        Some(granted) if granted.actor == actor && granted.intended_action == action_def => Ok(()),
        Some(granted) => Err(ActionError::PreconditionFailed(format!(
            "facility {facility} grant belongs to actor {} action {:?}, not actor {actor} action {:?}",
            granted.actor, granted.intended_action, action_def
        ))),
        None => Err(ActionError::PreconditionFailed(format!(
            "facility {facility} has no matching grant for actor {actor} action {action_def:?}"
        ))),
    }
}

fn consume_matching_facility_grant(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    facility: EntityId,
    action_def: ActionDefId,
) -> Result<(), ActionError> {
    ensure_matching_facility_grant(txn, actor, facility, action_def)?;
    if txn
        .get_component_exclusive_facility_policy(facility)
        .is_none()
        && txn.get_component_facility_use_queue(facility).is_none()
    {
        return Ok(());
    }
    let mut queue = txn
        .get_component_facility_use_queue(facility)
        .cloned()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "facility {facility} lacks FacilityUseQueue grant state"
            ))
        })?;
    queue.clear_grant();
    txn.set_component_facility_use_queue(facility, queue)
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
    ensure_matching_facility_grant(world, actor, facility, def.id)
}

fn start_harvest(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let _ = harvest_payload(def, instance)?;
    let workstation = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    consume_matching_facility_grant(txn, instance.actor, workstation, def.id)?;
    Ok(None)
}

#[allow(clippy::unnecessary_wraps)]
fn tick_harvest(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn start_craft(
    def: &ActionDef,
    instance: &ActionInstance,
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
    instance: &ActionInstance,
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

fn commit_harvest(
    def: &ActionDef,
    instance: &ActionInstance,
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
    source.available_quantity = source
        .available_quantity
        .checked_sub(payload.output_quantity)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "resource source {workstation} lacks {:?} units for harvest",
                payload.output_quantity
            ))
        })?;
    txn.set_component_resource_source(workstation, source)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;

    let lot = txn
        .create_item_lot(payload.output_commodity, payload.output_quantity)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.set_ground_location(lot, place)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.add_target(lot);
    Ok(CommitOutcome::empty())
}

fn commit_craft(
    def: &ActionDef,
    instance: &ActionInstance,
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

    for (commodity, quantity) in &payload.outputs {
        let lot = txn
            .create_item_lot(*commodity, *quantity)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
        txn.set_ground_location(lot, place)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
        txn.add_target(lot);
    }
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_harvest(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _reason: &AbortReason,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_craft(
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
    use super::{register_craft_actions, register_harvest_actions};
    use crate::needs::needs_system;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_believed_entity_state, build_prototype_world, AgentBeliefStore, BodyCostPerTick,
        CauseRef, CommodityKind, Container, ControlSource, DeprivationExposure, DriveThresholds,
        EntityId, EventId, EventLog, ExclusiveFacilityPolicy, FacilityUseQueue, GrantedFacilityUse,
        EventView, HomeostaticNeeds, LoadUnits, MetabolismProfile, PerceptionSource, Permille,
        Quantity, ResourceSource, Seed, Tick, VisibilitySpec, WitnessData, WorkstationMarker,
        WorkstationTag, World, WorldTxn,
    };
    use worldwake_sim::{
        abort_action, get_affordances, start_action, tick_action, ActionDefRegistry,
        ActionExecutionAuthority, ActionExecutionContext, ActionHandlerRegistry, ActionInstance,
        ActionInstanceId, ActionPayload, DeterministicRng, PerAgentBeliefView, RecipeRegistry,
        SystemExecutionContext, SystemId, TickOutcome, TradeActionPayload,
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
        queue.granted = Some(GrantedFacilityUse {
            actor,
            intended_action,
            granted_at: Tick(granted_at),
            expires_at: Tick(granted_at + 3),
        });
        txn.set_component_facility_use_queue(facility, queue)
            .unwrap();
        commit_txn(txn);
    }

    fn provision_facility_queue(world: &mut World, facility: EntityId, tick: u64) {
        let mut txn = new_txn(world, tick);
        let queue = ensure_facility_queue_components(&mut txn, facility);
        txn.set_component_facility_use_queue(facility, queue)
            .unwrap();
        commit_txn(txn);
    }

    fn ensure_facility_queue_components(
        txn: &mut WorldTxn<'_>,
        facility: EntityId,
    ) -> FacilityUseQueue {
        if txn
            .get_component_exclusive_facility_policy(facility)
            .is_none()
        {
            txn.set_component_exclusive_facility_policy(
                facility,
                ExclusiveFacilityPolicy {
                    grant_hold_ticks: nz(3),
                },
            )
            .unwrap();
        }
        txn.get_component_facility_use_queue(facility)
            .cloned()
            .unwrap_or_else(FacilityUseQueue::default)
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
                ActionExecutionContext {
                    cause: CauseRef::SystemTick(Tick(tick)),
                    tick: Tick(tick),
                },
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
                output_quantity: Quantity(2),
                required_tool_kinds: Vec::new(),
            })
        );
    }

    #[test]
    fn harvest_payload_rejects_trade_payloads() {
        let def = ActionDef {
            id: ActionDefId(77),
            name: "trade:test".to_string(),
            domain: worldwake_sim::ActionDomain::Trade,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(nz(1)),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::Trade(TradeActionPayload {
                counterparty: EntityId {
                    slot: 9,
                    generation: 0,
                },
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(3),
                requested_commodity: CommodityKind::Bread,
                requested_quantity: Quantity(1),
            }),
            handler: ActionHandlerId(0),
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
            remaining_duration: worldwake_sim::ActionDuration::Finite(1),
            status: worldwake_sim::ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
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
    fn harvest_affordance_requires_recipe_stock_and_matching_workstation() {
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, _) = setup_registries(&recipes);

        let (mut world_missing_recipe, actor_missing_recipe, _, _) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        assert!(affordances_for(
            &world_missing_recipe,
            actor_missing_recipe,
            &defs,
            &handlers
        )
        .is_empty());

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

    #[allow(clippy::too_many_lines)]
    #[test]
    fn harvest_start_requires_matching_grant_and_consumes_it() {
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, ids) = setup_registries(&recipes);
        let (mut world, actor, workstation, _place) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        grant_recipe(&mut world, actor, recipe_id);
        provision_facility_queue(&mut world, workstation, 9);
        let affordance = single_harvest_affordance(&world, actor, &defs, &handlers);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut rng = test_rng(0x91);
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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
        )
        .unwrap_err();
        assert!(matches!(
            missing_grant_err,
            ActionError::PreconditionFailed(message)
                if message.contains("no matching grant")
        ));

        grant_facility_use(&mut world, workstation, actor, ActionDefId(999), 10);
        let wrong_grant_err = start_action(
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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
        )
        .unwrap_err();
        assert!(matches!(
            wrong_grant_err,
            ActionError::PreconditionFailed(message)
                if message.contains("grant belongs")
        ));

        grant_facility_use(&mut world, workstation, actor, ids[0], 12);
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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(12),
            },
        )
        .unwrap();

        assert!(world
            .get_component_facility_use_queue(workstation)
            .unwrap()
            .granted
            .is_none());

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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(13),
            },
            worldwake_sim::ExternalAbortReason::Other,
        )
        .unwrap();

        let consumed_grant_err = start_action(
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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(14),
            },
        )
        .unwrap_err();
        assert!(matches!(
            consumed_grant_err,
            ActionError::PreconditionFailed(message)
                if message.contains("no matching grant")
        ));
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn harvest_reservation_blocks_second_actor_and_abort_preserves_source() {
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, ids) = setup_registries(&recipes);
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
        grant_facility_use(&mut world, workstation, actor_a, ids[0], 9);

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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
        )
        .unwrap_err();
        assert_eq!(
            second_start,
            ActionError::PreconditionFailed(format!(
                "facility {workstation} has no matching grant for actor {} action {:?}",
                actor_b, ids[0]
            ))
        );

        grant_facility_use(&mut world, workstation, actor_b, ids[0], 10);
        let second_with_grant = start_action(
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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
        )
        .unwrap_err();
        assert_eq!(
            second_with_grant,
            ActionError::ReservationUnavailable(workstation)
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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
            worldwake_sim::ExternalAbortReason::Other,
        )
        .unwrap();

        assert_eq!(
            world
                .get_component_resource_source(workstation)
                .unwrap()
                .available_quantity,
            Quantity(5)
        );
    }

    #[test]
    fn harvest_body_cost_flows_through_needs_system() {
        let body_cost = BodyCostPerTick::new(pm(2), pm(3), pm(5), pm(7));
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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
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
                ActionExecutionContext {
                    cause: CauseRef::SystemTick(Tick(tick)),
                    tick: Tick(tick),
                },
            )
            .unwrap();

            needs_system(SystemExecutionContext {
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
                active_actions: &active,
                action_defs: &defs,
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
                Constraint::ActorHasCommodity {
                    kind: CommodityKind::Grain,
                    min_qty: Quantity(2),
                },
            ]
        );
        assert!(def
            .preconditions
            .contains(&Precondition::TargetLacksProductionJob(0)));
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
            domain: worldwake_sim::ActionDomain::Trade,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(nz(1)),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::Trade(TradeActionPayload {
                counterparty: EntityId {
                    slot: 10,
                    generation: 0,
                },
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(4),
                requested_commodity: CommodityKind::Grain,
                requested_quantity: Quantity(2),
            }),
            handler: ActionHandlerId(0),
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
            remaining_duration: worldwake_sim::ActionDuration::Finite(1),
            status: worldwake_sim::ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
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
            ActionExecutionContext {
                cause: CauseRef::SystemTick(Tick(11)),
                tick: Tick(11),
            },
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
            ActionExecutionContext {
                cause: CauseRef::SystemTick(Tick(12)),
                tick: Tick(12),
            },
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
        assert!(affordances_for(
            &world_missing_recipe,
            actor_missing_recipe,
            &defs,
            &handlers
        )
        .is_empty());

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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
        )
        .unwrap();

        assert!(world
            .get_component_facility_use_queue(workstation)
            .unwrap()
            .granted
            .is_none());

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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(12),
            },
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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
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
        let body_cost = BodyCostPerTick::new(pm(2), pm(3), pm(5), pm(7));
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
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
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
                ActionExecutionContext {
                    cause: CauseRef::SystemTick(Tick(tick)),
                    tick: Tick(tick),
                },
            )
            .unwrap();

            needs_system(SystemExecutionContext {
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
                active_actions: &active,
                action_defs: &defs,
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
}
