use std::collections::BTreeSet;
use worldwake_core::{
    EntityKind, EventTag, VisibilitySpec, WorkstationMarker, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefId, ActionDefRegistry, ActionError, ActionHandler,
    ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress,
    ActionState, Constraint, DurationExpr, HarvestActionPayload, Interruptibility, Precondition,
    RecipeDefinition, RecipeRegistry, ReservationReq, TargetSpec,
};

pub fn register_harvest_actions(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
    recipes: &RecipeRegistry,
) -> Vec<ActionDefId> {
    let handler = handlers.register(ActionHandler::new(
        start_harvest,
        tick_harvest,
        commit_harvest,
        abort_harvest,
    ));

    let mut ids = Vec::new();
    for (recipe_id, recipe) in recipes.iter() {
        let Some(def) = harvest_action_def(ActionDefId(defs.len() as u32), handler, recipe_id, recipe)
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
    actor_constraints.extend(recipe.required_tool_kinds.iter().copied().map(|kind| {
        Constraint::ActorHasUniqueItemKind { kind, min_count: 1 }
    }));
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

fn harvest_payload(def: &ActionDef) -> Result<&HarvestActionPayload, ActionError> {
    match &def.payload {
        ActionPayload::Harvest(payload) => Ok(payload),
        ActionPayload::None => Err(ActionError::InternalError(format!(
            "action def {} is missing harvest payload",
            def.id
        ))),
    }
}

#[allow(clippy::unnecessary_wraps)]
fn start_harvest(
    def: &ActionDef,
    _instance: &ActionInstance,
) -> Result<Option<ActionState>, ActionError> {
    let _ = harvest_payload(def)?;
    Ok(None)
}

#[allow(clippy::unnecessary_wraps)]
fn tick_harvest(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_harvest(
    def: &ActionDef,
    instance: &ActionInstance,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    let payload = harvest_payload(def)?;
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
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_harvest(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _reason: &AbortReason,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::register_harvest_actions;
    use crate::needs::needs_system;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, BodyCostPerTick, CauseRef, CommodityKind, ControlSource,
        DeprivationExposure, DriveThresholds, EntityId, EventId, EventLog, HomeostaticNeeds,
        MetabolismProfile, Permille, Quantity, ResourceSource, Seed, Tick, VisibilitySpec,
        WitnessData, WorkstationMarker, WorkstationTag, World, WorldTxn,
    };
    use worldwake_sim::{
        abort_action, get_affordances, start_action, tick_action, ActionDefRegistry,
        ActionExecutionAuthority, ActionExecutionContext, ActionHandlerRegistry, ActionInstance,
        ActionInstanceId, OmniscientBeliefView, RecipeRegistry, SystemExecutionContext, SystemId,
        TickOutcome,
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

    fn harvest_recipe_registry(body_cost_per_tick: BodyCostPerTick) -> (RecipeRegistry, worldwake_core::RecipeId) {
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

    fn single_harvest_affordance(
        world: &World,
        actor: EntityId,
        defs: &ActionDefRegistry,
    ) -> worldwake_sim::Affordance {
        let affordances = get_affordances(&OmniscientBeliefView::new(world), actor, defs);
        assert_eq!(affordances.len(), 1);
        affordances.into_iter().next().unwrap()
    }

    fn run_to_completion(
        world: &mut World,
        event_log: &mut EventLog,
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
                },
                ActionExecutionContext {
                    cause: CauseRef::SystemTick(Tick(tick)),
                    tick: Tick(tick),
                },
            )
            .unwrap()
            {
                TickOutcome::Continuing => {}
                TickOutcome::Committed => return,
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
    fn harvest_happy_path_reduces_source_and_creates_output_lot() {
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, _) = setup_registries(&recipes);
        let (mut world, actor, workstation, place) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        grant_recipe(&mut world, actor, recipe_id);
        let affordance = single_harvest_affordance(&world, actor, &defs);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
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
            &defs,
            &handlers,
            instance_id,
            &mut active,
            11,
        );

        assert_eq!(
            world.get_component_resource_source(workstation).unwrap().available_quantity,
            Quantity(3)
        );
        let apple_lots = world
            .query_item_lot()
            .filter(|(entity, lot)| {
                lot.commodity == CommodityKind::Apple && world.effective_place(*entity) == Some(place)
            })
            .collect::<Vec<_>>();
        assert_eq!(apple_lots.len(), 1);
        assert_eq!(apple_lots[0].1.quantity, Quantity(2));
        let record = event_log.get(EventId(event_log.len() as u64 - 1)).unwrap();
        assert!(record.tags.contains(&EventTag::ActionCommitted));
        assert!(record.tags.contains(&EventTag::WorldMutation));
    }

    #[test]
    fn harvest_affordance_requires_recipe_stock_and_matching_workstation() {
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, _handlers, _) = setup_registries(&recipes);

        let (mut world_missing_recipe, actor_missing_recipe, _, _) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        assert!(get_affordances(
            &OmniscientBeliefView::new(&world_missing_recipe),
            actor_missing_recipe,
            &defs
        )
        .is_empty());

        let (mut world_wrong_tag, actor_wrong_tag, _, _) =
            setup_world(false, WorkstationTag::Mill, 5);
        grant_recipe(&mut world_wrong_tag, actor_wrong_tag, recipe_id);
        assert!(get_affordances(
            &OmniscientBeliefView::new(&world_wrong_tag),
            actor_wrong_tag,
            &defs
        )
        .is_empty());

        let (mut world_empty, actor_empty, _, _) =
            setup_world(false, WorkstationTag::OrchardRow, 1);
        grant_recipe(&mut world_empty, actor_empty, recipe_id);
        assert!(get_affordances(
            &OmniscientBeliefView::new(&world_empty),
            actor_empty,
            &defs
        )
        .is_empty());

        let _ = &mut world_missing_recipe;
    }

    #[test]
    fn harvest_affordance_requires_possessed_unique_tool_kind() {
        let (recipes, recipe_id) = harvest_recipe_registry_with_tools(
            BodyCostPerTick::zero(),
            vec![worldwake_core::UniqueItemKind::SimpleTool],
        );
        let (defs, _handlers, _) = setup_registries(&recipes);
        let (mut world, actor, _workstation, place) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        grant_recipe(&mut world, actor, recipe_id);

        assert!(get_affordances(
            &OmniscientBeliefView::new(&world),
            actor,
            &defs
        )
        .is_empty());

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

        let affordances = get_affordances(&OmniscientBeliefView::new(&world), actor, &defs);
        assert_eq!(affordances.len(), 1);
    }

    #[test]
    fn harvest_reservation_blocks_second_actor_and_abort_preserves_source() {
        let (recipes, recipe_id) = harvest_recipe_registry(BodyCostPerTick::zero());
        let (defs, handlers, _) = setup_registries(&recipes);
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

        let affordance_a = single_harvest_affordance(&world, actor_a, &defs);
        let affordance_b = single_harvest_affordance(&world, actor_b, &defs);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let first_id = start_action(
            &affordance_a,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
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
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
        )
        .unwrap_err();
        assert_eq!(second_start, ActionError::ReservationUnavailable(workstation));

        abort_action(
            first_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
            },
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(11),
            },
            "cancel".to_string(),
        )
        .unwrap();

        assert_eq!(
            world.get_component_resource_source(workstation).unwrap().available_quantity,
            Quantity(5)
        );
    }

    #[test]
    fn harvest_body_cost_flows_through_needs_system() {
        let body_cost = BodyCostPerTick::new(pm(2), pm(3), pm(5), pm(7));
        let (recipes, recipe_id) = harvest_recipe_registry(body_cost);
        let (defs, handlers, _) = setup_registries(&recipes);
        let (mut world, actor, _workstation, _) =
            setup_world(false, WorkstationTag::OrchardRow, 5);
        grant_recipe(&mut world, actor, recipe_id);
        let affordance = single_harvest_affordance(&world, actor, &defs);
        let mut active = BTreeMap::new();
        let mut event_log = EventLog::new();
        let mut next_id = ActionInstanceId(0);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut event_log,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
        )
        .unwrap();

        let mut rng = worldwake_sim::DeterministicRng::new(Seed([7; 32]));
        for tick in [10_u64, 11_u64] {
            let _ = tick_action(
                instance_id,
                &defs,
                &handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world: &mut world,
                    event_log: &mut event_log,
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
