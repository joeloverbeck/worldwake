//! Actions for moving goods between direct possession and facility stock
//! containers.  See S05 (Merchant Stock Storage and Stall Custody).

use std::collections::BTreeSet;
use std::num::NonZeroU32;
use worldwake_core::{
    ActionDefId, BodyCostPerTick, EntityId, EntityKind, EventTag, SaleListing, StockAssignment,
    StockAssignmentKind, StockStoragePolicy, VisibilitySpec, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerRegistry,
    ActionInstance, ActionPayload, ActionProgress, ActionState, CommitOutcome, Constraint,
    DeterministicRng, DurationExpr, Interruptibility, Precondition, TargetSpec,
};

use crate::inventory::move_entity_to_direct_possession;

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

pub fn register_stock_actions(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> Vec<ActionDefId> {
    let store_handler = handlers.register(ActionHandler::new(
        start_store_stock,
        tick_stock,
        commit_store_stock,
        abort_stock,
    ));
    let collect_handler = handlers.register(ActionHandler::new(
        start_collect_display_stock,
        tick_stock,
        commit_collect_display_stock,
        abort_stock,
    ));

    let stage_handler = handlers.register(ActionHandler::new(
        start_stage_stock_for_sale,
        tick_stock,
        commit_stage_stock_for_sale,
        abort_stock,
    ));
    let unstage_handler = handlers.register(ActionHandler::new(
        start_unstage_stock,
        tick_stock,
        commit_unstage_stock,
        abort_stock,
    ));

    let store_id = ActionDefId(defs.len() as u32);
    let collect_id = ActionDefId(store_id.0 + 1);
    let stage_id = ActionDefId(collect_id.0 + 1);
    let unstage_id = ActionDefId(stage_id.0 + 1);

    vec![
        defs.register(ActionDef {
            id: store_id,
            name: "store_stock".to_string(),
            domain: worldwake_core::ActionDomain::Transport,
            actor_constraints: vec![Constraint::ActorAlive, Constraint::ActorHasControl],
            targets: vec![TargetSpec::EntityDirectlyPossessedByActor {
                kind: EntityKind::ItemLot,
            }],
            preconditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
                Precondition::TargetKind {
                    target_index: 0,
                    kind: EntityKind::ItemLot,
                },
            ],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::MIN),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::InterruptibleWithPenalty,
            commit_conditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
            ],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::from([EventTag::Transfer, EventTag::WorldMutation]),
            payload: ActionPayload::None,
            handler: store_handler,
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
        }),
        defs.register(ActionDef {
            id: collect_id,
            name: "collect_display_stock".to_string(),
            domain: worldwake_core::ActionDomain::Transport,
            actor_constraints: vec![Constraint::ActorAlive, Constraint::ActorHasControl],
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
            ],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::MIN),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::InterruptibleWithPenalty,
            commit_conditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
            ],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::from([EventTag::Transfer, EventTag::WorldMutation]),
            payload: ActionPayload::None,
            handler: collect_handler,
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
        }),
        defs.register(ActionDef {
            id: stage_id,
            name: "stage_stock_for_sale".to_string(),
            domain: worldwake_core::ActionDomain::Trade,
            actor_constraints: vec![Constraint::ActorAlive, Constraint::ActorHasControl],
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
            ],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::MIN),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::InterruptibleWithPenalty,
            commit_conditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
            ],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::from([
                EventTag::Trade,
                EventTag::Transfer,
                EventTag::WorldMutation,
            ]),
            payload: ActionPayload::None,
            handler: stage_handler,
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
        }),
        defs.register(ActionDef {
            id: unstage_id,
            name: "unstage_stock".to_string(),
            domain: worldwake_core::ActionDomain::Trade,
            actor_constraints: vec![Constraint::ActorAlive, Constraint::ActorHasControl],
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
            ],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::MIN),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::InterruptibleWithPenalty,
            commit_conditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
            ],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::from([
                EventTag::Trade,
                EventTag::Transfer,
                EventTag::WorldMutation,
            ]),
            payload: ActionPayload::None,
            handler: unstage_handler,
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
        }),
    ]
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn require_item_lot_target(instance: &ActionInstance) -> Result<EntityId, ActionError> {
    instance
        .targets
        .first()
        .copied()
        .ok_or(ActionError::PreconditionFailed(
            "store/collect action requires an item lot target".to_string(),
        ))
}

fn controlled_facility_policy(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    facility: EntityId,
) -> Result<(EntityId, StockStoragePolicy), ActionError> {
    let actor_place = txn
        .effective_place(actor)
        .ok_or(ActionError::PreconditionFailed(
            "actor has no effective place".to_string(),
        ))?;
    if txn.effective_place(facility) != Some(actor_place) {
        return Err(ActionError::PreconditionFailed(
            "facility is not at actor's place".to_string(),
        ));
    }
    txn.can_exercise_control(actor, facility)
        .map_err(|err| ActionError::PreconditionFailed(err.to_string()))?;
    let policy = txn
        .get_component_stock_storage_policy(facility)
        .cloned()
        .ok_or(ActionError::PreconditionFailed(
            "facility lacks StockStoragePolicy".to_string(),
        ))?;
    Ok((facility, policy))
}

fn resolve_merchant_home_facility(
    txn: &WorldTxn<'_>,
    actor: EntityId,
) -> Result<(EntityId, StockStoragePolicy), ActionError> {
    let profile =
        txn.get_component_merchandise_profile(actor)
            .ok_or(ActionError::PreconditionFailed(
                "actor lacks MerchandiseProfile".to_string(),
            ))?;
    let facility = profile
        .home_facility
        .ok_or(ActionError::PreconditionFailed(
            "actor lacks a bound home facility".to_string(),
        ))?;
    controlled_facility_policy(txn, actor, facility)
}

/// Resolve the facility at the actor's place that has a `StockStoragePolicy`
/// and that the actor can control.
fn resolve_controlled_facility(
    txn: &WorldTxn<'_>,
    actor: EntityId,
) -> Result<(EntityId, StockStoragePolicy), ActionError> {
    let place = txn
        .effective_place(actor)
        .ok_or(ActionError::PreconditionFailed(
            "actor has no effective place".to_string(),
        ))?;

    // Find any Facility at the actor's place with a StockStoragePolicy that
    // the actor can control.
    for (entity, policy) in txn.query_stock_storage_policy() {
        if txn.effective_place(entity) != Some(place) {
            continue;
        }
        if txn.can_exercise_control(actor, entity).is_ok() {
            return Ok((entity, policy.clone()));
        }
    }

    Err(ActionError::PreconditionFailed(
        "no controlled facility with StockStoragePolicy at actor's place".to_string(),
    ))
}

fn resolve_facility_for_lot(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    lot: EntityId,
) -> Result<(EntityId, StockStoragePolicy), ActionError> {
    let facility = txn
        .get_component_stock_assignment(lot)
        .map(|assignment| assignment.facility)
        .ok_or(ActionError::PreconditionFailed(
            "target lot has no StockAssignment".to_string(),
        ))?;
    controlled_facility_policy(txn, actor, facility)
}

// ---------------------------------------------------------------------------
// store_stock handlers
// ---------------------------------------------------------------------------

fn start_store_stock(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let lot = require_item_lot_target(instance)?;
    // Actor must possess the lot.
    if txn.possessor_of(lot) != Some(instance.actor) {
        return Err(ActionError::PreconditionFailed(
            "actor does not possess the target lot".to_string(),
        ));
    }
    // Actor must control a local facility with StockStoragePolicy.
    let _ = resolve_merchant_home_facility(txn, instance.actor)
        .or_else(|_| resolve_controlled_facility(txn, instance.actor))?;
    Ok(None)
}

fn commit_store_stock(
    _def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let lot = require_item_lot_target(instance)?;
    let (facility, policy) = resolve_merchant_home_facility(txn, instance.actor)
        .or_else(|_| resolve_controlled_facility(txn, instance.actor))?;

    // Clear possession.
    txn.clear_possessor(lot)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    // Put lot into stock container.
    txn.put_into_container(lot, policy.stock_container)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    // Set stock assignment.
    txn.set_component_stock_assignment(
        lot,
        StockAssignment {
            facility,
            kind: StockAssignmentKind::Stored,
        },
    )
    .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.add_target(lot);
    Ok(CommitOutcome::empty())
}

// ---------------------------------------------------------------------------
// collect_display_stock handlers
// ---------------------------------------------------------------------------

fn start_collect_display_stock(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let lot = require_item_lot_target(instance)?;
    // Lot must be in a facility container (has StockAssignment).
    if txn.get_component_stock_assignment(lot).is_none() {
        return Err(ActionError::PreconditionFailed(
            "target lot has no StockAssignment (not in facility storage)".to_string(),
        ));
    }
    // Actor must control the facility.
    let _ = resolve_facility_for_lot(txn, instance.actor, lot)
        .or_else(|_| resolve_controlled_facility(txn, instance.actor))?;
    Ok(None)
}

fn commit_collect_display_stock(
    _def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let lot = require_item_lot_target(instance)?;
    let place = txn
        .effective_place(instance.actor)
        .ok_or(ActionError::PreconditionFailed(
            "actor has no effective place".to_string(),
        ))?;

    // Move lot out of container into direct possession.
    move_entity_to_direct_possession(txn, lot, instance.actor, place)?;
    // Clear stock assignment.
    txn.clear_component_stock_assignment(lot)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    Ok(CommitOutcome::empty())
}

// ---------------------------------------------------------------------------
// stage_stock_for_sale handlers
// ---------------------------------------------------------------------------

fn start_stage_stock_for_sale(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let lot = require_item_lot_target(instance)?;
    // Lot must be stored (not displayed, not possessed).
    match txn.get_component_stock_assignment(lot) {
        Some(assignment) if assignment.kind == StockAssignmentKind::Stored => {}
        _ => {
            return Err(ActionError::PreconditionFailed(
                "target lot must have StockAssignment::Stored to stage for sale".to_string(),
            ));
        }
    }
    // Facility must have a display container.
    let (_facility, policy) = resolve_facility_for_lot(txn, instance.actor, lot)
        .or_else(|_| resolve_controlled_facility(txn, instance.actor))?;
    if policy.display_container.is_none() {
        return Err(ActionError::PreconditionFailed(
            "facility has no display container for staging".to_string(),
        ));
    }
    Ok(None)
}

fn commit_stage_stock_for_sale(
    _def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let lot = require_item_lot_target(instance)?;
    let (facility, policy) = resolve_facility_for_lot(txn, instance.actor, lot)
        .or_else(|_| resolve_controlled_facility(txn, instance.actor))?;
    let display_container = policy.display_container.ok_or_else(|| {
        ActionError::PreconditionFailed("facility has no display container for staging".to_string())
    })?;

    // Move lot from stock container to display container.
    txn.remove_from_container(lot)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.put_into_container(lot, display_container)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    // Update assignment to Displayed.
    txn.set_component_stock_assignment(
        lot,
        StockAssignment {
            facility,
            kind: StockAssignmentKind::Displayed,
        },
    )
    .map_err(|err| ActionError::InternalError(err.to_string()))?;
    // Add SaleListing.
    txn.set_component_sale_listing(
        lot,
        SaleListing {
            listed_at: txn.tick(),
        },
    )
    .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.add_target(lot);
    Ok(CommitOutcome::empty())
}

// ---------------------------------------------------------------------------
// unstage_stock handlers
// ---------------------------------------------------------------------------

fn start_unstage_stock(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let lot = require_item_lot_target(instance)?;
    // Lot must be displayed.
    match txn.get_component_stock_assignment(lot) {
        Some(assignment) if assignment.kind == StockAssignmentKind::Displayed => {}
        _ => {
            return Err(ActionError::PreconditionFailed(
                "target lot must have StockAssignment::Displayed to unstage".to_string(),
            ));
        }
    }
    let _ = resolve_facility_for_lot(txn, instance.actor, lot)
        .or_else(|_| resolve_controlled_facility(txn, instance.actor))?;
    Ok(None)
}

fn commit_unstage_stock(
    _def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let lot = require_item_lot_target(instance)?;
    let (facility, policy) = resolve_facility_for_lot(txn, instance.actor, lot)
        .or_else(|_| resolve_controlled_facility(txn, instance.actor))?;

    // Move lot from display container back to stock container.
    txn.remove_from_container(lot)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.put_into_container(lot, policy.stock_container)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    // Update assignment to Stored.
    txn.set_component_stock_assignment(
        lot,
        StockAssignment {
            facility,
            kind: StockAssignmentKind::Stored,
        },
    )
    .map_err(|err| ActionError::InternalError(err.to_string()))?;
    // Clear SaleListing.
    let _ = txn.clear_component_sale_listing(lot);
    txn.add_target(lot);
    Ok(CommitOutcome::empty())
}

// ---------------------------------------------------------------------------
// Shared no-op tick and abort
// ---------------------------------------------------------------------------

#[allow(clippy::unnecessary_wraps)]
fn tick_stock(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

#[allow(clippy::unnecessary_wraps)]
fn abort_stock(
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use worldwake_core::{
        CommodityKind, ControlSource, EventLog, LoadUnits, Place, Quantity, StockAssignmentKind,
        Tick, Topology, World,
    };
    use worldwake_sim::{
        ActionDuration, ActionInstance, ActionInstanceId, ActionPayload, ActionStatus,
    };

    fn test_topology() -> Topology {
        let mut topo = Topology::new();
        topo.add_place(
            entity(100),
            Place {
                name: "Market".to_string(),
                capacity: None,
                tags: BTreeSet::new(),
            },
        )
        .unwrap();
        topo
    }

    fn entity(n: u64) -> EntityId {
        worldwake_core::test_utils::entity_id(n as u32, 1)
    }

    struct StockTestHarness {
        world: World,
        event_log: EventLog,
        place: EntityId,
        agent: EntityId,
        facility: EntityId,
        stock_container: EntityId,
        bread_lot: EntityId,
    }

    fn setup_harness() -> StockTestHarness {
        let place = entity(100);
        let topo = test_topology();
        let mut world = World::new(topo).unwrap();
        let mut log = EventLog::new();

        let (agent, facility, stock_container, bread_lot) = {
            let mut txn = new_txn(&mut world, &mut log);
            let agent = txn.create_agent("Merchant", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();

            let (facility, stock_container, _display) = txn
                .create_merchant_facility(place, agent, LoadUnits(200), None)
                .unwrap();

            let bread_lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(5))
                .unwrap();
            txn.set_ground_location(bread_lot, place).unwrap();
            txn.set_possessor(bread_lot, agent).unwrap();

            txn.commit(&mut log);
            (agent, facility, stock_container, bread_lot)
        };

        StockTestHarness {
            world,
            event_log: log,
            place,
            agent,
            facility,
            stock_container,
            bread_lot,
        }
    }

    fn new_txn<'w>(world: &'w mut World, _log: &mut EventLog) -> WorldTxn<'w> {
        use worldwake_core::WitnessData;
        WorldTxn::new(
            world,
            Tick(1),
            worldwake_core::CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        )
    }

    fn make_instance(actor: EntityId, target: EntityId) -> ActionInstance {
        ActionInstance {
            instance_id: ActionInstanceId(0),
            def_id: ActionDefId(999),
            payload: ActionPayload::None,
            actor,
            targets: vec![target],
            start_tick: Tick(0),
            remaining_duration: ActionDuration::new(1),
            status: ActionStatus::Active,
            reservation_ids: vec![],
            local_state: None,
            body_cost_override: None,
        }
    }

    fn dummy_def() -> ActionDef {
        ActionDef {
            id: ActionDefId(999),
            name: "test".to_string(),
            domain: worldwake_core::ActionDomain::Transport,
            actor_constraints: vec![],
            targets: vec![],
            preconditions: vec![],
            reservation_requirements: vec![],
            duration: DurationExpr::Fixed(NonZeroU32::MIN),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerRegistry::new().register(ActionHandler::new(
                start_store_stock,
                tick_stock,
                commit_store_stock,
                abort_stock,
            )),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
        }
    }

    fn dummy_rng() -> DeterministicRng {
        DeterministicRng::new(worldwake_core::Seed([0; 32]))
    }

    // -----------------------------------------------------------------------
    // store_stock
    // -----------------------------------------------------------------------

    #[test]
    fn store_stock_moves_lot_into_stock_container() {
        let mut h = setup_harness();
        let def = dummy_def();
        let mut rng = dummy_rng();

        // Pre: lot is possessed by agent.
        assert_eq!(h.world.possessor_of(h.bread_lot), Some(h.agent));

        {
            let mut txn = new_txn(&mut h.world, &mut h.event_log);
            let instance = make_instance(h.agent, h.bread_lot);
            commit_store_stock(
                &def,
                &instance,
                &worldwake_sim::ActionExecutionContext::without_recipes(
                    worldwake_core::CauseRef::Bootstrap,
                    txn.tick(),
                ),
                &EventLog::new(),
                &mut rng,
                &mut txn,
            )
            .unwrap();
            txn.commit(&mut h.event_log);
        }

        // Post: lot is in stock container, not possessed.
        assert_eq!(h.world.possessor_of(h.bread_lot), None);
        assert_eq!(
            h.world.direct_container(h.bread_lot),
            Some(h.stock_container)
        );
    }

    #[test]
    fn store_stock_sets_stock_assignment() {
        let mut h = setup_harness();
        let def = dummy_def();
        let mut rng = dummy_rng();

        {
            let mut txn = new_txn(&mut h.world, &mut h.event_log);
            let instance = make_instance(h.agent, h.bread_lot);
            commit_store_stock(
                &def,
                &instance,
                &worldwake_sim::ActionExecutionContext::without_recipes(
                    worldwake_core::CauseRef::Bootstrap,
                    txn.tick(),
                ),
                &EventLog::new(),
                &mut rng,
                &mut txn,
            )
            .unwrap();
            txn.commit(&mut h.event_log);
        }

        let assignment = h
            .world
            .get_component_stock_assignment(h.bread_lot)
            .expect("lot should have StockAssignment after store");
        assert_eq!(assignment.facility, h.facility);
        assert_eq!(assignment.kind, StockAssignmentKind::Stored);
    }

    // -----------------------------------------------------------------------
    // collect_display_stock
    // -----------------------------------------------------------------------

    #[test]
    fn collect_display_stock_moves_lot_to_possession() {
        let mut h = setup_harness();
        let def = dummy_def();
        let mut rng = dummy_rng();

        // First store the lot.
        {
            let mut txn = new_txn(&mut h.world, &mut h.event_log);
            let instance = make_instance(h.agent, h.bread_lot);
            commit_store_stock(
                &def,
                &instance,
                &worldwake_sim::ActionExecutionContext::without_recipes(
                    worldwake_core::CauseRef::Bootstrap,
                    txn.tick(),
                ),
                &EventLog::new(),
                &mut rng,
                &mut txn,
            )
            .unwrap();
            txn.commit(&mut h.event_log);
        }

        // Now collect it.
        {
            let mut txn = new_txn(&mut h.world, &mut h.event_log);
            let instance = make_instance(h.agent, h.bread_lot);
            commit_collect_display_stock(
                &def,
                &instance,
                &worldwake_sim::ActionExecutionContext::without_recipes(
                    worldwake_core::CauseRef::Bootstrap,
                    txn.tick(),
                ),
                &EventLog::new(),
                &mut rng,
                &mut txn,
            )
            .unwrap();
            txn.commit(&mut h.event_log);
        }

        // Post: lot is possessed by agent, not in container, no assignment.
        assert_eq!(h.world.possessor_of(h.bread_lot), Some(h.agent));
        assert!(h.world.direct_container(h.bread_lot).is_none());
        assert!(
            h.world
                .get_component_stock_assignment(h.bread_lot)
                .is_none()
        );
    }

    // -----------------------------------------------------------------------
    // Authorization
    // -----------------------------------------------------------------------

    #[test]
    fn store_stock_rejects_non_controller() {
        let mut h = setup_harness();
        let mut rng = dummy_rng();

        // Create a stranger who does not own/control the facility.
        let stranger = {
            let mut txn = new_txn(&mut h.world, &mut h.event_log);
            let stranger = txn.create_agent("Stranger", ControlSource::Ai).unwrap();
            txn.set_ground_location(stranger, h.place).unwrap();
            // Give bread lot to stranger.
            txn.clear_possessor(h.bread_lot).unwrap();
            txn.set_possessor(h.bread_lot, stranger).unwrap();
            txn.commit(&mut h.event_log);
            stranger
        };

        let def = dummy_def();
        let mut txn = new_txn(&mut h.world, &mut h.event_log);
        let mut instance = make_instance(stranger, h.bread_lot);
        let result = start_store_stock(
            &def,
            &mut instance,
            &worldwake_sim::ActionExecutionContext::without_recipes(
                worldwake_core::CauseRef::Bootstrap,
                txn.tick(),
            ),
            &mut rng,
            &mut txn,
        );

        assert!(
            result.is_err(),
            "non-controller should be rejected from store_stock"
        );
    }

    // -----------------------------------------------------------------------
    // Conservation
    // -----------------------------------------------------------------------

    #[test]
    fn store_and_collect_preserves_lot_quantity() {
        let mut h = setup_harness();
        let def = dummy_def();
        let mut rng = dummy_rng();

        let original_qty = h
            .world
            .get_component_item_lot(h.bread_lot)
            .unwrap()
            .quantity;

        // Store.
        {
            let mut txn = new_txn(&mut h.world, &mut h.event_log);
            let instance = make_instance(h.agent, h.bread_lot);
            commit_store_stock(
                &def,
                &instance,
                &worldwake_sim::ActionExecutionContext::without_recipes(
                    worldwake_core::CauseRef::Bootstrap,
                    txn.tick(),
                ),
                &EventLog::new(),
                &mut rng,
                &mut txn,
            )
            .unwrap();
            txn.commit(&mut h.event_log);
        }

        assert_eq!(
            h.world
                .get_component_item_lot(h.bread_lot)
                .unwrap()
                .quantity,
            original_qty,
            "quantity should not change during store"
        );

        // Collect.
        {
            let mut txn = new_txn(&mut h.world, &mut h.event_log);
            let instance = make_instance(h.agent, h.bread_lot);
            commit_collect_display_stock(
                &def,
                &instance,
                &worldwake_sim::ActionExecutionContext::without_recipes(
                    worldwake_core::CauseRef::Bootstrap,
                    txn.tick(),
                ),
                &EventLog::new(),
                &mut rng,
                &mut txn,
            )
            .unwrap();
            txn.commit(&mut h.event_log);
        }

        assert_eq!(
            h.world
                .get_component_item_lot(h.bread_lot)
                .unwrap()
                .quantity,
            original_qty,
            "quantity should not change during collect"
        );
    }

    // -----------------------------------------------------------------------
    // Staging harness (facility WITH display container)
    // -----------------------------------------------------------------------

    struct DisplayTestHarness {
        world: World,
        event_log: EventLog,
        #[allow(dead_code)]
        place: EntityId,
        agent: EntityId,
        facility: EntityId,
        stock_container: EntityId,
        display_container: EntityId,
        bread_lot: EntityId,
    }

    fn setup_display_harness() -> DisplayTestHarness {
        let place = entity(100);
        let topo = test_topology();
        let mut world = World::new(topo).unwrap();
        let mut log = EventLog::new();

        let (agent, facility, stock_container, display_container, bread_lot) = {
            let mut txn = new_txn(&mut world, &mut log);
            let agent = txn.create_agent("Merchant", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();

            let (facility, stock_container, display) = txn
                .create_merchant_facility(place, agent, LoadUnits(200), Some(LoadUnits(100)))
                .unwrap();
            let display_container = display.unwrap();

            let bread_lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(5))
                .unwrap();
            txn.set_ground_location(bread_lot, place).unwrap();
            txn.set_possessor(bread_lot, agent).unwrap();

            txn.commit(&mut log);
            (
                agent,
                facility,
                stock_container,
                display_container,
                bread_lot,
            )
        };

        DisplayTestHarness {
            world,
            event_log: log,
            place,
            agent,
            facility,
            stock_container,
            display_container,
            bread_lot,
        }
    }

    /// Store the bread lot into the facility (prerequisite for staging tests).
    fn store_lot(h: &mut DisplayTestHarness) {
        let def = dummy_def();
        let mut rng = dummy_rng();
        let mut txn = new_txn(&mut h.world, &mut h.event_log);
        let instance = make_instance(h.agent, h.bread_lot);
        commit_store_stock(
            &def,
            &instance,
            &worldwake_sim::ActionExecutionContext::without_recipes(
                worldwake_core::CauseRef::Bootstrap,
                txn.tick(),
            ),
            &EventLog::new(),
            &mut rng,
            &mut txn,
        )
        .unwrap();
        txn.commit(&mut h.event_log);
    }

    // -----------------------------------------------------------------------
    // stage_stock_for_sale
    // -----------------------------------------------------------------------

    #[test]
    fn stage_stock_moves_lot_to_display_and_adds_listing() {
        let mut h = setup_display_harness();
        let def = dummy_def();
        let mut rng = dummy_rng();

        store_lot(&mut h);

        // Stage.
        {
            let mut txn = new_txn(&mut h.world, &mut h.event_log);
            let instance = make_instance(h.agent, h.bread_lot);
            commit_stage_stock_for_sale(
                &def,
                &instance,
                &worldwake_sim::ActionExecutionContext::without_recipes(
                    worldwake_core::CauseRef::Bootstrap,
                    txn.tick(),
                ),
                &EventLog::new(),
                &mut rng,
                &mut txn,
            )
            .unwrap();
            txn.commit(&mut h.event_log);
        }

        // Lot is in display container.
        assert_eq!(
            h.world.direct_container(h.bread_lot),
            Some(h.display_container),
            "staged lot should be in display container"
        );
        // Assignment is Displayed.
        let assignment = h
            .world
            .get_component_stock_assignment(h.bread_lot)
            .expect("staged lot should have StockAssignment");
        assert_eq!(assignment.facility, h.facility);
        assert_eq!(assignment.kind, StockAssignmentKind::Displayed);
        // SaleListing present.
        assert!(
            h.world.get_component_sale_listing(h.bread_lot).is_some(),
            "staged lot should have SaleListing"
        );
    }

    // -----------------------------------------------------------------------
    // unstage_stock
    // -----------------------------------------------------------------------

    #[test]
    fn unstage_stock_reverses_staging() {
        let mut h = setup_display_harness();
        let def = dummy_def();
        let mut rng = dummy_rng();

        store_lot(&mut h);

        // Stage.
        {
            let mut txn = new_txn(&mut h.world, &mut h.event_log);
            let instance = make_instance(h.agent, h.bread_lot);
            commit_stage_stock_for_sale(
                &def,
                &instance,
                &worldwake_sim::ActionExecutionContext::without_recipes(
                    worldwake_core::CauseRef::Bootstrap,
                    txn.tick(),
                ),
                &EventLog::new(),
                &mut rng,
                &mut txn,
            )
            .unwrap();
            txn.commit(&mut h.event_log);
        }

        // Unstage.
        {
            let mut txn = new_txn(&mut h.world, &mut h.event_log);
            let instance = make_instance(h.agent, h.bread_lot);
            commit_unstage_stock(
                &def,
                &instance,
                &worldwake_sim::ActionExecutionContext::without_recipes(
                    worldwake_core::CauseRef::Bootstrap,
                    txn.tick(),
                ),
                &EventLog::new(),
                &mut rng,
                &mut txn,
            )
            .unwrap();
            txn.commit(&mut h.event_log);
        }

        // Lot is back in stock container.
        assert_eq!(
            h.world.direct_container(h.bread_lot),
            Some(h.stock_container),
            "unstaged lot should be back in stock container"
        );
        // Assignment is Stored.
        let assignment = h
            .world
            .get_component_stock_assignment(h.bread_lot)
            .expect("unstaged lot should have StockAssignment");
        assert_eq!(assignment.kind, StockAssignmentKind::Stored);
        // SaleListing cleared.
        assert!(
            h.world.get_component_sale_listing(h.bread_lot).is_none(),
            "unstaged lot should have no SaleListing"
        );
    }

    // -----------------------------------------------------------------------
    // No display container
    // -----------------------------------------------------------------------

    #[test]
    fn stage_stock_fails_without_display_container() {
        // Use the original harness which has NO display container.
        let mut h = setup_harness();
        let def = dummy_def();
        let mut rng = dummy_rng();

        // Store lot first.
        {
            let mut txn = new_txn(&mut h.world, &mut h.event_log);
            let instance = make_instance(h.agent, h.bread_lot);
            commit_store_stock(
                &def,
                &instance,
                &worldwake_sim::ActionExecutionContext::without_recipes(
                    worldwake_core::CauseRef::Bootstrap,
                    txn.tick(),
                ),
                &EventLog::new(),
                &mut rng,
                &mut txn,
            )
            .unwrap();
            txn.commit(&mut h.event_log);
        }

        // Attempt to stage — should fail.
        let mut txn = new_txn(&mut h.world, &mut h.event_log);
        let mut instance = make_instance(h.agent, h.bread_lot);
        let result = start_stage_stock_for_sale(
            &def,
            &mut instance,
            &worldwake_sim::ActionExecutionContext::without_recipes(
                worldwake_core::CauseRef::Bootstrap,
                txn.tick(),
            ),
            &mut rng,
            &mut txn,
        );

        assert!(
            result.is_err(),
            "staging should fail when facility has no display container"
        );
    }

    // -----------------------------------------------------------------------
    // Full round-trip conservation
    // -----------------------------------------------------------------------

    #[test]
    fn full_round_trip_store_stage_unstage_collect_preserves_quantity() {
        let mut h = setup_display_harness();
        let def = dummy_def();
        let mut rng = dummy_rng();

        let original_qty = h
            .world
            .get_component_item_lot(h.bread_lot)
            .unwrap()
            .quantity;

        // Store.
        store_lot(&mut h);
        // Stage.
        {
            let mut txn = new_txn(&mut h.world, &mut h.event_log);
            let instance = make_instance(h.agent, h.bread_lot);
            commit_stage_stock_for_sale(
                &def,
                &instance,
                &worldwake_sim::ActionExecutionContext::without_recipes(
                    worldwake_core::CauseRef::Bootstrap,
                    txn.tick(),
                ),
                &EventLog::new(),
                &mut rng,
                &mut txn,
            )
            .unwrap();
            txn.commit(&mut h.event_log);
        }
        // Unstage.
        {
            let mut txn = new_txn(&mut h.world, &mut h.event_log);
            let instance = make_instance(h.agent, h.bread_lot);
            commit_unstage_stock(
                &def,
                &instance,
                &worldwake_sim::ActionExecutionContext::without_recipes(
                    worldwake_core::CauseRef::Bootstrap,
                    txn.tick(),
                ),
                &EventLog::new(),
                &mut rng,
                &mut txn,
            )
            .unwrap();
            txn.commit(&mut h.event_log);
        }
        // Collect.
        {
            let mut txn = new_txn(&mut h.world, &mut h.event_log);
            let instance = make_instance(h.agent, h.bread_lot);
            commit_collect_display_stock(
                &def,
                &instance,
                &worldwake_sim::ActionExecutionContext::without_recipes(
                    worldwake_core::CauseRef::Bootstrap,
                    txn.tick(),
                ),
                &EventLog::new(),
                &mut rng,
                &mut txn,
            )
            .unwrap();
            txn.commit(&mut h.event_log);
        }

        // Lot is back in possession, quantity preserved.
        assert_eq!(h.world.possessor_of(h.bread_lot), Some(h.agent));
        assert!(h.world.direct_container(h.bread_lot).is_none());
        assert!(
            h.world
                .get_component_stock_assignment(h.bread_lot)
                .is_none()
        );
        assert!(h.world.get_component_sale_listing(h.bread_lot).is_none());
        assert_eq!(
            h.world
                .get_component_item_lot(h.bread_lot)
                .unwrap()
                .quantity,
            original_qty,
            "quantity should be preserved through full round-trip"
        );
    }
}
