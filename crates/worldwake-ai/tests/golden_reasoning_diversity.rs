//! Golden tests proving per-agent reasoning diversity via split execution budgets.

mod golden_harness;

use golden_harness::*;
use worldwake_ai::{DecisionOutcome, GoalKind, PlannerOpKind, SelectedPlanSource};
use worldwake_core::{
    BeliefConfidencePolicy, CognitiveProfile, CommodityKind, EntityId, ExecutionBudget,
    HomeostaticNeeds, KnownRecipes, MetabolismProfile, PerceptionProfile, Quantity, Seed,
    StateHash, Tick, UtilityProfile, WorkstationTag, hash_event_log, hash_world,
};
use worldwake_sim::ActionTraceKind;

fn planning_trace_at(
    h: &GoldenHarness,
    agent: EntityId,
    tick: Tick,
) -> &worldwake_ai::PlanningPipelineTrace {
    let trace = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(agent, tick)
        .unwrap_or_else(|| panic!("missing planning trace for agent {agent} at tick {tick:?}"));
    match &trace.outcome {
        DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected planning trace at tick {tick:?}, got {other:?}"),
    }
}

fn configure_perception(h: &mut GoldenHarness, agent: EntityId) {
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        PerceptionProfile {
            entity_memory_capacity: 64,
            entity_claim_capacity: 64,
            memory_retention_ticks: 240,
            observation_fidelity: pm(875),
            confidence_policy: BeliefConfidencePolicy::default(),
            institutional_memory_capacity: 20,
            consultation_speed_factor: pm(500),
            contradiction_tolerance: pm(300),
        },
    );
}

fn setup_search_depth_harness(
    seed: Seed,
    cognitive_profile: CognitiveProfile,
    execution_budget: ExecutionBudget,
) -> (GoldenHarness, EntityId) {
    let mut h = GoldenHarness::with_recipes(seed, build_multi_recipe_registry());
    let bread_recipe = h
        .recipes
        .recipe_by_name("Bake Bread")
        .map(|(id, _)| id)
        .expect("bake bread recipe should exist");

    let baker = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Baker",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::with([bread_recipe]),
    );
    configure_perception(&mut h, baker);
    set_agent_cognitive_profile(&mut h.world, &mut h.event_log, baker, cognitive_profile);
    set_agent_execution_budget(&mut h.world, &mut h.event_log, baker, execution_budget);

    place_workstation(
        &mut h.world,
        &mut h.event_log,
        VILLAGE_SQUARE,
        WorkstationTag::Mill,
        ProductionOutputOwner::Actor,
    );

    let mut txn = new_txn(&mut h.world, 0);
    let firewood = txn
        .create_item_lot(CommodityKind::Firewood, Quantity(1))
        .expect("remote firewood lot should be creatable");
    txn.set_ground_location(firewood, ORCHARD_FARM).unwrap();
    commit_txn(txn, &mut h.event_log);

    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        baker,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );

    h.driver.enable_tracing();
    (h, baker)
}

fn run_search_depth_hashes(seed: Seed) -> (StateHash, StateHash, StateHash, StateHash) {
    let tight_reasoning = CognitiveProfile {
        max_node_expansions: 2,
        ..CognitiveProfile::default()
    };
    let (mut tight, _tight_agent) =
        setup_search_depth_harness(seed, tight_reasoning, ExecutionBudget::default());
    let (mut thorough, _thorough_agent) = setup_search_depth_harness(
        seed,
        CognitiveProfile::default(),
        ExecutionBudget::default(),
    );

    tight.step_once();
    thorough.step_once();

    for _ in 0..12 {
        tight.step_once();
        thorough.step_once();
    }

    (
        hash_world(&tight.world).unwrap(),
        hash_event_log(&tight.event_log).unwrap(),
        hash_world(&thorough.world).unwrap(),
        hash_event_log(&thorough.event_log).unwrap(),
    )
}

fn run_landmark_depth_hashes(seed: Seed) -> (StateHash, StateHash, StateHash, StateHash) {
    let zero_landmarks = CognitiveProfile {
        landmark_extraction_depth: 0,
        ..CognitiveProfile::default()
    };
    let (mut zero_h, _zero_agent) =
        setup_search_depth_harness(seed, zero_landmarks, ExecutionBudget::default());
    let (mut default_h, _default_agent) = setup_search_depth_harness(
        seed,
        CognitiveProfile::default(),
        ExecutionBudget::default(),
    );

    zero_h.step_once();
    default_h.step_once();

    for _ in 0..12 {
        zero_h.step_once();
        default_h.step_once();
    }

    (
        hash_world(&zero_h.world).unwrap(),
        hash_event_log(&zero_h.event_log).unwrap(),
        hash_world(&default_h.world).unwrap(),
        hash_event_log(&default_h.event_log).unwrap(),
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct UtilityProfileDiversityObservation {
    selected_goals: Vec<String>,
    first_started_actions: Vec<String>,
}

fn utility_diversity_profile(hunger_weight: u16, thirst_weight: u16) -> UtilityProfile {
    UtilityProfile {
        hunger_weight: pm(hunger_weight),
        thirst_weight: pm(thirst_weight),
        ..UtilityProfile::default()
    }
}

fn selected_goal_label(goal: GoalKind) -> String {
    match goal {
        GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        } => "eat".to_string(),
        GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Water,
        } => "drink".to_string(),
        other => format!("{other:?}"),
    }
}

fn first_started_action_name(h: &GoldenHarness, agent: EntityId) -> Option<String> {
    h.action_trace_sink()?
        .events_for(agent)
        .iter()
        .find(|event| matches!(event.kind, ActionTraceKind::Started { .. }))
        .map(|event| event.action_name.clone())
}

fn run_utility_profile_diversity(seed: Seed) -> UtilityProfileDiversityObservation {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let agents = [
        seed_agent(
            &mut h.world,
            &mut h.event_log,
            "HungerDriven",
            VILLAGE_SQUARE,
            HomeostaticNeeds::new(pm(900), pm(900), pm(0), pm(0), pm(0)),
            MetabolismProfile::default(),
            utility_diversity_profile(900, 100),
        ),
        seed_agent(
            &mut h.world,
            &mut h.event_log,
            "ThirstDriven",
            VILLAGE_SQUARE,
            HomeostaticNeeds::new(pm(900), pm(900), pm(0), pm(0), pm(0)),
            MetabolismProfile::default(),
            utility_diversity_profile(100, 900),
        ),
    ];

    for agent in agents {
        configure_perception(&mut h, agent);
        give_commodity(
            &mut h.world,
            &mut h.event_log,
            agent,
            VILLAGE_SQUARE,
            CommodityKind::Bread,
            Quantity(1),
        );
        give_commodity(
            &mut h.world,
            &mut h.event_log,
            agent,
            VILLAGE_SQUARE,
            CommodityKind::Water,
            Quantity(1),
        );
        seed_actor_local_beliefs(
            &mut h.world,
            &mut h.event_log,
            agent,
            Tick(0),
            worldwake_core::PerceptionSource::DirectObservation,
        );
    }

    h.step_once();

    let selected_goals = agents
        .iter()
        .map(|agent| {
            let planning = planning_trace_at(&h, *agent, Tick(0));
            selected_goal_label(
                planning
                    .selection
                    .selected_goal()
                    .expect("each agent should select a self-care goal at tick 0")
                    .kind,
            )
        })
        .collect::<Vec<_>>();

    let mut first_started_actions = agents
        .iter()
        .map(|agent| first_started_action_name(&h, *agent))
        .collect::<Vec<_>>();
    for _ in 0..4 {
        if first_started_actions.iter().all(Option::is_some) {
            break;
        }
        h.step_once();
        first_started_actions = agents
            .iter()
            .map(|agent| first_started_action_name(&h, *agent))
            .collect::<Vec<_>>();
    }

    UtilityProfileDiversityObservation {
        selected_goals,
        first_started_actions: first_started_actions
            .into_iter()
            .map(|action| action.expect("each agent should start its selected self-care action"))
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Scenario 97: Search Depth Drives Multi-Step Plan Divergence
// ---------------------------------------------------------------------------
//
// Systems: Production, AI, Travel
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity
// ActionDomains: Production, Travel, Needs
// Places: VillageSquare, OrchardFarm
// Principles: 20
//
// Setup: Two isolated harness runs share the same baker, recipe registry,
//   remote firewood input, beliefs, and RNG seed. The only difference is
//   `CognitiveProfile.max_node_expansions`: tight budget `2` vs default.
//
// Proves: Per-agent reasoning style changes which multi-step plan search can
//   actually select. The default budget finds the remote input -> return ->
//   craft chain, while the tight budget fails to select that plan from the
//   same tick-0 planning boundary.
//
// Chain: Shared initial state -> same candidates generated -> search budget
//   caps expansion depth -> default run finds remote craft plan -> tight run
//   fails to select the same plan.

#[test]
fn search_depth_divergence() {
    let seed = Seed([32; 32]);
    let tight_reasoning = CognitiveProfile {
        max_node_expansions: 2,
        ..CognitiveProfile::default()
    };
    let (mut tight_h, tight_agent) =
        setup_search_depth_harness(seed, tight_reasoning, ExecutionBudget::default());
    let (mut thorough_h, thorough_agent) = setup_search_depth_harness(
        seed,
        CognitiveProfile::default(),
        ExecutionBudget::default(),
    );

    tight_h.step_once();
    thorough_h.step_once();

    let tight_planning = planning_trace_at(&tight_h, tight_agent, Tick(0));
    let thorough_planning = planning_trace_at(&thorough_h, thorough_agent, Tick(0));

    assert_eq!(
        thorough_planning.selection.selected_plan_source,
        Some(SelectedPlanSource::SearchSelection),
        "default reasoning should choose a fresh searched plan"
    );
    let thorough_plan = thorough_planning
        .selection
        .selected_plan
        .as_ref()
        .expect("default reasoning should select a multi-step plan");
    assert!(
        thorough_plan
            .steps
            .iter()
            .any(|step| step.op_kind == PlannerOpKind::Travel && step.targets == vec![ORCHARD_FARM]),
        "default reasoning plan should travel to Orchard Farm for the remote firewood"
    );
    assert!(
        thorough_plan
            .steps
            .iter()
            .any(|step| step.op_kind == PlannerOpKind::MoveCargo),
        "default reasoning plan should include remote firewood pickup"
    );
    assert!(
        thorough_plan
            .steps
            .iter()
            .any(|step| step.op_kind == PlannerOpKind::Craft),
        "default reasoning plan should include the bake step"
    );
    assert!(
        thorough_planning.selection.selected_goal().is_some(),
        "default reasoning should still select a concrete goal; planning={thorough_planning:?}"
    );

    assert!(
        tight_planning.selection.selected_plan.is_none(),
        "tight reasoning should fail to select the remote craft plan under the reduced expansion budget"
    );
    assert_eq!(
        tight_planning.selection.selected_plan_source, None,
        "tight reasoning should stop before a plan source is recorded"
    );
}

#[test]
fn search_depth_divergence_replays_deterministically() {
    let first = run_search_depth_hashes(Seed([32; 32]));
    let second = run_search_depth_hashes(Seed([32; 32]));
    assert_eq!(
        first, second,
        "search-depth divergence scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 141: Landmark Depth Changes Two-Phase Trace Shape Without Breaking Planning
// ---------------------------------------------------------------------------
//
// Systems: Production, AI, Travel
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity
// ActionDomains: Production, Travel, Needs
// Places: VillageSquare, OrchardFarm
// Principles: 20, 22
//
// Setup: Two isolated harness runs share the same baker, recipe registry,
//   remote firewood input, beliefs, and RNG seed. The only difference is
//   `CognitiveProfile.landmark_extraction_depth`: disabled `0` versus default.
//
// Proves: Landmark-depth diversity changes two-phase planner guidance and trace
//   metadata without breaking the live remote-input craft plan. The default run
//   extracts landmarks and preferred candidates; the zero-depth run degrades
//   gracefully to zero landmark counts while still selecting the lawful remote
//   pickup -> return -> craft plan.
//
// Chain: Shared initial state -> same remote production need -> depth-driven
//   landmark extraction difference -> divergent planner trace guidance ->
//   both runs still produce the same lawful remote craft plan family.

#[test]
fn landmark_depth_divergence() {
    let zero_landmarks = CognitiveProfile {
        landmark_extraction_depth: 0,
        ..CognitiveProfile::default()
    };
    let (mut zero_h, zero_agent) =
        setup_search_depth_harness(Seed([141; 32]), zero_landmarks, ExecutionBudget::default());
    let (mut default_h, default_agent) = setup_search_depth_harness(
        Seed([141; 32]),
        CognitiveProfile::default(),
        ExecutionBudget::default(),
    );

    zero_h.step_once();
    default_h.step_once();

    let zero_planning = planning_trace_at(&zero_h, zero_agent, Tick(0));
    let default_planning = planning_trace_at(&default_h, default_agent, Tick(0));

    for planning in [zero_planning, default_planning] {
        assert_eq!(
            planning.selection.selected_plan_source,
            Some(SelectedPlanSource::SearchSelection),
            "landmark-depth variation should still select a fresh searched plan"
        );
        let plan = planning
            .selection
            .selected_plan
            .as_ref()
            .expect("landmark-depth variation should preserve the remote craft plan");
        assert!(
            plan.steps.iter().any(
                |step| step.op_kind == PlannerOpKind::Travel && step.targets == vec![ORCHARD_FARM]
            ),
            "plan should still travel to Orchard Farm for the remote firewood"
        );
        assert!(
            plan.steps
                .iter()
                .any(|step| step.op_kind == PlannerOpKind::MoveCargo),
            "plan should still include remote firewood pickup"
        );
        assert!(
            plan.steps
                .iter()
                .any(|step| step.op_kind == PlannerOpKind::Craft),
            "plan should still include the bake step"
        );
    }

    let zero_attempt = zero_planning
        .planning
        .attempts
        .iter()
        .find(|attempt| matches!(attempt.outcome, worldwake_ai::PlanSearchOutcome::Found { .. }))
        .expect("zero-depth run should include a successful search attempt");
    let default_attempt = default_planning
        .planning
        .attempts
        .iter()
        .find(|attempt| matches!(attempt.outcome, worldwake_ai::PlanSearchOutcome::Found { .. }))
        .expect("default-depth run should include a successful search attempt");

    assert_eq!(
        zero_attempt.landmarks_extracted, 0,
        "zero landmark depth should disable landmark extraction"
    );
    assert_eq!(
        zero_attempt.landmark_orderings, 0,
        "zero landmark depth should disable landmark ordering extraction"
    );
    assert!(
        zero_attempt
            .expansion_summaries
            .iter()
            .all(|summary| summary.preferred_candidates == 0),
        "zero landmark depth should not mark preferred candidates"
    );
    assert!(
        zero_attempt
            .expansion_summaries
            .iter()
            .all(|summary| summary.landmark_heuristic == 0),
        "zero landmark depth should degrade to zero landmark heuristic"
    );

    assert!(
        default_attempt.landmarks_extracted > 0,
        "default landmark depth should extract at least one landmark"
    );
    assert!(
        default_attempt
            .expansion_summaries
            .iter()
            .any(|summary| summary.preferred_candidates > 0),
        "default landmark depth should expose preferred candidates"
    );
    assert!(
        default_attempt
            .expansion_summaries
            .iter()
            .any(|summary| summary.landmark_heuristic > 0),
        "default landmark depth should expose a positive landmark heuristic"
    );
}

#[test]
fn landmark_depth_divergence_replays_deterministically() {
    let first = run_landmark_depth_hashes(Seed([141; 32]));
    let second = run_landmark_depth_hashes(Seed([141; 32]));
    assert_eq!(
        first, second,
        "landmark-depth divergence scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 129: Utility Profiles Diverge Under Identical Self-Care Pressure
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI
// GoalKinds: ConsumeOwnedCommodity
// ActionDomains: Needs
// Places: VillageSquare
// Principles: 20, 22
//
// Setup: Two AI agents share the same place, the same critical hunger/thirst
//   state, and the same owned local bread+water substrate. Only their
//   `UtilityProfile` weights differ: hunger-driven versus thirst-driven.
//
// Proves: UtilityProfile divergence alone changes both the tick-0 selected
//   self-care goal and the first started self-care action: `eat` versus
//   `drink`.
//
// Cross-system chain: identical local need state + identical local substrate ->
//   utility-weighted candidate ranking -> divergent selected goals ->
//   divergent self-care action starts.

#[test]
fn golden_utility_profile_diversity() {
    let observation = run_utility_profile_diversity(Seed([179; 32]));

    assert_eq!(
        observation.selected_goals,
        vec!["eat", "drink"],
        "utility divergence should change tick-0 selected goals under identical self-care pressure; observation={observation:?}"
    );
    assert_eq!(
        observation.first_started_actions,
        vec!["eat", "drink"],
        "utility divergence should produce distinct first self-care actions under identical local substrate; observation={observation:?}"
    );
}

#[test]
fn golden_utility_profile_diversity_replays_deterministically() {
    let first = run_utility_profile_diversity(Seed([179; 32]));
    let second = run_utility_profile_diversity(Seed([179; 32]));
    assert_eq!(first, second);
}
