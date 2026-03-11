use crate::inventory::consume_one_unit;
use std::collections::BTreeSet;
use std::num::NonZeroU32;
use worldwake_core::{
    CommodityKind, EntityId, EventTag, HomeostaticNeeds, ItemLot, MetabolismProfile, Permille,
    Quantity, VisibilitySpec, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefId, ActionDefRegistry, ActionError, ActionHandler,
    ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress,
    ActionState, Constraint, ConsumableEffect, DeterministicRng, DurationExpr, Interruptibility,
    MetabolismDurationKind, Precondition, TargetSpec,
};

pub fn register_needs_actions(defs: &mut ActionDefRegistry, handlers: &mut ActionHandlerRegistry) {
    let eat_handler = handlers.register(ActionHandler::new(
        start_noop,
        tick_continue,
        commit_eat,
        abort_noop,
    ));
    let drink_handler = handlers.register(ActionHandler::new(
        start_noop,
        tick_continue,
        commit_drink,
        abort_noop,
    ));
    let sleep_handler = handlers.register(ActionHandler::new(
        start_noop,
        tick_sleep,
        commit_noop,
        abort_noop,
    ));
    let toilet_handler = handlers.register(ActionHandler::new(
        start_noop,
        tick_continue,
        commit_toilet,
        abort_noop,
    ));
    let wash_handler = handlers.register(ActionHandler::new(
        start_noop,
        tick_continue,
        commit_wash,
        abort_noop,
    ));

    register_def(
        defs,
        "eat",
        eat_handler,
        eat_preconditions(),
        DurationExpr::TargetConsumable { target_index: 0 },
    );
    register_def(
        defs,
        "drink",
        drink_handler,
        drink_preconditions(),
        DurationExpr::TargetConsumable { target_index: 0 },
    );
    register_def(
        defs,
        "sleep",
        sleep_handler,
        vec![Precondition::ActorAlive],
        DurationExpr::Fixed(NonZeroU32::MIN),
    );
    register_def(
        defs,
        "toilet",
        toilet_handler,
        vec![Precondition::ActorAlive],
        DurationExpr::ActorMetabolism {
            kind: MetabolismDurationKind::Toilet,
        },
    );
    register_def(
        defs,
        "wash",
        wash_handler,
        wash_preconditions(),
        DurationExpr::ActorMetabolism {
            kind: MetabolismDurationKind::Wash,
        },
    );
}

fn register_def(
    defs: &mut ActionDefRegistry,
    name: &str,
    handler: ActionHandlerId,
    preconditions: Vec<Precondition>,
    duration: DurationExpr,
) -> ActionDefId {
    let id = ActionDefId(defs.len() as u32);
    defs.register(ActionDef {
        id,
        name: name.to_string(),
        domain: worldwake_sim::ActionDomain::Needs,
        actor_constraints: vec![Constraint::ActorAlive],
        targets: match name {
            "eat" | "drink" | "wash" => vec![TargetSpec::EntityAtActorPlace {
                kind: worldwake_core::EntityKind::ItemLot,
            }],
            _ => Vec::new(),
        },
        preconditions: preconditions.clone(),
        reservation_requirements: Vec::new(),
        duration,
        body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
        interruptibility: Interruptibility::InterruptibleWithPenalty,
        commit_conditions: preconditions,
        visibility: VisibilitySpec::ParticipantsOnly,
        causal_event_tags: BTreeSet::from([EventTag::WorldMutation]),
        payload: ActionPayload::None,
        handler,
    })
}

fn eat_preconditions() -> Vec<Precondition> {
    vec![
        Precondition::ActorAlive,
        Precondition::TargetExists(0),
        Precondition::TargetAtActorPlace(0),
        Precondition::TargetKind {
            target_index: 0,
            kind: worldwake_core::EntityKind::ItemLot,
        },
        Precondition::ActorCanControlTarget(0),
        Precondition::TargetHasConsumableEffect {
            target_index: 0,
            effect: ConsumableEffect::Hunger,
        },
    ]
}

fn drink_preconditions() -> Vec<Precondition> {
    vec![
        Precondition::ActorAlive,
        Precondition::TargetExists(0),
        Precondition::TargetAtActorPlace(0),
        Precondition::TargetKind {
            target_index: 0,
            kind: worldwake_core::EntityKind::ItemLot,
        },
        Precondition::ActorCanControlTarget(0),
        Precondition::TargetHasConsumableEffect {
            target_index: 0,
            effect: ConsumableEffect::Thirst,
        },
    ]
}

fn wash_preconditions() -> Vec<Precondition> {
    vec![
        Precondition::ActorAlive,
        Precondition::TargetExists(0),
        Precondition::TargetAtActorPlace(0),
        Precondition::TargetKind {
            target_index: 0,
            kind: worldwake_core::EntityKind::ItemLot,
        },
        Precondition::ActorCanControlTarget(0),
        Precondition::TargetCommodity {
            target_index: 0,
            kind: CommodityKind::Water,
        },
    ]
}

fn lot_profile(
    txn: &WorldTxn<'_>,
    lot_id: EntityId,
) -> Result<worldwake_core::CommodityConsumableProfile, ActionError> {
    let lot = lot(txn, lot_id)?;
    lot.commodity
        .spec()
        .consumable_profile
        .ok_or_else(|| ActionError::PreconditionFailed(format!("lot {lot_id} is not consumable")))
}

fn lot(txn: &WorldTxn<'_>, lot_id: EntityId) -> Result<ItemLot, ActionError> {
    txn.get_component_item_lot(lot_id)
        .cloned()
        .ok_or(ActionError::InvalidTarget(lot_id))
}

fn actor_needs(txn: &WorldTxn<'_>, actor: EntityId) -> Result<HomeostaticNeeds, ActionError> {
    txn.get_component_homeostatic_needs(actor)
        .copied()
        .ok_or_else(|| ActionError::InternalError(format!("actor {actor} lacks needs component")))
}

fn actor_profile(txn: &WorldTxn<'_>, actor: EntityId) -> Result<MetabolismProfile, ActionError> {
    txn.get_component_metabolism_profile(actor)
        .copied()
        .ok_or_else(|| {
            ActionError::InternalError(format!("actor {actor} lacks metabolism profile"))
        })
}

fn set_actor_needs(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    needs: HomeostaticNeeds,
) -> Result<(), ActionError> {
    txn.set_component_homeostatic_needs(actor, needs)
        .map_err(|err| ActionError::InternalError(err.to_string()))
}

#[allow(clippy::unnecessary_wraps)]
fn start_noop(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    Ok(None)
}

#[allow(clippy::unnecessary_wraps)]
fn tick_continue(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

#[allow(clippy::unnecessary_wraps)]
fn commit_noop(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_noop(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _reason: &AbortReason,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

fn tick_sleep(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    let needs = actor_needs(txn, instance.actor)?;
    let profile = actor_profile(txn, instance.actor)?;
    let next = HomeostaticNeeds::new(
        needs.hunger,
        needs.thirst,
        needs.fatigue.saturating_sub(profile.rest_efficiency),
        needs.bladder,
        needs.dirtiness,
    );
    set_actor_needs(txn, instance.actor, next)?;
    Ok(ActionProgress::Continue)
}

fn commit_eat(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    apply_consumable_effects(instance, txn, true)
}

fn commit_drink(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    apply_consumable_effects(instance, txn, false)
}

fn apply_consumable_effects(
    instance: &ActionInstance,
    txn: &mut WorldTxn<'_>,
    require_hunger_effect: bool,
) -> Result<(), ActionError> {
    let target = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    let profile = lot_profile(txn, target)?;
    if require_hunger_effect && profile.hunger_relief_per_unit.value() == 0 {
        return Err(ActionError::PreconditionFailed(format!(
            "lot {target} has no hunger relief"
        )));
    }
    if !require_hunger_effect && profile.thirst_relief_per_unit.value() == 0 {
        return Err(ActionError::PreconditionFailed(format!(
            "lot {target} has no thirst relief"
        )));
    }

    let needs = actor_needs(txn, instance.actor)?;
    let next = HomeostaticNeeds::new(
        needs.hunger.saturating_sub(profile.hunger_relief_per_unit),
        needs.thirst.saturating_sub(profile.thirst_relief_per_unit),
        needs.fatigue,
        needs.bladder.saturating_add(profile.bladder_fill_per_unit),
        needs.dirtiness,
    );
    consume_one_unit(txn, target)?;
    set_actor_needs(txn, instance.actor, next)
}

fn commit_toilet(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    let needs = actor_needs(txn, instance.actor)?;
    let place = txn.effective_place(instance.actor).ok_or_else(|| {
        ActionError::InternalError(format!("actor {} has no place", instance.actor))
    })?;
    let waste = txn
        .create_item_lot(CommodityKind::Waste, Quantity(1))
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.set_ground_location(waste, place)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    set_actor_needs(
        txn,
        instance.actor,
        HomeostaticNeeds::new(
            needs.hunger,
            needs.thirst,
            needs.fatigue,
            pm(0),
            needs.dirtiness,
        ),
    )
}

fn commit_wash(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    let target = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    let needs = actor_needs(txn, instance.actor)?;
    consume_one_unit(txn, target)?;
    set_actor_needs(
        txn,
        instance.actor,
        HomeostaticNeeds::new(
            needs.hunger,
            needs.thirst,
            needs.fatigue,
            needs.bladder,
            pm(0),
        ),
    )
}

const fn pm(value: u16) -> Permille {
    Permille::new_unchecked(value)
}

#[cfg(test)]
mod tests {
    use super::register_needs_actions;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, CauseRef, CommodityKind, Container, ControlSource,
        DeprivationExposure, DriveThresholds, EntityId, EventLog, HomeostaticNeeds, LoadUnits,
        MetabolismProfile, Permille, Quantity, Seed, Tick, VisibilitySpec, WitnessData, World,
        WorldTxn,
    };
    use worldwake_sim::{
        abort_action, get_affordances, start_action, tick_action, ActionDefRegistry,
        ActionExecutionAuthority, ActionExecutionContext, ActionHandlerRegistry, ActionInstance,
        ActionInstanceId, DeterministicRng, OmniscientBeliefView, TickOutcome,
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

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    fn test_rng() -> DeterministicRng {
        DeterministicRng::new(Seed([0x41; 32]))
    }

    fn setup_actor(world: &mut World) -> (EntityId, EntityId) {
        let place = world.topology().place_ids().next().unwrap();
        let mut txn = new_txn(world, 1);
        let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, place).unwrap();
        txn.set_component_homeostatic_needs(
            actor,
            HomeostaticNeeds::new(pm(700), pm(650), pm(400), pm(200), pm(350)),
        )
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
                pm(40),
                NonZeroU32::new(10).unwrap(),
                NonZeroU32::new(10).unwrap(),
                NonZeroU32::new(10).unwrap(),
                NonZeroU32::new(10).unwrap(),
                NonZeroU32::new(2).unwrap(),
                NonZeroU32::new(3).unwrap(),
            ),
        )
        .unwrap();
        commit_txn(txn);
        (actor, place)
    }

    fn setup_registries() -> (ActionDefRegistry, ActionHandlerRegistry) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_needs_actions(&mut defs, &mut handlers);
        (defs, handlers)
    }

    fn run_action_to_completion(
        actor: EntityId,
        affordance_index: usize,
        world: &mut World,
        log: &mut EventLog,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
    ) -> ActionInstanceId {
        let mut active = BTreeMap::<ActionInstanceId, ActionInstance>::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = test_rng();
        let affordances = get_affordances(&OmniscientBeliefView::new(world), actor, defs, handlers);
        let affordance = affordances[affordance_index].clone();
        let instance_id = start_action(
            &affordance,
            defs,
            handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world,
                event_log: log,
                rng: &mut rng,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
        )
        .unwrap();

        for tick in 11..40 {
            match tick_action(
                instance_id,
                defs,
                handlers,
                ActionExecutionAuthority {
                    active_actions: &mut active,
                    world,
                    event_log: log,
                    rng: &mut rng,
                },
                ActionExecutionContext {
                    cause: CauseRef::Bootstrap,
                    tick: Tick(tick),
                },
            )
            .unwrap()
            {
                TickOutcome::Continuing => {}
                TickOutcome::Committed => break,
                TickOutcome::Aborted { reason, .. } => panic!("unexpected abort: {reason:?}"),
            }
        }

        instance_id
    }

    #[test]
    fn register_needs_actions_adds_all_five_defs_and_handlers() {
        let (defs, handlers) = setup_registries();
        assert_eq!(defs.len(), 5);
        assert_eq!(handlers.len(), 5);
        assert_eq!(defs.get(worldwake_sim::ActionDefId(0)).unwrap().name, "eat");
        assert_eq!(
            defs.get(worldwake_sim::ActionDefId(4)).unwrap().name,
            "wash"
        );
    }

    #[test]
    fn eat_consumes_one_unit_and_applies_consumable_effects() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        let bread = {
            let mut txn = new_txn(&mut world, 2);
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(2))
                .unwrap();
            let satchel = txn
                .create_container(Container {
                    capacity: LoadUnits(20),
                    allowed_commodities: None,
                    allows_unique_items: true,
                    allows_nested_containers: true,
                })
                .unwrap();
            txn.set_ground_location(satchel, place).unwrap();
            txn.set_possessor(satchel, actor).unwrap();
            txn.put_into_container(bread, satchel).unwrap();
            commit_txn(txn);
            bread
        };
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        run_action_to_completion(actor, 0, &mut world, &mut log, &defs, &handlers);

        let needs = world.get_component_homeostatic_needs(actor).unwrap();
        let lot = world.get_component_item_lot(bread).unwrap();
        let profile = CommodityKind::Bread.spec().consumable_profile.unwrap();
        assert_eq!(lot.quantity, Quantity(1));
        assert_eq!(
            needs.hunger,
            pm(700).saturating_sub(profile.hunger_relief_per_unit)
        );
        assert_eq!(
            needs.thirst,
            pm(650).saturating_sub(profile.thirst_relief_per_unit)
        );
        assert_eq!(
            needs.bladder,
            pm(200).saturating_add(profile.bladder_fill_per_unit)
        );
    }

    #[test]
    fn drink_consumes_one_unit_and_applies_consumable_effects() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        let water = {
            let mut txn = new_txn(&mut world, 2);
            let water = txn
                .create_item_lot(CommodityKind::Water, Quantity(2))
                .unwrap();
            txn.set_ground_location(water, place).unwrap();
            txn.set_possessor(water, actor).unwrap();
            commit_txn(txn);
            water
        };
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let affordances = get_affordances(&OmniscientBeliefView::new(&world), actor, &defs, &handlers);
        let drink_index = affordances
            .iter()
            .position(|affordance| affordance.def_id == worldwake_sim::ActionDefId(1))
            .unwrap();
        run_action_to_completion(actor, drink_index, &mut world, &mut log, &defs, &handlers);

        let needs = world.get_component_homeostatic_needs(actor).unwrap();
        let lot = world.get_component_item_lot(water).unwrap();
        let profile = CommodityKind::Water.spec().consumable_profile.unwrap();
        assert_eq!(lot.quantity, Quantity(1));
        assert_eq!(
            needs.thirst,
            pm(650).saturating_sub(profile.thirst_relief_per_unit)
        );
        assert_eq!(
            needs.bladder,
            pm(200).saturating_add(profile.bladder_fill_per_unit)
        );
    }

    #[test]
    fn aborted_eat_does_not_consume_item() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        let bread = {
            let mut txn = new_txn(&mut world, 2);
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_possessor(bread, actor).unwrap();
            commit_txn(txn);
            bread
        };
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();
        let mut active = BTreeMap::<ActionInstanceId, ActionInstance>::new();
        let mut next_id = ActionInstanceId(0);
        let mut rng = test_rng();
        let affordance =
            get_affordances(&OmniscientBeliefView::new(&world), actor, &defs, &handlers)[0].clone();
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_id,
            ActionExecutionContext {
                cause: CauseRef::Bootstrap,
                tick: Tick(10),
            },
        )
        .unwrap();

        abort_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active,
                world: &mut world,
                event_log: &mut log,
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
            world.get_component_item_lot(bread).unwrap().quantity,
            Quantity(1)
        );
    }

    #[test]
    fn sleep_reduces_fatigue_without_a_bed() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, _) = setup_actor(&mut world);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let affordances = get_affordances(&OmniscientBeliefView::new(&world), actor, &defs, &handlers);
        let sleep_index = affordances
            .iter()
            .position(|affordance| affordance.def_id == worldwake_sim::ActionDefId(2))
            .unwrap();
        run_action_to_completion(actor, sleep_index, &mut world, &mut log, &defs, &handlers);

        assert_eq!(
            world
                .get_component_homeostatic_needs(actor)
                .unwrap()
                .fatigue,
            pm(400).saturating_sub(pm(40))
        );
    }

    #[test]
    fn toilet_reduces_bladder_and_creates_waste() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let affordances = get_affordances(&OmniscientBeliefView::new(&world), actor, &defs, &handlers);
        let toilet_index = affordances
            .iter()
            .position(|affordance| affordance.def_id == worldwake_sim::ActionDefId(3))
            .unwrap();
        run_action_to_completion(actor, toilet_index, &mut world, &mut log, &defs, &handlers);

        assert_eq!(
            world
                .get_component_homeostatic_needs(actor)
                .unwrap()
                .bladder,
            pm(0)
        );
        let waste_count = world
            .ground_entities_at(place)
            .into_iter()
            .filter(|entity| {
                world
                    .get_component_item_lot(*entity)
                    .is_some_and(|lot| lot.commodity == CommodityKind::Waste)
            })
            .count();
        assert_eq!(waste_count, 1);
    }

    #[test]
    fn wash_consumes_water_and_clears_dirtiness() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        let water = {
            let mut txn = new_txn(&mut world, 2);
            let water = txn
                .create_item_lot(CommodityKind::Water, Quantity(2))
                .unwrap();
            txn.set_ground_location(water, place).unwrap();
            txn.set_possessor(water, actor).unwrap();
            commit_txn(txn);
            water
        };
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let affordances = get_affordances(&OmniscientBeliefView::new(&world), actor, &defs, &handlers);
        let wash_index = affordances
            .iter()
            .position(|affordance| affordance.def_id == worldwake_sim::ActionDefId(4))
            .unwrap();
        run_action_to_completion(actor, wash_index, &mut world, &mut log, &defs, &handlers);

        assert_eq!(
            world.get_component_item_lot(water).unwrap().quantity,
            Quantity(1)
        );
        assert_eq!(
            world
                .get_component_homeostatic_needs(actor)
                .unwrap()
                .dirtiness,
            pm(0)
        );
    }

    #[test]
    fn uncontrolled_ground_item_does_not_produce_eat_affordance() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        {
            let mut txn = new_txn(&mut world, 2);
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(bread, place).unwrap();
            commit_txn(txn);
        }
        let (defs, handlers) = setup_registries();

        let affordances = get_affordances(&OmniscientBeliefView::new(&world), actor, &defs, &handlers);
        assert!(affordances
            .iter()
            .all(|affordance| affordance.def_id != worldwake_sim::ActionDefId(0)));
    }
}
