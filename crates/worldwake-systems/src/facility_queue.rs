use crate::facility_queue_actions::exclusive_facility_workstation_tag;
use std::collections::BTreeMap;
use worldwake_core::{
    CauseRef, EntityId, EntityKind, EventLog, EventTag, FacilityUseQueue, Tick, VisibilitySpec,
    WitnessData, World, WorldTxn,
};
use worldwake_sim::{
    validate_action_def_authoritatively, ActionDefRegistry, ActionInstance, ActionStatus,
    SystemError, SystemExecutionContext,
};

pub fn facility_queue_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let SystemExecutionContext {
        world,
        event_log,
        rng: _rng,
        active_actions,
        action_defs,
        tick,
        system_id: _system_id,
    } = ctx;

    let facilities = world.all_entities().collect::<Vec<_>>();
    for facility in facilities {
        if world.get_component_facility_use_queue(facility).is_none() {
            continue;
        }

        prune_invalid_waiters(world, event_log, facility, tick)?;
        expire_stale_grant(world, event_log, facility, tick)?;
        prune_structurally_invalid_heads(world, event_log, action_defs, facility, tick)?;
        promote_ready_head(world, event_log, active_actions, action_defs, facility, tick)?;
    }

    Ok(())
}

fn prune_invalid_waiters(
    world: &mut World,
    event_log: &mut EventLog,
    facility: EntityId,
    tick: Tick,
) -> Result<(), SystemError> {
    let facility_place = world.effective_place(facility);
    let Some(mut queue) = world.get_component_facility_use_queue(facility).cloned() else {
        return Ok(());
    };
    let initial_len = queue.waiting.len();
    queue.waiting.retain(|_, queued| {
        world.entity_kind(queued.actor).is_some()
            && world.get_component_dead_at(queued.actor).is_none()
            && world.effective_place(queued.actor) == facility_place
    });

    if queue.waiting.len() != initial_len {
        commit_queue_update(world, event_log, facility, queue, tick, None, None)?;
    }

    Ok(())
}

fn expire_stale_grant(
    world: &mut World,
    event_log: &mut EventLog,
    facility: EntityId,
    tick: Tick,
) -> Result<(), SystemError> {
    let Some(mut queue) = world.get_component_facility_use_queue(facility).cloned() else {
        return Ok(());
    };
    let Some(granted) = queue.granted.clone() else {
        return Ok(());
    };
    if tick < granted.expires_at {
        return Ok(());
    }

    queue.clear_grant();
    commit_queue_update(
        world,
        event_log,
        facility,
        queue,
        tick,
        Some(EventTag::QueueGrantExpired),
        Some(granted.actor),
    )
}

fn prune_structurally_invalid_heads(
    world: &mut World,
    event_log: &mut EventLog,
    action_defs: &ActionDefRegistry,
    facility: EntityId,
    tick: Tick,
) -> Result<(), SystemError> {
    loop {
        let Some(queue) = world.get_component_facility_use_queue(facility).cloned() else {
            return Ok(());
        };
        let Some((&ordinal, queued)) = queue.waiting.iter().next() else {
            return Ok(());
        };
        let queued_actor = queued.actor;
        let intended_action = queued.intended_action;

        if !head_is_structurally_invalid(world, action_defs, facility, queued_actor, intended_action) {
            return Ok(());
        }

        let mut next_queue = queue;
        next_queue.waiting.remove(&ordinal);
        commit_queue_update(
            world,
            event_log,
            facility,
            next_queue,
            tick,
            Some(EventTag::QueueHeadFailed),
            Some(queued_actor),
        )?;
    }
}

fn promote_ready_head(
    world: &mut World,
    event_log: &mut EventLog,
    active_actions: &BTreeMap<worldwake_sim::ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
    facility: EntityId,
    tick: Tick,
) -> Result<(), SystemError> {
    let Some(policy) = world.get_component_exclusive_facility_policy(facility).cloned() else {
        return Ok(());
    };
    let Some(mut queue) = world.get_component_facility_use_queue(facility).cloned() else {
        return Ok(());
    };
    if queue.granted.is_some() || active_exclusive_action_on_facility(active_actions, action_defs, facility) {
        return Ok(());
    }
    let Some(queued) = queue.waiting.values().next() else {
        return Ok(());
    };
    if !head_is_ready_to_start(world, active_actions, action_defs, facility, queued.actor, queued.intended_action)
    {
        return Ok(());
    }

    let granted = queue
        .promote_head(tick, policy.grant_hold_ticks)
        .cloned()
        .expect("queue head exists when promotion is attempted");
    commit_queue_update(
        world,
        event_log,
        facility,
        queue,
        tick,
        Some(EventTag::QueueGrantPromoted),
        Some(granted.actor),
    )
}

fn head_is_structurally_invalid(
    world: &World,
    action_defs: &ActionDefRegistry,
    facility: EntityId,
    _actor: EntityId,
    intended_action: worldwake_core::ActionDefId,
) -> bool {
    if world.entity_kind(facility) != Some(EntityKind::Facility) || !world.is_alive(facility) {
        return true;
    }

    let Some(def) = action_defs.get(intended_action) else {
        return true;
    };
    let Some(required_tag) = exclusive_facility_workstation_tag(def) else {
        return true;
    };

    world
        .get_component_workstation_marker(facility)
        .is_none_or(|marker| marker.0 != required_tag)
}

fn head_is_ready_to_start(
    world: &World,
    active_actions: &BTreeMap<worldwake_sim::ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
    facility: EntityId,
    actor: EntityId,
    intended_action: worldwake_core::ActionDefId,
) -> bool {
    let Some(def) = action_defs.get(intended_action) else {
        return false;
    };
    let targets = [facility];

    validate_action_def_authoritatively(world, def, actor, &targets).is_ok()
        && !active_exclusive_action_on_facility(active_actions, action_defs, facility)
}

fn active_exclusive_action_on_facility(
    active_actions: &BTreeMap<worldwake_sim::ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
    facility: EntityId,
) -> bool {
    active_actions.values().any(|instance| {
        instance.status == ActionStatus::Active
            && instance.targets.first().copied() == Some(facility)
            && action_defs
                .get(instance.def_id)
                .and_then(exclusive_facility_workstation_tag)
                .is_some()
    })
}

fn commit_queue_update(
    world: &mut World,
    event_log: &mut EventLog,
    facility: EntityId,
    queue: FacilityUseQueue,
    tick: Tick,
    extra_tag: Option<EventTag>,
    extra_target: Option<EntityId>,
) -> Result<(), SystemError> {
    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        None,
        world.effective_place(facility),
        VisibilitySpec::SamePlace,
        WitnessData::default(),
    );
    txn.add_tag(EventTag::System)
        .add_tag(EventTag::WorldMutation)
        .add_target(facility);
    if let Some(tag) = extra_tag {
        txn.add_tag(tag);
    }
    if let Some(target) = extra_target {
        txn.add_target(target);
    }
    txn.set_component_facility_use_queue(facility, queue)
        .map_err(|error| SystemError::new(error.to_string()))?;
    let _ = txn.commit(event_log);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::facility_queue_system;
    use crate::{register_craft_actions, register_harvest_actions};
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, ActionDefId, CauseRef, CommodityKind, ControlSource, EntityId,
        EntityKind, EventLog, EventTag, ExclusiveFacilityPolicy, FacilityUseQueue, KnownRecipes,
        ProductionJob, Quantity, ResourceSource, Seed, Tick, VisibilitySpec, WitnessData,
        WorkstationMarker, WorkstationTag, World, WorldTxn,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionDuration, ActionHandlerRegistry, ActionInstance, ActionInstanceId,
        ActionPayload, ActionState, ActionStatus, DeterministicRng, RecipeDefinition,
        RecipeRegistry, SystemExecutionContext, SystemId,
    };

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

    fn add_possessed_lot(
        world: &mut World,
        actor: EntityId,
        place: EntityId,
        commodity: CommodityKind,
        quantity: u32,
    ) {
        let mut txn = new_txn(world, 2);
        let lot = txn.create_item_lot(commodity, Quantity(quantity)).unwrap();
        txn.set_ground_location(lot, place).unwrap();
        txn.set_possessor(lot, actor).unwrap();
        commit_txn(txn);
    }

    fn build_recipe_registry() -> RecipeRegistry {
        let mut recipes = RecipeRegistry::new();
        let _ = recipes.register(RecipeDefinition {
            name: "Harvest Apples".to_string(),
            inputs: Vec::new(),
            outputs: vec![(CommodityKind::Apple, Quantity(2))],
            work_ticks: nz(2),
            required_workstation_tag: Some(WorkstationTag::OrchardRow),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
        });
        let _ = recipes.register(RecipeDefinition {
            name: "Craft Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(1))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: nz(3),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
        });
        recipes
    }

    fn setup_registries(
        recipes: &RecipeRegistry,
    ) -> (ActionDefRegistry, ActionHandlerRegistry, ActionDefId, ActionDefId) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let harvest_id = register_harvest_actions(&mut defs, &mut handlers, recipes)[0];
        let craft_id = register_craft_actions(&mut defs, &mut handlers, recipes)[0];
        (defs, handlers, harvest_id, craft_id)
    }

    fn setup_world(tag: WorkstationTag, stock: u32) -> (World, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let facility = {
            let mut txn = new_txn(&mut world, 1);
            let facility = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(facility, place).unwrap();
            txn.set_component_workstation_marker(facility, WorkstationMarker(tag))
                .unwrap();
            txn.set_component_exclusive_facility_policy(
                facility,
                ExclusiveFacilityPolicy {
                    grant_hold_ticks: nz(3),
                },
            )
            .unwrap();
            txn.set_component_facility_use_queue(facility, FacilityUseQueue::default())
                .unwrap();
            if tag == WorkstationTag::OrchardRow {
                txn.set_component_resource_source(
                    facility,
                    ResourceSource {
                        commodity: CommodityKind::Apple,
                        available_quantity: Quantity(stock),
                        max_quantity: Quantity(8),
                        regeneration_ticks_per_unit: None,
                        last_regeneration_tick: None,
                    },
                )
                .unwrap();
            }
            commit_txn(txn);
            facility
        };
        (world, place, facility)
    }

    fn add_actor(world: &mut World, place: EntityId, recipe_id: worldwake_core::RecipeId) -> EntityId {
        let mut txn = new_txn(world, 1);
        let actor = txn.create_agent("Queue Actor", ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, place).unwrap();
        txn.set_component_known_recipes(actor, KnownRecipes::with([recipe_id]))
            .unwrap();
        commit_txn(txn);
        actor
    }

    fn enqueue(world: &mut World, facility: EntityId, actor: EntityId, intended_action: ActionDefId) {
        let mut txn = new_txn(world, 2);
        let mut queue = txn.get_component_facility_use_queue(facility).cloned().unwrap();
        queue.enqueue(actor, intended_action, Tick(2)).unwrap();
        txn.set_component_facility_use_queue(facility, queue).unwrap();
        commit_txn(txn);
    }

    fn run_system(
        world: &mut World,
        log: &mut EventLog,
        defs: &ActionDefRegistry,
        active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,
        tick: u64,
    ) {
        let mut rng = DeterministicRng::new(Seed([0xAB; 32]));
        facility_queue_system(SystemExecutionContext {
            world,
            event_log: log,
            rng: &mut rng,
            active_actions,
            action_defs: defs,
            tick: Tick(tick),
            system_id: SystemId::FacilityQueue,
        })
        .unwrap();
    }

    fn active_action(
        def_id: ActionDefId,
        payload: ActionPayload,
        actor: EntityId,
        facility: EntityId,
    ) -> ActionInstance {
        ActionInstance {
            instance_id: ActionInstanceId(1),
            def_id,
            payload,
            actor,
            targets: vec![facility],
            start_tick: Tick(5),
            remaining_duration: ActionDuration::Finite(2),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: Some(ActionState::Empty),
        }
    }

    #[test]
    fn dead_actor_is_pruned_from_queue() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, harvest_id);
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_dead_at(actor, worldwake_core::DeadAt(Tick(3)))
                .unwrap();
            commit_txn(txn);
        }

        run_system(&mut world, &mut EventLog::new(), &defs, &BTreeMap::new(), 3);

        assert!(
            world
                .get_component_facility_use_queue(facility)
                .unwrap()
                .waiting
                .is_empty()
        );
    }

    #[test]
    fn departed_actor_is_pruned_from_queue() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        let other_place = world
            .topology()
            .place_ids()
            .find(|candidate| *candidate != place)
            .unwrap();
        enqueue(&mut world, facility, actor, harvest_id);
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_ground_location(actor, other_place).unwrap();
            commit_txn(txn);
        }

        run_system(&mut world, &mut EventLog::new(), &defs, &BTreeMap::new(), 3);

        assert!(
            world
                .get_component_facility_use_queue(facility)
                .unwrap()
                .waiting
                .is_empty()
        );
    }

    #[test]
    fn deallocated_actor_is_pruned_from_queue() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let _ = place;
        let actor = EntityId {
            slot: 999,
            generation: 1,
        };
        enqueue(&mut world, facility, actor, harvest_id);

        run_system(&mut world, &mut EventLog::new(), &defs, &BTreeMap::new(), 3);

        assert!(
            world
                .get_component_facility_use_queue(facility)
                .unwrap()
                .waiting
                .is_empty()
        );
    }

    #[test]
    fn expired_grant_is_cleared_and_emits_event() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, harvest_id);
        {
            let mut txn = new_txn(&mut world, 3);
            let mut queue = txn.get_component_facility_use_queue(facility).cloned().unwrap();
            queue.promote_head(Tick(3), nz(2));
            txn.set_component_facility_use_queue(facility, queue).unwrap();
            commit_txn(txn);
        }
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 5);

        let queue = world.get_component_facility_use_queue(facility).unwrap();
        assert!(queue.granted.is_none());
        assert_eq!(log.events_by_tag(EventTag::QueueGrantExpired).len(), 1);
        let record = log
            .get(log.events_by_tag(EventTag::QueueGrantExpired)[0])
            .unwrap();
        assert_eq!(record.visibility, VisibilitySpec::SamePlace);
    }

    #[test]
    fn structurally_invalid_head_is_pruned_and_emits_event() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, harvest_id);
        {
            let mut txn = new_txn(&mut world, 3);
            txn.clear_component_workstation_marker(facility).unwrap();
            commit_txn(txn);
        }
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 3);

        assert!(
            world
                .get_component_facility_use_queue(facility)
                .unwrap()
                .waiting
                .is_empty()
        );
        assert_eq!(log.events_by_tag(EventTag::QueueHeadFailed).len(), 1);
    }

    #[test]
    fn missing_intended_action_head_is_pruned_and_emits_event() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, _harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, ActionDefId(999));
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 3);

        assert!(
            world
                .get_component_facility_use_queue(facility)
                .unwrap()
                .waiting
                .is_empty()
        );
        assert_eq!(log.events_by_tag(EventTag::QueueHeadFailed).len(), 1);
    }

    #[test]
    fn depleted_stock_stalls_queue_without_pruning_or_promotion() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 0);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, harvest_id);
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 3);

        let queue = world.get_component_facility_use_queue(facility).unwrap();
        assert_eq!(queue.position_of(actor), Some(0));
        assert!(queue.granted.is_none());
        assert!(log.events_by_tag(EventTag::QueueHeadFailed).is_empty());
        assert!(log.events_by_tag(EventTag::QueueGrantPromoted).is_empty());
    }

    #[test]
    fn occupied_craft_workstation_stalls_queue_without_pruning_or_promotion() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, _harvest_id, craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::Mill, 0);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(1));
        add_possessed_lot(&mut world, actor, place, CommodityKind::Grain, 1);
        enqueue(&mut world, facility, actor, craft_id);
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_production_job(
                facility,
                ProductionJob {
                    recipe_id: worldwake_core::RecipeId(1),
                    worker: actor,
                    staged_inputs_container: facility,
                    progress_ticks: 1,
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        run_system(&mut world, &mut EventLog::new(), &defs, &BTreeMap::new(), 3);

        let queue = world.get_component_facility_use_queue(facility).unwrap();
        assert_eq!(queue.position_of(actor), Some(0));
        assert!(queue.granted.is_none());
    }

    #[test]
    fn ready_head_is_promoted_with_expected_expiry_and_event() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, harvest_id);
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 3);

        let queue = world.get_component_facility_use_queue(facility).unwrap();
        let granted = queue.granted.as_ref().unwrap();
        assert_eq!(granted.actor, actor);
        assert_eq!(granted.expires_at, Tick(6));
        assert_eq!(log.events_by_tag(EventTag::QueueGrantPromoted).len(), 1);
    }

    #[test]
    fn active_exclusive_action_blocks_promotion() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, harvest_id);
        let harvest_payload = defs.get(harvest_id).unwrap().payload.clone();
        let active = BTreeMap::from([(
            ActionInstanceId(9),
            active_action(harvest_id, harvest_payload, actor, facility),
        )]);

        run_system(&mut world, &mut EventLog::new(), &defs, &active, 3);

        assert!(
            world
                .get_component_facility_use_queue(facility)
                .unwrap()
                .granted
                .is_none()
        );
    }

    #[test]
    fn system_is_idempotent_within_same_tick() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, place, facility) = setup_world(WorkstationTag::OrchardRow, 4);
        let actor = add_actor(&mut world, place, worldwake_core::RecipeId(0));
        enqueue(&mut world, facility, actor, harvest_id);
        let mut log = EventLog::new();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 3);
        let first_queue = world.get_component_facility_use_queue(facility).unwrap().clone();
        let first_event_count = log.len();

        run_system(&mut world, &mut log, &defs, &BTreeMap::new(), 3);

        assert_eq!(
            world.get_component_facility_use_queue(facility).unwrap(),
            &first_queue
        );
        assert_eq!(log.len(), first_event_count);
    }

}
