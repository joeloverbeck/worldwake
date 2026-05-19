//! Golden tests for S38 learned route preferences.

use crate::golden_harness::*;
use std::collections::{BTreeMap, BTreeSet};
use worldwake_ai::{CommodityPurpose, DecisionOutcome, PlannerOpKind, SelectedPlanSource};
use worldwake_core::{
    AcquisitionQuantity, AgentData, BeliefConfidencePolicy, CauseRef, CommodityKind, ControlSource,
    EdgeExperience, EntityId, EventLog, EventPayload, EventTag, GoalKey, GoalKind,
    HomeostaticNeeds, PendingEvent, PerceptionProfile, PerceptionSource, Place, PlaceTag,
    PreferenceProfile, ProductionOutputOwner, Quantity, ResourceSource, RouteExperience, Seed,
    Tick, Topology, TravelEdge, TravelEdgeId, UtilityProfile, VisibilitySpec, WitnessData,
    WorkstationTag, World, hash_event_log, hash_world,
};
use worldwake_sim::{
    ActionExecutionContext, ActionRequestMode, InputKind, InterruptReason, RequestProvenance,
    Scheduler, SchedulerActionRuntime, SystemManifest,
};

const START_MARKET: EntityId = entity(500);
const DANGEROUS_ROAD: EntityId = entity(501);
const SAFE_ROUTE: EntityId = entity(502);
const ORCHARD_FARM: EntityId = entity(503);
const DANGEROUS_EDGE_FORWARD: TravelEdgeId = TravelEdgeId(700);

const fn entity(slot: u32) -> EntityId {
    EntityId {
        slot,
        generation: 0,
    }
}

fn place(name: &str, tags: &[PlaceTag]) -> Place {
    Place {
        name: name.to_string(),
        capacity: None,
        tags: tags.iter().copied().collect(),
    }
}

fn connect(topology: &mut Topology, base_id: u32, from: EntityId, to: EntityId, ticks: u32) {
    topology
        .add_edge(TravelEdge::new(TravelEdgeId(base_id), from, to, ticks, None).unwrap())
        .unwrap();
    topology
        .add_edge(TravelEdge::new(TravelEdgeId(base_id + 1), to, from, ticks, None).unwrap())
        .unwrap();
}

fn build_s38_topology() -> Topology {
    let mut topology = Topology::new();
    topology
        .add_place(
            START_MARKET,
            place("S38 Market", &[PlaceTag::Village, PlaceTag::Store]),
        )
        .unwrap();
    topology
        .add_place(
            DANGEROUS_ROAD,
            place("S38 Dangerous Road", &[PlaceTag::Road, PlaceTag::Forest]),
        )
        .unwrap();
    topology
        .add_place(
            SAFE_ROUTE,
            place("S38 Safe Route", &[PlaceTag::Road, PlaceTag::Field]),
        )
        .unwrap();
    topology
        .add_place(
            ORCHARD_FARM,
            place("S38 Orchard", &[PlaceTag::Farm, PlaceTag::Field]),
        )
        .unwrap();

    connect(&mut topology, 700, START_MARKET, DANGEROUS_ROAD, 3);
    connect(&mut topology, 710, DANGEROUS_ROAD, ORCHARD_FARM, 1);
    connect(&mut topology, 720, START_MARKET, SAFE_ROUTE, 2);
    connect(&mut topology, 730, SAFE_ROUTE, ORCHARD_FARM, 3);
    topology
}

fn build_harness_with_topology(seed: Seed, topology: Topology) -> GoldenHarness {
    let mut h = GoldenHarness::new(seed);
    h.world = World::new(topology).unwrap();
    h.event_log = EventLog::new();
    h.scheduler = Scheduler::new(SystemManifest::canonical());
    h.controller = worldwake_sim::ControllerState::new();
    h
}

fn default_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        entity_activation_threshold: pm(64),
        claim_confidence_threshold: pm(50),
        observation_buffer_capacity: 64,
        observation_budget: 24,
        salience_policy: worldwake_core::SaliencePolicy::default(),
        omission_log_capacity: worldwake_core::default_omission_log_capacity(),
        opportunity_floor_permille: worldwake_core::default_opportunity_floor_permille(),
        need_salience_boost: pm(500),
        need_salience_urgency_threshold: pm(500),
        observation_fidelity: pm(1000),
        confidence_policy: BeliefConfidencePolicy::default(),
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
    }
}

fn route_profile(route_caution_weight: u16) -> PreferenceProfile {
    PreferenceProfile {
        route_caution_weight: pm(route_caution_weight),
        source_trust_weight: pm(0),
        route_memory_capacity: 8,
        source_memory_capacity: 8,
        memory_retention_ticks: 240,
        wait_sensitivity_weight: pm(150),
        capacity_observation_weight: pm(20),
    }
}

fn hungry_agent(
    h: &mut GoldenHarness,
    name: &str,
    control_source: ControlSource,
    route_caution_weight: u16,
) -> EntityId {
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        name,
        START_MARKET,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(h, agent, control_source, 0);
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        default_perception_profile(),
    );
    set_preference_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        route_profile(route_caution_weight),
    );
    agent
}

fn set_control_source(
    h: &mut GoldenHarness,
    agent: EntityId,
    control_source: ControlSource,
    tick: u64,
) {
    let mut txn = new_txn(&mut h.world, tick);
    txn.set_component_agent_data(agent, AgentData { control_source })
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn set_preference_profile(
    world: &mut World,
    event_log: &mut EventLog,
    agent: EntityId,
    profile: PreferenceProfile,
) {
    let mut txn = new_txn(world, 0);
    txn.set_component_preference_profile(agent, profile)
        .unwrap();
    commit_txn(txn, event_log);
}

fn set_route_experience(
    world: &mut World,
    event_log: &mut EventLog,
    agent: EntityId,
    route_experience: RouteExperience,
) {
    let mut txn = new_txn(world, 0);
    txn.set_component_route_experience(agent, route_experience)
        .unwrap();
    commit_txn(txn, event_log);
}

fn relocate_agent(
    world: &mut World,
    event_log: &mut EventLog,
    agent: EntityId,
    destination: EntityId,
    tick: Tick,
) {
    let mut txn = new_txn(world, tick.0);
    txn.set_ground_location(agent, destination).unwrap();
    commit_txn(txn, event_log);
}

fn place_orchard_source(h: &mut GoldenHarness) -> EntityId {
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(12),
            max_quantity: Quantity(12),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
        },
        ProductionOutputOwner::Actor,
    )
}

fn request_travel(h: &mut GoldenHarness, actor: EntityId, destination: EntityId) {
    let travel_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "travel")
        .map(|def| def.id)
        .expect("full registries should include travel");
    let tick = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        tick,
        InputKind::RequestAction {
            actor,
            def_id: travel_def_id,
            targets: vec![destination],
            payload_override: None,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
}

fn emit_combat_event(
    log: &mut EventLog,
    tick: Tick,
    place: EntityId,
    actor: EntityId,
    target: EntityId,
) {
    let _ = log.emit(PendingEvent::from_payload(EventPayload {
        tick,
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

fn interrupt_active_travel(h: &mut GoldenHarness, actor: EntityId) {
    let instance_id = h
        .scheduler
        .active_actions()
        .iter()
        .find_map(|(id, instance)| (instance.actor == actor).then_some(*id))
        .expect("actor should have an active travel action to interrupt");
    let tick = h.scheduler.current_tick();
    let _ = h
        .scheduler
        .interrupt_active_action(
            instance_id,
            SchedulerActionRuntime {
                action_defs: &h.defs,
                action_handlers: &h.handlers,
                world: &mut h.world,
                event_log: &mut h.event_log,
                rng: &mut h.rng,
            },
            ActionExecutionContext::without_recipes(CauseRef::Bootstrap, tick),
            InterruptReason::DangerNearby,
        )
        .expect("interrupting a live travel action should succeed");
}

fn latest_selected_apple_travel_destination(
    h: &GoldenHarness,
    agent: EntityId,
) -> Option<EntityId> {
    h.driver
        .trace_sink()?
        .traces_for(agent)
        .into_iter()
        .rev()
        .find_map(|trace| {
            let DecisionOutcome::Planning(planning) = &trace.outcome else {
                return None;
            };
            if planning.selection.selected_plan_source != Some(SelectedPlanSource::SearchSelection)
            {
                return None;
            }
            if !planning
                .selection
                .selected_goal_is(GoalKey::from(GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Apple,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                }))
            {
                return None;
            }
            planning
                .selection
                .selected_plan
                .as_ref()?
                .steps
                .iter()
                .find(|step| step.op_kind == PlannerOpKind::Travel)
                .and_then(|step| step.targets.first().copied())
        })
}

fn run_until(limit: usize, mut step: impl FnMut() -> bool) -> bool {
    for _ in 0..limit {
        if step() {
            return true;
        }
    }
    false
}

fn plan_destination_snapshot(
    seed: Seed,
    route_caution_weight: u16,
    route_experience: Option<RouteExperience>,
) -> Option<EntityId> {
    let mut h = build_harness_with_topology(seed, build_s38_topology());
    let _orchard = place_orchard_source(&mut h);
    let agent = hungry_agent(&mut h, "Planner", ControlSource::Ai, route_caution_weight);
    if let Some(route_experience) = route_experience {
        set_route_experience(&mut h.world, &mut h.event_log, agent, route_experience);
    }
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        PerceptionSource::Inference,
    );

    h.driver.enable_tracing();
    h.step_once();
    latest_selected_apple_travel_destination(&h, agent)
}

fn trace_summaries(h: &GoldenHarness, agent: EntityId) -> Vec<String> {
    h.driver
        .trace_sink()
        .expect("decision tracing should stay enabled")
        .traces_for(agent)
        .into_iter()
        .map(|trace| format!("{:?}: {}", trace.tick, trace.outcome.summary()))
        .collect()
}

fn run_hostile_route_learning_scenario(
    seed: Seed,
    abort_after_combat: bool,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let baseline_destination =
        plan_destination_snapshot(Seed([seed.0[0].wrapping_add(1); 32]), 1000, None);
    assert_eq!(
        baseline_destination,
        Some(DANGEROUS_ROAD),
        "without personal hostile route experience, the shorter road should stay attractive",
    );

    let mut h = build_harness_with_topology(seed, build_s38_topology());
    let _orchard = place_orchard_source(&mut h);
    let agent = hungry_agent(&mut h, "Traveler", ControlSource::Human, 1000);

    request_travel(&mut h, agent, DANGEROUS_ROAD);
    let entered_transit = run_until(6, || {
        h.step_once();
        h.world.is_in_transit(agent)
    });
    assert!(
        entered_transit,
        "manual travel request should enter real in-transit state before route learning",
    );

    let combat_tick = h.scheduler.current_tick();
    emit_combat_event(
        &mut h.event_log,
        combat_tick,
        START_MARKET,
        entity(999),
        agent,
    );

    if abort_after_combat {
        h.step_once();
        assert!(
            h.world.is_in_transit(agent),
            "travel should still be active one tick after the combat event so the hostile abort path stays real",
        );
        interrupt_active_travel(&mut h, agent);
        assert_eq!(
            h.world.effective_place(agent),
            Some(START_MARKET),
            "combat interruption should return the traveler to the origin place",
        );
    } else {
        let reached_road = run_until(8, || {
            h.step_once();
            h.world.effective_place(agent) == Some(DANGEROUS_ROAD) && !h.world.is_in_transit(agent)
        });
        assert!(
            reached_road,
            "hostile travel should still be able to commit through the ordinary travel lifecycle",
        );
        relocate_agent(
            &mut h.world,
            &mut h.event_log,
            agent,
            START_MARKET,
            h.scheduler.current_tick(),
        );
    }

    let learned_edge = h
        .world
        .get_component_route_experience(agent)
        .and_then(|route| route.edges.get(&DANGEROUS_EDGE_FORWARD))
        .copied()
        .expect("hostile travel should leave route experience for the dangerous edge");
    assert_eq!(
        learned_edge.safe_trips, 0,
        "hostile travel should not also count as a safe trip on the same edge",
    );
    assert_eq!(
        learned_edge.hostile_encounters, 1,
        "hostile travel should record exactly one hostile encounter on the dangerous edge",
    );
    assert!(
        learned_edge.last_travel_tick.0 >= combat_tick.0,
        "the recorded route-memory tick should be at or after the hostile travel window",
    );

    let ai_tick = h.scheduler.current_tick().0;
    set_control_source(&mut h, agent, ControlSource::Ai, ai_tick);
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        h.scheduler.current_tick(),
        PerceptionSource::Inference,
    );

    h.driver.enable_tracing();
    h.step_once();
    let selected_destination = latest_selected_apple_travel_destination(&h, agent);
    let summaries = trace_summaries(&h, agent);
    assert_eq!(
        selected_destination,
        Some(SAFE_ROUTE),
        "after learning the dangerous edge personally, the next food journey should begin via the safer route; traces={summaries:?}",
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

fn run_preference_diversity_scenario(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = build_harness_with_topology(seed, build_s38_topology());
    let _orchard = place_orchard_source(&mut h);
    let cautious = hungry_agent(&mut h, "Cautious", ControlSource::Ai, 800);
    let bold = hungry_agent(&mut h, "Bold", ControlSource::Ai, 100);

    let hostile_history = RouteExperience {
        edges: BTreeMap::from([(
            DANGEROUS_EDGE_FORWARD,
            EdgeExperience {
                safe_trips: 0,
                hostile_encounters: 1,
                last_travel_tick: Tick(0),
            },
        )]),
    };
    set_route_experience(
        &mut h.world,
        &mut h.event_log,
        cautious,
        hostile_history.clone(),
    );
    set_route_experience(&mut h.world, &mut h.event_log, bold, hostile_history);
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        cautious,
        Tick(0),
        PerceptionSource::Inference,
    );
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        bold,
        Tick(0),
        PerceptionSource::Inference,
    );

    h.driver.enable_tracing();
    h.step_once();

    let cautious_destination = latest_selected_apple_travel_destination(&h, cautious);
    let bold_destination = latest_selected_apple_travel_destination(&h, bold);
    assert_eq!(
        cautious_destination,
        Some(SAFE_ROUTE),
        "high route caution should make the longer safe route worth taking after a hostile memory",
    );
    assert_eq!(
        bold_destination,
        Some(DANGEROUS_ROAD),
        "low route caution should keep the shorter dangerous route attractive on the same topology",
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// Scenario 91: Hostile Completed Travel Reweights the Next Route Choice
// ---------------------------------------------------------------------------
//
// Systems: Travel, learned route experience, belief view, AI planning
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Production
// Places: S38 Market, S38 Dangerous Road, S38 Safe Route, S38 Orchard
//
// Setup: A hungry traveler first traverses the shorter road under human control.
// A combat event occurs during that travel leg, the travel still commits, and
// the actor is then returned to the market before the next AI-planned food trip.
//
// Proves:
//   1. Hostile completed travel records `RouteExperience`.
//   2. The recorded hostile edge is read back through the belief-facing travel
//      cost surface.
//   3. The next apple-acquisition plan flips from the short road to the longer
//      safe route.

#[test]
fn golden_hostile_completed_travel_flips_next_route_choice() {
    let _ = run_hostile_route_learning_scenario(Seed([91; 32]), false);
}

// Scenario 92: Combat-Aborted Travel Still Creates Hostile Route Memory
// ---------------------------------------------------------------------------
//
// Systems: Travel, interrupt/abort, learned route experience, AI planning
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Production
// Places: S38 Market, S38 Dangerous Road, S38 Safe Route, S38 Orchard
//
// Setup: Same topology and motive, but the dangerous-road travel is interrupted
// after a combat event instead of committing.
//
// Proves:
//   1. Combat-aborted travel still records hostile route experience.
//   2. Failure leaves durable preference aftermath.
//   3. The next planned apple journey still flips to the safer route.

#[test]
fn golden_combat_aborted_travel_flips_next_route_choice() {
    let _ = run_hostile_route_learning_scenario(Seed([92; 32]), true);
}

// Scenario 93: Preference Profiles Create Route Diversity From the Same Memory
// ---------------------------------------------------------------------------
//
// Systems: learned route experience, belief view, AI planning
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Production
// Places: S38 Market, S38 Dangerous Road, S38 Safe Route, S38 Orchard
//
// Setup: Two equally hungry agents share the same topology, source beliefs, and
// hostile dangerous-road memory, but have different `route_caution_weight`.
//
// Proves:
//   1. The same recorded route history can lead to different travel choices.
//   2. `PreferenceProfile` acts as a tie-breaking cost influence rather than a
//      suppression gate.

#[test]
fn golden_preference_profile_diversifies_route_selection() {
    let _ = run_preference_diversity_scenario(Seed([93; 32]));
}
