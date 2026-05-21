#![allow(dead_code)]

use crate::golden_harness::*;
use std::collections::BTreeMap;
use std::num::NonZeroU32;
use worldwake_ai::{DecisionOutcome, GoalKind, PlanSearchOutcome, PlannerOpKind};
use worldwake_core::{
    AgentData, BelievedActivity, CarryCapacity, CombatProfile, CommodityKind, ControlSource,
    DisposalProfile, DriveThresholds, EntityId, EventLog, ExplorationProfile, HomeostaticNeeds,
    IntentionDispositionProfile, IntentionDomainTag, LastSeenMemory, LoadUnits, MetabolismProfile,
    PatrolProfile, PatrolRoute, PerceptionProfile, Permille, Place, PlaceTag, PreferenceProfile,
    PursuitProfile, Quantity, ResourceSource, Seed, TheftDispositionProfile, Tick, Topology,
    TravelEdge, TravelEdgeId, UtilityProfile, ViolationDispositionProfile, WorkstationMarker,
    WorkstationTag, World, hash_event_log, hash_world,
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
                &[
                    PlaceTag::Village,
                    PlaceTag::Store,
                    PlaceTag::Hall,
                    PlaceTag::Gate,
                ],
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
            place(
                "Dusty Trail",
                &[PlaceTag::Trail, PlaceTag::Road, PlaceTag::Crossroads],
            ),
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
        .add_edge(
            TravelEdge::new(TravelEdgeId(840), ELDERGROVE_FOREST, DUSTY_TRAIL, 2, None).unwrap(),
        )
        .unwrap();
    topology
        .add_edge(
            TravelEdge::new(TravelEdgeId(850), HEARTHSTONE_INN, GOLDEN_FIELDS, 8, None).unwrap(),
        )
        .unwrap();
    topology
        .add_edge(
            TravelEdge::new(TravelEdgeId(851), GOLDEN_FIELDS, HEARTHSTONE_INN, 8, None).unwrap(),
        )
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

fn cli_scenario_seed(seed: u64) -> Seed {
    let mut bytes = [0_u8; 32];
    bytes[..8].copy_from_slice(&seed.to_le_bytes());
    Seed(bytes)
}

fn guard_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        entity_activation_threshold: pm(125),
        claim_confidence_threshold: pm(50),
        observation_buffer_capacity: 16,
        observation_budget: 24,
        salience_policy: worldwake_core::SaliencePolicy::default(),
        omission_log_capacity: worldwake_core::default_omission_log_capacity(),
        opportunity_floor_permille: worldwake_core::default_opportunity_floor_permille(),
        need_salience_boost: pm(500),
        need_salience_urgency_threshold: pm(500),
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

fn place_ground_commodity(
    world: &mut World,
    event_log: &mut EventLog,
    place: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
) {
    let mut txn = new_txn(world, 0);
    let lot = txn.create_item_lot(commodity, quantity).unwrap();
    txn.set_ground_location(lot, place).unwrap();
    commit_txn(txn, event_log);
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
            office_duty_weight: pm(500),
            loyalty_weight: pm(500),
            greed_weight: pm(500),
            shame_weight: pm(400),
            revenge_weight: pm(400),
        },
        worldwake_core::KnownRecipes::with([water_recipe]),
    );

    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        guard_perception_profile(),
    );

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_drive_thresholds(agent, DriveThresholds::default())
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

fn seed_forager_lina_cli_evaluation_slice(h: &mut GoldenHarness) -> EntityId {
    let harvest_apples = h
        .recipes
        .recipe_by_name("Harvest Apples")
        .expect("pathology harness should include Harvest Apples")
        .0;
    let lina = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Forager Lina",
        ELDERGROVE_FOREST,
        HomeostaticNeeds::new(pm(200), pm(600), pm(100), pm(100), pm(100)),
        MetabolismProfile {
            hunger_rate: pm(2),
            thirst_rate: pm(5),
            fatigue_rate: pm(2),
            bladder_rate: pm(4),
            dirtiness_rate: pm(1),
            rest_efficiency: pm(20),
            starvation_tolerance_ticks: NonZeroU32::new(480).unwrap(),
            dehydration_tolerance_ticks: NonZeroU32::new(180).unwrap(),
            exhaustion_collapse_ticks: NonZeroU32::new(120).unwrap(),
            bladder_accident_tolerance_ticks: NonZeroU32::new(40).unwrap(),
            toilet_ticks: NonZeroU32::new(8).unwrap(),
            wash_ticks: NonZeroU32::new(12).unwrap(),
            min_sleep_ticks: NonZeroU32::new(8).unwrap(),
            travel_fatigue_multiplier: pm(0),
            travel_thirst_multiplier: pm(0),
            travel_bladder_multiplier: pm(0),
            wilderness_relief_dirtiness_penalty: pm(0),
        },
        UtilityProfile {
            hunger_weight: pm(600),
            thirst_weight: pm(700),
            fatigue_weight: pm(300),
            bladder_weight: pm(200),
            dirtiness_weight: pm(200),
            pain_weight: pm(500),
            danger_weight: pm(400),
            enterprise_weight: pm(400),
            social_weight: pm(300),
            activity_awareness_weight: pm(500),
            side_benefit_weight: pm(500),
            bounty_posting_weight: pm(0),
            notice_posting_weight: pm(0),
            courage: pm(400),
            care_weight: pm(300),
            office_duty_weight: pm(500),
            loyalty_weight: pm(500),
            greed_weight: pm(500),
            shame_weight: pm(400),
            revenge_weight: pm(400),
        },
        worldwake_core::KnownRecipes::with([harvest_apples]),
    );

    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        lina,
        PerceptionProfile::default(),
    );
    set_agent_cognitive_profile(
        &mut h.world,
        &mut h.event_log,
        lina,
        worldwake_core::CognitiveProfile::default(),
    );
    set_agent_execution_budget(
        &mut h.world,
        &mut h.event_log,
        lina,
        worldwake_core::ExecutionBudget::default(),
    );

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_drive_thresholds(
        lina,
        DriveThresholds {
            hunger: worldwake_core::ThresholdBand::new(pm(250), pm(500), pm(750), pm(900)).unwrap(),
            thirst: worldwake_core::ThresholdBand::new(pm(100), pm(300), pm(550), pm(750)).unwrap(),
            fatigue: worldwake_core::ThresholdBand::new(pm(300), pm(550), pm(800), pm(920))
                .unwrap(),
            bladder: worldwake_core::ThresholdBand::new(pm(350), pm(600), pm(800), pm(930))
                .unwrap(),
            dirtiness: worldwake_core::ThresholdBand::new(pm(400), pm(650), pm(850), pm(950))
                .unwrap(),
            pain: worldwake_core::ThresholdBand::new(pm(150), pm(350), pm(600), pm(850)).unwrap(),
            danger: worldwake_core::ThresholdBand::new(pm(100), pm(300), pm(550), pm(800)).unwrap(),
        },
    )
    .unwrap();
    txn.set_component_preference_profile(
        lina,
        PreferenceProfile {
            route_caution_weight: pm(400),
            source_trust_weight: pm(300),
            route_memory_capacity: 24,
            source_memory_capacity: 18,
            memory_retention_ticks: 400,
            wait_sensitivity_weight: pm(150),
            capacity_observation_weight: pm(20),
        },
    )
    .unwrap();
    txn.set_component_theft_disposition_profile(
        lina,
        TheftDispositionProfile {
            steal_duration_ticks: NonZeroU32::new(3).unwrap(),
            theft_motive_weight: pm(200),
            witness_risk_penalty: pm(400),
        },
    )
    .unwrap();
    txn.set_component_exploration_profile(
        lina,
        ExplorationProfile {
            curiosity_weight: pm(650),
            need_activation_threshold: pm(350),
            visit_lookback_ticks: 150,
            ..ExplorationProfile::default()
        },
    )
    .unwrap();
    txn.set_component_disposal_profile(
        lina,
        DisposalProfile {
            capacity_strain_threshold: pm(700),
        },
    )
    .unwrap();
    txn.set_component_carry_capacity(lina, CarryCapacity(LoadUnits(20)))
        .unwrap();

    let chopping_block = txn.create_entity(worldwake_core::EntityKind::Facility);
    txn.set_ground_location(chopping_block, ELDERGROVE_FOREST)
        .unwrap();
    txn.set_component_workstation_marker(
        chopping_block,
        WorkstationMarker(WorkstationTag::ChoppingBlock),
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);

    place_ground_commodity(
        &mut h.world,
        &mut h.event_log,
        ELDERGROVE_FOREST,
        CommodityKind::Water,
        Quantity(5),
    );
    place_ground_commodity(
        &mut h.world,
        &mut h.event_log,
        ELDERGROVE_FOREST,
        CommodityKind::Apple,
        Quantity(8),
    );

    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ELDERGROVE_FOREST,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(20),
            max_quantity: Quantity(20),
            regeneration_ticks_per_unit: Some(NonZeroU32::new(2).unwrap()),
            last_regeneration_tick: None,
            extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
        },
        ProductionOutputOwner::Actor,
    );

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        lina,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );

    lina
}

fn run_obligation_satiation_allows_survival(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = build_pathology_harness(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let guard = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "S96 Guard",
        DUSTY_TRAIL,
        HomeostaticNeeds::new(pm(850), pm(850), pm(100), pm(100), pm(100)),
        MetabolismProfile::default(),
        UtilityProfile {
            hunger_weight: pm(500),
            thirst_weight: pm(500),
            fatigue_weight: pm(200),
            bladder_weight: pm(100),
            dirtiness_weight: pm(100),
            pain_weight: pm(300),
            danger_weight: pm(600),
            enterprise_weight: pm(0),
            social_weight: pm(0),
            activity_awareness_weight: pm(200),
            side_benefit_weight: pm(100),
            bounty_posting_weight: pm(0),
            notice_posting_weight: pm(900),
            courage: pm(700),
            care_weight: pm(200),
            office_duty_weight: pm(500),
            loyalty_weight: pm(500),
            greed_weight: pm(500),
            shame_weight: pm(400),
            revenge_weight: pm(400),
        },
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        guard,
        guard_perception_profile(),
    );

    let hostile = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "S96 Roadside Hostile",
        DUSTY_TRAIL,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, hostile, ControlSource::None, 0);

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        guard,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );

    {
        let mut belief = seed_belief_from_world(
            &mut h.world,
            &mut h.event_log,
            guard,
            hostile,
            Tick(0),
            worldwake_core::PerceptionSource::DirectObservation,
        );
        belief.believed_activity = Some(BelievedActivity {
            action_domain: worldwake_core::ActionDomain::Combat,
            target: Some(guard),
            observed_tick: Tick(0),
        });
        seed_belief(&mut h.world, &mut h.event_log, guard, hostile, belief);
    }

    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_ground_location(guard, HEARTHSTONE_INN).unwrap();
        txn.set_component_obligation_satiation_profile(
            guard,
            worldwake_core::ObligationSatiationProfile::default(),
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    let _bread = give_commodity(
        &mut h.world,
        &mut h.event_log,
        guard,
        HEARTHSTONE_INN,
        CommodityKind::Bread,
        Quantity(12),
    );
    let _water = give_commodity(
        &mut h.world,
        &mut h.event_log,
        guard,
        HEARTHSTONE_INN,
        CommodityKind::Water,
        Quantity(12),
    );
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        guard,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );

    for _ in 0..200_u32 {
        h.step_once();
    }

    let guard_events = h
        .action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(guard);
    let committed_events = guard_events
        .iter()
        .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
        .collect::<Vec<_>>();
    let post_notice_commits = committed_events
        .iter()
        .filter(|event| event.action_name == "post_notice")
        .count();
    let eat_commits = committed_events
        .iter()
        .filter(|event| event.action_name == "eat")
        .count();
    let drink_commits = committed_events
        .iter()
        .filter(|event| event.action_name == "drink")
        .count();
    let first_notice_commit = committed_events.iter().find_map(|event| {
        (event.action_name == "post_notice").then_some((event.tick, event.sequence_in_tick))
    });
    let first_self_care_after_notice = committed_events.iter().find_map(|event| {
        (matches!(event.action_name.as_str(), "eat" | "drink")
            && first_notice_commit
                .is_some_and(|first_notice| (event.tick, event.sequence_in_tick) > first_notice))
        .then_some((event.tick, event.sequence_in_tick))
    });
    let notice_commits_before_recovery = committed_events
        .iter()
        .filter(|event| event.action_name == "post_notice")
        .take_while(|event| {
            first_self_care_after_notice
                .is_none_or(|recovery| (event.tick, event.sequence_in_tick) < recovery)
        })
        .count();

    assert!(
        post_notice_commits > 2,
        "expected repeated post_notice commits so satiation actually matters; saw {post_notice_commits}"
    );
    assert!(
        eat_commits > 0,
        "guard should commit eat despite repeated notice pressure; actions={:?}",
        committed_events
            .iter()
            .map(|event| event.summary())
            .collect::<Vec<_>>()
    );
    assert!(
        drink_commits > 0,
        "guard should commit drink despite repeated notice pressure; actions={:?}",
        committed_events
            .iter()
            .map(|event| event.summary())
            .collect::<Vec<_>>()
    );
    assert!(
        first_notice_commit.is_some(),
        "expected at least one committed post_notice"
    );
    assert!(
        first_self_care_after_notice.is_some(),
        "expected self-care to reappear after the notice streak instead of obligation posting dominating indefinitely; actions={:?}",
        committed_events
            .iter()
            .map(|event| event.summary())
            .collect::<Vec<_>>()
    );
    assert!(
        notice_commits_before_recovery > 2,
        "expected a sustained notice streak before self-care recovered; notice_commits_before_recovery={notice_commits_before_recovery}"
    );
    assert!(
        post_notice_commits * 100 <= committed_events.len() * 80,
        "post_notice should not dominate more than 80% of committed actions; notice_commits={post_notice_commits} total_commits={}",
        committed_events.len()
    );
    assert!(
        !h.agent_is_dead(guard),
        "guard should survive the scenario instead of dying from need deprivation"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
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

pub fn assert_cross_location_water_acquisition_succeeds_without_budget_exhaustion() {
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
            extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
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

// ---------------------------------------------------------------------------
// Scenario 143: CLI Evaluation Lina 0-step FreeCarryCapacity Loop
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Production
// GoalKinds: FreeCarryCapacity
// ActionDomains: Needs, Production
// Places: EldergroveForest
// Principles: 3, 7, 20, 21
//
// Setup: Rebuild the exact Forager Lina Eldergrove Forest substrate from `scenarios/cli-evaluation.ron` using the live place graph, Lina's scenario profile values, 8 ground Apples, 5 ground Water, and the `Eldergrove Orchard` Apple source. The run uses the scenario seed `7777` and only seeds Lina's local Eldergrove beliefs, matching the observer report's locality boundary.
//
// Proves: After the real waste-accumulation phase from the cli-evaluation scenario, Lina no longer enters the observer-reported degenerate loop. The late-run decision window now lawfully includes both planning and active-action execution ticks: when planning occurs it either switches away from `FreeCarryCapacity` or produces executable disposal work, and the overall window still shows resumed eating and falling hunger.
//
// Chain: Eldergrove harvest/eat/waste accumulation -> carry strain assessed from actual carried load -> no spurious `FreeCarryCapacity` loop -> lawful self-care resumes -> late eat commit -> falling hunger.

pub fn assert_degenerate_zero_step_loop_blocks_actionable_goals() {
    let mut h = build_pathology_harness(cli_scenario_seed(7777));
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let lina = seed_forager_lina_cli_evaluation_slice(&mut h);
    let observation_start = 780_u32;
    let observation_end = 900_u32;
    let mut window_selected = 0_u32;
    let mut window_zero_step = 0_u32;
    let mut window_drop_item_plans = 0_u32;
    let mut window_decision_traces = 0_u32;
    let mut window_planning_traces = 0_u32;
    let mut window_active_action_traces = 0_u32;
    let mut hunger_at_window_start = None;
    let mut min_hunger_in_window = Permille::new(1000).unwrap();

    for tick in 0..observation_end {
        h.step_once();

        if tick == observation_start {
            hunger_at_window_start = Some(h.agent_hunger(lina));
        }

        if tick < observation_start {
            continue;
        }

        let hunger = h.agent_hunger(lina);
        min_hunger_in_window = min_hunger_in_window.min(hunger);

        let Some(trace) = h
            .driver
            .trace_sink()
            .and_then(|sink| sink.trace_at(lina, Tick(tick.into())))
        else {
            continue;
        };
        window_decision_traces += 1;

        let DecisionOutcome::Planning(planning) = &trace.outcome else {
            window_active_action_traces += 1;
            continue;
        };
        window_planning_traces += 1;

        if planning
            .selection
            .selected_goal()
            .is_some_and(|goal| goal.kind == GoalKind::FreeCarryCapacity)
        {
            window_selected += 1;
        }

        if planning.planning.attempts.iter().any(|attempt| {
            attempt.goal.kind == GoalKind::FreeCarryCapacity
                && matches!(
                    &attempt.outcome,
                    PlanSearchOutcome::Found { steps, .. } if steps.is_empty()
                )
        }) {
            window_zero_step += 1;
        }
        if planning.planning.attempts.iter().any(|attempt| {
            attempt.goal.kind == GoalKind::FreeCarryCapacity
                && matches!(
                    &attempt.outcome,
                    PlanSearchOutcome::Found { steps, .. }
                        if steps.iter().any(|step| step.op_kind == PlannerOpKind::DropItem)
                )
        }) {
            window_drop_item_plans += 1;
        }
    }

    let hunger_at_window_start =
        hunger_at_window_start.expect("observation window should capture a starting hunger value");
    let hunger_after = h.agent_hunger(lina);
    let late_eat_commit = h
        .action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(lina)
        .iter()
        .any(|event| {
            event.tick.0 >= u64::from(observation_start)
                && event.action_name == "eat"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        });

    assert!(
        window_decision_traces == observation_end.saturating_sub(observation_start),
        "expected one decision trace per late-run tick, saw {window_decision_traces} over window [{observation_start}, {observation_end})"
    );
    assert!(
        window_planning_traces + window_active_action_traces == window_decision_traces,
        "late-run traces should be fully accounted for by planning or active-action execution; planning={window_planning_traces} active_action={window_active_action_traces} total={window_decision_traces}"
    );
    assert!(
        window_zero_step == 0,
        "expected no repeated 0-step FreeCarryCapacity plans in the late-run cli-evaluation Lina window, saw {window_zero_step} zero-step attempts across {window_planning_traces} planning ticks"
    );
    assert!(
        window_selected == 0 || window_drop_item_plans > 0,
        "late-run FreeCarryCapacity should either disappear as a selected goal or produce executable DropItem plans; selections={window_selected} drop_item_plans={window_drop_item_plans} planning_ticks={window_planning_traces} active_action_ticks={window_active_action_traces}"
    );
    assert!(
        late_eat_commit,
        "expected a late-run eat commit once the FreeCarryCapacity loop is gone"
    );
    assert!(
        min_hunger_in_window < hunger_at_window_start,
        "expected hunger to fall somewhere during the late-run recovery window; start={hunger_at_window_start:?} min={min_hunger_in_window:?} end={hunger_after:?}"
    );
}

// ---------------------------------------------------------------------------
// Scenario 144: Obligation satiation allows survival needs to override posting
// ---------------------------------------------------------------------------
//
// Systems: Social artifact actions, Needs, AI, Perception
// GoalKinds: PostNotice, AcquireCommodity(SelfConsume)
// ActionDomains: Social, Needs
// Places: DustyTrail, HearthstoneInn
// Principles: 3, 7, 11, 22, 26
//
// Setup: A guard first directly observes a hostile at Dusty Trail, then starts
//   the scenario window at Hearthstone Inn with possessed Bread and Water, critical
//   hunger/thirst, `notice_posting_weight=900`, and the default obligation
//   satiation profile. The remembered combat belief keeps a remote
//   `ThreatWarning` notice branch lawful without reintroducing co-located combat
//   as a competing explanation.
//
// Proves: Repeated `PostNotice` still happens enough for satiation to matter,
//   but the guard also commits both `eat` and `drink`, survives the window, and
//   does not let notice posting dominate the committed action mix indefinitely.
//
// Chain: remembered hostile belief -> repeated PostNotice commits append
//   obligation tracker state -> ranking dampens saturated notice motive ->
//   self-care commits become competitive -> guard survives.

pub fn assert_obligation_satiation_allows_survival_needs_to_override_posting() {
    let _ = run_obligation_satiation_allows_survival(Seed([144; 32]));
}
