//! Golden coverage for S147 HTN method decomposition.

mod golden_harness;

use std::collections::BTreeSet;

use golden_harness::*;
use worldwake_ai::htn::{build_method_registry, select_method};
use worldwake_ai::{DecisionOutcome, GoalKind, GoalOffer};
use worldwake_core::{
    AgentSchemaContextProfile, BodyCostPerTick, CommodityKind, EntityId, GoalKey,
    GoalPlanningBudget, HomeostaticNeedId, HomeostaticNeeds, KnownRecipes, MetabolismProfile,
    MethodSchemaId, MotiveSource, MotiveSourceRef, OpportunityAnchor, PerceptionSource, Quantity,
    ResourceSource, Seed, Tick, UtilityProfile, WorkstationTag,
};
use worldwake_sim::PerAgentBeliefView;
use worldwake_sim::RecipeDefinition;

const PRODUCE_WITH_GATHER: MethodSchemaId = MethodSchemaId(5);
const PRODUCE_METHODS: [MethodSchemaId; 3] =
    [MethodSchemaId(4), MethodSchemaId(5), MethodSchemaId(6)];

#[derive(Clone, Debug, Eq, PartialEq)]
struct HtnMethodObservation {
    selected_method: Option<MethodSchemaId>,
    subgoal_kinds: Vec<String>,
    motive_score: u32,
    strategic_sub_goals: Vec<String>,
    goal_budget: GoalPlanningBudget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SelectorObservation {
    selected_method: Option<MethodSchemaId>,
}

fn build_single_input_recipe() -> (worldwake_sim::RecipeRegistry, worldwake_core::RecipeId) {
    let mut recipes = worldwake_sim::RecipeRegistry::new();
    let recipe_id = recipes.register(RecipeDefinition {
        name: "Bake HTN Bread".to_string(),
        inputs: vec![(CommodityKind::Firewood, Quantity(1))],
        outputs: vec![(CommodityKind::Bread, Quantity(1))],
        work_ticks: nz(3),
        required_workstation_tag: Some(WorkstationTag::Mill),
        required_tool_kinds: Vec::new(),
        body_cost_per_tick: BodyCostPerTick::new(pm(3), pm(2), pm(5), pm(0), pm(1)),
    });
    (recipes, recipe_id)
}

fn setup_htn_production_harness(
    disabled_methods: BTreeSet<MethodSchemaId>,
) -> (GoldenHarness, EntityId, worldwake_core::RecipeId) {
    let (recipes, recipe_id) = build_single_input_recipe();
    let mut h = GoldenHarness::with_recipes(Seed([147; 32]), recipes);
    let baker = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "HTN Baker",
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
        worldwake_core::PerceptionProfile {
            observation_buffer_capacity: 64,
            observation_budget: 24,
            ..worldwake_core::PerceptionProfile::default()
        },
    );
    set_agent_execution_budget(
        &mut h.world,
        &mut h.event_log,
        baker,
        worldwake_core::ExecutionBudget::default(),
    );
    set_schema_context_profile(&mut h, baker, disabled_methods);

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
        WorkstationTag::ChoppingBlock,
        ResourceSource {
            commodity: CommodityKind::Firewood,
            available_quantity: Quantity(3),
            max_quantity: Quantity(3),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: nz(1),
        },
        ProductionOutputOwner::Actor,
    );

    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        baker,
        Tick(0),
        PerceptionSource::Inference,
    );
    h.driver.enable_tracing();
    (h, baker, recipe_id)
}

fn set_schema_context_profile(
    h: &mut GoldenHarness,
    actor: EntityId,
    disabled_methods: BTreeSet<MethodSchemaId>,
) {
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_agent_schema_context_profile(
        actor,
        AgentSchemaContextProfile {
            disabled_methods,
            ..AgentSchemaContextProfile::default()
        },
    )
    .expect("golden harness should keep schema context profiles writable");
    commit_txn(txn, &mut h.event_log);
}

fn observe_htn_production(disabled_methods: BTreeSet<MethodSchemaId>) -> HtnMethodObservation {
    let (mut h, baker, recipe_id) = setup_htn_production_harness(disabled_methods);

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
                "HTN ProduceCommodity attempt should enter planning; attempts={:?}",
                planning.planning.attempts
            )
        });

    let method_trace = attempt.method_trace.as_ref();
    HtnMethodObservation {
        selected_method: method_trace.and_then(|trace| trace.method_id),
        subgoal_kinds: method_trace.map_or_else(Vec::new, |trace| {
            trace
                .subgoals_attempted
                .iter()
                .map(|subgoal| format!("{:?}", subgoal.kind))
                .collect()
        }),
        motive_score: method_trace.map_or(0, |trace| trace.motive_score),
        strategic_sub_goals: attempt
            .strategic_plan
            .as_ref()
            .map_or_else(Vec::new, |plan| {
                plan.iter().map(|step| step.sub_goal.clone()).collect()
            }),
        goal_budget: attempt.goal_budget,
    }
}

fn observe_selector(disabled_methods: BTreeSet<MethodSchemaId>) -> SelectorObservation {
    let (h, baker, recipe_id) = setup_htn_production_harness(disabled_methods.clone());
    let belief_store = h
        .world
        .get_component_agent_belief_store(baker)
        .expect("baker should have seeded world beliefs");
    let view = PerAgentBeliefView::new_with_recipes(baker, &h.world, &h.recipes, belief_store);
    let registry = build_method_registry();
    let profile = AgentSchemaContextProfile {
        disabled_methods,
        ..AgentSchemaContextProfile::default()
    };
    let goal = GoalOffer {
        key: GoalKey::from(GoalKind::ProduceCommodity { recipe_id }),
        anchor: OpportunityAnchor::Place(VILLAGE_SQUARE),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([VILLAGE_SQUARE, ORCHARD_FARM]),
        obligation_source: None,
        commitment_impact_if_ignored: pm(0),
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: vec![MotiveSourceRef {
            source: MotiveSource::NeedPressure {
                need: HomeostaticNeedId::Hunger,
            },
            introduced_tick: Tick(0),
        }],
        acquisition_quantity: None,
    };

    SelectorObservation {
        selected_method: select_method(
            baker,
            &goal,
            &registry,
            &profile,
            &view,
            &goal.motive_sources,
        )
        .map(|method| method.id),
    }
}

// ---------------------------------------------------------------------------
// Scenario 431: S147 ProduceWithGather Method Selection
// ---------------------------------------------------------------------------
//
// Systems: AI, Search, Production
// GoalKinds: ProduceCommodity
// ActionDomains: Production, Travel
// Places: Village Square, Orchard Farm
// Principles: 7, 14, 20, 22, 26, 28, 29
//
// Setup: a hungry baker knows a bread recipe, a local mill, and a remote
//   firewood source. The agent has no firewood in inventory.
// Proves: HTN method selection chooses ProduceWithGather from the agent's
//   belief view and a goal offer whose evidence places include the known
//   resource source.
// Cross-system chain: belief-seeded resource source -> MethodSelector
//   preconditions -> selected method schema id.
#[test]
fn produce_with_gather_selector_uses_belief_view_evidence() {
    let observation = observe_selector(BTreeSet::new());

    assert_eq!(observation.selected_method, Some(PRODUCE_WITH_GATHER));
}

#[test]
fn produce_with_gather_selector_replays_deterministically() {
    let first = observe_selector(BTreeSet::new());
    let second = observe_selector(BTreeSet::new());

    assert_eq!(first, second);
}

// ---------------------------------------------------------------------------
// Scenario 432: S147 Disabled Methods Fall Back To Flat Strategic Search
// ---------------------------------------------------------------------------
//
// Systems: AI, Search, Production
// GoalKinds: ProduceCommodity
// ActionDomains: Production, Travel
// Places: Village Square, Orchard Farm
// Principles: 20, 22, 28, 29
//
// Setup: a hungry baker knows a bread recipe, a local mill, and a remote
//   firewood source, but disables all ProduceCommodity methods in
//   AgentSchemaContextProfile.disabled_methods.
// Proves: method dispatch returns to the flat strategic path with no method
//   trace while the ordinary ProduceCommodity planning attempt still exists.
// Cross-system chain: scenario-authored profile denylist -> MethodSelector
//   exclusion -> flat strategic fallback -> decision trace method_trace=None.
#[test]
fn disabled_produce_methods_fall_back_to_flat_strategic_search() {
    let observation = observe_htn_production(BTreeSet::from(PRODUCE_METHODS));

    assert_eq!(observation.selected_method, None);
    assert!(observation.subgoal_kinds.is_empty());
    assert_eq!(observation.motive_score, 0);
    assert_eq!(
        observe_selector(BTreeSet::from(PRODUCE_METHODS)).selected_method,
        None
    );
    assert!(
        observation
            .strategic_sub_goals
            .iter()
            .any(|sub_goal| sub_goal.contains("AcquirePrerequisite(Firewood)")),
        "flat fallback should still plan through ordinary missing-input acquisition: {observation:?}"
    );
}

#[test]
fn disabled_method_fallback_replays_deterministically() {
    let first = observe_htn_production(BTreeSet::from(PRODUCE_METHODS));
    let second = observe_htn_production(BTreeSet::from(PRODUCE_METHODS));

    assert_eq!(first, second);
}
