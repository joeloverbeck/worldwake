//! Golden tests proving per-agent reasoning diversity via split execution budgets.

mod golden_harness;

use golden_harness::*;
use worldwake_ai::{DecisionOutcome, PlannerOpKind, SelectedPlanSource};
use worldwake_core::{
    BeliefConfidencePolicy, CognitiveProfile, CommodityKind, EntityId, ExecutionBudget,
    HomeostaticNeeds, KnownRecipes, MetabolismProfile, PerceptionProfile, Quantity, Seed,
    StateHash, Tick, UtilityProfile, WorkstationTag, hash_event_log, hash_world,
};

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
            memory_capacity: 64,
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
