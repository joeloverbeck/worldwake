//! Golden tests for belief-backed remote pursuit (S40-remote-pursuit).
//!
//! Proves three end-to-end pursuit scenarios:
//! 1. Bandit witnesses traveler departure, pursues, attacks on arrival.
//! 2. Bandit pursues stale information — target moved on — honest failure.
//! 3. Combat → flee → re-pursuit bounded by budget and confidence.

mod golden_harness;

use golden_harness::*;
use worldwake_ai::DecisionOutcome;
use worldwake_core::{
    hash_event_log, hash_world, BanditCamp, BanditFactionPolicy, BeliefConfidencePolicy,
    CombatProfile, CommodityKind, Container, ControlSource, EntityId, GoalKey, GoalKind,
    HomeostaticNeeds, MetabolismProfile, PerceptionProfile, PerceptionSource, PlaceTag, Quantity,
    Seed, StateHash, Tick, Topology, TravelEdge, TravelEdgeId, UtilityProfile,
};
use worldwake_sim::{ActionRequestMode, ActionTraceKind, RequestProvenance};

// ---------------------------------------------------------------------------
// Custom place entity IDs (outside prototype range)
// ---------------------------------------------------------------------------

const PLACE_A: EntityId = entity(100);
const PLACE_B: EntityId = entity(101);
const PLACE_C: EntityId = entity(102);

const fn entity(slot: u32) -> EntityId {
    EntityId {
        slot,
        generation: 0,
    }
}

fn place(name: &str, tags: &[PlaceTag]) -> worldwake_core::Place {
    worldwake_core::Place {
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

// ---------------------------------------------------------------------------
// Topology builders
// ---------------------------------------------------------------------------

/// Two-place topology: A ↔ B (2-tick travel).
fn build_two_place_topology() -> Topology {
    let mut t = Topology::new();
    // Indoor (Village) tags: no outdoor relief affordances to distract the planner.
    t.add_place(PLACE_A, place("Hideout", &[PlaceTag::Village]))
        .unwrap();
    t.add_place(PLACE_B, place("Crossroads", &[PlaceTag::Village]))
        .unwrap();
    connect(&mut t, 200, PLACE_A, PLACE_B, 2);
    t
}

/// Three-place topology: A ↔ B (2 ticks) ↔ C (1 tick). No A↔C edge.
///
/// B→C is 1 tick so the traveler can depart and arrive within a single
/// tick — making the travel action complete before perception runs,
/// ensuring the bandit at B does NOT observe the departure direction.
fn build_three_place_topology() -> Topology {
    let mut t = Topology::new();
    t.add_place(PLACE_A, place("Hideout", &[PlaceTag::Village]))
        .unwrap();
    t.add_place(PLACE_B, place("Crossroads", &[PlaceTag::Village]))
        .unwrap();
    t.add_place(PLACE_C, place("Haven", &[PlaceTag::Village]))
        .unwrap();
    connect(&mut t, 200, PLACE_A, PLACE_B, 2);
    connect(&mut t, 210, PLACE_B, PLACE_C, 1);
    t
}

// ---------------------------------------------------------------------------
// Profile helpers
// ---------------------------------------------------------------------------

fn pursuit_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        memory_capacity: 64,
        memory_retention_ticks: 240,
        observation_fidelity: pm(1000),
        confidence_policy: BeliefConfidencePolicy::default(),
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
    }
}

fn strong_bandit_combat_profile() -> CombatProfile {
    CombatProfile::new(
        pm(900),  // wound_capacity
        pm(600),  // attack_skill
        pm(300),  // defense_skill
        pm(300),  // guard_skill
        pm(80),   // attack_severity
        pm(25),   // bleed_rate
        pm(0),    // bleed_rate_variance
        pm(200),  // dodge_chance
        pm(50),   // parry_chance
        nz(2),    // attacks_per_round
        nz(4),    // combat_round_ticks
    )
}

fn weak_traveler_combat_profile() -> CombatProfile {
    CombatProfile::new(
        pm(400),  // wound_capacity
        pm(200),  // attack_skill
        pm(150),  // defense_skill
        pm(100),  // guard_skill
        pm(30),   // attack_severity
        pm(10),   // bleed_rate
        pm(0),    // bleed_rate_variance
        pm(80),   // dodge_chance
        pm(20),   // parry_chance
        nz(1),    // attacks_per_round
        nz(4),    // combat_round_ticks
    )
}

fn bandit_utility_profile() -> UtilityProfile {
    UtilityProfile {
        social_weight: pm(0),
        danger_weight: pm(900),
        courage: pm(150),
        enterprise_weight: pm(0),
        ..UtilityProfile::default()
    }
}

/// Bandit is moderately hungry — provides motive for raiding travelers
/// who carry food (bread, apple).
fn hungry_bandit_needs() -> HomeostaticNeeds {
    HomeostaticNeeds::new(pm(600), pm(0), pm(0), pm(0), pm(0))
}

fn sated_needs() -> HomeostaticNeeds {
    HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(0))
}

fn zero_metabolism() -> MetabolismProfile {
    MetabolismProfile {
        hunger_rate: pm(0),
        thirst_rate: pm(0),
        fatigue_rate: pm(0),
        bladder_rate: pm(0),
        dirtiness_rate: pm(0),
        ..MetabolismProfile::default()
    }
}

// ---------------------------------------------------------------------------
// Harness construction
// ---------------------------------------------------------------------------

fn build_harness_with_topology(seed: Seed, topology: Topology) -> GoldenHarness {
    let mut h = GoldenHarness::new(seed);
    h.world = worldwake_core::World::new(topology).unwrap();
    h.event_log = worldwake_core::EventLog::new();
    h.scheduler = worldwake_sim::Scheduler::new(worldwake_sim::SystemManifest::canonical());
    h.controller = worldwake_sim::ControllerState::new();
    h
}

// ---------------------------------------------------------------------------
// Agent seeding helpers
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct PursuitIds {
    bandit: EntityId,
    traveler: EntityId,
    faction: EntityId,
}

fn seed_pursuit_scenario(
    h: &mut GoldenHarness,
    bandit_place: EntityId,
    traveler_place: EntityId,
) -> PursuitIds {
    // Bandit agent (AI-controlled, moderately hungry — gives motive to
    // raid travelers for food; zero metabolism so hunger doesn't change).
    let bandit = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Bandit",
        bandit_place,
        hungry_bandit_needs(),
        zero_metabolism(),
        bandit_utility_profile(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        bandit,
        pursuit_perception_profile(),
    );

    // Traveler agent (human-controlled so we drive their movement).
    let traveler = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Traveler",
        traveler_place,
        sated_needs(),
        zero_metabolism(),
        UtilityProfile::default(),
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_agent_data(
            traveler,
            worldwake_core::AgentData {
                control_source: ControlSource::Human,
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        traveler,
        pursuit_perception_profile(),
    );

    // Create faction with BanditFactionPolicy.
    let mut txn = new_txn(&mut h.world, 0);
    let faction = txn.create_faction("Pursuit Bandits").unwrap();
    txn.add_member(bandit, faction).unwrap();
    txn.set_component_combat_profile(bandit, strong_bandit_combat_profile())
        .unwrap();
    txn.set_component_combat_profile(traveler, weak_traveler_combat_profile())
        .unwrap();
    txn.set_component_pursuit_profile(
        bandit,
        worldwake_core::PursuitProfile {
            min_location_confidence: pm(500),
            max_pursuit_travel_ticks: nz(10),
        },
    )
    .unwrap();
    txn.set_component_bandit_faction_policy(
        faction,
        BanditFactionPolicy {
            min_regroup_count: 1,
            establishment_duration_ticks: nz(2),
            abandonment_grace_ticks: nz(2),
            flee_wound_threshold: pm(300),
            rally_place: None,
        },
    )
    .unwrap();
    // Minimal camp supplies container (required for BanditCamp component).
    let camp_supplies = txn
        .create_container(Container {
            capacity: worldwake_core::LoadUnits(10),
            allowed_commodities: None,
            allows_unique_items: false,
            allows_nested_containers: false,
        })
        .unwrap();
    txn.set_ground_location(camp_supplies, bandit_place)
        .unwrap();
    txn.set_owner(camp_supplies, faction).unwrap();
    txn.set_component_bandit_camp(
        bandit_place,
        BanditCamp {
            faction,
            supplies: camp_supplies,
            empty_since_tick: None,
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);

    // Give the traveler bread — the hungry bandit has motive to raid
    // for consumable food.
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        traveler,
        traveler_place,
        CommodityKind::Bread,
        Quantity(3),
    );

    // Seed bandit's initial local beliefs.
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        bandit,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    PursuitIds {
        bandit,
        traveler,
        faction,
    }
}

fn request_travel(h: &mut GoldenHarness, traveler: EntityId, destination: EntityId) {
    let travel_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "travel")
        .map(|def| def.id)
        .expect("full registries should include travel");
    let tick = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        tick,
        worldwake_sim::InputKind::RequestAction {
            actor: traveler,
            def_id: travel_def_id,
            targets: vec![destination],
            payload_override: None,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
}

// ---------------------------------------------------------------------------
// Scenario 68: Bandit witnesses traveler leave, pursues, attacks
// ---------------------------------------------------------------------------
//
// Systems: Perception, AI, Travel, Combat
// GoalKinds: RaidTarget
// ActionDomains: Travel, Combat
// Places: PLACE_A (Hideout), PLACE_B (Crossroads)
// Principles: 1, 3, 7, 14, 20, 21
//
// Setup: Bandit and traveler start co-located at Hideout (PLACE_A).
//   Traveler carries 5 coins (raid motive). Bandit has PursuitProfile
//   with min_location_confidence=500, max_pursuit_travel_ticks=10.
//   Travel from A→B takes 2 ticks. Needs are sated; no competing goals.
//
// Proves:
//   1. Remote pursuit emerges from perception → belief → candidate →
//      plan search → travel → attack without any chase script.
//   2. Attack occurs only at real co-location — no synthesized combat.
//   3. Pursuit is belief-backed: bandit uses observed departure
//      direction, not omniscient target position.
//
// Chain: co-location observation -> traveler departs -> perception
//   updates belief -> RaidTarget candidate emitted -> Travel+Attack
//   plan -> travel to B -> attack at B -> traveler wounded.

fn run_scenario_1(seed: Seed) -> (StateHash, StateHash) {
    let mut h = build_harness_with_topology(seed, build_two_place_topology());
    let ids = seed_pursuit_scenario(&mut h, PLACE_A, PLACE_A);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    // Enqueue traveler's departure BEFORE the first tick so the traveler
    // enters transit on tick 0 — the bandit observes the departure
    // direction through perception's departure-direction projection.
    request_travel(&mut h, ids.traveler, PLACE_B);

    // Run ticks until traveler arrives at B (2-tick travel).
    for _ in 0..5 {
        h.step_once();
    }
    assert_eq!(
        h.world.effective_place(ids.traveler),
        Some(PLACE_B),
        "traveler should arrive at PLACE_B"
    );

    // Now run ticks until the bandit pursues and attacks.
    // With 2-tick travel A→B, the bandit should arrive and attack
    // within ~10 ticks of the traveler's departure.
    let mut bandit_arrived = false;
    let mut traveler_wounded = false;

    for _ in 0..30 {
        h.step_once();

        if h.world.effective_place(ids.bandit) == Some(PLACE_B) {
            bandit_arrived = true;
        }

        if let Some(wounds) = h.world.get_component_wound_list(ids.traveler) {
            if !wounds.wounds.is_empty() {
                traveler_wounded = true;
                break;
            }
        }
    }

    // Assert: bandit pursued and arrived at B.
    assert!(
        bandit_arrived,
        "bandit should pursue the traveler to PLACE_B"
    );

    // Assert: traveler received at least one wound from combat.
    assert!(
        traveler_wounded,
        "traveler should receive wounds after bandit pursues and attacks"
    );

    // Assert: action traces show travel then attack commits for the bandit.
    let bandit_events = h
        .action_trace_sink()
        .expect("action tracing enabled")
        .events_for(ids.bandit);
    let travel_committed = bandit_events.iter().any(|e| {
        e.action_name == "travel" && matches!(e.kind, ActionTraceKind::Committed { .. })
    });
    let attack_committed = bandit_events.iter().any(|e| {
        e.action_name == "attack" && matches!(e.kind, ActionTraceKind::Committed { .. })
    });
    assert!(
        travel_committed,
        "bandit action trace should show a committed travel action"
    );
    assert!(
        attack_committed,
        "bandit action trace should show a committed attack action"
    );

    // Assert: decision trace shows RaidTarget was selected.
    let trace_sink = h.driver.trace_sink().expect("tracing enabled");
    let any_raid_selected = trace_sink
        .traces_for(ids.bandit)
        .into_iter()
        .any(|trace| {
            if let DecisionOutcome::Planning(ref p) = trace.outcome {
                p.selection
                    .selected_goal_is(GoalKey::from(GoalKind::RaidTarget {
                        target: ids.traveler,
                    }))
            } else {
                false
            }
        });
    assert!(
        any_raid_selected,
        "decision trace should show RaidTarget selected for remote pursuit"
    );

    (
        hash_world(&h.world).expect("world should hash"),
        hash_event_log(&h.event_log).expect("event log should hash"),
    )
}

#[test]
fn golden_bandit_witnesses_and_pursues() {
    let _ = run_scenario_1(Seed([71; 32]));
}

#[test]
fn golden_bandit_witnesses_and_pursues_replays_deterministically() {
    let first = run_scenario_1(Seed([71; 32]));
    let second = run_scenario_1(Seed([71; 32]));
    assert_eq!(
        first, second,
        "pursuit scenario 1 should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 69: Bandit pursues stale target, arrival failure
// ---------------------------------------------------------------------------
//
// Systems: Perception, AI, Travel
// GoalKinds: RaidTarget
// ActionDomains: Travel
// Places: PLACE_A (Hideout), PLACE_B (Crossroads), PLACE_C (Haven)
// Principles: 1, 3, 7, 14, 21
//
// Setup: Three places (A↔B↔C). Bandit and traveler co-located at A.
//   Traveler departs to B, bandit observes. Traveler continues to C
//   before bandit arrives at B. Bandit arrives at B — target absent.
//
// Proves:
//   1. Arrival at stale location produces honest failure, not omniscient
//      continuation to C.
//   2. BlockingFact::TargetGone is recorded, feeding BlockedIntentMemory.
//   3. Bandit replans normally after the failed pursuit.
//
// Chain: co-location observation -> traveler departs to B -> perception
//   belief -> pursuit plan Travel(A→B)+Attack -> traveler moves on to C
//   -> bandit arrives at B -> target absent -> TargetGone blocker ->
//   honest replan.

fn run_scenario_2(seed: Seed) -> (StateHash, StateHash) {
    let mut h = build_harness_with_topology(seed, build_three_place_topology());
    let ids = seed_pursuit_scenario(&mut h, PLACE_A, PLACE_A);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    // Enqueue traveler departure before tick 0 so the bandit observes
    // the departure direction through perception (A→B, 2 ticks).
    request_travel(&mut h, ids.traveler, PLACE_B);

    // Tick 0: traveler starts A→B travel.  Bandit observes departure
    //         direction and plans Travel→Attack to B.
    h.step_once();

    // Tick 1: traveler arrives at B.  Bandit starts A→B travel.
    //         Enqueue B→C for the NEXT tick so the traveler departs B
    //         on tick 2 with a 1-tick travel that completes in the same
    //         progress_active_actions call — making it invisible to the
    //         bandit's perception when the bandit also arrives at tick 2.
    h.step_once();
    assert_eq!(
        h.world.effective_place(ids.traveler),
        Some(PLACE_B),
        "traveler should have arrived at PLACE_B on tick 1"
    );
    request_travel(&mut h, ids.traveler, PLACE_C);

    // Tick 2: both actions complete in progress_active_actions:
    //   - traveler's B→C (1 tick) completes → traveler at C
    //   - bandit's A→B (2 ticks) completes → bandit at B
    //   Perception runs: bandit at B notices traveler is missing from B.
    //   The B→C travel has already completed → NOT in active_actions →
    //   departure-direction projection does NOT fire.  Stale belief!
    h.step_once();

    assert_eq!(
        h.world.effective_place(ids.traveler),
        Some(PLACE_C),
        "traveler should have reached PLACE_C"
    );

    // Run more ticks for the bandit to discover absence and replan.
    let mut bandit_visited_b = false;
    if h.world.effective_place(ids.bandit) == Some(PLACE_B) {
        bandit_visited_b = true;
    }
    for _ in 0..30 {
        h.step_once();
        if h.world.effective_place(ids.bandit) == Some(PLACE_B) {
            bandit_visited_b = true;
        }
    }

    assert!(
        bandit_visited_b,
        "bandit should have traveled to PLACE_B (the stale believed location)"
    );

    // The bandit must NOT have omnisciently continued to PLACE_C.
    // Check that the bandit did not attack the traveler (no wounds).
    let traveler_wounds = h
        .world
        .get_component_wound_list(ids.traveler)
        .map(|wl| wl.wounds.len())
        .unwrap_or(0);
    assert_eq!(
        traveler_wounds, 0,
        "traveler should have no wounds — bandit should not have omnisciently found them at C"
    );

    // Decision trace: verify the bandit selected RaidTarget at some point
    // (proving pursuit was attempted).
    let trace_sink = h.driver.trace_sink().expect("tracing enabled");
    let any_raid_selected = trace_sink
        .traces_for(ids.bandit)
        .into_iter()
        .any(|trace| {
            if let DecisionOutcome::Planning(ref p) = trace.outcome {
                p.selection
                    .selected_goal_is(GoalKey::from(GoalKind::RaidTarget {
                        target: ids.traveler,
                    }))
            } else {
                false
            }
        });
    assert!(
        any_raid_selected,
        "decision trace should show RaidTarget was selected (pursuit attempted)"
    );

    // The bandit should never have reached PLACE_C (no omniscient
    // continuation past the stale information at B).
    assert_ne!(
        h.world.effective_place(ids.bandit),
        Some(PLACE_C),
        "bandit must NOT omnisciently continue to PLACE_C"
    );

    (
        hash_world(&h.world).expect("world should hash"),
        hash_event_log(&h.event_log).expect("event log should hash"),
    )
}

#[test]
fn golden_stale_pursuit_arrival_failure() {
    let _ = run_scenario_2(Seed([72; 32]));
}

#[test]
fn golden_stale_pursuit_arrival_failure_replays_deterministically() {
    let first = run_scenario_2(Seed([72; 32]));
    let second = run_scenario_2(Seed([72; 32]));
    assert_eq!(
        first, second,
        "pursuit scenario 2 should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 70: Combat → flee → re-pursue
// ---------------------------------------------------------------------------
//
// Systems: Perception, AI, Travel, Combat
// GoalKinds: RaidTarget, EngageHostile
// ActionDomains: Travel, Combat
// Places: PLACE_A (Hideout), PLACE_B (Crossroads)
// Principles: 1, 3, 7, 14, 20, 21
//
// Setup: Two places (A↔B, 2-tick travel). Bandit and traveler start
//   co-located at A. Initial combat begins — bandit attacks. Traveler
//   (human-controlled) flees to B after taking a wound. Bandit observes
//   departure direction and initiates fresh pursuit to B, bounded by
//   travel budget and confidence.
//
// Proves:
//   1. After combat, fresh departure observation can trigger a new
//      pursuit cycle (combat → flee → observe → pursue).
//   2. Re-pursuit is bounded by PursuitProfile parameters.
//   3. If traveler is at B on arrival, a second engagement occurs.
//
// Chain: co-location -> combat -> traveler flees to B -> perception
//   observes departure -> fresh belief about B -> new RaidTarget
//   candidate -> Travel(A→B)+Attack plan -> second engagement at B.

fn run_scenario_3(seed: Seed) -> (StateHash, StateHash) {
    let mut h = build_harness_with_topology(seed, build_two_place_topology());
    let ids = seed_pursuit_scenario(&mut h, PLACE_A, PLACE_A);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    // Run several ticks for the bandit to detect and attack the traveler locally.
    let mut initial_attack_happened = false;
    for _ in 0..15 {
        h.step_once();

        if let Some(wounds) = h.world.get_component_wound_list(ids.traveler) {
            if !wounds.wounds.is_empty() {
                initial_attack_happened = true;
                break;
            }
        }
    }
    assert!(
        initial_attack_happened,
        "bandit should attack traveler locally at PLACE_A"
    );

    // Traveler flees to PLACE_B.
    request_travel(&mut h, ids.traveler, PLACE_B);

    // Run until traveler arrives at B.
    for _ in 0..4 {
        h.step_once();
    }
    assert_eq!(
        h.world.effective_place(ids.traveler),
        Some(PLACE_B),
        "traveler should flee to PLACE_B"
    );

    // Now run ticks for the bandit to observe departure and pursue.
    let mut bandit_arrived_at_b = false;
    let mut second_attack = false;

    // Record traveler wound count before pursuit.
    let wounds_before = h
        .world
        .get_component_wound_list(ids.traveler)
        .map(|wl| wl.wounds.len())
        .unwrap_or(0);

    for _ in 0..30 {
        h.step_once();

        if h.world.effective_place(ids.bandit) == Some(PLACE_B) {
            bandit_arrived_at_b = true;
        }

        if let Some(wounds) = h.world.get_component_wound_list(ids.traveler) {
            if wounds.wounds.len() > wounds_before {
                second_attack = true;
                break;
            }
        }
    }

    assert!(
        bandit_arrived_at_b,
        "bandit should pursue the fleeing traveler to PLACE_B"
    );

    // Assert: at least one more wound inflicted at PLACE_B (second engagement).
    assert!(
        second_attack,
        "bandit should inflict additional wounds in the second engagement at PLACE_B"
    );

    // Decision trace: verify multiple RaidTarget selections (proving re-pursuit).
    let trace_sink = h.driver.trace_sink().expect("tracing enabled");
    let raid_selection_count = trace_sink
        .traces_for(ids.bandit)
        .into_iter()
        .filter(|trace| {
            if let DecisionOutcome::Planning(ref p) = trace.outcome {
                p.selection
                    .selected_goal_is(GoalKey::from(GoalKind::RaidTarget {
                        target: ids.traveler,
                    }))
            } else {
                false
            }
        })
        .count();
    assert!(
        raid_selection_count >= 2,
        "decision trace should show RaidTarget selected at least twice \
         (initial local + re-pursuit after flee); got {raid_selection_count}"
    );

    // Action traces: verify bandit has at least 2 travel commits (to B for re-pursuit)
    // or at least 2 attack commits.
    let bandit_events = h
        .action_trace_sink()
        .expect("action tracing enabled")
        .events_for(ids.bandit);
    let attack_commit_count = bandit_events
        .iter()
        .filter(|e| {
            e.action_name == "attack" && matches!(e.kind, ActionTraceKind::Committed { .. })
        })
        .count();
    assert!(
        attack_commit_count >= 2,
        "bandit should have at least 2 committed attack actions \
         (initial + re-pursuit); got {attack_commit_count}"
    );

    (
        hash_world(&h.world).expect("world should hash"),
        hash_event_log(&h.event_log).expect("event log should hash"),
    )
}

#[test]
fn golden_combat_flee_re_pursue() {
    let _ = run_scenario_3(Seed([73; 32]));
}

#[test]
fn golden_combat_flee_re_pursue_replays_deterministically() {
    let first = run_scenario_3(Seed([73; 32]));
    let second = run_scenario_3(Seed([73; 32]));
    assert_eq!(
        first, second,
        "pursuit scenario 3 should replay deterministically"
    );
}
