use crate::inventory::consume_one_unit;
use std::collections::BTreeSet;
use std::num::NonZeroU32;
use worldwake_core::{
    ActionDefId, CommodityKind, EntityId, EventTag, HomeostaticNeeds, ItemLot, MetabolismProfile,
    OUTDOOR_RELIEF_TAGS, Permille, PlaceTag, Quantity, VisibilitySpec, WorkstationTag, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerId,
    ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress, ActionState,
    CommitOutcome, Constraint, ConsumableEffect, DeterministicRng, DurationExpr, Interruptibility,
    MetabolismDurationKind, Precondition, TargetSpec,
};

use crate::evidence_support::emit_evidence;

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
    let relieve_wilderness_handler = handlers.register(ActionHandler::new(
        start_noop,
        tick_continue,
        commit_relieve_wilderness,
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

    // relieve_wilderness: registered directly because it needs SamePlace visibility
    // and WildernessRelief event tag, unlike the other needs actions.
    let rw_id = ActionDefId(defs.len() as u32);
    defs.register(ActionDef {
        id: rw_id,
        name: "relieve_wilderness".to_string(),
        domain: worldwake_core::ActionDomain::Needs,
        actor_constraints: vec![
            Constraint::ActorAlive,
            Constraint::ActorNotIncapacitated,
            Constraint::ActorNotInTransit,
            Constraint::ActorAtPlaceWithAnyTag(OUTDOOR_RELIEF_TAGS),
        ],
        targets: Vec::new(),
        preconditions: vec![Precondition::ActorAlive],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::ActorMetabolism {
            kind: MetabolismDurationKind::Toilet,
        },
        body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
        attention_cost: Permille::ZERO,
        interruptibility: Interruptibility::InterruptibleWithPenalty,
        commit_conditions: vec![Precondition::ActorAlive],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::WorldMutation, EventTag::WildernessRelief]),
        payload: ActionPayload::None,
        handler: relieve_wilderness_handler,
        binding_strictness: worldwake_sim::BindingStrictness::AnyLegalTarget,
        guard_template: None,
        expectation_template: vec![],
    });
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
        domain: worldwake_core::ActionDomain::Needs,
        actor_constraints: match name {
            "toilet" => vec![
                Constraint::ActorAlive,
                Constraint::ActorAtPlaceTag(PlaceTag::Latrine),
            ],
            _ => vec![Constraint::ActorAlive],
        },
        targets: match name {
            "eat" | "drink" => vec![TargetSpec::EntityDirectlyPossessedByActor {
                kind: worldwake_core::EntityKind::ItemLot,
            }],
            "wash" => vec![
                TargetSpec::EntityAtActorPlace {
                    kind: worldwake_core::EntityKind::Facility,
                },
                TargetSpec::EntityAtActorPlace {
                    kind: worldwake_core::EntityKind::Facility,
                },
            ],
            _ => Vec::new(),
        },
        preconditions: preconditions.clone(),
        reservation_requirements: Vec::new(),
        duration,
        body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
        attention_cost: Permille::ZERO,
        interruptibility: Interruptibility::InterruptibleWithPenalty,
        commit_conditions: preconditions,
        visibility: VisibilitySpec::ParticipantsOnly,
        causal_event_tags: BTreeSet::from([EventTag::WorldMutation]),
        payload: ActionPayload::None,
        handler,
        binding_strictness: match name {
            "eat" | "drink" => worldwake_sim::BindingStrictness::FungibleEquivalentCommodity,
            "sleep" => worldwake_sim::BindingStrictness::AnyLegalTarget,
            "toilet" | "wash" => {
                worldwake_sim::BindingStrictness::EquivalentWorkstationTagAtSamePlace
            }
            other => panic!("unexpected needs action {other}"),
        },
        guard_template: None,
        expectation_template: vec![],
    })
}

fn eat_preconditions() -> Vec<Precondition> {
    vec![
        Precondition::ActorAlive,
        Precondition::TargetExists(0),
        Precondition::TargetKind {
            target_index: 0,
            kind: worldwake_core::EntityKind::ItemLot,
        },
        Precondition::TargetDirectlyPossessedByActor(0),
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
        Precondition::TargetKind {
            target_index: 0,
            kind: worldwake_core::EntityKind::ItemLot,
        },
        Precondition::TargetDirectlyPossessedByActor(0),
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
            kind: worldwake_core::EntityKind::Facility,
        },
        Precondition::TargetHasWorkstationTag {
            target_index: 0,
            tag: WorkstationTag::WashBasin,
        },
        Precondition::TargetExists(1),
        Precondition::TargetAtActorPlace(1),
        Precondition::TargetKind {
            target_index: 1,
            kind: worldwake_core::EntityKind::Facility,
        },
        Precondition::TargetHasResourceSource {
            target_index: 1,
            commodity: CommodityKind::Water,
            min_available: Quantity(1),
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
    _instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    Ok(None)
}

#[allow(clippy::unnecessary_wraps)]
fn tick_continue(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

#[allow(clippy::unnecessary_wraps)]
fn commit_noop(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_noop(
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

fn tick_sleep(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
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
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    apply_consumable_effects(instance, txn, true)?;
    Ok(CommitOutcome::empty())
}

fn commit_drink(
    _def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    apply_consumable_effects(instance, txn, false)?;
    Ok(CommitOutcome::empty())
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
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
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
    )?;
    Ok(CommitOutcome::empty())
}

fn commit_relieve_wilderness(
    _def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let needs = actor_needs(txn, instance.actor)?;
    let profile = actor_profile(txn, instance.actor)?;
    let place = txn.effective_place(instance.actor).ok_or_else(|| {
        ActionError::InternalError(format!("actor {} has no place", instance.actor))
    })?;
    let waste = txn
        .create_item_lot(CommodityKind::Waste, Quantity(1))
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.set_ground_location(waste, place)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    let current_tick = txn.tick();
    emit_evidence(
        txn,
        place,
        worldwake_core::EvidenceKind::DisturbanceMarker {
            place,
            kind: worldwake_core::DisturbanceKind::WildernessRelief,
            created_at: current_tick,
        },
        50,
    )
    .map_err(|err| ActionError::InternalError(err.to_string()))?;
    set_actor_needs(
        txn,
        instance.actor,
        HomeostaticNeeds::new(
            needs.hunger,
            needs.thirst,
            needs.fatigue,
            pm(0),
            needs
                .dirtiness
                .saturating_add(profile.wilderness_relief_dirtiness_penalty),
        ),
    )?;
    Ok(CommitOutcome::empty())
}

fn commit_wash(
    _def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let source = *instance
        .targets
        .get(1)
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    let needs = actor_needs(txn, instance.actor)?;
    let mut resource = txn
        .get_component_resource_source(source)
        .cloned()
        .ok_or(ActionError::InvalidTarget(source))?;
    resource.available_quantity = resource
        .available_quantity
        .checked_sub(Quantity(1))
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "resource source {source} lacks {:?} units for wash",
                Quantity(1)
            ))
        })?;
    txn.set_component_resource_source(source, resource)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
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
    )?;
    Ok(CommitOutcome::empty())
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
        ActionDefId, AgentBeliefStore, CauseRef, CommodityKind, ControlSource, DeprivationExposure,
        DisturbanceKind, DriveThresholds, EntityId, EntityKind, EventLog, EventTag, EvidenceKind,
        HomeostaticNeeds, MetabolismProfile, PerceptionSource, Permille, PrototypePlace, Quantity,
        ResourceSource, Seed, Tick, VisibilitySpec, WitnessData, WorkstationMarker, WorkstationTag,
        World, WorldTxn, build_believed_entity_state, build_prototype_world,
        prototype_place_entity,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionExecutionAuthority, ActionHandlerRegistry, ActionInstance,
        ActionInstanceId, DeterministicRng, PerAgentBeliefView, TickOutcome, abort_action,
        get_affordances, start_action, tick_action,
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
                NonZeroU32::new(8).unwrap(),
                pm(0),
                pm(0),
                pm(0),
                pm(0),
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

    fn setup_wash_access(
        world: &mut World,
        place: EntityId,
        available_water: u32,
    ) -> (EntityId, EntityId) {
        let mut txn = new_txn(world, 2);
        let basin = txn.create_entity(EntityKind::Facility);
        let source = txn.create_entity(EntityKind::Facility);
        txn.set_ground_location(basin, place).unwrap();
        txn.set_ground_location(source, place).unwrap();
        txn.set_component_workstation_marker(basin, WorkstationMarker(WorkstationTag::WashBasin))
            .unwrap();
        txn.set_component_resource_source(
            source,
            ResourceSource {
                commodity: CommodityKind::Water,
                available_quantity: Quantity(available_water),
                max_quantity: Quantity(available_water),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        )
        .unwrap();
        commit_txn(txn);
        (basin, source)
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
        let affordances = affordances_for(world, actor, defs, handlers);
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
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(10)),
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
                worldwake_sim::ActionExecutionContext::without_recipes(
                    CauseRef::Bootstrap,
                    Tick(tick),
                ),
            )
            .unwrap()
            {
                TickOutcome::Continuing => {}
                TickOutcome::Committed { .. } => break,
                TickOutcome::Aborted { reason, .. } => panic!("unexpected abort: {reason:?}"),
            }
        }

        instance_id
    }

    #[test]
    fn register_needs_actions_adds_all_six_defs_and_handlers() {
        let (defs, handlers) = setup_registries();
        assert_eq!(defs.len(), 6);
        assert_eq!(handlers.len(), 6);
        assert_eq!(defs.get(ActionDefId(0)).unwrap().name, "eat");
        assert_eq!(defs.get(ActionDefId(4)).unwrap().name, "wash");
        assert_eq!(defs.get(ActionDefId(5)).unwrap().name, "relieve_wilderness");
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
            txn.set_ground_location(bread, place).unwrap();
            txn.set_possessor(bread, actor).unwrap();
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

        let affordances = affordances_for(&world, actor, &defs, &handlers);
        let drink_index = affordances
            .iter()
            .position(|affordance| affordance.def_id == ActionDefId(1))
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
        let affordance = affordances_for(&world, actor, &defs, &handlers)[0].clone();
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
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(11)),
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

        let affordances = affordances_for(&world, actor, &defs, &handlers);
        let sleep_index = affordances
            .iter()
            .position(|affordance| affordance.def_id == ActionDefId(2))
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
        let place = prototype_place_entity(PrototypePlace::PublicLatrine);
        let mut txn = new_txn(&mut world, 1);
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
                NonZeroU32::new(8).unwrap(),
                pm(0),
                pm(0),
                pm(0),
                pm(0),
            ),
        )
        .unwrap();
        commit_txn(txn);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let affordances = affordances_for(&world, actor, &defs, &handlers);
        let toilet_index = affordances
            .iter()
            .position(|affordance| affordance.def_id == ActionDefId(3))
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
    fn toilet_affordance_requires_latrine_tagged_place() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, village_square) = setup_actor(&mut world);
        let public_latrine = prototype_place_entity(PrototypePlace::PublicLatrine);
        let (defs, handlers) = setup_registries();

        let square_affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            square_affordances
                .iter()
                .all(|affordance| affordance.def_id != ActionDefId(3)),
            "toilet should not be available away from a latrine; actor_place={village_square}"
        );

        let mut txn = new_txn(&mut world, 2);
        txn.set_ground_location(actor, public_latrine).unwrap();
        commit_txn(txn);

        let latrine_affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            latrine_affordances
                .iter()
                .any(|affordance| affordance.def_id == ActionDefId(3)),
            "toilet should be available at the public latrine; actor_place={public_latrine}"
        );
    }

    #[test]
    fn wash_consumes_local_water_source_and_clears_dirtiness() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        let (_basin, source) = setup_wash_access(&mut world, place, 2);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let affordances = affordances_for(&world, actor, &defs, &handlers);
        let wash_index = affordances
            .iter()
            .position(|affordance| affordance.def_id == ActionDefId(4))
            .unwrap();
        run_action_to_completion(actor, wash_index, &mut world, &mut log, &defs, &handlers);

        assert_eq!(
            world
                .get_component_resource_source(source)
                .unwrap()
                .available_quantity,
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

        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .all(|affordance| affordance.def_id != ActionDefId(0))
        );
    }

    // --- Possession-requirement tests (S01PROOUTOWNCLA-010) ---

    fn eat_def_id() -> ActionDefId {
        ActionDefId(0)
    }
    fn drink_def_id() -> ActionDefId {
        ActionDefId(1)
    }
    fn wash_def_id() -> ActionDefId {
        ActionDefId(4)
    }

    #[test]
    fn eat_rejects_unpossessed_owned_ground_lot() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        {
            let mut txn = new_txn(&mut world, 2);
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_owner(bread, actor).unwrap();
            commit_txn(txn);
        }
        let (defs, handlers) = setup_registries();
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances.iter().all(|a| a.def_id != eat_def_id()),
            "eat should not be offered for owned-but-unpossessed ground lot"
        );
    }

    #[test]
    fn eat_accepts_possessed_lot() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        {
            let mut txn = new_txn(&mut world, 2);
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_possessor(bread, actor).unwrap();
            commit_txn(txn);
        }
        let (defs, handlers) = setup_registries();
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances.iter().any(|a| a.def_id == eat_def_id()),
            "eat should be offered for possessed lot"
        );
    }

    #[test]
    fn drink_rejects_unpossessed_owned_ground_lot() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        {
            let mut txn = new_txn(&mut world, 2);
            let water = txn
                .create_item_lot(CommodityKind::Water, Quantity(1))
                .unwrap();
            txn.set_ground_location(water, place).unwrap();
            txn.set_owner(water, actor).unwrap();
            commit_txn(txn);
        }
        let (defs, handlers) = setup_registries();
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances.iter().all(|a| a.def_id != drink_def_id()),
            "drink should not be offered for owned-but-unpossessed ground lot"
        );
    }

    #[test]
    fn drink_accepts_possessed_lot() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        {
            let mut txn = new_txn(&mut world, 2);
            let water = txn
                .create_item_lot(CommodityKind::Water, Quantity(1))
                .unwrap();
            txn.set_ground_location(water, place).unwrap();
            txn.set_possessor(water, actor).unwrap();
            commit_txn(txn);
        }
        let (defs, handlers) = setup_registries();
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances.iter().any(|a| a.def_id == drink_def_id()),
            "drink should be offered for possessed lot"
        );
    }

    #[test]
    fn wash_rejects_water_source_without_wash_basin() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        let mut txn = new_txn(&mut world, 2);
        let source = txn.create_entity(EntityKind::Facility);
        txn.set_ground_location(source, place).unwrap();
        txn.set_component_resource_source(
            source,
            ResourceSource {
                commodity: CommodityKind::Water,
                available_quantity: Quantity(1),
                max_quantity: Quantity(1),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        )
        .unwrap();
        commit_txn(txn);
        let (defs, handlers) = setup_registries();
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances.iter().all(|a| a.def_id != wash_def_id()),
            "wash should not be offered without a local wash basin"
        );
    }

    #[test]
    fn wash_accepts_local_basin_and_water_source() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, place) = setup_actor(&mut world);
        let _ = setup_wash_access(&mut world, place, 1);
        let (defs, handlers) = setup_registries();
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances.iter().any(|a| a.def_id == wash_def_id()),
            "wash should be offered for a local wash basin plus water source"
        );
    }

    fn relieve_wilderness_def_id() -> ActionDefId {
        ActionDefId(5)
    }

    fn setup_actor_at_place(world: &mut World, place: EntityId) -> EntityId {
        let mut txn = new_txn(world, 1);
        let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, place).unwrap();
        txn.set_component_homeostatic_needs(
            actor,
            HomeostaticNeeds::new(pm(700), pm(650), pm(400), pm(800), pm(100)),
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
                NonZeroU32::new(8).unwrap(),
                pm(0),
                pm(0),
                pm(0),
                pm(150), // wilderness_relief_dirtiness_penalty
            ),
        )
        .unwrap();
        commit_txn(txn);
        actor
    }

    #[test]
    fn relieve_wilderness_accepts_outdoor_places() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (defs, handlers) = setup_registries();

        // ForestPath has tags Forest + Trail — both outdoor
        let forest_path = prototype_place_entity(PrototypePlace::ForestPath);
        let actor = setup_actor_at_place(&mut world, forest_path);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .any(|a| a.def_id == relieve_wilderness_def_id()),
            "relieve_wilderness should be available at ForestPath"
        );

        // EastFieldTrail has tags Trail + Field — both outdoor
        let east_field = prototype_place_entity(PrototypePlace::EastFieldTrail);
        let mut txn = new_txn(&mut world, 2);
        txn.set_ground_location(actor, east_field).unwrap();
        commit_txn(txn);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .any(|a| a.def_id == relieve_wilderness_def_id()),
            "relieve_wilderness should be available at EastFieldTrail"
        );

        // NorthCrossroads has tags Crossroads + Road — Road is outdoor
        let crossroads = prototype_place_entity(PrototypePlace::NorthCrossroads);
        let mut txn = new_txn(&mut world, 3);
        txn.set_ground_location(actor, crossroads).unwrap();
        commit_txn(txn);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .any(|a| a.def_id == relieve_wilderness_def_id()),
            "relieve_wilderness should be available at NorthCrossroads (has Road tag)"
        );

        // OrchardFarm has tags Farm + Field — both outdoor
        let farm = prototype_place_entity(PrototypePlace::OrchardFarm);
        let mut txn = new_txn(&mut world, 4);
        txn.set_ground_location(actor, farm).unwrap();
        commit_txn(txn);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .any(|a| a.def_id == relieve_wilderness_def_id()),
            "relieve_wilderness should be available at OrchardFarm"
        );
    }

    #[test]
    fn relieve_wilderness_rejects_indoor_places() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (defs, handlers) = setup_registries();

        // VillageSquare has tag Village — not outdoor
        let (actor, _) = setup_actor(&mut world);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .all(|a| a.def_id != relieve_wilderness_def_id()),
            "relieve_wilderness should not be available at VillageSquare"
        );

        // PublicLatrine has tags Latrine + Village — not outdoor
        let latrine = prototype_place_entity(PrototypePlace::PublicLatrine);
        let mut txn = new_txn(&mut world, 2);
        txn.set_ground_location(actor, latrine).unwrap();
        commit_txn(txn);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .all(|a| a.def_id != relieve_wilderness_def_id()),
            "relieve_wilderness should not be available at PublicLatrine"
        );

        // CommonHouse has tags Inn + Village — not outdoor
        let inn = prototype_place_entity(PrototypePlace::CommonHouse);
        let mut txn = new_txn(&mut world, 3);
        txn.set_ground_location(actor, inn).unwrap();
        commit_txn(txn);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .all(|a| a.def_id != relieve_wilderness_def_id()),
            "relieve_wilderness should not be available at CommonHouse"
        );

        // RulersHall has tags Hall + Village — not outdoor
        let hall = prototype_place_entity(PrototypePlace::RulersHall);
        let mut txn = new_txn(&mut world, 4);
        txn.set_ground_location(actor, hall).unwrap();
        commit_txn(txn);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .all(|a| a.def_id != relieve_wilderness_def_id()),
            "relieve_wilderness should not be available at RulersHall"
        );

        // GuardPost has tags Barracks + Village — not outdoor
        let barracks = prototype_place_entity(PrototypePlace::GuardPost);
        let mut txn = new_txn(&mut world, 5);
        txn.set_ground_location(actor, barracks).unwrap();
        commit_txn(txn);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .all(|a| a.def_id != relieve_wilderness_def_id()),
            "relieve_wilderness should not be available at GuardPost"
        );

        // GeneralStore has tags Store + Village — not outdoor
        let store = prototype_place_entity(PrototypePlace::GeneralStore);
        let mut txn = new_txn(&mut world, 6);
        txn.set_ground_location(actor, store).unwrap();
        commit_txn(txn);
        let affordances = affordances_for(&world, actor, &defs, &handlers);
        assert!(
            affordances
                .iter()
                .all(|a| a.def_id != relieve_wilderness_def_id()),
            "relieve_wilderness should not be available at GeneralStore"
        );
    }

    #[test]
    fn relieve_wilderness_commit_effects() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let forest_path = prototype_place_entity(PrototypePlace::ForestPath);
        let actor = setup_actor_at_place(&mut world, forest_path);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let affordances = affordances_for(&world, actor, &defs, &handlers);
        let rw_index = affordances
            .iter()
            .position(|a| a.def_id == relieve_wilderness_def_id())
            .expect("relieve_wilderness affordance should exist at ForestPath");
        run_action_to_completion(actor, rw_index, &mut world, &mut log, &defs, &handlers);

        let needs = world.get_component_homeostatic_needs(actor).unwrap();
        // Bladder should be 0
        assert_eq!(needs.bladder, pm(0));
        // Dirtiness should be original (100) + penalty (150) = 250
        assert_eq!(needs.dirtiness, pm(250));

        // Waste entity should exist at the place
        let waste_count = world
            .ground_entities_at(forest_path)
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
    fn relieve_wilderness_visibility_is_same_place() {
        let (defs, _) = setup_registries();
        let def = defs.get(relieve_wilderness_def_id()).unwrap();
        assert_eq!(def.visibility, VisibilitySpec::SamePlace);
    }

    #[test]
    fn relieve_wilderness_has_wilderness_relief_event_tag() {
        let (defs, _) = setup_registries();
        let def = defs.get(relieve_wilderness_def_id()).unwrap();
        assert!(
            def.causal_event_tags.contains(&EventTag::WildernessRelief),
            "relieve_wilderness should have WildernessRelief event tag"
        );
    }

    #[test]
    fn relieve_wilderness_commit_emits_scene_evidence() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let forest_path = prototype_place_entity(PrototypePlace::ForestPath);
        let actor = setup_actor_at_place(&mut world, forest_path);
        let (defs, handlers) = setup_registries();
        let mut log = EventLog::new();

        let affordances = affordances_for(&world, actor, &defs, &handlers);
        let rw_index = affordances
            .iter()
            .position(|a| a.def_id == relieve_wilderness_def_id())
            .expect("relieve_wilderness affordance should exist at ForestPath");
        run_action_to_completion(actor, rw_index, &mut world, &mut log, &defs, &handlers);

        let scene = world
            .get_component_scene_evidence(forest_path)
            .expect("wilderness relief should leave scene evidence");
        assert!(scene.evidence.iter().any(|entry| {
            matches!(
                entry.kind,
                EvidenceKind::DisturbanceMarker {
                    place,
                    kind: DisturbanceKind::WildernessRelief,
                    ..
                } if place == forest_path
            )
        }));
    }
}
