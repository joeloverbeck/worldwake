use std::collections::BTreeSet;

use worldwake_core::{
    ActionDefId, ActionDomain, BodyCostPerTick, EntityId, EntityKind, EventLog, EventTag,
    EvidenceKind, InTransitOnEdge, Tick, VisibilitySpec, World, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionAbortRequestReason, ActionDef, ActionDefRegistry, ActionError,
    ActionHandler, ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload,
    ActionProgress, ActionState, CommitOutcome, Constraint, DeterministicRng, DurationExpr,
    EscortToSafetyActionPayload, Interruptibility, PerAgentBeliefView, Precondition,
    RuntimeBeliefView, TargetSpec,
};

use crate::contention_support::ensure_care_contention_state;
use crate::evidence_support::emit_evidence;
use crate::facility_queue_actions::enqueue_for_contention;
use crate::travel_actions::{
    apply_travel_body_cost, direct_possessions, had_combat_during_travel, record_route_experience,
};

pub fn register_escort_to_safety_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    assert!(
        defs.iter().any(|def| def.name == "heal"),
        "escort_to_safety requires heal to be registered first"
    );

    let handler = handlers.register(
        ActionHandler::new(
            start_escort_to_safety,
            tick_escort_to_safety,
            commit_escort_to_safety,
            abort_escort_to_safety,
        )
        .with_affordance_payloads(enumerate_escort_payloads)
        .with_payload_override_validator(validate_escort_payload_override)
        .with_authoritative_payload_validator(validate_escort_payload_authoritatively),
    );
    let id = ActionDefId(defs.len() as u32);
    defs.register(escort_to_safety_action_def(id, handler))
}

fn escort_to_safety_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "escort_to_safety".to_string(),
        domain: ActionDomain::Care,
        actor_constraints: vec![
            Constraint::ActorAlive,
            Constraint::ActorHasControl,
            Constraint::ActorNotIncapacitated,
            Constraint::ActorNotInTransit,
        ],
        targets: vec![TargetSpec::EntityAtActorPlace {
            kind: EntityKind::Agent,
        }],
        preconditions: vec![
            Precondition::TargetExists(0),
            Precondition::TargetAtActorPlace(0),
            Precondition::TargetAlive(0),
            Precondition::TargetIsAgent(0),
            Precondition::TargetHasWounds(0),
        ],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::EscortRouteTravel,
        body_cost_per_tick: BodyCostPerTick::zero(),
        attention_cost: worldwake_core::Permille::new_unchecked(400),
        interruptibility: Interruptibility::InterruptibleWithPenalty,
        commit_conditions: vec![
            Precondition::TargetExists(0),
            Precondition::TargetAlive(0),
            Precondition::TargetIsAgent(0),
            Precondition::TargetHasWounds(0),
        ],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([
            EventTag::Travel,
            EventTag::WorldMutation,
            EventTag::Discovery,
        ]),
        payload: ActionPayload::None,
        handler,
        binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
        guard_template: None,
        expectation_template: vec![],
    }
}

fn escort_payload(payload: &ActionPayload) -> Result<&EscortToSafetyActionPayload, ActionError> {
    payload.as_escort_to_safety().ok_or_else(|| {
        ActionError::PreconditionFailed(
            "escort_to_safety action requires escort_to_safety payload".to_string(),
        )
    })
}

fn escort_payload_mut(
    payload: &mut ActionPayload,
) -> Result<&mut EscortToSafetyActionPayload, ActionError> {
    match payload {
        ActionPayload::EscortToSafety(payload) => Ok(payload),
        _ => Err(ActionError::PreconditionFailed(
            "escort_to_safety action requires escort_to_safety payload".to_string(),
        )),
    }
}

fn escort_state(
    instance: &ActionInstance,
) -> Result<(EntityId, EntityId, u16, Tick, Tick), ActionError> {
    match instance.local_state {
        Some(ActionState::Escort {
            subject,
            destination,
            leg_index,
            departure_tick,
            arrival_tick,
        }) => Ok((
            subject,
            destination,
            leg_index,
            departure_tick,
            arrival_tick,
        )),
        _ => Err(ActionError::InternalError(format!(
            "escort_to_safety action instance {} is missing escort state",
            instance.instance_id
        ))),
    }
}

fn reachable_destinations(view: &dyn RuntimeBeliefView, origin: EntityId) -> Vec<EntityId> {
    let mut seen = BTreeSet::from([origin]);
    let mut frontier = vec![origin];
    let mut index = 0;

    while index < frontier.len() {
        let place = frontier[index];
        index += 1;
        for adjacent in view.adjacent_places(place) {
            if seen.insert(adjacent) {
                frontier.push(adjacent);
            }
        }
    }

    let mut places = seen.into_iter().collect::<Vec<_>>();
    places.retain(|place| *place != origin);
    places
}

fn build_escort_payload(
    subject: EntityId,
    destination: EntityId,
    intended_heal_action: ActionDefId,
) -> ActionPayload {
    ActionPayload::EscortToSafety(EscortToSafetyActionPayload {
        subject,
        destination,
        intended_heal_action,
        route_places: Vec::new(),
        route_edges: Vec::new(),
    })
}

fn heal_action_id(action_defs: &ActionDefRegistry) -> Result<ActionDefId, ActionError> {
    action_defs
        .iter()
        .find(|def| def.name == "heal")
        .map(|def| def.id)
        .ok_or_else(|| {
            ActionError::InternalError(
                "escort_to_safety requires registered heal action".to_string(),
            )
        })
}

fn enumerate_escort_payloads(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    view: &dyn RuntimeBeliefView,
) -> Vec<ActionPayload> {
    let Some(subject) = targets.first().copied() else {
        return Vec::new();
    };
    if subject == actor || !view.has_wounds(subject) || view.is_dead(subject) {
        return Vec::new();
    }
    let Some(origin) = view.effective_place(actor) else {
        return Vec::new();
    };

    reachable_destinations(view, origin)
        .into_iter()
        .filter(|destination| view.route_exists(origin, *destination))
        .map(|destination| build_escort_payload(subject, destination, ActionDefId(u32::MAX)))
        .collect()
}

fn validate_escort_payload(
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(subject) = targets.first().copied() else {
        return false;
    };
    let Some(origin) = view.effective_place(actor) else {
        return false;
    };
    let Some(payload) = payload.as_escort_to_safety() else {
        return false;
    };
    if payload.subject != subject || payload.subject == actor || payload.destination == origin {
        return false;
    }
    view.has_wounds(subject)
        && !view.is_dead(subject)
        && view.route_exists(origin, payload.destination)
}

fn validate_escort_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn RuntimeBeliefView,
) -> bool {
    validate_escort_payload(actor, targets, payload, view)
}

fn validate_escort_payload_authoritatively(
    _def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let view = PerAgentBeliefView::from_world(actor, world);
    if !validate_escort_payload(actor, targets, payload, &view) {
        let subject = targets.first().copied().unwrap_or(actor);
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} cannot escort subject {subject} with payload {payload:?}"
        )));
    }
    Ok(())
}

fn moving_entities(txn: &WorldTxn<'_>, actor: EntityId, subject: EntityId) -> Vec<EntityId> {
    let mut entities = vec![actor, subject];
    entities.extend(direct_possessions(txn, actor));
    entities.extend(direct_possessions(txn, subject));
    entities.sort();
    entities.dedup();
    entities
}

fn stage_leg_departure(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    subject: EntityId,
    payload: &EscortToSafetyActionPayload,
    leg_index: usize,
    departure_tick: Tick,
) -> Result<Tick, ActionError> {
    let edge_id = *payload
        .route_edges
        .get(leg_index)
        .ok_or_else(|| ActionError::InternalError("escort route missing edge".to_string()))?;
    let origin = *payload.route_places.get(leg_index).ok_or_else(|| {
        ActionError::InternalError("escort route missing origin place".to_string())
    })?;
    let destination = *payload.route_places.get(leg_index + 1).ok_or_else(|| {
        ActionError::InternalError("escort route missing destination place".to_string())
    })?;
    let edge = txn.topology().edge(edge_id).ok_or_else(|| {
        ActionError::InternalError(format!("escort route edge {edge_id:?} missing"))
    })?;
    let arrival_tick = Tick(
        departure_tick
            .0
            .checked_add(u64::from(edge.travel_time_ticks()))
            .ok_or_else(|| {
                ActionError::InternalError("escort arrival tick overflowed".to_string())
            })?,
    );

    for entity in moving_entities(txn, actor, subject) {
        txn.set_in_transit(entity)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    for entity in [actor, subject] {
        txn.set_component_in_transit_on_edge(
            entity,
            InTransitOnEdge {
                edge_id,
                origin,
                destination,
                departure_tick,
                arrival_tick,
            },
        )
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }

    Ok(arrival_tick)
}

fn settle_leg_arrival(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    subject: EntityId,
    payload: &EscortToSafetyActionPayload,
    leg_index: usize,
) -> Result<(), ActionError> {
    let origin = *payload.route_places.get(leg_index).ok_or_else(|| {
        ActionError::InternalError("escort route missing origin place".to_string())
    })?;
    let destination = *payload.route_places.get(leg_index + 1).ok_or_else(|| {
        ActionError::InternalError("escort route missing destination place".to_string())
    })?;

    for entity in [actor, subject] {
        txn.clear_component_in_transit_on_edge(entity)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    for entity in moving_entities(txn, actor, subject) {
        txn.set_ground_location(entity, destination)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }

    let current_tick = txn.tick();
    emit_evidence(
        txn,
        origin,
        EvidenceKind::MovementTrace {
            entity: actor,
            departed_from: origin,
            direction: destination,
            observed_at: current_tick,
        },
        30,
    )
    .map_err(|err| ActionError::InternalError(err.to_string()))?;
    Ok(())
}

fn start_escort_to_safety(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let origin = txn.effective_place(instance.actor).ok_or_else(|| {
        ActionError::AbortRequested(ActionAbortRequestReason::ActorNotPlaced {
            actor: instance.actor,
        })
    })?;
    let (subject, destination) = {
        let payload = escort_payload_mut(&mut instance.payload)?;
        (payload.subject, payload.destination)
    };
    if subject == instance.actor {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {} cannot escort self",
            instance.actor
        )));
    }
    let route = txn
        .topology()
        .shortest_path(origin, destination)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "no route connects {origin} -> {destination} for escort",
            ))
        })?;
    if route.edges.is_empty() {
        return Err(ActionError::PreconditionFailed(format!(
            "escort destination {destination} must differ from actor place {origin}",
        )));
    }
    {
        let payload = escort_payload_mut(&mut instance.payload)?;
        payload.intended_heal_action = heal_action_id(context.action_defs)?;
        payload.route_places.clone_from(&route.places);
        payload.route_edges.clone_from(&route.edges);
    }

    let departure_tick = instance.start_tick;
    let arrival_tick = {
        let payload = escort_payload(&instance.payload)?;
        stage_leg_departure(txn, instance.actor, subject, payload, 0, departure_tick)?
    };
    txn.add_tag(EventTag::Travel);
    apply_travel_body_cost(instance, txn);

    Ok(Some(ActionState::Escort {
        subject,
        destination,
        leg_index: 0,
        departure_tick,
        arrival_tick,
    }))
}

fn tick_escort_to_safety(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    let payload = escort_payload(&instance.payload)?;
    let (subject, _destination, leg_index, _departure_tick, arrival_tick) = escort_state(instance)?;
    if txn.get_component_dead_at(subject).is_some() {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetNotAlive { target: subject },
        ));
    }
    if txn.tick().0 < arrival_tick.0 {
        return Ok(ActionProgress::Continue);
    }

    let leg_index = usize::from(leg_index);
    if leg_index + 1 >= payload.route_edges.len() {
        return Ok(ActionProgress::Continue);
    }

    settle_leg_arrival(txn, instance.actor, subject, payload, leg_index)?;
    let next_leg_index = leg_index + 1;
    let departure_tick = txn.tick();
    let arrival_tick = stage_leg_departure(
        txn,
        instance.actor,
        subject,
        payload,
        next_leg_index,
        departure_tick,
    )?;
    instance.local_state = Some(ActionState::Escort {
        subject,
        destination: payload.destination,
        leg_index: next_leg_index as u16,
        departure_tick,
        arrival_tick,
    });
    txn.add_tag(EventTag::Travel);

    Ok(ActionProgress::Continue)
}

fn commit_escort_to_safety(
    _def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    event_log: &EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = escort_payload(&instance.payload)?;
    let (subject, _destination, leg_index, departure_tick, _arrival_tick) = escort_state(instance)?;
    settle_leg_arrival(
        txn,
        instance.actor,
        subject,
        payload,
        usize::from(leg_index),
    )?;
    let edge_id = *payload
        .route_edges
        .get(usize::from(leg_index))
        .ok_or_else(|| ActionError::InternalError("escort route missing final edge".to_string()))?;
    let hostile = had_combat_during_travel(event_log, instance.actor, departure_tick, txn.tick());
    record_route_experience(txn, instance.actor, edge_id, txn.tick(), hostile)?;
    ensure_care_contention_state(txn, subject).map_err(ActionError::InternalError)?;
    enqueue_for_contention(txn, instance.actor, subject, payload.intended_heal_action)?;
    Ok(CommitOutcome::empty())
}

fn abort_escort_to_safety(
    _def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _reason: &AbortReason,
    event_log: &EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    let payload = escort_payload(&instance.payload)?;
    let (subject, _destination, leg_index, departure_tick, _arrival_tick) = escort_state(instance)?;
    let leg_index = usize::from(leg_index);
    let origin = *payload.route_places.get(leg_index).ok_or_else(|| {
        ActionError::InternalError("escort route missing abort origin".to_string())
    })?;
    let edge_id = *payload
        .route_edges
        .get(leg_index)
        .ok_or_else(|| ActionError::InternalError("escort route missing abort edge".to_string()))?;

    for entity in [instance.actor, subject] {
        txn.clear_component_in_transit_on_edge(entity)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    for entity in moving_entities(txn, instance.actor, subject) {
        txn.set_ground_location(entity, origin)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }

    let current_tick = txn.tick();
    if had_combat_during_travel(event_log, instance.actor, departure_tick, current_tick) {
        record_route_experience(txn, instance.actor, edge_id, current_tick, true)?;
    }
    txn.add_tag(EventTag::Travel);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{build_escort_payload, register_escort_to_safety_action};
    use crate::register_heal_action;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        ActionDefId, ActionDomain, BodyPart, CauseRef, CombatProfile, CombatWeaponRef,
        ContentionStatus, ControlSource, EntityId, EventLog, PerceptionSource, Permille, Seed,
        Tick, Topology, TravelEdge, TravelEdgeId, VisibilitySpec, WitnessData, World, WorldTxn,
        Wound, WoundCause, WoundId, WoundList, build_believed_entity_state,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionError, ActionExecutionAuthority, ActionHandlerRegistry,
        ActionInstance, ActionInstanceId, ActionPayload, Affordance, DeterministicRng,
        PerAgentBeliefView, TickOutcome, abort_action, get_affordances, start_action, tick_action,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn escort_topology() -> Topology {
        let mut topology = Topology::new();
        for (slot, name) in [(1, "Origin"), (2, "Bridge"), (3, "Sanctuary")] {
            topology
                .add_place(
                    entity(slot),
                    worldwake_core::Place {
                        name: name.to_string(),
                        capacity: None,
                        tags: std::collections::BTreeSet::new(),
                    },
                )
                .unwrap();
        }
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(1), entity(1), entity(2), 2, None).unwrap())
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(2), entity(2), entity(3), 2, None).unwrap())
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(3), entity(2), entity(1), 2, None).unwrap())
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(4), entity(3), entity(2), 2, None).unwrap())
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
        DeterministicRng::new(Seed([0x51; 32]))
    }

    fn wound_subject(world: &mut World, subject: EntityId, tick: u64) {
        let mut txn = new_txn(world, tick);
        txn.set_component_combat_profile(
            subject,
            CombatProfile::new(
                pm(1000),
                pm(700),
                pm(600),
                pm(550),
                pm(75),
                pm(20),
                pm(15),
                pm(120),
                pm(30),
                nz(6),
                nz(10),
            ),
        )
        .unwrap();
        txn.set_component_wound_list(
            subject,
            WoundList {
                wounds: vec![Wound {
                    id: WoundId(1),
                    body_part: BodyPart::Torso,
                    cause: WoundCause::Combat {
                        attacker: subject,
                        weapon: CombatWeaponRef::Unarmed,
                    },
                    severity: pm(300),
                    inflicted_at: Tick(tick),
                    bleed_rate_per_tick: pm(50),
                }],
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    fn setup_world() -> (World, EntityId, EntityId, EntityId, EntityId, EntityId) {
        let mut world = World::new(escort_topology()).unwrap();
        let origin = entity(1);
        let waypoint = entity(2);
        let destination = entity(3);
        let actor;
        let subject;
        {
            let mut txn = new_txn(&mut world, 1);
            actor = txn.create_agent("Escorter", ControlSource::Ai).unwrap();
            subject = txn.create_agent("Patient", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, origin).unwrap();
            txn.set_ground_location(subject, origin).unwrap();
            commit_txn(txn);
        }
        wound_subject(&mut world, subject, 2);
        seed_subject_belief(&mut world, actor, subject, 3);
        (world, actor, subject, origin, waypoint, destination)
    }

    fn seed_subject_belief(world: &mut World, actor: EntityId, subject: EntityId, tick: u64) {
        let mut store = world
            .get_component_agent_belief_store(actor)
            .cloned()
            .unwrap_or_default();
        let state = build_believed_entity_state(
            world,
            subject,
            Tick(tick),
            PerceptionSource::DirectObservation,
        )
        .expect("subject belief state should be constructible");
        store.update_entity(subject, state);

        let mut txn = new_txn(world, tick);
        txn.set_component_agent_belief_store(actor, store).unwrap();
        commit_txn(txn);
    }

    fn setup_registries() -> (
        ActionDefRegistry,
        ActionHandlerRegistry,
        ActionDefId,
        ActionDefId,
    ) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let heal_id = register_heal_action(&mut defs, &mut handlers);
        let escort_id = register_escort_to_safety_action(&mut defs, &mut handlers);
        (defs, handlers, heal_id, escort_id)
    }

    fn escort_affordance(
        world: &World,
        actor: EntityId,
        subject: EntityId,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        destination: EntityId,
    ) -> Affordance {
        let view = PerAgentBeliefView::from_world(actor, world);
        get_affordances(&view, actor, defs, handlers)
            .into_iter()
            .find(|affordance| {
                affordance.bound_targets == vec![subject]
                    && affordance
                        .payload_override
                        .as_ref()
                        .and_then(ActionPayload::as_escort_to_safety)
                        .is_some_and(|payload| payload.destination == destination)
            })
            .expect("escort affordance should exist")
    }

    fn action_context(
        defs: &ActionDefRegistry,
        tick: u64,
    ) -> worldwake_sim::ActionExecutionContext<'_> {
        let base =
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(tick));
        worldwake_sim::ActionExecutionContext {
            cause: base.cause,
            tick: base.tick,
            recipe_registry: base.recipe_registry,
            action_defs: defs,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn start_escort(
        world: &mut World,
        log: &mut EventLog,
        active_actions: &mut BTreeMap<ActionInstanceId, ActionInstance>,
        rng: &mut DeterministicRng,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        affordance: &Affordance,
    ) -> ActionInstanceId {
        let mut next_instance_id = ActionInstanceId(1);
        start_action(
            affordance,
            defs,
            handlers,
            ActionExecutionAuthority {
                active_actions,
                world,
                event_log: log,
                rng,
            },
            &mut next_instance_id,
            action_context(defs, 1),
        )
        .unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    fn tick_escort(
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
            action_context(defs, tick),
        )
        .unwrap()
    }

    #[test]
    fn escort_affordance_enumerates_non_adjacent_reachable_destination() {
        let (world, actor, subject, _origin, _waypoint, destination) = setup_world();
        let (defs, handlers, _heal_id, _escort_id) = setup_registries();
        let view = PerAgentBeliefView::from_world(actor, &world);

        let affordances = get_affordances(&view, actor, &defs, &handlers);
        let affordance = affordances
            .iter()
            .find(|affordance| {
                affordance.bound_targets == vec![subject]
                    && affordance
                        .payload_override
                        .as_ref()
                        .and_then(ActionPayload::as_escort_to_safety)
                        .is_some_and(|payload| payload.destination == destination)
            })
            .expect("escort affordance for non-adjacent destination should exist");
        let payload = affordance
            .payload_override
            .as_ref()
            .and_then(ActionPayload::as_escort_to_safety)
            .unwrap();

        assert_eq!(affordance.bound_targets, vec![subject]);
        assert_eq!(payload.subject, subject);
        assert_eq!(payload.destination, destination);
    }

    #[test]
    fn escort_to_safety_advances_both_entities_to_second_leg_before_commit() {
        let (mut world, actor, subject, _origin, waypoint, destination) = setup_world();
        let (defs, handlers, _heal_id, _escort_id) = setup_registries();
        let affordance = escort_affordance(&world, actor, subject, &defs, &handlers, destination);
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();
        let instance_id = start_escort(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            &affordance,
        );

        assert!(matches!(
            tick_escort(
                &mut world,
                &mut log,
                &mut active_actions,
                &mut rng,
                &defs,
                &handlers,
                instance_id,
                2
            ),
            TickOutcome::Continuing
        ));
        assert!(matches!(
            tick_escort(
                &mut world,
                &mut log,
                &mut active_actions,
                &mut rng,
                &defs,
                &handlers,
                instance_id,
                3
            ),
            TickOutcome::Continuing
        ));

        assert_eq!(world.effective_place(actor), None);
        assert_eq!(world.effective_place(subject), None);
        let actor_transit = world
            .get_component_in_transit_on_edge(actor)
            .expect("actor should be in transit on second leg");
        let subject_transit = world
            .get_component_in_transit_on_edge(subject)
            .expect("subject should be in transit on second leg");
        assert_eq!(actor_transit.origin, waypoint);
        assert_eq!(actor_transit.destination, destination);
        assert_eq!(subject_transit.origin, waypoint);
        assert_eq!(subject_transit.destination, destination);
    }

    #[test]
    fn escort_to_safety_commit_moves_both_entities_and_queues_care_handoff() {
        let (mut world, actor, subject, _origin, _waypoint, destination) = setup_world();
        let (defs, handlers, heal_id, _escort_id) = setup_registries();
        let affordance = escort_affordance(&world, actor, subject, &defs, &handlers, destination);
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();
        let instance_id = start_escort(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            &affordance,
        );

        for tick in 2..=5 {
            let outcome = tick_escort(
                &mut world,
                &mut log,
                &mut active_actions,
                &mut rng,
                &defs,
                &handlers,
                instance_id,
                tick,
            );
            if tick < 5 {
                assert!(matches!(outcome, TickOutcome::Continuing));
            } else {
                assert!(matches!(outcome, TickOutcome::Committed { .. }));
            }
        }

        assert_eq!(world.effective_place(actor), Some(destination));
        assert_eq!(world.effective_place(subject), Some(destination));
        let queue = world
            .get_component_contention_queue(subject)
            .expect("subject should have care contention queue after escort handoff");
        assert_eq!(queue.waiting.len(), 1);
        let waiter = queue.waiting.values().next().unwrap();
        assert_eq!(waiter.actor, actor);
        assert_eq!(waiter.intended_action, heal_id);
    }

    #[test]
    fn escort_to_safety_abort_mid_route_returns_both_entities_to_last_waypoint() {
        let (mut world, actor, subject, _origin, waypoint, destination) = setup_world();
        let (defs, handlers, _heal_id, _escort_id) = setup_registries();
        let affordance = escort_affordance(&world, actor, subject, &defs, &handlers, destination);
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = test_rng();
        let instance_id = start_escort(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            &affordance,
        );

        let _ = tick_escort(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            2,
        );
        let _ = tick_escort(
            &mut world,
            &mut log,
            &mut active_actions,
            &mut rng,
            &defs,
            &handlers,
            instance_id,
            3,
        );

        let _ = abort_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            action_context(&defs, 4),
            worldwake_sim::ExternalAbortReason::Other,
        )
        .unwrap();

        assert_eq!(world.effective_place(actor), Some(waypoint));
        assert_eq!(world.effective_place(subject), Some(waypoint));
    }

    #[test]
    fn escort_to_safety_rejects_current_place_as_destination() {
        let (mut world, actor, subject, origin, _waypoint, _destination) = setup_world();
        let (defs, handlers, heal_id, escort_id) = setup_registries();
        let affordance = Affordance {
            def_id: escort_id,
            actor,
            bound_targets: vec![subject],
            payload_override: Some(build_escort_payload(subject, origin, heal_id)),
            explanation: None,
            contention_status: ContentionStatus::Available,
        };
        let mut active_actions = BTreeMap::new();
        let mut log = EventLog::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng();

        let error = start_action(
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
            action_context(&defs, 1),
        )
        .unwrap_err();

        assert!(matches!(error, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn escort_to_safety_registry_registration_is_complete() {
        let (defs, handlers, _heal_id, escort_id) = setup_registries();
        let def = defs.get(escort_id).unwrap();
        assert_eq!(def.name, "escort_to_safety");
        assert_eq!(def.domain, ActionDomain::Care);
        assert!(handlers.get(def.handler).is_some());
    }
}
