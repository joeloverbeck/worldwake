//! Golden tests for S145 strategic-budget scaling.

use crate::golden_harness::*;
use worldwake_ai::{DecisionOutcome, PlanSearchOutcome};
use worldwake_core::{
    BodyCostPerTick, CommodityKind, EntityId, ExecutionBudget, GoalKind, HomeostaticNeeds,
    KnownRecipes, MetabolismProfile, PerceptionProfile, Quantity, Seed, Tick, UtilityProfile,
    WorkstationTag,
};
use worldwake_sim::{RecipeDefinition, RecipeRegistry};

const EXPECTED_STAGE_COUNT: u16 = 5;

#[derive(Debug, Eq, PartialEq)]
struct BudgetObservation {
    stages_count: u16,
    budget_total: u32,
    budget_used: u32,
    exhausted: bool,
    strategic_sub_goals: Vec<String>,
    budget_exhausted_outcome: bool,
}

fn build_five_stage_recipe_registry() -> (RecipeRegistry, worldwake_core::RecipeId) {
    let mut recipes = RecipeRegistry::new();
    let recipe_id = recipes.register(RecipeDefinition {
        name: "Bake Budget Loaf".to_string(),
        inputs: vec![
            (CommodityKind::Firewood, Quantity(1)),
            (CommodityKind::Sword, Quantity(1)),
            (CommodityKind::Bow, Quantity(1)),
            (CommodityKind::Medicine, Quantity(1)),
        ],
        outputs: vec![(CommodityKind::Bread, Quantity(1))],
        work_ticks: nz(3),
        required_workstation_tag: Some(WorkstationTag::Mill),
        required_tool_kinds: vec![],
        body_cost_per_tick: BodyCostPerTick::new(pm(3), pm(2), pm(5), pm(0), pm(1)),
    });
    (recipes, recipe_id)
}

fn setup_five_stage_budget_harness(
    seed: Seed,
) -> (GoldenHarness, EntityId, worldwake_core::RecipeId) {
    let (recipes, recipe_id) = build_five_stage_recipe_registry();
    let mut h = GoldenHarness::with_recipes(seed, recipes);
    let baker = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Budget Baker",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::with([recipe_id]),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        baker,
        PerceptionProfile {
            observation_buffer_capacity: 64,
            observation_budget: 24,
            ..PerceptionProfile::default()
        },
    );
    set_agent_execution_budget(
        &mut h.world,
        &mut h.event_log,
        baker,
        ExecutionBudget::default(),
    );

    place_workstation(
        &mut h.world,
        &mut h.event_log,
        VILLAGE_SQUARE,
        WorkstationTag::Mill,
        ProductionOutputOwner::Actor,
    );

    let mut txn = new_txn(&mut h.world, 0);
    for commodity in [
        CommodityKind::Firewood,
        CommodityKind::Sword,
        CommodityKind::Bow,
        CommodityKind::Medicine,
    ] {
        let lot = txn
            .create_item_lot(commodity, Quantity(1))
            .expect("prerequisite lot should be creatable");
        txn.set_ground_location(lot, ORCHARD_FARM)
            .expect("prerequisite lot should be placeable at the prerequisite site");
    }
    commit_txn(txn, &mut h.event_log);

    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        baker,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );
    h.driver.enable_tracing();
    (h, baker, recipe_id)
}

fn observe_five_stage_budget(seed: Seed) -> BudgetObservation {
    let (mut h, baker, recipe_id) = setup_five_stage_budget_harness(seed);

    h.step_once();

    let trace = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(baker, Tick(0))
        .expect("baker should have a tick-0 decision trace");
    let planning = match &trace.outcome {
        DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected a planning trace for the baker, got {other:?}"),
    };
    let attempt = planning
        .planning
        .attempts
        .iter()
        .find(|attempt| attempt.goal.kind == GoalKind::ProduceCommodity { recipe_id })
        .unwrap_or_else(|| {
            panic!(
                "five-stage ProduceCommodity attempt should enter planning; attempts={:?}",
                planning.planning.attempts
            )
        });

    let budget = attempt
        .strategic_budget
        .as_ref()
        .expect("five-stage attempt should record strategic-budget provenance");
    let strategic_plan = attempt
        .strategic_plan
        .as_ref()
        .expect("five-stage attempt should preserve the strategic itinerary");

    BudgetObservation {
        stages_count: budget.stages_count,
        budget_total: budget.budget_total,
        budget_used: budget.budget_used,
        exhausted: budget.exhausted,
        strategic_sub_goals: strategic_plan
            .iter()
            .map(|step| step.sub_goal.clone())
            .collect(),
        budget_exhausted_outcome: matches!(
            attempt.outcome,
            PlanSearchOutcome::BudgetExhausted { .. }
        ),
    }
}

// ---------------------------------------------------------------------------
// Scenario 423: S145 Five-Stage Strategic Budget Scaling
// ---------------------------------------------------------------------------
//
// Systems: AI, Search, Production
// GoalKinds: ProduceCommodity
// ActionDomains: Production, Travel
// Places: Village Square, Orchard Farm
// Principles: 12, 20, 27, 29
//
// Setup: a baker knows one recipe whose four missing inputs are available away from the local production workstation.
// Proves: stage-aware strategic budget records a non-exhausted five-stage strategic itinerary.
// Cross-system chain: production goal candidate -> strategic itinerary over
// missing inputs plus goal place -> decision trace strategic-budget provenance.
#[test]
fn five_stage_production_chain_records_stage_aware_budget() {
    let observation = observe_five_stage_budget(Seed([145; 32]));

    assert_eq!(observation.stages_count, EXPECTED_STAGE_COUNT);
    assert_eq!(
        observation.budget_total,
        ExecutionBudget::default().strategic_budget_for_stages(usize::from(EXPECTED_STAGE_COUNT))
            as u32
    );
    assert!(
        observation.budget_used <= observation.budget_total,
        "strategic search should stay within the stage-aware budget: {observation:?}"
    );
    assert!(
        !observation.exhausted,
        "five-stage strategic search should complete without budget exhaustion: {observation:?}"
    );
    assert!(
        !observation.budget_exhausted_outcome,
        "the full tactical search may stop later, but the five-stage strategic attempt must not be classified as budget-exhausted: {observation:?}"
    );

    assert_eq!(
        observation.strategic_sub_goals.len(),
        usize::from(EXPECTED_STAGE_COUNT),
        "strategic itinerary should cover four missing prerequisites plus the goal place: {observation:?}"
    );
}

#[test]
fn five_stage_production_chain_replays_deterministically() {
    let first = observe_five_stage_budget(Seed([145; 32]));
    let second = observe_five_stage_budget(Seed([145; 32]));

    assert_eq!(
        first, second,
        "strategic-budget observation should be deterministic for the same seed"
    );
}
