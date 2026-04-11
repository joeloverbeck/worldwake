mod golden_harness;

use golden_harness::*;
use std::collections::BTreeMap;
use std::num::NonZeroU32;
use worldwake_ai::{DecisionOutcome, GoalKind, PlanSearchOutcome};
use worldwake_core::{
    CombatProfile, CommodityKind, DriveThresholds, EntityId, EventLog, HomeostaticNeeds,
    IntentionDomainTag, IntentionDispositionProfile, LastSeenMemory, MetabolismProfile,
    PatrolProfile, PatrolRoute, PerceptionProfile, Place, PlaceTag, PursuitProfile, Quantity,
    ResourceSource, Seed, Tick, Topology, TravelEdge, TravelEdgeId, UtilityProfile,
    ViolationDispositionProfile, WorkstationTag, World,
};
use worldwake_sim::{ActionTraceKind, ControllerState, Scheduler, SystemManifest};

const THORNWALL_VILLAGE: EntityId = entity(700);
const DUSTY_TRAIL: EntityId = entity(701);
const ELDERGROVE_FOREST: EntityId = entity(702);
const HEARTHSTONE_INN: EntityId = entity(703);
const GOLDEN_FIELDS: EntityId = entity(704);

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

fn build_cli_evaluation_topology() -> Topology {
    let mut topology = Topology::new();
    topology
        .add_place(
            THORNWALL_VILLAGE,
            place(
                "Thornwall Village",
                &[PlaceTag::Village, PlaceTag::Store, PlaceTag::Hall, PlaceTag::Gate],
            ),
        )
        .unwrap();
    topology
        .add_place(
            ELDERGROVE_FOREST,
            place("Eldergrove Forest", &[PlaceTag::Forest, PlaceTag::Camp]),
        )
        .unwrap();
    topology
        .add_place(
            DUSTY_TRAIL,
            place("Dusty Trail", &[PlaceTag::Trail, PlaceTag::Road, PlaceTag::Crossroads]),
        )
        .unwrap();
    topology
        .add_place(
            HEARTHSTONE_INN,
            place(
                "Hearthstone Inn",
                &[PlaceTag::Inn, PlaceTag::Latrine, PlaceTag::Barracks],
            ),
        )
        .unwrap();
    topology
        .add_place(
            GOLDEN_FIELDS,
            place("Golden Fields", &[PlaceTag::Field, PlaceTag::Farm]),
        )
        .unwrap();

    connect(&mut topology, 800, THORNWALL_VILLAGE, ELDERGROVE_FOREST, 3);
    connect(&mut topology, 810, THORNWALL_VILLAGE, DUSTY_TRAIL, 2);
    connect(&mut topology, 820, THORNWALL_VILLAGE, HEARTHSTONE_INN, 4);
    connect(&mut topology, 830, THORNWALL_VILLAGE, GOLDEN_FIELDS, 5);
    topology
        .add_edge(TravelEdge::new(
            TravelEdgeId(840),
            ELDERGROVE_FOREST,
            DUSTY_TRAIL,
            2,
            None,
        )
        .unwrap())
        .unwrap();
    topology
        .add_edge(TravelEdge::new(
            TravelEdgeId(850),
            HEARTHSTONE_INN,
            GOLDEN_FIELDS,
            8,
            None,
        )
        .unwrap())
        .unwrap();
    topology
        .add_edge(TravelEdge::new(
            TravelEdgeId(851),
            GOLDEN_FIELDS,
            HEARTHSTONE_INN,
            8,
            None,
        )
        .unwrap())
        .unwrap();
    topology
}

fn build_pathology_harness(seed: Seed) -> GoldenHarness {
    let mut h = GoldenHarness::with_recipes(seed, build_multi_recipe_registry());
    h.world = World::new(build_cli_evaluation_topology()).unwrap();
    h.event_log = EventLog::new();
    h.scheduler = Scheduler::new(SystemManifest::canonical());
    h.controller = ControllerState::new();
    h
}

fn guard_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        entity_memory_capacity: 16,
        entity_claim_capacity: 16,
        memory_retention_ticks: 64,
        observation_fidelity: pm(950),
        confidence_policy: worldwake_core::BeliefConfidencePolicy::default(),
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
    }
}

fn planning_trace_at(
    h: &GoldenHarness,
    agent: EntityId,
    tick: Tick,
) -> Option<&worldwake_ai::PlanningPipelineTrace> {
    let trace = h.driver.trace_sink()?.trace_at(agent, tick)?;
    match &trace.outcome {
        DecisionOutcome::Planning(planning) => Some(planning),
        _ => None,
    }
}

fn seed_guard_theron(h: &mut GoldenHarness) -> EntityId {
    let water_recipe = h
        .recipes
        .recipe_by_name("Harvest Water")
        .expect("pathology harness should include Harvest Water")
        .0;
    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Guard Theron",
        DUSTY_TRAIL,
        HomeostaticNeeds::new(pm(300), pm(800), pm(100), pm(100), pm(100)),
        MetabolismProfile::default(),
        UtilityProfile {
            hunger_weight: pm(400),
            thirst_weight: pm(400),
            fatigue_weight: pm(300),
            bladder_weight: pm(200),
            dirtiness_weight: pm(200),
            pain_weight: pm(600),
            danger_weight: pm(800),
            enterprise_weight: pm(200),
            social_weight: pm(300),
            activity_awareness_weight: pm(400),
            side_benefit_weight: pm(500),
            bounty_posting_weight: pm(700),
            notice_posting_weight: pm(900),
            courage: pm(850),
            care_weight: pm(400),
        },
        worldwake_core::KnownRecipes::with([water_recipe]),
    );

    set_agent_perception_profile(&mut h.world, &mut h.event_log, agent, guard_perception_profile());

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_drive_thresholds(
        agent,
        DriveThresholds::default(),
    )
    .unwrap();
    txn.set_component_combat_profile(
        agent,
        CombatProfile::new(
            pm(900),
            pm(750),
            pm(600),
            pm(550),
            pm(150),
            pm(400),
            pm(60),
            pm(250),
            pm(120),
            NonZeroU32::new(3).unwrap(),
            NonZeroU32::new(10).unwrap(),
        ),
    )
    .unwrap();
    txn.set_component_patrol_profile(
        agent,
        PatrolProfile {
            base_dwell_ticks: 5,
            dwell_vigilance_scale_ticks: 3,
            vigilance: pm(700),
            route_adaptation_sensitivity: pm(400),
            patrol_motive_weight: pm(600),
        },
    )
    .unwrap();
    txn.set_component_patrol_route(
        agent,
        PatrolRoute {
            assigned_places: vec![DUSTY_TRAIL, THORNWALL_VILLAGE],
            current_index: 0,
        },
    )
    .unwrap();
    txn.set_component_pursuit_profile(
        agent,
        PursuitProfile {
            min_location_confidence: pm(500),
            max_pursuit_travel_ticks: NonZeroU32::new(8).unwrap(),
        },
    )
    .unwrap();
    txn.set_component_intention_disposition_profile(
        agent,
        IntentionDispositionProfile {
            domain_patience: BTreeMap::from([
                (IntentionDomainTag::Travel, NonZeroU32::new(15).unwrap()),
                (IntentionDomainTag::Care, NonZeroU32::new(20).unwrap()),
            ]),
            default_patience_ticks: NonZeroU32::new(30).unwrap(),
            commitment_switch_margin: pm(200),
        },
    )
    .unwrap();
    txn.set_component_violation_disposition_profile(
        agent,
        ViolationDispositionProfile {
            investigation_duration_ticks: NonZeroU32::new(3).unwrap(),
            violation_memory_retention_ticks: 100,
            investigation_motive_weight: pm(600),
            ownership_motive_bonus: pm(300),
        },
    )
    .unwrap();
    txn.set_component_last_seen_memory(agent, LastSeenMemory::default())
        .unwrap();
    commit_txn(txn, &mut h.event_log);

    agent
}

// ---------------------------------------------------------------------------
// Scenario 142: Dusty Trail Remote Water Acquisition Recovery
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Travel, Production
// GoalKinds: AcquireCommodity
// ActionDomains: Travel, Production, Needs
// Places: DustyTrail, ThornwallVillage
// Principles: 7, 14, 20
//
// Setup: A Dusty Trail guard uses the `cli-evaluation.ron` place graph slice:
//   Thornwall Village with a well 2 ticks away, Dusty Trail as the starting
//   place, and the guard's patrol-style profile boundary. The guard starts
//   thirsty enough for water acquisition to compete immediately.
//
// Proves: The exact Dusty Trail-style cross-location water path now produces a
//   lawful `AcquireCommodity(Water)` plan without budget exhaustion, commits a
//   `drink`, and lowers thirst within the scenario window.
//
// Chain: Dusty Trail thirst pressure -> AcquireCommodity(Water) found plan ->
//   travel to Thornwall Village -> committed drink -> reduced thirst.

#[test]
fn cross_location_water_acquisition_succeeds_without_budget_exhaustion() {
    let mut h = build_pathology_harness(Seed([201; 32]));
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let guard = seed_guard_theron(&mut h);

    let village_well = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        THORNWALL_VILLAGE,
        WorkstationTag::Well,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(15),
            max_quantity: Quantity(15),
            regeneration_ticks_per_unit: Some(NonZeroU32::new(3).unwrap()),
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        guard,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        guard,
        THORNWALL_VILLAGE,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        guard,
        village_well,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    let thirst_before = h.agent_thirst(guard).value();
    let mut saw_budget_exhausted = false;
    let mut saw_found_plan = false;
    for tick in 0..60_u32 {
        h.step_once();
        if let Some(planning) = planning_trace_at(&h, guard, Tick(tick.into())) {
            for attempt in &planning.planning.attempts {
                if !matches!(
                    attempt.goal.kind,
                    GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Water,
                        ..
                    }
                ) {
                    continue;
                }
                saw_budget_exhausted |=
                    matches!(attempt.outcome, PlanSearchOutcome::BudgetExhausted { .. });
                saw_found_plan |= matches!(
                    &attempt.outcome,
                    PlanSearchOutcome::Found { steps, .. } if !steps.is_empty()
                );
            }
        }
    }

    let thirst_after = h.agent_thirst(guard).value();
    let drank_water = h
        .action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(guard)
        .iter()
        .any(|event| {
            event.action_name == "drink" && matches!(event.kind, ActionTraceKind::Committed { .. })
        });

    assert!(
        saw_found_plan,
        "expected a Dusty Trail AcquireCommodity(Water) plan within the scenario window"
    );
    assert!(
        !saw_budget_exhausted,
        "Dusty Trail AcquireCommodity(Water) should no longer budget-exhaust after the prerequisite-guidance fix"
    );
    assert!(
        drank_water,
        "guard should commit drink once the Dusty Trail remote-water plan is found"
    );
    assert!(
        thirst_after < thirst_before,
        "thirst should fall after the Dusty Trail remote-water plan completes"
    );
}
