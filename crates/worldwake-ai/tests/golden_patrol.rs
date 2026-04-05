//! Golden tests for patrol lifecycle, belief-driven motive, and route adaptation.

mod golden_harness;

use golden_harness::*;
use worldwake_ai::{
    DecisionOutcome, GoalKey, OpportunityAnchor, OpportunityKey, PlanningPipelineTrace,
};
use worldwake_core::{
    AgentBeliefStore, BeliefConfidencePolicy, CommodityKind, GoalKind, HomeostaticNeeds,
    InstitutionalKnowledgeSource, PatrolProfile, PatrolRoute, PerceptionProfile, PerceptionSource,
    Permille, PrototypePlace, Quantity, RecordedViolation, Seed, SocialObservation,
    SocialObservationDetail, TheftFacts, Tick, UtilityProfile, ViolationDispositionProfile,
    ViolationId, ViolationKind, ViolationMemory, prototype_place_entity,
};
use worldwake_sim::{ActionTraceDetail, ActionTraceKind};

fn default_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        entity_memory_capacity: 32,
        entity_claim_capacity: 32,
        memory_retention_ticks: 240,
        observation_fidelity: pm(1000),
        confidence_policy: BeliefConfidencePolicy::default(),
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
    }
}

fn patrol_profile(base_dwell_ticks: u32, vigilance: u16, motive: u16) -> PatrolProfile {
    PatrolProfile {
        base_dwell_ticks,
        dwell_vigilance_scale_ticks: base_dwell_ticks,
        vigilance: Permille::new(vigilance).unwrap(),
        route_adaptation_sensitivity: pm(1000),
        patrol_motive_weight: pm(motive),
    }
}

fn set_patrol_state(
    h: &mut GoldenHarness,
    agent: worldwake_core::EntityId,
    route: PatrolRoute,
    profile: PatrolProfile,
    tick: u64,
) {
    let mut txn = new_txn(&mut h.world, tick);
    txn.set_component_patrol_route(agent, route).unwrap();
    txn.set_component_patrol_profile(agent, profile).unwrap();
    txn.set_component_agent_belief_store(agent, AgentBeliefStore::default())
        .unwrap();
    txn.set_component_violation_memory(agent, ViolationMemory::default())
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn set_violation_profile(
    h: &mut GoldenHarness,
    agent: worldwake_core::EntityId,
    profile: ViolationDispositionProfile,
    tick: u64,
) {
    let mut txn = new_txn(&mut h.world, tick);
    txn.set_component_violation_disposition_profile(agent, profile)
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn set_hunger(h: &mut GoldenHarness, agent: worldwake_core::EntityId, hunger: u16, tick: u64) {
    let mut needs = h
        .world
        .get_component_homeostatic_needs(agent)
        .copied()
        .expect("agent should have needs");
    needs.hunger = pm(hunger);
    let mut txn = new_txn(&mut h.world, tick);
    txn.set_component_homeostatic_needs(agent, needs).unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn theft(place: worldwake_core::EntityId) -> TheftFacts {
    TheftFacts {
        missing_entity: worldwake_core::EntityId {
            slot: place.slot + 5000,
            generation: 0,
        },
        expected_place: place,
        commodity: CommodityKind::Bread,
        quantity: Quantity(1),
    }
}

fn add_social_observation(
    h: &mut GoldenHarness,
    guard: worldwake_core::EntityId,
    place: worldwake_core::EntityId,
    observed_tick: u64,
) {
    let mut store = h
        .world
        .get_component_agent_belief_store(guard)
        .cloned()
        .unwrap_or_else(AgentBeliefStore::new);
    store.record_social_observation(SocialObservation {
        detail: SocialObservationDetail::SuspectedTheft {
            theft: theft(place),
            suspect: None,
        },
        place,
        observed_tick: Tick(observed_tick),
        source: PerceptionSource::DirectObservation,
    });

    let mut txn = new_txn(&mut h.world, observed_tick + 1);
    txn.set_component_agent_belief_store(guard, store).unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn add_violation_record(
    h: &mut GoldenHarness,
    guard: worldwake_core::EntityId,
    place: worldwake_core::EntityId,
    observed_tick: u64,
    expires_tick: u64,
) {
    let mut memory = h
        .world
        .get_component_violation_memory(guard)
        .cloned()
        .unwrap_or_default();
    let next_id = memory.next_violation_id();
    memory.violations.push(RecordedViolation {
        id: next_id,
        kind: ViolationKind::SuspectedTheft {
            theft: theft(place),
            suspect: None,
        },
        observed_tick: Tick(observed_tick),
        resolved_tick: None,
        expires_tick: Tick(expires_tick),
    });
    memory.next_violation_id = ViolationId(next_id.0 + 1);

    let mut txn = new_txn(&mut h.world, observed_tick + 1);
    txn.set_component_violation_memory(guard, memory).unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn planning_trace_at(
    h: &GoldenHarness,
    agent: worldwake_core::EntityId,
    tick: Tick,
) -> &PlanningPipelineTrace {
    let trace = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(agent, tick)
        .unwrap_or_else(|| panic!("missing decision trace for agent {agent} at tick {tick:?}"));
    match &trace.outcome {
        DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected planning trace at tick {tick:?}, got {other:?}"),
    }
}

fn maybe_planning_trace_at(
    h: &GoldenHarness,
    agent: worldwake_core::EntityId,
    tick: Tick,
) -> Option<&PlanningPipelineTrace> {
    h.driver
        .trace_sink()
        .and_then(|sink| sink.trace_at(agent, tick))
        .and_then(|trace| match &trace.outcome {
            DecisionOutcome::Planning(planning) => Some(planning),
            _ => None,
        })
        .map(|planning| &**planning)
}

fn patrol_motive_score(planning: &PlanningPipelineTrace, place: worldwake_core::EntityId) -> u32 {
    planning
        .candidates
        .ranked_summary_for_opportunity(OpportunityKey {
            goal_key: GoalKey::from(GoalKind::Patrol { place }),
            anchor: OpportunityAnchor::Place(place),
        })
        .unwrap_or_else(|| panic!("missing ranked patrol summary for place {place}"))
        .motive_score
}

fn patrol_goal(place: worldwake_core::EntityId) -> GoalKey {
    GoalKey::from(GoalKind::Patrol { place })
}

fn run_patrol_cycle(seed: Seed) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let village_square = prototype_place_entity(PrototypePlace::VillageSquare);
    let guard_post = prototype_place_entity(PrototypePlace::GuardPost);

    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let guard = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Guard",
        village_square,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        guard,
        default_perception_profile(),
    );
    set_patrol_state(
        &mut h,
        guard,
        PatrolRoute {
            assigned_places: vec![village_square, guard_post],
            current_index: 0,
        },
        patrol_profile(1, 0, 600),
        0,
    );

    h.step_once();
    let tick_zero_planning = planning_trace_at(&h, guard, Tick(0));
    assert!(
        tick_zero_planning
            .selection
            .selected_goal()
            .is_some_and(|goal| matches!(goal.kind, GoalKind::Patrol { .. })),
        "guard should select patrol on the opening tick"
    );

    for _ in 0..32 {
        h.step_once();
    }

    let patrol_events = h
        .action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(guard)
        .into_iter()
        .collect::<Vec<_>>();
    let patrol_commit_ticks = patrol_events
        .iter()
        .filter(|event| {
            event.action_name == "patrol" && matches!(event.kind, ActionTraceKind::Committed { .. })
        })
        .map(|event| event.tick)
        .collect::<Vec<_>>();
    let patrol_started_targets = patrol_events
        .iter()
        .filter_map(|event| match &event.kind {
            ActionTraceKind::Started { targets } if event.action_name == "patrol" => {
                targets.first().copied()
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let patrol_start_failures = patrol_events
        .iter()
        .filter(|event| {
            event.action_name == "patrol"
                && matches!(event.kind, ActionTraceKind::StartFailed { .. })
        })
        .count();

    assert_eq!(
        patrol_start_failures, 0,
        "patrol should not expose stale-start failures once the route advances; patrol_events={patrol_events:?}"
    );
    assert!(
        patrol_commit_ticks.len() >= 3,
        "guard should keep patrolling across multiple route wraps; patrol_events={patrol_events:?}"
    );
    assert_eq!(
        patrol_started_targets.get(0..3),
        Some(&[village_square, guard_post, village_square][..]),
        "patrol starts should alternate across the two-waypoint route and wrap to the first waypoint"
    );

    (
        worldwake_core::hash_world(&h.world).unwrap(),
        worldwake_core::hash_event_log(&h.event_log).unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Scenario 52: Patrol Cycle Wraps Route
// ---------------------------------------------------------------------------
//
// Systems: AI, Travel, Patrol
// GoalKinds: Patrol
// ActionDomains: Travel, Generic
// Places: VillageSquare, GuardPost
// Principles: 8, 20, 21
//
// Setup: One guard at VillageSquare with a two-waypoint patrol route
//   [VillageSquare, GuardPost].
//
// Proves: The guard selects patrol, completes patrol dwell, travels to the
//   next waypoint, patrols again, and continues alternating after the route wraps.
//
// Chain: Patrol candidate -> plan selection -> patrol commit -> travel ->
//   patrol commit -> authoritative route wrap -> continued alternating patrol.

#[test]
fn golden_patrol_cycle_wraps_route() {
    let _ = run_patrol_cycle(Seed([0x51; 32]));
}

#[test]
fn golden_patrol_cycle_wraps_route_replays_deterministically() {
    let first = run_patrol_cycle(Seed([0x51; 32]));
    let second = run_patrol_cycle(Seed([0x51; 32]));
    assert_eq!(
        first, second,
        "patrol cycle scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 53: Patrol Interruption Preserves Waypoint Until Resume
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Patrol
// GoalKinds: Patrol, ConsumeOwnedCommodity
// ActionDomains: Generic, Needs
// Places: GuardPost
// Principles: 8, 20, 21
//
// Setup: One guard is already patrolling at GuardPost with bread in inventory.
//   Hunger is raised to the critical band after patrol starts.
//
// Proves: Patrol is interrupted for self-care, `current_index` does not advance
//   during the interruption, and the guard later resumes patrol from the same
//   waypoint before advancing.

#[test]
fn golden_patrol_interruption_preserves_waypoint_until_resume() {
    let guard_post = prototype_place_entity(PrototypePlace::GuardPost);
    let village_square = prototype_place_entity(PrototypePlace::VillageSquare);

    let mut h = GoldenHarness::new(Seed([0x52; 32]));
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let guard = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Guard",
        guard_post,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        guard,
        default_perception_profile(),
    );
    set_patrol_state(
        &mut h,
        guard,
        PatrolRoute {
            assigned_places: vec![village_square, guard_post],
            current_index: 1,
        },
        patrol_profile(6, 0, 600),
        0,
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        guard,
        guard_post,
        CommodityKind::Bread,
        Quantity(1),
    );

    let patrol_started = (0..8).any(|_| {
        let tick_before = h.scheduler.current_tick();
        h.step_once();
        h.action_trace_sink().is_some_and(|sink| {
            sink.events_for_at(guard, tick_before).iter().any(|event| {
                event.action_name == "patrol"
                    && matches!(event.kind, ActionTraceKind::Started { .. })
            })
        })
    });
    assert!(
        patrol_started,
        "guard should start patrolling before interruption"
    );

    let hunger_tick = h.scheduler.current_tick().0;
    set_hunger(&mut h, guard, 950, hunger_tick);

    let mut eat_commit_tick = None;
    for _ in 0..12 {
        let tick_before = h.scheduler.current_tick();
        h.step_once();
        if h.action_trace_sink().is_some_and(|sink| {
            sink.events_for_at(guard, tick_before).iter().any(|event| {
                event.action_name == "eat"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            })
        }) {
            eat_commit_tick = Some(tick_before);
            break;
        }
    }

    let _eat_commit_tick = eat_commit_tick.expect("guard should interrupt patrol to eat");
    assert_eq!(
        h.world
            .get_component_patrol_route(guard)
            .unwrap()
            .current_index,
        1,
        "route index should remain on the interrupted waypoint after eating"
    );

    let resumed_patrol_started = (0..16).any(|_| {
        let tick_before = h.scheduler.current_tick();
        h.step_once();
        h.action_trace_sink().is_some_and(|sink| {
            sink.events_for_at(guard, tick_before).iter().any(|event| {
                event.action_name == "patrol"
                    && matches!(event.kind, ActionTraceKind::Started { .. })
            })
        })
    });
    assert!(
        resumed_patrol_started,
        "guard should restart patrol after resolving critical hunger"
    );
    assert_eq!(
        h.world
            .get_component_patrol_route(guard)
            .unwrap()
            .current_index,
        1,
        "route index should still point at the interrupted waypoint when patrol restarts"
    );

    let patrol_commit_after_resume = (0..16).find(|_| {
        let tick_before = h.scheduler.current_tick();
        h.step_once();
        h.action_trace_sink().is_some_and(|sink| {
            sink.events_for_at(guard, tick_before).iter().any(|event| {
                event.action_name == "patrol"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            })
        })
    });
    assert!(
        patrol_commit_after_resume.is_some(),
        "guard should eventually complete the resumed patrol leg"
    );
    assert_eq!(
        h.world
            .get_component_patrol_route(guard)
            .unwrap()
            .current_index,
        0,
        "route should advance only after the resumed patrol leg commits"
    );
}

// ---------------------------------------------------------------------------
// Scenario 54: Patrol Belief Urgency Scales From Local Crime And Vacancy
// ---------------------------------------------------------------------------
//
// Systems: AI, Patrol, Institutional beliefs
// GoalKinds: Patrol
// ActionDomains: Generic
// Places: VillageSquare
// Principles: 7, 14, 20
//
// Setup: Two comparable guards share the same patrol route and profile. Only
//   one holds a local suspected-theft memory and a vacancy belief for an office
//   on the patrol jurisdiction.
//
// Proves: The informed guard's patrol motive is higher at the decision-trace
//   layer without any authoritative world-state shortcut.

#[test]
fn golden_patrol_belief_urgency_scales_from_local_crime_and_vacancy() {
    let village_square = prototype_place_entity(PrototypePlace::VillageSquare);

    let mut h = GoldenHarness::new(Seed([0x53; 32]));
    h.driver.enable_tracing();

    let baseline_guard = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Baseline Guard",
        village_square,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let informed_guard = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Informed Guard",
        village_square,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    for guard in [baseline_guard, informed_guard] {
        set_agent_perception_profile(
            &mut h.world,
            &mut h.event_log,
            guard,
            default_perception_profile(),
        );
        set_patrol_state(
            &mut h,
            guard,
            PatrolRoute {
                assigned_places: vec![village_square],
                current_index: 0,
            },
            patrol_profile(2, 0, 150),
            0,
        );
    }

    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "Village Elder",
        village_square,
        worldwake_core::SuccessionLaw::Support,
        5,
        vec![],
    );
    let holder = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Holder",
        village_square,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        baseline_guard,
        office,
        Some(holder),
        Tick(0),
        InstitutionalKnowledgeSource::WitnessedEvent,
        Some(village_square),
    );
    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        informed_guard,
        office,
        None,
        Tick(0),
        InstitutionalKnowledgeSource::WitnessedEvent,
        Some(village_square),
    );
    add_violation_record(&mut h, informed_guard, village_square, 0, 20);

    h.step_once();

    let baseline_planning = planning_trace_at(&h, baseline_guard, Tick(0));
    let informed_planning = planning_trace_at(&h, informed_guard, Tick(0));

    let baseline_motive = patrol_motive_score(baseline_planning, village_square);
    let informed_motive = patrol_motive_score(informed_planning, village_square);
    assert_eq!(
        baseline_motive, 150,
        "baseline patrol motive should stay at profile weight"
    );
    assert!(
        informed_motive > baseline_motive,
        "local theft memory and vacancy belief should increase patrol motive"
    );
}

// ---------------------------------------------------------------------------
// Scenario 55: Patrol Route Adaptation Retargets After Local Report
// ---------------------------------------------------------------------------
//
// Systems: Patrol adaptation, AI, Travel
// GoalKinds: Patrol
// ActionDomains: Travel, Generic
// Places: VillageSquare, SouthGate, GeneralStore
// Principles: 7, 20, 24
//
// Setup: One guard has a baseline patrol route but already holds a local
//   suspected-theft social observation for GeneralStore.
//
// Proves: The authoritative patrol system adapts the route from the guard's
//   local report, and later AI selection retargets patrol to the adapted place.

#[test]
fn golden_patrol_route_adaptation_retargets_after_local_report() {
    let village_square = prototype_place_entity(PrototypePlace::VillageSquare);
    let south_gate = prototype_place_entity(PrototypePlace::SouthGate);
    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);

    let mut h = GoldenHarness::new(Seed([0x54; 32]));
    h.driver.enable_tracing();

    let guard = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Guard",
        village_square,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        guard,
        default_perception_profile(),
    );
    set_patrol_state(
        &mut h,
        guard,
        PatrolRoute {
            assigned_places: vec![village_square, south_gate],
            current_index: 0,
        },
        patrol_profile(1, 0, 300),
        0,
    );
    add_social_observation(&mut h, guard, general_store, 0);

    let adapted = (0..20).find_map(|_| {
        h.step_once();
        let route = h.world.get_component_patrol_route(guard).unwrap();
        route
            .assigned_places
            .contains(&general_store)
            .then_some(route.clone())
    });
    let adapted = adapted.expect("route should adapt from the guard-local report");
    assert_eq!(
        adapted.assigned_places[0], general_store,
        "reported place should become the front of the patrol route"
    );

    let mut retarget_tick = None;
    for _ in 0..20 {
        let tick_before = h.scheduler.current_tick();
        h.step_once();
        let planning = planning_trace_at(&h, guard, tick_before);
        if planning
            .selection
            .selected_goal_is(patrol_goal(general_store))
        {
            retarget_tick = Some(tick_before);
            break;
        }
    }
    let retarget_tick = retarget_tick.expect("AI should retarget patrol to the adapted place");

    let planning = planning_trace_at(&h, guard, retarget_tick);
    assert!(
        planning
            .selection
            .selected_goal_is(patrol_goal(general_store)),
        "retargeted planning tick should target the adapted waypoint"
    );
    assert_eq!(planning.patrol_route.route, Some(adapted.clone()));
    assert_eq!(planning.patrol_route.current_waypoint, Some(general_store));
    assert_eq!(
        planning.selected_patrol_anchor,
        Some(OpportunityAnchor::Place(general_store))
    );
}

// ---------------------------------------------------------------------------
// Scenario 56: Patrol Locality Requires Guard-Local Report
// ---------------------------------------------------------------------------
//
// Systems: Patrol adaptation, AI
// GoalKinds: Patrol
// ActionDomains: Generic
// Places: VillageSquare, SouthGate, GeneralStore
// Principles: 7, 14, 20
//
// Setup: Another agent holds a theft report for GeneralStore, but the guard
//   does not. The guard has the same baseline patrol route and profile.
//
// Proves: The guard's motive remains baseline and the route does not adapt from
//   another agent's memory or from authoritative truth alone.

#[test]
fn golden_patrol_locality_requires_guard_local_report() {
    let village_square = prototype_place_entity(PrototypePlace::VillageSquare);
    let south_gate = prototype_place_entity(PrototypePlace::SouthGate);
    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);

    let mut h = GoldenHarness::new(Seed([0x55; 32]));
    h.driver.enable_tracing();

    let guard = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Guard",
        village_square,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let witness = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Witness",
        prototype_place_entity(PrototypePlace::OrchardFarm),
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    for agent in [guard, witness] {
        set_agent_perception_profile(
            &mut h.world,
            &mut h.event_log,
            agent,
            default_perception_profile(),
        );
    }
    set_patrol_state(
        &mut h,
        guard,
        PatrolRoute {
            assigned_places: vec![village_square, south_gate],
            current_index: 0,
        },
        patrol_profile(2, 0, 175),
        0,
    );
    add_social_observation(&mut h, witness, general_store, 0);

    for _ in 0..6 {
        h.step_once();
    }

    let planning = maybe_planning_trace_at(&h, guard, Tick(0))
        .expect("guard should produce an opening planning trace");
    assert!(
        planning
            .selection
            .selected_goal_is(patrol_goal(village_square)),
        "guard should keep the baseline patrol goal without a local report"
    );
    assert_eq!(
        patrol_motive_score(planning, village_square),
        175,
        "another agent's report should not inflate this guard's patrol motive"
    );
    assert!(
        !h.world
            .get_component_patrol_route(guard)
            .unwrap()
            .assigned_places
            .contains(&general_store),
        "guard route must not adapt from another agent's report"
    );
}

// ---------------------------------------------------------------------------
// Scenario 57: Patrol-Driven Crime Discovery Chain
// ---------------------------------------------------------------------------
//
// Systems: AI, Travel, Patrol, Perception, Investigation
// GoalKinds: Patrol, InvestigateViolation
// ActionDomains: Travel, Generic
// Places: VillageSquare, GeneralStore
// Principles: 1, 7, 14, 17
//
// Setup: One guard patrols between VillageSquare and GeneralStore. The guard
//   begins with a stale direct belief that a bread lot is at GeneralStore.
//   Before the guard arrives there, the lot is lawfully relocated to a
//   different non-local place the guard cannot currently observe.
//
// Proves: Patrol creates lawful arrival, local perception detects the violated
//   expectation as `EntityMissing`, AI selects `InvestigateViolation`, and the
//   investigation records `WitnessedAbsence` without upgrading the incident to
//   `SuspectedTheft` for a non-owner investigator.

#[allow(clippy::too_many_lines)]
fn run_patrol_driven_crime_discovery(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let village_square = prototype_place_entity(PrototypePlace::VillageSquare);
    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);
    let common_house = prototype_place_entity(PrototypePlace::CommonHouse);

    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let guard = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Guard",
        village_square,
        HomeostaticNeeds::default(),
        worldwake_core::MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let mut sharp_perception = default_perception_profile();
    sharp_perception.observation_fidelity = pm(1000);
    set_agent_perception_profile(&mut h.world, &mut h.event_log, guard, sharp_perception);
    set_patrol_state(
        &mut h,
        guard,
        PatrolRoute {
            assigned_places: vec![general_store],
            current_index: 0,
        },
        patrol_profile(2, 0, 600),
        0,
    );
    set_violation_profile(
        &mut h,
        guard,
        ViolationDispositionProfile {
            investigation_duration_ticks: nz(2),
            violation_memory_retention_ticks: 50,
            investigation_motive_weight: pm(1000),
            ownership_motive_bonus: pm(0),
        },
        0,
    );

    let bread_lot = {
        let mut txn = new_txn(&mut h.world, 0);
        let lot = txn
            .create_item_lot(CommodityKind::Bread, Quantity(1))
            .unwrap();
        txn.set_ground_location(lot, general_store).unwrap();
        commit_txn(txn, &mut h.event_log);
        lot
    };
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        guard,
        bread_lot,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    worldwake_core::verify_live_lot_conservation(&h.world, CommodityKind::Bread, 1).unwrap();

    h.step_once();

    let opening_planning = planning_trace_at(&h, guard, Tick(0));
    assert!(
        opening_planning
            .selection
            .selected_goal_is(patrol_goal(general_store)),
        "guard should open by selecting patrol for the remote waypoint"
    );

    {
        let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
        txn.set_ground_location(bread_lot, common_house).unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    worldwake_core::verify_live_lot_conservation(&h.world, CommodityKind::Bread, 1).unwrap();

    let mut arrival_tick = None;
    let mut generated_violation_id = None;
    let mut saw_unresolved_entity_missing = false;
    let mut investigate_commit_seen = false;

    for _ in 0..60 {
        let tick_before = h.scheduler.current_tick();
        h.step_once();
        worldwake_core::verify_live_lot_conservation(&h.world, CommodityKind::Bread, 1).unwrap();

        if h.world.effective_place(guard) == Some(general_store) && arrival_tick.is_none() {
            arrival_tick = Some(tick_before);
        }

        if let Some(trace) = h
            .driver
            .trace_sink()
            .expect("decision tracing should stay enabled")
            .trace_at(guard, tick_before)
            && let DecisionOutcome::Planning(planning) = &trace.outcome
        {
            let planning = planning.as_ref();
            if generated_violation_id.is_none() {
                generated_violation_id = planning.candidates.generated.iter().find_map(|goal| {
                    match goal.goal_key.kind {
                        GoalKind::InvestigateViolation {
                            violation_id,
                            place,
                        } if place == general_store => Some(violation_id),
                        _ => None,
                    }
                });
            }
        }

        if generated_violation_id.is_some_and(|violation_id| {
            h.world
                .get_component_violation_memory(guard)
                .is_some_and(|memory| {
                    memory
                        .unresolved_by_id(violation_id, h.scheduler.current_tick())
                        .is_some()
                })
        }) {
            saw_unresolved_entity_missing = true;
        }

        let events = h
            .action_trace_sink()
            .expect("action tracing should stay enabled")
            .events_for(guard);
        investigate_commit_seen = generated_violation_id.is_some_and(|violation_id| {
            events.iter().any(|event| {
                event.action_name == "investigate"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
                    && event.detail == Some(ActionTraceDetail::Investigate { violation_id })
            })
        });
        if arrival_tick.is_some() && generated_violation_id.is_some() && investigate_commit_seen {
            break;
        }
    }

    let arrival_tick = arrival_tick.expect("guard should eventually arrive at GeneralStore");
    let generated_violation_id =
        generated_violation_id.expect("arrival should generate an investigate-violation goal");
    assert!(
        investigate_commit_seen,
        "guard should eventually commit investigate for the discovered missing lot; action_events={:?}",
        h.action_trace_sink()
            .expect("action tracing should stay enabled")
            .events_for(guard)
    );
    assert!(
        saw_unresolved_entity_missing,
        "patrol arrival should expose an unresolved entity-missing violation before investigation resolves it"
    );

    let action_events = h
        .action_trace_sink()
        .expect("action tracing should stay enabled")
        .events_for(guard)
        .into_iter()
        .collect::<Vec<_>>();
    assert!(
        action_events.iter().any(|event| {
            event.action_name == "travel"
                && matches!(event.kind, ActionTraceKind::Started { ref targets } if targets == &[general_store])
        }),
        "guard should start travel toward the GeneralStore waypoint"
    );
    assert!(
        action_events.iter().any(|event| {
            event.action_name == "patrol"
                && matches!(event.kind, ActionTraceKind::Started { ref targets } if targets == &[general_store])
        }),
        "patrol should start once the guard reaches GeneralStore"
    );
    assert_eq!(
        h.world.effective_place(guard),
        Some(general_store),
        "guard should investigate only after patrol lawfully reaches GeneralStore"
    );

    let planning_after_arrival = planning_trace_at(&h, guard, arrival_tick);
    assert!(
        planning_after_arrival
            .selection
            .selected_goal_is(patrol_goal(general_store))
            || planning_after_arrival
                .candidates
                .generated
                .iter()
                .any(|goal| matches!(
                    goal.goal_key.kind,
                    GoalKind::InvestigateViolation { violation_id, place }
                        if violation_id == generated_violation_id && place == general_store
                )),
        "the arrival tick should still reflect patrol locality and/or the generated local investigate branch"
    );

    let final_memory = h
        .world
        .get_component_violation_memory(guard)
        .expect("guard should keep violation memory");
    let entity_missing_record = final_memory
        .violations
        .iter()
        .find(|record| record.id == generated_violation_id)
        .expect("generated violation should remain recorded after investigation");
    assert_eq!(
        entity_missing_record.kind,
        ViolationKind::EntityMissing {
            entity: bread_lot,
            expected_place: general_store,
        },
        "patrol discovery should record the missing-lot mismatch at the guarded place"
    );
    assert!(
        entity_missing_record.resolved_tick.is_some(),
        "the original entity-missing violation should be resolved after investigate commits"
    );
    assert!(
        final_memory
            .violations
            .iter()
            .all(|record| !matches!(record.kind, ViolationKind::SuspectedTheft { .. })),
        "non-owner patrol investigation must not mint suspected-theft violation records"
    );

    let final_store = h
        .world
        .get_component_agent_belief_store(guard)
        .expect("guard should keep a belief store");
    assert!(
        final_store.social_observations.iter().any(|observation| {
            observation.detail
                == SocialObservationDetail::WitnessedAbsence {
                    missing_entity: bread_lot,
                    expected_place: general_store,
                }
                && observation.place == general_store
        }),
        "investigation should leave a witnessed-absence social observation for the missing lot"
    );
    assert!(
        final_store.social_observations.iter().all(|observation| {
            !matches!(
                observation.detail,
                SocialObservationDetail::SuspectedTheft { .. }
            )
        }),
        "non-owner patrol investigation must not infer suspected theft directly"
    );

    (
        worldwake_core::hash_world(&h.world).unwrap(),
        worldwake_core::hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_patrol_driven_crime_discovery() {
    let _ = run_patrol_driven_crime_discovery(Seed([0x56; 32]));
}

#[test]
fn golden_patrol_driven_crime_discovery_replays_deterministically() {
    let first = run_patrol_driven_crime_discovery(Seed([0x56; 32]));
    let second = run_patrol_driven_crime_discovery(Seed([0x56; 32]));
    assert_eq!(
        first, second,
        "patrol-driven crime discovery scenario should replay deterministically"
    );
}
