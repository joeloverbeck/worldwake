use std::collections::BTreeSet;
use worldwake_core::{
    ActionDefId, BlockerClearingCondition, BodyCostPerTick, EdgeExperience, EntityId, EntityKind,
    EventLog, EventTag, EventView, ExpectationId, GoalKind, Permille, Quantity, RouteSegment, Tick,
    TravelEdgeId, VisibilitySpec, WorldTxn, WoundCause, build_believed_entity_state,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerRegistry,
    ActionInstance, ActionPayload, ActionProgress, ActionState, CommitOutcome, Constraint,
    DeterministicRng, DurationExpr, EffectEvaluationContext, EffectMode, EffectPrecondition,
    EffectSchema, EffectSink, EffectStep, Interruptibility, Precondition, TargetSpec,
    apply_effects_with_context,
};

use crate::evidence_support::emit_evidence;

pub fn register_travel_actions(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(ActionHandler::new(
        start_travel,
        tick_travel,
        commit_travel,
        abort_travel,
    ));
    let id = ActionDefId(defs.len() as u32);
    defs.register(ActionDef {
        id,
        name: "travel".to_string(),
        domain: worldwake_core::ActionDomain::Travel,
        actor_constraints: vec![
            Constraint::ActorAlive,
            Constraint::ActorHasControl,
            Constraint::ActorNotInTransit,
        ],
        targets: vec![TargetSpec::AdjacentPlace],
        preconditions: vec![
            Precondition::TargetExists(0),
            Precondition::TargetAdjacentToActor(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Place,
            },
        ],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::TravelToTarget { target_index: 0 },
        body_cost_per_tick: BodyCostPerTick::zero(),
        attention_cost: Permille::new_unchecked(100),
        interruptibility: Interruptibility::InterruptibleWithPenalty,
        commit_conditions: vec![
            Precondition::TargetExists(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Place,
            },
        ],
        visibility: VisibilitySpec::ParticipantsOnly,
        causal_event_tags: BTreeSet::from([EventTag::Travel]),
        payload: ActionPayload::None,
        handler,
        binding_strictness: worldwake_sim::BindingStrictness::EquivalentRouteStep,
        guard_template: None,
        expectation_template: vec![],
        effect_schema: travel_effect_schema(),
    })
}

fn travel_effect_schema() -> EffectSchema {
    EffectSchema {
        preconditions: Vec::new(),
        steps: vec![EffectStep::CompleteTravel],
    }
}

fn travel_state(
    instance: &ActionInstance,
) -> Result<(TravelEdgeId, EntityId, EntityId, Tick, Tick), ActionError> {
    match instance.local_state {
        Some(ActionState::Travel {
            edge_id,
            origin,
            destination,
            departure_tick,
            arrival_tick,
        }) => Ok((edge_id, origin, destination, departure_tick, arrival_tick)),
        Some(
            ActionState::Empty
            | ActionState::Heal { .. }
            | ActionState::Escort { .. }
            | ActionState::Investigate { .. }
            | ActionState::Trade { .. },
        )
        | None => Err(ActionError::InternalError(format!(
            "travel action instance {} is missing travel state",
            instance.instance_id
        ))),
    }
}

pub(crate) fn direct_possessions(txn: &WorldTxn<'_>, actor: EntityId) -> Vec<EntityId> {
    let mut possessions = txn.possessions_of(actor);
    possessions.sort();
    possessions
}

pub(crate) fn had_combat_during_travel(
    event_log: &EventLog,
    agent: EntityId,
    start_tick: Tick,
    end_tick: Tick,
) -> bool {
    event_log
        .events_by_tag(EventTag::Combat)
        .iter()
        .filter_map(|event_id| event_log.get(*event_id))
        .any(|record| {
            let tick = record.tick();
            tick.0 >= start_tick.0
                && tick.0 < end_tick.0
                && (record.actor_id() == Some(agent) || record.target_ids().contains(&agent))
        })
}

pub(crate) fn record_route_experience(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    edge_id: TravelEdgeId,
    current_tick: Tick,
    hostile: bool,
) -> Result<(), ActionError> {
    let mut route_experience = txn
        .get_component_route_experience(actor)
        .cloned()
        .unwrap_or_default();
    let experience = route_experience
        .edges
        .entry(edge_id)
        .or_insert(EdgeExperience {
            safe_trips: 0,
            hostile_encounters: 0,
            last_travel_tick: current_tick,
        });
    if hostile {
        experience.hostile_encounters = experience.hostile_encounters.saturating_add(1);
    } else {
        experience.safe_trips = experience.safe_trips.saturating_add(1);
    }
    experience.last_travel_tick = current_tick;

    let profile = txn
        .get_component_preference_profile(actor)
        .copied()
        .unwrap_or_else(|| panic!("actor {actor} lacks PreferenceProfile"));
    route_experience.enforce_limits(current_tick, &profile);

    txn.set_component_route_experience(actor, route_experience)
        .map_err(|err| ActionError::InternalError(err.to_string()))
}

fn resolve_travel(
    txn: &WorldTxn<'_>,
    actor: EntityId,
    destination: EntityId,
) -> Result<(TravelEdgeId, u32, EntityId), ActionError> {
    let origin = txn.effective_place(actor).ok_or_else(|| {
        ActionError::PreconditionFailed(format!("actor {actor} has no origin place"))
    })?;
    let edge = txn
        .topology()
        .unique_direct_edge(origin, destination)
        .map_err(|err| ActionError::PreconditionFailed(err.to_string()))?
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "no directed travel edge connects {origin} -> {destination}"
            ))
        })?;
    Ok((edge.id(), edge.travel_time_ticks(), origin))
}

fn start_travel(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let destination = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    let (edge_id, travel_time_ticks, origin) = resolve_travel(txn, instance.actor, destination)?;
    let departure_tick = instance.start_tick;
    let arrival_tick = Tick(
        departure_tick
            .0
            .checked_add(u64::from(travel_time_ticks))
            .ok_or_else(|| {
                ActionError::InternalError("travel arrival tick overflowed".to_string())
            })?,
    );

    txn.set_in_transit(instance.actor)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    for entity in direct_possessions(txn, instance.actor) {
        txn.set_in_transit(entity)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    txn.set_component_in_transit_on_edge(
        instance.actor,
        worldwake_core::InTransitOnEdge {
            edge_id,
            origin,
            destination,
            departure_tick,
            arrival_tick,
        },
    )
    .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.add_tag(EventTag::Travel);

    apply_travel_body_cost(instance, txn);

    Ok(Some(ActionState::Travel {
        edge_id,
        origin,
        destination,
        departure_tick,
        arrival_tick,
    }))
}

pub(crate) fn apply_travel_body_cost(instance: &mut ActionInstance, txn: &WorldTxn<'_>) {
    // Resolve per-agent travel body cost from MetabolismProfile.
    // Cost = basal_rate * travel_multiplier / 1000 for each need.
    if let Some(profile) = txn.get_component_metabolism_profile(instance.actor) {
        let fatigue_val = u16::try_from(
            u32::from(profile.fatigue_rate.value())
                * u32::from(profile.travel_fatigue_multiplier.value())
                / 1000,
        )
        .unwrap_or(1000);
        let thirst_val = u16::try_from(
            u32::from(profile.thirst_rate.value())
                * u32::from(profile.travel_thirst_multiplier.value())
                / 1000,
        )
        .unwrap_or(1000);
        let bladder_val = u16::try_from(
            u32::from(profile.bladder_rate.value())
                * u32::from(profile.travel_bladder_multiplier.value())
                / 1000,
        )
        .unwrap_or(1000);
        let zero = Permille::new_unchecked(0);
        let cost = BodyCostPerTick::new(
            zero,
            Permille::new_unchecked(thirst_val),
            Permille::new_unchecked(fatigue_val),
            Permille::new_unchecked(bladder_val),
            zero,
        );
        instance.body_cost_override = Some(cost);
    }
}

#[allow(clippy::unnecessary_wraps)]
fn tick_travel(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn apply_travel_arrival(
    instance: &ActionInstance,
    event_log: &worldwake_core::EventLog,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    let (edge_id, origin, destination, departure_tick, _) = travel_state(instance)?;
    txn.clear_component_in_transit_on_edge(instance.actor)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.set_ground_location(instance.actor, destination)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    for entity in direct_possessions(txn, instance.actor) {
        txn.set_ground_location(entity, destination)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    let current_tick = txn.tick();
    emit_evidence(
        txn,
        origin,
        worldwake_core::EvidenceKind::MovementTrace {
            entity: instance.actor,
            departed_from: origin,
            direction: destination,
            observed_at: current_tick,
        },
        30,
    )
    .map_err(|err| ActionError::InternalError(err.to_string()))?;
    let hostile = had_combat_during_travel(event_log, instance.actor, departure_tick, current_tick);
    record_route_experience(txn, instance.actor, edge_id, current_tick, hostile)?;
    if !hostile {
        clear_route_retraversed_blockers(txn, instance.actor, origin, destination)?;
    }
    reinforce_exploration_arrival_belief(instance.actor, destination, current_tick, txn)?;
    Ok(())
}

fn clear_route_retraversed_blockers(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    origin: EntityId,
    destination: EntityId,
) -> Result<(), ActionError> {
    let Some(mut memory) = txn.get_component_blocker_memory(actor).cloned() else {
        return Ok(());
    };
    let before = memory.intents.len();
    let segment = RouteSegment::new(origin, destination);
    memory.sweep_cleared(|blocker| {
        matches!(
            blocker.clearing_condition,
            BlockerClearingCondition::RouteRetraversedSafely(cleared) if cleared == segment
        )
    });
    if memory.intents.len() != before {
        txn.set_component_blocker_memory(actor, memory)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    Ok(())
}

fn reinforce_exploration_arrival_belief(
    actor: EntityId,
    destination: EntityId,
    current_tick: Tick,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    let Some(frame) = txn.get_component_intention_frame(actor).cloned() else {
        return Ok(());
    };
    let GoalKind::ExploreLocation { target_place, .. } = frame.goal.kind else {
        return Ok(());
    };
    if target_place != destination {
        return Ok(());
    }

    let boost = txn
        .get_component_exploration_profile(actor)
        .copied()
        .unwrap_or_default()
        .exploration_arrival_boost;
    if boost == Permille::ZERO {
        return Ok(());
    }

    let mut beliefs = txn
        .get_component_agent_belief_store(actor)
        .cloned()
        .ok_or_else(|| {
            ActionError::InternalError(format!("actor {actor} lacks AgentBeliefStore"))
        })?;

    let belief = beliefs
        .known_entities
        .entry(destination)
        .or_insert_with(|| {
            build_believed_entity_state(
                txn,
                destination,
                current_tick,
                worldwake_core::PerceptionSource::DirectObservation,
            )
            .expect("travel destinations must build believed place state")
        });
    let buffer_capacity = u8::try_from(belief.presentation_ticks.len()).unwrap_or(u8::MAX);
    let synthetic_ticks = ((u32::from(boost.value()) * u32::from(buffer_capacity)) / 1000)
        .min(u32::from(u8::MAX)) as u8;
    for _ in 0..synthetic_ticks {
        belief.push_presentation_tick(current_tick, buffer_capacity);
    }

    txn.set_component_agent_belief_store(actor, beliefs)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    Ok(())
}

struct TravelEffectSink<'txn, 'world, 'instance, 'log> {
    txn: &'txn mut WorldTxn<'world>,
    instance: &'instance ActionInstance,
    event_log: &'log EventLog,
    action_error: Option<ActionError>,
}

impl TravelEffectSink<'_, '_, '_, '_> {
    fn record_error(&mut self, error: ActionError) -> worldwake_core::Discrepancy {
        self.action_error = Some(error);
        worldwake_core::Discrepancy::PartialExecutionDrift
    }

    fn take_error(self, discrepancy: worldwake_core::Discrepancy) -> ActionError {
        self.action_error.unwrap_or_else(|| {
            ActionError::PreconditionFailed(format!("effect schema failed: {discrepancy:?}"))
        })
    }
}

impl EffectSink for TravelEffectSink<'_, '_, '_, '_> {
    fn check_precondition(
        &self,
        _precondition: &EffectPrecondition,
        _actor: EntityId,
        _targets: &[EntityId],
    ) -> Result<(), worldwake_core::Discrepancy> {
        Ok(())
    }

    fn checkpoint(&mut self) -> usize {
        0
    }

    fn restore(&mut self, _checkpoint: usize) -> Result<(), worldwake_core::Discrepancy> {
        Err(worldwake_core::Discrepancy::ImproperPlanningState)
    }

    fn write_transfer(
        &mut self,
        _source: EntityId,
        _dest: EntityId,
        _commodity: worldwake_core::CommodityKind,
        _quantity: Quantity,
    ) -> Result<(), worldwake_core::Discrepancy> {
        Err(worldwake_core::Discrepancy::ImproperPlanningState)
    }

    fn write_consume(
        &mut self,
        _source: EntityId,
        _commodity: worldwake_core::CommodityKind,
        _quantity: Quantity,
    ) -> Result<(), worldwake_core::Discrepancy> {
        Err(worldwake_core::Discrepancy::ImproperPlanningState)
    }

    fn write_produce(
        &mut self,
        _sink: EntityId,
        _commodity: worldwake_core::CommodityKind,
        _quantity: Quantity,
    ) -> Result<(), worldwake_core::Discrepancy> {
        Err(worldwake_core::Discrepancy::ImproperPlanningState)
    }

    fn write_wound(
        &mut self,
        _target: EntityId,
        _cause: WoundCause,
    ) -> Result<(), worldwake_core::Discrepancy> {
        Err(worldwake_core::Discrepancy::ImproperPlanningState)
    }

    fn write_event(&mut self, _tag: EventTag) -> Result<(), worldwake_core::Discrepancy> {
        Err(worldwake_core::Discrepancy::ImproperPlanningState)
    }

    fn assert_expectation_fulfilled(
        &mut self,
        _expectation: ExpectationId,
    ) -> Result<(), worldwake_core::Discrepancy> {
        Err(worldwake_core::Discrepancy::ImproperPlanningState)
    }

    fn consume_grant(&mut self, _grant: EntityId) -> Result<(), worldwake_core::Discrepancy> {
        Err(worldwake_core::Discrepancy::ImproperPlanningState)
    }

    fn complete_travel(&mut self, actor: EntityId) -> Result<(), worldwake_core::Discrepancy> {
        if actor != self.instance.actor {
            return Err(self.record_error(ActionError::InvalidTarget(actor)));
        }
        apply_travel_arrival(self.instance, self.event_log, self.txn)
            .map_err(|err| self.record_error(err))
    }
}

fn apply_travel_effect_schema(
    def: &ActionDef,
    instance: &ActionInstance,
    event_log: &EventLog,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let mut sink = TravelEffectSink {
        txn,
        instance,
        event_log,
        action_error: None,
    };
    match apply_effects_with_context(
        &def.effect_schema,
        EffectEvaluationContext {
            actor: instance.actor,
            targets: &instance.targets,
            payload: &instance.payload,
            action_def_id: def.id,
        },
        &mut sink,
        EffectMode::Authoritative,
    ) {
        Ok(_) => Ok(CommitOutcome::empty()),
        Err(discrepancy) => Err(sink.take_error(discrepancy)),
    }
}

fn commit_travel(
    def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    apply_travel_effect_schema(def, instance, event_log, txn)
}

fn abort_travel(
    _def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _reason: &AbortReason,
    event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    let (edge_id, origin, _, departure_tick, _) = travel_state(instance)?;
    txn.clear_component_in_transit_on_edge(instance.actor)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.set_ground_location(instance.actor, origin)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    for entity in direct_possessions(txn, instance.actor) {
        txn.set_ground_location(entity, origin)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    txn.add_tag(EventTag::Travel);
    let current_tick = txn.tick();
    if had_combat_during_travel(event_log, instance.actor, departure_tick, current_tick) {
        record_route_experience(txn, instance.actor, edge_id, current_tick, true)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::register_travel_actions;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        AgentBeliefStore, Blocker, BlockerMemory, BlockerScope, BlockingFact, CauseRef, Container,
        ControlSource, EdgeExperience, EventId, EventLog, EventPayload, EventView, EvidenceKind,
        ExplorationProfile, FrameState, GoalKey, GoalKind, HomeostaticNeedId, InTransitOnEdge,
        IntentionDomain, IntentionFrame, LoadUnits, MetabolismProfile, PendingEvent,
        PerceptionSource, Place, PreferenceProfile, Quantity, RouteExperience, Seed, Tick,
        Topology, TravelEdge, WitnessData, World, build_believed_entity_state,
    };
    use worldwake_sim::{
        ActionExecutionAuthority, ActionInstance, ActionInstanceId, DeterministicRng,
        PerAgentBeliefView, TickOutcome, abort_action, get_affordances, start_action, tick_action,
    };

    use super::*;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn travel_topology() -> Topology {
        let mut topology = Topology::new();
        for (slot, name) in [(1, "Square"), (2, "Gate"), (3, "Forest")] {
            topology
                .add_place(
                    entity(slot),
                    Place {
                        name: name.to_string(),
                        capacity: None,
                        tags: std::collections::BTreeSet::default(),
                    },
                )
                .unwrap();
        }
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(10), entity(1), entity(2), 3, None).unwrap())
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(11), entity(2), entity(1), 3, None).unwrap())
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(12), entity(2), entity(3), 2, None).unwrap())
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
        DeterministicRng::new(Seed([0x71; 32]))
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

    fn setup_world() -> (World, EntityId, EntityId, EntityId, EntityId, EntityId) {
        let mut world = World::new(travel_topology()).unwrap();
        let (actor, bag, bread) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let bag = txn
                .create_container(Container {
                    capacity: LoadUnits(20),
                    allowed_commodities: None,
                    allows_unique_items: true,
                    allows_nested_containers: true,
                })
                .unwrap();
            let bread = txn
                .create_item_lot(worldwake_core::CommodityKind::Bread, Quantity(2))
                .unwrap();
            txn.set_ground_location(actor, entity(1)).unwrap();
            txn.set_ground_location(bag, entity(1)).unwrap();
            txn.put_into_container(bread, bag).unwrap();
            txn.set_possessor(bag, actor).unwrap();
            commit_txn(txn);
            (actor, bag, bread)
        };
        (world, actor, bag, bread, entity(1), entity(2))
    }

    fn setup_registries() -> (ActionDefRegistry, ActionHandlerRegistry, ActionDefId) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let id = register_travel_actions(&mut defs, &mut handlers);
        (defs, handlers, id)
    }

    #[allow(clippy::too_many_arguments)]
    fn start_travel_action(
        world: &mut World,
        log: &mut EventLog,
        active_actions: &mut BTreeMap<ActionInstanceId, ActionInstance>,
        rng: &mut DeterministicRng,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        actor: EntityId,
        destination: EntityId,
    ) -> ActionInstanceId {
        let affordance = affordances_for(world, actor, defs, handlers)
            .into_iter()
            .find(|affordance| affordance.bound_targets == vec![destination])
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

    #[allow(clippy::too_many_arguments)]
    fn tick_travel_action(
        world: &mut World,
        log: &mut EventLog,
        active_actions: &mut BTreeMap<ActionInstanceId, ActionInstance>,
        rng: &mut DeterministicRng,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        instance_id: ActionInstanceId,
        tick: u64,
    ) -> TickOutcome {
        tick_action(
            instance_id,
            defs,
            handlers,
            ActionExecutionAuthority {
                active_actions,
                world,
                event_log: log,
                rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(tick)),
        )
        .unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    fn abort_travel_action(
        world: &mut World,
        log: &mut EventLog,
        active_actions: &mut BTreeMap<ActionInstanceId, ActionInstance>,
        rng: &mut DeterministicRng,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        instance_id: ActionInstanceId,
        tick: u64,
    ) {
        let _ = abort_action(
            instance_id,
            defs,
            handlers,
            ActionExecutionAuthority {
                active_actions,
                world,
                event_log: log,
                rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(tick)),
            worldwake_sim::ExternalAbortReason::Other,
        )
        .unwrap();
    }

    #[allow(clippy::too_many_arguments)]
    fn complete_travel_action(
        world: &mut World,
        log: &mut EventLog,
        active_actions: &mut BTreeMap<ActionInstanceId, ActionInstance>,
        rng: &mut DeterministicRng,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        instance_id: ActionInstanceId,
    ) -> TickOutcome {
        for tick in [6, 7] {
            assert_eq!(
                tick_travel_action(
                    world,
                    log,
                    active_actions,
                    rng,
                    defs,
                    handlers,
                    instance_id,
                    tick,
                ),
                TickOutcome::Continuing
            );
        }
        tick_travel_action(
            world,
            log,
            active_actions,
            rng,
            defs,
            handlers,
            instance_id,
            8,
        )
    }

    fn emit_combat_event(
        log: &mut EventLog,
        tick: u64,
        place: EntityId,
        actor: EntityId,
        target: EntityId,
    ) {
        let _ = log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(tick),
            cause: CauseRef::Bootstrap,
            actor_id: Some(actor),
            action_name: None,
            target_ids: vec![target],
            evidence: Vec::new(),
            place_id: Some(place),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData::default(),
            tags: BTreeSet::from([EventTag::Combat]),
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,
        }));
    }

    fn set_preference_profile(world: &mut World, actor: EntityId, profile: PreferenceProfile) {
        let mut txn = new_txn(world, 2);
        txn.set_component_preference_profile(actor, profile)
            .unwrap();
        commit_txn(txn);
    }

    fn set_route_experience(world: &mut World, actor: EntityId, route: RouteExperience) {
        let mut txn = new_txn(world, 2);
        txn.set_component_route_experience(actor, route).unwrap();
        commit_txn(txn);
    }

    fn set_intention_frame(world: &mut World, actor: EntityId, goal: GoalKind) {
        let mut txn = new_txn(world, 2);
        txn.set_component_intention_frame(
            actor,
            IntentionFrame {
                goal: GoalKey::from(goal),
                domain: IntentionDomain::Travel {
                    destination: entity(2),
                },
                assumptions: vec![],
                state: FrameState::Active,
                established_at: Tick(2),
                last_progress_tick: None,
                stalled_ticks: 0,
                patience_limit: 10,
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    fn set_exploration_profile(world: &mut World, actor: EntityId, profile: ExplorationProfile) {
        let mut txn = new_txn(world, 2);
        txn.set_component_exploration_profile(actor, profile)
            .unwrap();
        commit_txn(txn);
    }

    fn set_belief_store(world: &mut World, actor: EntityId, store: AgentBeliefStore) {
        let mut txn = new_txn(world, 2);
        txn.set_component_agent_belief_store(actor, store).unwrap();
        commit_txn(txn);
    }

    #[test]
    fn register_travel_actions_creates_single_generic_travel_def() {
        let (defs, _, id) = setup_registries();
        let def = defs.get(id).unwrap();

        assert_eq!(def.name, "travel");
        assert_eq!(def.targets, vec![TargetSpec::AdjacentPlace]);
        assert_eq!(
            def.actor_constraints,
            vec![
                Constraint::ActorAlive,
                Constraint::ActorHasControl,
                Constraint::ActorNotInTransit,
            ]
        );
        assert_eq!(
            def.duration,
            DurationExpr::TravelToTarget { target_index: 0 }
        );
        assert_eq!(def.effect_schema.steps, vec![EffectStep::CompleteTravel]);
    }

    #[test]
    fn travel_affordances_only_offer_adjacent_places() {
        let (world, actor, _, _, _, destination) = setup_world();
        let (defs, handlers, _) = setup_registries();
        let affordances = affordances_for(&world, actor, &defs, &handlers);

        assert_eq!(affordances.len(), 1);
        assert_eq!(affordances[0].bound_targets, vec![destination]);
    }

    #[test]
    fn travel_happy_path_moves_actor_and_possessions_through_transit() {
        let (mut world, actor, bag, bread, origin, destination) = setup_world();
        let (defs, handlers, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();
        let instance_id = start_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            destination,
        );

        assert_eq!(world.effective_place(actor), None);
        assert_eq!(world.effective_place(bag), None);
        assert_eq!(world.effective_place(bread), None);
        assert!(world.is_in_transit(actor));
        assert!(world.is_in_transit(bag));
        assert!(world.is_in_transit(bread));
        assert_eq!(
            world.get_component_in_transit_on_edge(actor),
            Some(&InTransitOnEdge {
                edge_id: TravelEdgeId(10),
                origin,
                destination,
                departure_tick: Tick(5),
                arrival_tick: Tick(8),
            })
        );
        let start_record = log
            .get(log.events_by_tag(EventTag::ActionStarted)[0])
            .unwrap();
        assert!(start_record.tags().contains(&EventTag::Travel));

        for tick in [6, 7] {
            let outcome = tick_travel_action(
                &mut world,
                &mut log,
                &mut active_actions,
                &mut rng,
                &defs,
                &handlers,
                instance_id,
                tick,
            );
            assert_eq!(outcome, TickOutcome::Continuing);
            assert_eq!(world.effective_place(actor), None);
        }

        let outcome = tick_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            8,
        );

        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        assert_eq!(world.effective_place(actor), Some(destination));
        assert_eq!(world.effective_place(bag), Some(destination));
        assert_eq!(world.effective_place(bread), Some(destination));
        assert!(!world.is_in_transit(actor));
        assert!(!world.is_in_transit(bag));
        assert!(!world.is_in_transit(bread));
        assert_eq!(world.get_component_in_transit_on_edge(actor), None);

        let commit_record = log
            .get(log.events_by_tag(EventTag::ActionCommitted)[0])
            .unwrap();
        assert!(commit_record.tags().contains(&EventTag::Travel));
    }

    #[test]
    fn safe_travel_commit_clears_matching_route_retraversal_blocker() {
        let (mut world, actor, _, _, origin, destination) = setup_world();
        let segment = RouteSegment::new(origin, destination);
        let retained_segment = RouteSegment::new(destination, entity(3));
        let mut memory = BlockerMemory::default();
        memory.record(Blocker {
            scope: BlockerScope::RouteSegment(segment),
            blocking_fact: BlockingFact::DangerTooHigh,
            diagnostic_context: None,
            observed_tick: Tick(4),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::RouteRetraversedSafely(segment),
            baseline_snapshot: None,
            source_event: EventId(1),
        });
        memory.record(Blocker {
            scope: BlockerScope::RouteSegment(retained_segment),
            blocking_fact: BlockingFact::DangerTooHigh,
            diagnostic_context: None,
            observed_tick: Tick(4),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::RouteRetraversedSafely(retained_segment),
            baseline_snapshot: None,
            source_event: EventId(2),
        });
        let mut txn = new_txn(&mut world, 4);
        txn.set_component_blocker_memory(actor, memory).unwrap();
        commit_txn(txn);

        let (defs, handlers, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();
        let instance_id = start_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            destination,
        );

        assert!(matches!(
            complete_travel_action(
                &mut world,
                &mut log,
                &mut active_actions,
                &mut rng,
                &defs,
                &handlers,
                instance_id,
            ),
            TickOutcome::Committed { .. }
        ));

        let memory = world.get_component_blocker_memory(actor).unwrap();
        assert!(
            !memory
                .intents
                .contains_key(&BlockerScope::RouteSegment(segment))
        );
        assert!(
            memory
                .intents
                .contains_key(&BlockerScope::RouteSegment(retained_segment))
        );
    }

    #[test]
    fn travel_fails_without_directed_edge() {
        let (mut world, actor, _, _, _, _) = setup_world();
        let (defs, handlers, travel_def) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let affordance = worldwake_sim::Affordance {
            def_id: travel_def,
            actor,
            bound_targets: vec![entity(3)],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };
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
            ActionError::PreconditionFailed("TargetAdjacentToActor(0)".to_string())
        );
    }

    #[test]
    fn travel_commit_emits_movement_trace_at_departure_place() {
        let (mut world, actor, bag, bread, origin, destination) = setup_world();
        let (defs, handlers, travel_def) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng();
        let affordance = worldwake_sim::Affordance {
            def_id: travel_def,
            actor,
            bound_targets: vec![destination],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };

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

        for tick in [6, 7, 8] {
            let _ = tick_travel_action(
                &mut world,
                &mut log,
                &mut active_actions,
                &mut rng,
                &defs,
                &handlers,
                instance_id,
                tick,
            );
        }

        let scene = world
            .get_component_scene_evidence(origin)
            .expect("travel should leave evidence at origin");
        assert!(scene.evidence.iter().any(|entry| {
            entry.kind
                == EvidenceKind::MovementTrace {
                    entity: actor,
                    departed_from: origin,
                    direction: destination,
                    observed_at: Tick(8),
                }
        }));
        assert_eq!(world.effective_place(actor), Some(destination));
        assert_eq!(world.effective_place(bag), Some(destination));
        assert_eq!(world.effective_place(bread), Some(destination));
    }

    #[test]
    fn explore_location_travel_pushes_synthetic_presentation_ticks() {
        let (mut world, actor, _, _, _, destination) = setup_world();
        let (defs, handlers, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();

        set_intention_frame(
            &mut world,
            actor,
            GoalKind::ExploreLocation {
                target_place: destination,
                motivating_need: worldwake_core::ExplorationMotivation::NeedDriven(
                    HomeostaticNeedId::Hunger,
                ),
                hypothesis: worldwake_core::HypothesisKind::MayContainCommodity {
                    commodity: worldwake_core::CommodityKind::Apple,
                },
            },
        );
        set_exploration_profile(
            &mut world,
            actor,
            ExplorationProfile {
                exploration_arrival_boost: Permille::new(500).unwrap(),
                ..ExplorationProfile::default()
            },
        );

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(
            destination,
            build_believed_entity_state(
                &world,
                destination,
                Tick(2),
                PerceptionSource::DirectObservation,
            )
            .unwrap(),
        );
        set_belief_store(&mut world, actor, beliefs);

        let instance_id = start_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            destination,
        );
        assert!(matches!(
            complete_travel_action(
                &mut world,
                &mut log,
                &mut active_actions,
                &mut rng,
                &defs,
                &handlers,
                instance_id,
            ),
            TickOutcome::Committed { .. }
        ));

        let belief = world
            .get_component_agent_belief_store(actor)
            .unwrap()
            .get_entity(&destination)
            .unwrap();
        assert_eq!(belief.presentation_tick_count, 5);
        assert_eq!(
            belief.presentation_ticks[..5],
            [Tick(2), Tick(8), Tick(8), Tick(8), Tick(8)]
        );
    }

    #[test]
    fn non_explore_travel_does_not_push_synthetic_presentation_ticks() {
        let (mut world, actor, _, _, _, destination) = setup_world();
        let (defs, handlers, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();

        set_intention_frame(&mut world, actor, GoalKind::Sleep);
        set_exploration_profile(
            &mut world,
            actor,
            ExplorationProfile {
                exploration_arrival_boost: Permille::new(500).unwrap(),
                ..ExplorationProfile::default()
            },
        );

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(
            destination,
            build_believed_entity_state(
                &world,
                destination,
                Tick(2),
                PerceptionSource::DirectObservation,
            )
            .unwrap(),
        );
        set_belief_store(&mut world, actor, beliefs);

        let instance_id = start_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            destination,
        );
        let _ = complete_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
        );

        let belief = world
            .get_component_agent_belief_store(actor)
            .unwrap()
            .get_entity(&destination)
            .unwrap();
        assert_eq!(belief.presentation_tick_count, 1);
        assert_eq!(belief.presentation_ticks[0], Tick(2));
    }

    #[test]
    fn zero_exploration_arrival_boost_is_no_op() {
        let (mut world, actor, _, _, _, destination) = setup_world();
        let (defs, handlers, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();

        set_intention_frame(
            &mut world,
            actor,
            GoalKind::ExploreLocation {
                target_place: destination,
                motivating_need: worldwake_core::ExplorationMotivation::NeedDriven(
                    HomeostaticNeedId::Hunger,
                ),
                hypothesis: worldwake_core::HypothesisKind::MayContainCommodity {
                    commodity: worldwake_core::CommodityKind::Apple,
                },
            },
        );
        set_exploration_profile(
            &mut world,
            actor,
            ExplorationProfile {
                exploration_arrival_boost: Permille::ZERO,
                ..ExplorationProfile::default()
            },
        );

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(
            destination,
            build_believed_entity_state(
                &world,
                destination,
                Tick(2),
                PerceptionSource::DirectObservation,
            )
            .unwrap(),
        );
        set_belief_store(&mut world, actor, beliefs);

        let instance_id = start_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            destination,
        );
        let _ = complete_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
        );

        let belief = world
            .get_component_agent_belief_store(actor)
            .unwrap()
            .get_entity(&destination)
            .unwrap();
        assert_eq!(belief.presentation_tick_count, 1);
        assert_eq!(belief.presentation_ticks[0], Tick(2));
    }

    #[test]
    fn explore_location_travel_seeds_destination_belief_before_applying_boost() {
        let (mut world, actor, _, _, _, destination) = setup_world();
        let (defs, handlers, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();

        set_intention_frame(
            &mut world,
            actor,
            GoalKind::ExploreLocation {
                target_place: destination,
                motivating_need: worldwake_core::ExplorationMotivation::NeedDriven(
                    HomeostaticNeedId::Hunger,
                ),
                hypothesis: worldwake_core::HypothesisKind::MayContainCommodity {
                    commodity: worldwake_core::CommodityKind::Apple,
                },
            },
        );
        set_exploration_profile(
            &mut world,
            actor,
            ExplorationProfile {
                exploration_arrival_boost: Permille::new(500).unwrap(),
                ..ExplorationProfile::default()
            },
        );
        set_belief_store(&mut world, actor, AgentBeliefStore::new());

        let instance_id = start_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            destination,
        );
        let _ = complete_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
        );

        let belief = world
            .get_component_agent_belief_store(actor)
            .unwrap()
            .get_entity(&destination)
            .unwrap();
        assert_eq!(belief.believed_kind, Some(EntityKind::Place));
        assert_eq!(belief.last_known_place, None);
        assert_eq!(belief.presentation_tick_count, 5);
        assert_eq!(
            belief.presentation_ticks[..5],
            [Tick(8), Tick(8), Tick(8), Tick(8), Tick(8)]
        );
    }

    #[test]
    fn travel_fails_if_actor_is_already_in_transit() {
        let (mut world, actor, _, _, _, destination) = setup_world();
        let (defs, handlers, travel_def) = setup_registries();
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_in_transit(actor).unwrap();
            txn.set_component_in_transit_on_edge(
                actor,
                InTransitOnEdge {
                    edge_id: TravelEdgeId(10),
                    origin: entity(1),
                    destination,
                    departure_tick: Tick(3),
                    arrival_tick: Tick(6),
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let affordance = worldwake_sim::Affordance {
            def_id: travel_def,
            actor,
            bound_targets: vec![destination],
            payload_override: None,
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };
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
            ActionError::ConstraintFailed("ActorNotInTransit".to_string())
        );
    }

    #[test]
    fn aborted_travel_returns_actor_and_possessions_to_origin() {
        let (mut world, actor, bag, bread, origin, destination) = setup_world();
        let (defs, handlers, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();
        let instance_id = start_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            destination,
        );

        abort_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            6,
        );

        assert_eq!(world.effective_place(actor), Some(origin));
        assert_eq!(world.effective_place(bag), Some(origin));
        assert_eq!(world.effective_place(bread), Some(origin));
        assert_eq!(world.get_component_in_transit_on_edge(actor), None);

        let record = log
            .get(log.events_by_tag(EventTag::ActionAborted)[0])
            .unwrap();
        assert!(record.tags().contains(&EventTag::Travel));
    }

    #[test]
    fn committed_safe_travel_creates_route_experience() {
        let (mut world, actor, _, _, _, destination) = setup_world();
        let (defs, handlers, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();
        let instance_id = start_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            destination,
        );

        assert_eq!(
            tick_travel_action(
                &mut world,
                &mut log,
                &mut active_actions,
                &mut rng,
                &defs,
                &handlers,
                instance_id,
                6,
            ),
            TickOutcome::Continuing
        );
        assert_eq!(
            tick_travel_action(
                &mut world,
                &mut log,
                &mut active_actions,
                &mut rng,
                &defs,
                &handlers,
                instance_id,
                7,
            ),
            TickOutcome::Continuing
        );
        assert!(matches!(
            tick_travel_action(
                &mut world,
                &mut log,
                &mut active_actions,
                &mut rng,
                &defs,
                &handlers,
                instance_id,
                8,
            ),
            TickOutcome::Committed { .. }
        ));

        assert_eq!(
            world
                .get_component_route_experience(actor)
                .unwrap()
                .edges
                .get(&TravelEdgeId(10)),
            Some(&EdgeExperience {
                safe_trips: 1,
                hostile_encounters: 0,
                last_travel_tick: Tick(8),
            })
        );
    }

    #[test]
    fn committed_hostile_travel_records_hostile_encounter_from_event_log() {
        let (mut world, actor, _, _, origin, destination) = setup_world();
        let opponent = entity(99);
        let (defs, handlers, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();
        let instance_id = start_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            destination,
        );

        let _ = tick_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            6,
        );
        emit_combat_event(&mut log, 6, origin, opponent, actor);
        let _ = tick_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            7,
        );
        assert!(matches!(
            tick_travel_action(
                &mut world,
                &mut log,
                &mut active_actions,
                &mut rng,
                &defs,
                &handlers,
                instance_id,
                8,
            ),
            TickOutcome::Committed { .. }
        ));

        assert_eq!(
            world
                .get_component_route_experience(actor)
                .unwrap()
                .edges
                .get(&TravelEdgeId(10)),
            Some(&EdgeExperience {
                safe_trips: 0,
                hostile_encounters: 1,
                last_travel_tick: Tick(8),
            })
        );
    }

    #[test]
    fn hostile_abort_travel_records_route_experience() {
        let (mut world, actor, _, _, origin, destination) = setup_world();
        let opponent = entity(99);
        let (defs, handlers, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();
        let instance_id = start_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            destination,
        );

        let _ = tick_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            6,
        );
        emit_combat_event(&mut log, 6, origin, opponent, actor);
        abort_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            7,
        );

        assert_eq!(
            world
                .get_component_route_experience(actor)
                .unwrap()
                .edges
                .get(&TravelEdgeId(10)),
            Some(&EdgeExperience {
                safe_trips: 0,
                hostile_encounters: 1,
                last_travel_tick: Tick(7),
            })
        );
    }

    #[test]
    fn non_combat_abort_does_not_record_route_experience() {
        let (mut world, actor, _, _, _, destination) = setup_world();
        let (defs, handlers, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();
        let instance_id = start_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            destination,
        );

        let _ = tick_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            6,
        );
        abort_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            7,
        );

        assert_eq!(world.get_component_route_experience(actor), None);
    }

    #[test]
    fn travel_recording_enforces_route_memory_capacity_after_update() {
        let (mut world, actor, _, _, _, destination) = setup_world();
        set_preference_profile(
            &mut world,
            actor,
            PreferenceProfile {
                route_caution_weight: pm(0),
                source_trust_weight: pm(0),
                route_memory_capacity: 1,
                source_memory_capacity: 1,
                memory_retention_ticks: 100,
                wait_sensitivity_weight: pm(150),
                capacity_observation_weight: pm(20),
            },
        );
        set_route_experience(
            &mut world,
            actor,
            RouteExperience {
                edges: BTreeMap::from([(
                    TravelEdgeId(11),
                    EdgeExperience {
                        safe_trips: 3,
                        hostile_encounters: 0,
                        last_travel_tick: Tick(1),
                    },
                )]),
            },
        );

        let (defs, handlers, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();
        let instance_id = start_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            destination,
        );

        let _ = tick_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            6,
        );
        let _ = tick_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            7,
        );
        let _ = tick_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            8,
        );

        let route = world.get_component_route_experience(actor).unwrap();
        assert_eq!(route.edges.len(), 1);
        assert_eq!(
            route.edges.get(&TravelEdgeId(10)),
            Some(&EdgeExperience {
                safe_trips: 1,
                hostile_encounters: 0,
                last_travel_tick: Tick(8),
            })
        );
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    #[test]
    fn start_travel_sets_body_cost_from_metabolism_profile() {
        let (mut world, actor, _, _, _, destination) = setup_world();
        // Set a MetabolismProfile with non-zero travel multipliers.
        // fatigue_rate=10, travel_fatigue_multiplier=500 → 10*500/1000 = 5
        // thirst_rate=20, travel_thirst_multiplier=300 → 20*300/1000 = 6
        // bladder_rate=15, travel_bladder_multiplier=200 → 15*200/1000 = 3
        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_component_metabolism_profile(
                actor,
                MetabolismProfile::new(
                    pm(1),   // hunger_rate
                    pm(20),  // thirst_rate
                    pm(10),  // fatigue_rate
                    pm(15),  // bladder_rate
                    pm(1),   // dirtiness_rate
                    pm(40),  // rest_efficiency
                    nz(10),  // starvation_tolerance_ticks
                    nz(10),  // dehydration_tolerance_ticks
                    nz(10),  // exhaustion_collapse_ticks
                    nz(10),  // bladder_accident_tolerance_ticks
                    nz(2),   // toilet_ticks
                    nz(3),   // wash_ticks
                    nz(8),   // min_sleep_ticks
                    pm(500), // travel_fatigue_multiplier
                    pm(300), // travel_thirst_multiplier
                    pm(200), // travel_bladder_multiplier
                    pm(0),   // wilderness_relief_dirtiness_penalty
                ),
            )
            .unwrap();
            commit_txn(txn);
        }
        let (defs, handlers, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();
        let instance_id = start_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            destination,
        );
        let instance = active_actions.get(&instance_id).unwrap();
        let override_cost = instance.body_cost_override.as_ref().unwrap();
        assert_eq!(override_cost.hunger_delta, pm(0));
        assert_eq!(override_cost.thirst_delta, pm(6));
        assert_eq!(override_cost.fatigue_delta, pm(5));
        assert_eq!(override_cost.bladder_delta, pm(3));
        assert_eq!(override_cost.dirtiness_delta, pm(0));
    }

    #[test]
    fn start_travel_zero_multipliers_produce_zero_cost() {
        let (mut world, actor, _, _, _, destination) = setup_world();
        // Set a MetabolismProfile with default (zero) travel multipliers.
        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_component_metabolism_profile(actor, MetabolismProfile::default())
                .unwrap();
            commit_txn(txn);
        }
        let (defs, handlers, _) = setup_registries();
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();
        let instance_id = start_travel_action(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            actor,
            destination,
        );
        let instance = active_actions.get(&instance_id).unwrap();
        let override_cost = instance.body_cost_override.as_ref().unwrap();
        assert_eq!(*override_cost, BodyCostPerTick::zero());
    }
}
