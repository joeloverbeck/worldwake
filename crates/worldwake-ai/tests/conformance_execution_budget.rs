//! Conformance checks for `ExecutionBudget` field classification.
//!
//! This file intentionally does not try to subprocess-drive every golden test.
//! Instead it exercises the strongest reusable in-process scenario boundaries
//! the current harness supports:
//! - immediate local consume behavior under the full minimum budget bundle
//! - bounded multi-step search under the compression-style fields that stayed safe
//! - explicit divergence proofs for `snapshot_travel_horizon` and `max_node_expansions`

mod golden_harness;

use golden_harness::*;
use worldwake_ai::{DecisionOutcome, GoalKey, GoalKind, PlannerOpKind, SelectedPlanSource};
use worldwake_core::{
    BeliefConfidencePolicy, CognitiveProfile, CommodityKind, EntityId, ExecutionBudget,
    HomeostaticNeeds, KnownRecipes, MetabolismProfile, PerceptionProfile, Quantity, ResourceSource,
    Seed, Tick, UtilityProfile, WorkstationTag,
};

fn minimum_bundle_cognitive() -> CognitiveProfile {
    CognitiveProfile {
        snapshot_travel_horizon: 2,
        max_node_expansions: 50,
        ..CognitiveProfile::default()
    }
}

fn tight_search_cognitive() -> CognitiveProfile {
    CognitiveProfile {
        max_node_expansions: 2,
        ..CognitiveProfile::default()
    }
}

fn tight_snapshot_cognitive() -> CognitiveProfile {
    CognitiveProfile {
        snapshot_travel_horizon: 2,
        ..CognitiveProfile::default()
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

fn selected_goal_sequence(
    h: &mut GoldenHarness,
    agent: EntityId,
    steps: u64,
) -> Vec<Option<GoalKey>> {
    let mut sequence = Vec::new();
    for tick in 0..steps {
        h.step_once();
        sequence.push(
            planning_trace_at(h, agent, Tick(tick))
                .and_then(|planning| planning.selection.selected_goal()),
        );
    }
    sequence
}

fn selected_plan_ops(h: &GoldenHarness, agent: EntityId, tick: Tick) -> Option<Vec<PlannerOpKind>> {
    planning_trace_at(h, agent, tick)?
        .selection
        .selected_plan
        .as_ref()
        .map(|plan| plan.steps.iter().map(|step| step.op_kind).collect())
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

fn setup_local_consume_harness(
    seed: Seed,
    cognitive_profile: CognitiveProfile,
    execution_budget: ExecutionBudget,
) -> (GoldenHarness, EntityId) {
    let mut h = GoldenHarness::new(seed);
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Alice",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_cognitive_profile(&mut h.world, &mut h.event_log, agent, cognitive_profile);
    set_agent_execution_budget(&mut h.world, &mut h.event_log, agent, execution_budget);
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
    );
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );
    h.driver.enable_tracing();
    (h, agent)
}

fn setup_remote_acquire_harness(
    seed: Seed,
    cognitive_profile: CognitiveProfile,
    execution_budget: ExecutionBudget,
) -> (GoldenHarness, EntityId) {
    let mut h = GoldenHarness::new(seed);
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Forager",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_cognitive_profile(&mut h.world, &mut h.event_log, agent, cognitive_profile);
    set_agent_execution_budget(&mut h.world, &mut h.event_log, agent, execution_budget);
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(10),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
    );
    h.driver.enable_tracing();
    (h, agent)
}

fn setup_bounded_multistep_harness(
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
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(10),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
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

#[test]
fn conformance_minimum_bundle_preserves_immediate_local_consume_sequence() {
    let seed = Seed([81; 32]);
    let (mut baseline, baseline_agent) = setup_local_consume_harness(
        seed,
        CognitiveProfile::default(),
        ExecutionBudget::default(),
    );
    let (mut minimum, minimum_agent) = setup_local_consume_harness(
        seed,
        minimum_bundle_cognitive(),
        ExecutionBudget {
            beam_width: 3,
            max_prerequisite_locations: 1,
        },
    );

    let baseline_sequence = selected_goal_sequence(&mut baseline, baseline_agent, 3);
    let minimum_sequence = selected_goal_sequence(&mut minimum, minimum_agent, 3);

    assert_eq!(
        baseline_sequence.first().copied().flatten(),
        Some(GoalKey::from(GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        })),
        "baseline run should start by selecting local bread consumption"
    );
    assert_eq!(
        minimum_sequence, baseline_sequence,
        "full minimum execution-budget bundle should preserve immediate local consume behavior"
    );
}

#[test]
fn conformance_beam_width_and_prerequisite_limit_preserve_bounded_multistep_selection() {
    let seed = Seed([82; 32]);
    let cases = [
        (
            "beam_width",
            ExecutionBudget {
                beam_width: 3,
                ..ExecutionBudget::default()
            },
        ),
        (
            "max_prerequisite_locations",
            ExecutionBudget {
                max_prerequisite_locations: 1,
                ..ExecutionBudget::default()
            },
        ),
    ];

    for (label, budget) in cases {
        let (mut baseline, baseline_agent) = setup_bounded_multistep_harness(
            seed,
            CognitiveProfile::default(),
            ExecutionBudget::default(),
        );
        let (mut reduced, reduced_agent) =
            setup_bounded_multistep_harness(seed, CognitiveProfile::default(), budget);

        baseline.step_once();
        reduced.step_once();

        let baseline_planning = planning_trace_at(&baseline, baseline_agent, Tick(0))
            .expect("baseline run should produce a planning trace at tick 0");
        let reduced_planning = planning_trace_at(&reduced, reduced_agent, Tick(0))
            .expect("reduced run should produce a planning trace at tick 0");

        assert_eq!(
            baseline_planning.selection.selected_goal(),
            reduced_planning.selection.selected_goal(),
            "{label} reduction should preserve selected goal on the bounded multi-step scenario"
        );
        assert_eq!(
            baseline_planning.selection.selected_plan_source,
            Some(SelectedPlanSource::SearchSelection),
            "baseline run should find a fresh searched plan on the bounded multi-step scenario"
        );
        assert_eq!(
            reduced_planning.selection.selected_plan_source,
            Some(SelectedPlanSource::SearchSelection),
            "{label} reduction should still find a searched plan on the bounded multi-step scenario"
        );
        assert_eq!(
            selected_plan_ops(&baseline, baseline_agent, Tick(0)),
            selected_plan_ops(&reduced, reduced_agent, Tick(0)),
            "{label} reduction should preserve the bounded multi-step plan shape"
        );
        let baseline_ops = selected_plan_ops(&baseline, baseline_agent, Tick(0))
            .expect("baseline bounded multi-step run should produce a plan");
        assert!(
            baseline_ops.contains(&PlannerOpKind::Travel),
            "bounded multi-step scenario should exercise travel search"
        );
        assert!(
            baseline_ops.contains(&PlannerOpKind::MoveCargo),
            "bounded multi-step scenario should exercise remote pickup search"
        );
        assert!(
            baseline_ops.contains(&PlannerOpKind::Craft),
            "bounded multi-step scenario should exercise craft search"
        );
    }
}

#[test]
fn conformance_snapshot_horizon_is_behavior_changing() {
    let seed = Seed([84; 32]);
    let (mut baseline, baseline_agent) = setup_remote_acquire_harness(
        seed,
        CognitiveProfile::default(),
        ExecutionBudget::default(),
    );
    let (mut reduced, reduced_agent) =
        setup_remote_acquire_harness(seed, tight_snapshot_cognitive(), ExecutionBudget::default());

    baseline.step_once();
    reduced.step_once();

    let baseline_planning = planning_trace_at(&baseline, baseline_agent, Tick(0))
        .expect("baseline remote-acquire run should produce a planning trace at tick 0");
    let reduced_planning = planning_trace_at(&reduced, reduced_agent, Tick(0))
        .expect("reduced remote-acquire run should produce a planning trace at tick 0");

    assert_eq!(
        baseline_planning.selection.selected_plan_source,
        Some(SelectedPlanSource::SearchSelection),
        "baseline remote-acquire run should find a searched plan"
    );
    let baseline_ops = selected_plan_ops(&baseline, baseline_agent, Tick(0))
        .expect("baseline remote-acquire run should produce a plan");
    assert!(
        baseline_ops.contains(&PlannerOpKind::Travel),
        "one-hop remote-acquire scenario should exercise travel search"
    );
    assert_eq!(
        reduced_planning.selection.selected_goal(),
        None,
        "reduced snapshot_travel_horizon should suppress one-hop remote-acquire selection"
    );
    assert_ne!(
        baseline_planning.selection.selected_goal(),
        reduced_planning.selection.selected_goal(),
        "snapshot_travel_horizon is behavior-changing and therefore not conformance-safe"
    );
}

#[test]
fn conformance_max_node_expansions_is_behavior_changing() {
    let seed = Seed([83; 32]);
    let (mut tight, tight_agent) =
        setup_bounded_multistep_harness(seed, tight_search_cognitive(), ExecutionBudget::default());
    let (mut thorough, thorough_agent) = setup_bounded_multistep_harness(
        seed,
        CognitiveProfile::default(),
        ExecutionBudget::default(),
    );

    tight.step_once();
    thorough.step_once();

    let tight_planning = planning_trace_at(&tight, tight_agent, Tick(0))
        .expect("tight run should produce a planning trace at tick 0");
    let thorough_planning = planning_trace_at(&thorough, thorough_agent, Tick(0))
        .expect("default run should produce a planning trace at tick 0");

    assert_eq!(
        thorough_planning.selection.selected_plan_source,
        Some(SelectedPlanSource::SearchSelection),
        "default search depth should find the bounded multi-step plan"
    );
    assert!(
        tight_planning.selection.selected_plan.is_none(),
        "tight max_node_expansions should fail to select the bounded multi-step plan"
    );
    assert_ne!(
        thorough_planning.selection.selected_plan_source,
        tight_planning.selection.selected_plan_source,
        "max_node_expansions is behavior-changing and therefore not conformance-safe"
    );
}
