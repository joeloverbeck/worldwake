//! Golden coverage for S147 HTN method decomposition.

use std::collections::BTreeSet;

use crate::golden_harness::*;
use worldwake_ai::htn::{build_method_registry, select_method, select_method_with_recipes};
use worldwake_ai::{
    AgentDecisionRuntime, DecisionOutcome, GoalKind, GoalOffer, PlanFailureContext,
    PlanTerminalKind, PlannedPlan, PlannedStep, PlannerOpKind, PlanningState,
    StrategicFallbackReason, build_planning_snapshot, generate_candidates, handle_plan_failure,
};
use worldwake_core::{
    ActionDefId, AgentSchemaContextProfile, ArtifactHeader, ArtifactKind, BlockerMemory,
    BodyCostPerTick, BountyTarget, BountyTerms, CognitiveProfile, CommodityKind, ContentionIntents,
    Discrepancy, DiscrepancyMemory, EntityId, EntityKind, GoalKey, GoalPlanningBudget,
    HomeostaticNeedId, HomeostaticNeeds, IntentionFrame, KnownRecipes, MetabolismProfile,
    MethodFailureContext, MethodFailureKind, MethodSchemaId, MotiveSource, MotiveSourceRef,
    OpportunityAnchor, OpportunityKey, PerceptionSource, ProofRequirement, Quantity,
    ResourceSource, RewardSource, Seed, Tick, UtilityProfile, WorkstationTag,
};
use worldwake_sim::RecipeDefinition;
use worldwake_sim::{PerAgentBeliefView, ProfileBeliefView};

const PRODUCE_WITH_GATHER: MethodSchemaId = MethodSchemaId(5);
const FULFILL_BOUNTY_DIRECT: MethodSchemaId = MethodSchemaId(1);
const FULFILL_BOUNTY_INVESTIGATION: MethodSchemaId = MethodSchemaId(2);
const ESCORT_TO_HOME: MethodSchemaId = MethodSchemaId(12);
const PRODUCE_METHODS: [MethodSchemaId; 3] =
    [MethodSchemaId(4), MethodSchemaId(5), MethodSchemaId(6)];

#[derive(Clone, Debug, Eq, PartialEq)]
struct HtnMethodObservation {
    selected_method: Option<MethodSchemaId>,
    rejected_methods: Vec<(MethodSchemaId, String)>,
    fallback_reason: Option<StrategicFallbackReason>,
    subgoal_kinds: Vec<String>,
    motive_score: u32,
    strategic_sub_goals: Vec<String>,
    goal_budget: GoalPlanningBudget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SelectorObservation {
    selected_method: Option<MethodSchemaId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GeneratedProduceOfferObservation {
    evidence_entities: BTreeSet<EntityId>,
    evidence_places: BTreeSet<EntityId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GeneratedBountyOfferObservation {
    selected_method: Option<MethodSchemaId>,
    evidence_entities: BTreeSet<EntityId>,
    evidence_places: BTreeSet<EntityId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GeneratedEscortOfferObservation {
    goal_key: GoalKey,
    selected_method: Option<MethodSchemaId>,
    evidence_entities: BTreeSet<EntityId>,
    evidence_places: BTreeSet<EntityId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MethodFailureObservation {
    selected_method: Option<MethodSchemaId>,
    recorded_discrepancy: Option<Discrepancy>,
    runtime_plan_cleared: bool,
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
            quality: None,
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

fn setup_htn_bounty_harness() -> (GoldenHarness, EntityId, EntityId) {
    setup_htn_bounty_harness_with_reported_source(true)
}

fn setup_htn_bounty_harness_with_reported_source(
    reported: bool,
) -> (GoldenHarness, EntityId, EntityId) {
    let mut h = GoldenHarness::new(Seed([147; 32]));
    let hunter = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "HTN Hunter",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let target = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Bounty Target",
        ORCHARD_FARM,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        hunter,
        worldwake_core::PerceptionProfile {
            observation_buffer_capacity: 64,
            observation_budget: 24,
            ..worldwake_core::PerceptionProfile::default()
        },
    );
    set_agent_execution_budget(
        &mut h.world,
        &mut h.event_log,
        hunter,
        worldwake_core::ExecutionBudget::default(),
    );
    set_schema_context_profile(&mut h, hunter, BTreeSet::new());

    let issuer = hunter;
    let bounty = {
        let mut txn = new_txn(&mut h.world, 0);
        let bounty = txn.create_entity(EntityKind::SocialArtifact);
        txn.set_component_artifact_header(
            bounty,
            ArtifactHeader::posted_active(
                ArtifactKind::Bounty,
                issuer,
                None,
                Tick(0),
                None,
                None,
                VILLAGE_SQUARE,
            ),
        )
        .expect("golden harness should create a bounty artifact header");
        txn.set_component_bounty_terms(
            bounty,
            BountyTerms {
                target: BountyTarget::EliminateEntity { target },
                proof_requirement: ProofRequirement::PhysicalEvidence,
                reward_commodity: CommodityKind::Coin,
                reward_quantity: Quantity(4),
                reward_source: RewardSource::PersonalFunds { issuer },
                claim_place: VILLAGE_SQUARE,
            },
        )
        .expect("golden harness should create bounty terms");
        txn.set_ground_location(bounty, VILLAGE_SQUARE)
            .expect("golden harness should place the bounty artifact");
        commit_txn(txn, &mut h.event_log);
        bounty
    };

    let source = if reported {
        PerceptionSource::Report {
            from: issuer,
            chain_len: 0,
        }
    } else {
        PerceptionSource::DirectObservation
    };
    seed_actor_world_beliefs(&mut h.world, &mut h.event_log, hunter, Tick(0), source);
    h.driver.enable_tracing();
    (h, hunter, bounty)
}

fn setup_htn_direct_bounty_harness() -> (GoldenHarness, EntityId, EntityId) {
    setup_htn_bounty_harness_with_reported_source(false)
}

fn setup_htn_escort_harness() -> (GoldenHarness, EntityId, EntityId) {
    let mut h = GoldenHarness::new(Seed([147; 32]));
    let caretaker = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "HTN Caretaker",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let ward = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Wounded Ward",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        caretaker,
        worldwake_core::PerceptionProfile {
            observation_buffer_capacity: 64,
            observation_budget: 24,
            ..worldwake_core::PerceptionProfile::default()
        },
    );
    set_agent_execution_budget(
        &mut h.world,
        &mut h.event_log,
        caretaker,
        worldwake_core::ExecutionBudget::default(),
    );
    set_schema_context_profile(&mut h, caretaker, BTreeSet::new());
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_wound_list(ward, stable_wound_list(700))
            .expect("golden harness should wound the escortee");
        commit_txn(txn, &mut h.event_log);
    }
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        caretaker,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    h.driver.enable_tracing();
    (h, caretaker, ward)
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
        rejected_methods: method_trace.map_or_else(Vec::new, |trace| {
            trace
                .rejected_methods
                .iter()
                .map(|rejected| {
                    (
                        rejected.method_id,
                        format!("{:?}", rejected.failed_precondition),
                    )
                })
                .collect()
        }),
        fallback_reason: method_trace.and_then(|trace| trace.fallback_reason),
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

fn observe_generated_produce_offer() -> GeneratedProduceOfferObservation {
    let (h, baker, recipe_id) = setup_htn_production_harness(BTreeSet::new());
    let belief_store = h
        .world
        .get_component_agent_belief_store(baker)
        .expect("baker should have seeded world beliefs");
    let view = PerAgentBeliefView::new_with_recipes(baker, &h.world, &h.recipes, belief_store);
    let candidates =
        generate_candidates(&view, baker, &BlockerMemory::default(), &h.recipes, Tick(0));
    let offer = candidates
        .into_iter()
        .find(|offer| offer.key.kind == GoalKind::ProduceCommodity { recipe_id })
        .expect("autonomous candidate generation should emit ProduceCommodity");

    GeneratedProduceOfferObservation {
        evidence_entities: offer.evidence_entities,
        evidence_places: offer.evidence_places,
    }
}

fn observe_generated_bounty_offer() -> GeneratedBountyOfferObservation {
    let (h, hunter, bounty) = setup_htn_bounty_harness();
    let belief_store = h
        .world
        .get_component_agent_belief_store(hunter)
        .expect("hunter should have seeded bounty beliefs");
    let view = PerAgentBeliefView::new(hunter, &h.world, belief_store);
    let candidates = generate_candidates(
        &view,
        hunter,
        &BlockerMemory::default(),
        &h.recipes,
        Tick(0),
    );
    let offer = candidates
        .into_iter()
        .find(|offer| offer.key.kind == GoalKind::FulfillBounty { bounty })
        .expect("autonomous candidate generation should emit FulfillBounty");
    let snapshot = build_planning_snapshot(
        &view,
        hunter,
        &offer.evidence_entities,
        &offer.evidence_places,
        6,
    );
    let state = PlanningState::new(&snapshot);
    let registry = build_method_registry();
    let profile = state
        .agent_schema_context_profile(hunter)
        .unwrap_or_default();
    let selected_method = select_method(
        hunter,
        &offer,
        &registry,
        &profile,
        &state,
        &offer.motive_sources,
    )
    .map(|method| method.id);

    GeneratedBountyOfferObservation {
        selected_method,
        evidence_entities: offer.evidence_entities,
        evidence_places: offer.evidence_places,
    }
}

fn observe_generated_direct_bounty_offer() -> GeneratedBountyOfferObservation {
    let (h, hunter, bounty) = setup_htn_direct_bounty_harness();
    observe_bounty_offer_from_harness(&h, hunter, bounty)
}

fn observe_bounty_offer_from_harness(
    h: &GoldenHarness,
    hunter: EntityId,
    bounty: EntityId,
) -> GeneratedBountyOfferObservation {
    let belief_store = h
        .world
        .get_component_agent_belief_store(hunter)
        .expect("hunter should have seeded bounty beliefs");
    let view = PerAgentBeliefView::new(hunter, &h.world, belief_store);
    let candidates = generate_candidates(
        &view,
        hunter,
        &BlockerMemory::default(),
        &h.recipes,
        Tick(0),
    );
    let offer = candidates
        .into_iter()
        .find(|offer| offer.key.kind == GoalKind::FulfillBounty { bounty })
        .expect("autonomous candidate generation should emit FulfillBounty");
    let snapshot = build_planning_snapshot(
        &view,
        hunter,
        &offer.evidence_entities,
        &offer.evidence_places,
        6,
    );
    let state = PlanningState::new(&snapshot);
    let registry = build_method_registry();
    let profile = state
        .agent_schema_context_profile(hunter)
        .unwrap_or_default();
    let selected_method = select_method(
        hunter,
        &offer,
        &registry,
        &profile,
        &state,
        &offer.motive_sources,
    )
    .map(|method| method.id);

    GeneratedBountyOfferObservation {
        selected_method,
        evidence_entities: offer.evidence_entities,
        evidence_places: offer.evidence_places,
    }
}

fn observe_generated_escort_offer() -> GeneratedEscortOfferObservation {
    let (h, caretaker, ward) = setup_htn_escort_harness();
    let belief_store = h
        .world
        .get_component_agent_belief_store(caretaker)
        .expect("caretaker should have seeded escort beliefs");
    let view = PerAgentBeliefView::new(caretaker, &h.world, belief_store);
    let candidates = generate_candidates(
        &view,
        caretaker,
        &BlockerMemory::default(),
        &h.recipes,
        Tick(0),
    );
    let offer = candidates
        .into_iter()
        .find(|offer| {
            matches!(
                offer.key.kind,
                GoalKind::EscortToSafety { subject, .. } if subject == ward
            )
        })
        .expect("autonomous candidate generation should emit EscortToSafety");
    let snapshot = build_planning_snapshot(
        &view,
        caretaker,
        &offer.evidence_entities,
        &offer.evidence_places,
        6,
    );
    let state = PlanningState::new(&snapshot);
    let registry = build_method_registry();
    let profile = state
        .agent_schema_context_profile(caretaker)
        .unwrap_or_default();
    let selected_method = select_method(
        caretaker,
        &offer,
        &registry,
        &profile,
        &state,
        &offer.motive_sources,
    )
    .map(|method| method.id);

    GeneratedEscortOfferObservation {
        goal_key: offer.key,
        selected_method,
        evidence_entities: offer.evidence_entities,
        evidence_places: offer.evidence_places,
    }
}

fn observe_escort_method_failure_producer() -> MethodFailureObservation {
    let (h, caretaker, ward) = setup_htn_escort_harness();
    let belief_store = h
        .world
        .get_component_agent_belief_store(caretaker)
        .expect("caretaker should have seeded escort beliefs");
    let view = PerAgentBeliefView::new(caretaker, &h.world, belief_store);
    let candidates = generate_candidates(
        &view,
        caretaker,
        &BlockerMemory::default(),
        &h.recipes,
        Tick(0),
    );
    let offer = candidates
        .into_iter()
        .find(|offer| {
            matches!(
                offer.key.kind,
                GoalKind::EscortToSafety { subject, .. } if subject == ward
            )
        })
        .expect("autonomous candidate generation should emit EscortToSafety");
    let snapshot = build_planning_snapshot(
        &view,
        caretaker,
        &offer.evidence_entities,
        &offer.evidence_places,
        6,
    );
    let state = PlanningState::new(&snapshot);
    let registry = build_method_registry();
    let profile = state
        .agent_schema_context_profile(caretaker)
        .unwrap_or_default();
    let selected_method = select_method(
        caretaker,
        &offer,
        &registry,
        &profile,
        &state,
        &offer.motive_sources,
    )
    .map(|method| method.id);
    let failed_step = PlannedStep {
        def_id: ActionDefId(0),
        targets: Vec::new(),
        target_place: None,
        payload_override: None,
        op_kind: PlannerOpKind::Sleep,
        estimated_ticks: 1,
        is_materialization_barrier: false,
        expected_materializations: Vec::new(),
        guard: None,
        expectations: Vec::new(),
    };
    let mut runtime = AgentDecisionRuntime {
        current_plan: Some(
            PlannedPlan::new(
                OpportunityKey {
                    goal_key: offer.key,
                    anchor: OpportunityAnchor::Place(VILLAGE_SQUARE),
                },
                offer.key,
                vec![failed_step.clone()],
                PlanTerminalKind::SearchBudgetExhausted {
                    budget_consumed: 0,
                    budget_total: 0,
                },
            )
            .with_method_id(selected_method),
        ),
        ..AgentDecisionRuntime::default()
    };
    let method_id_from_plan = runtime
        .current_plan
        .as_ref()
        .and_then(|plan| plan.method_id);
    let mut frame: Option<IntentionFrame> = None;
    let mut blocked = BlockerMemory::default();
    let mut discrepancies = DiscrepancyMemory::default();
    let mut facility_intents = ContentionIntents::default();

    handle_plan_failure(
        &PlanFailureContext {
            view: &view,
            agent: caretaker,
            goal_key: offer.key,
            failed_step: &failed_step,
            method_id: method_id_from_plan,
            execution_failure: None,
            belief_discrepancy: None,
            current_tick: Tick(4),
        },
        &mut runtime,
        &mut frame,
        &mut blocked,
        &mut discrepancies,
        &mut facility_intents,
        &CognitiveProfile::default(),
    );

    MethodFailureObservation {
        selected_method,
        recorded_discrepancy: discrepancies
            .entries
            .values()
            .next()
            .map(|entry| entry.discrepancy),
        runtime_plan_cleared: runtime.current_plan.is_none(),
    }
}

fn observe_htn_bounty() -> HtnMethodObservation {
    let (mut h, hunter, bounty) = setup_htn_bounty_harness();

    h.step_once();

    let trace = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(hunter, Tick(0))
        .expect("hunter should have a tick-0 decision trace");
    let planning = match &trace.outcome {
        DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected a planning trace for the hunter, got {other:?}"),
    };
    let attempt = planning
        .planning
        .attempts
        .iter()
        .find(|attempt| attempt.goal.kind == GoalKind::FulfillBounty { bounty })
        .unwrap_or_else(|| {
            panic!(
                "HTN FulfillBounty attempt should enter planning; attempts={:?}",
                planning.planning.attempts
            )
        });

    let method_trace = attempt.method_trace.as_ref();
    HtnMethodObservation {
        selected_method: method_trace.and_then(|trace| trace.method_id),
        rejected_methods: method_trace.map_or_else(Vec::new, |trace| {
            trace
                .rejected_methods
                .iter()
                .map(|rejected| {
                    (
                        rejected.method_id,
                        format!("{:?}", rejected.failed_precondition),
                    )
                })
                .collect()
        }),
        fallback_reason: method_trace.and_then(|trace| trace.fallback_reason),
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

fn observe_snapshot_selector_for_generated_offer() -> SelectorObservation {
    let (h, baker, recipe_id) = setup_htn_production_harness(BTreeSet::new());
    let belief_store = h
        .world
        .get_component_agent_belief_store(baker)
        .expect("baker should have seeded world beliefs");
    let view = PerAgentBeliefView::new_with_recipes(baker, &h.world, &h.recipes, belief_store);
    let candidates =
        generate_candidates(&view, baker, &BlockerMemory::default(), &h.recipes, Tick(0));
    let offer = candidates
        .into_iter()
        .find(|offer| offer.key.kind == GoalKind::ProduceCommodity { recipe_id })
        .expect("autonomous candidate generation should emit ProduceCommodity");
    let snapshot = build_planning_snapshot(
        &view,
        baker,
        &offer.evidence_entities,
        &offer.evidence_places,
        6,
    );
    let state = PlanningState::new(&snapshot);
    let registry = build_method_registry();
    let profile = state
        .agent_schema_context_profile(baker)
        .unwrap_or_default();

    SelectorObservation {
        selected_method: select_method_with_recipes(
            baker,
            &offer,
            &registry,
            &profile,
            &state,
            &offer.motive_sources,
            Some(&h.recipes),
        )
        .selected
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

    assert_eq!(
        observation.selected_method,
        Some(PRODUCE_WITH_GATHER),
        "autonomous production should select ProduceWithGather: {observation:?}"
    );
}

#[test]
fn produce_with_gather_selector_replays_deterministically() {
    let first = observe_selector(BTreeSet::new());
    let second = observe_selector(BTreeSet::new());

    assert_eq!(first, second);
}

// ---------------------------------------------------------------------------
// Scenario 433: S147 Autonomous Produce Method Trace Propagation
// ---------------------------------------------------------------------------
//
// Systems: AI, Search, Production
// GoalKinds: ProduceCommodity
// ActionDomains: Production, Travel
// Places: Village Square, Orchard Farm
// Principles: 7, 14, 20, 22, 26, 28, 29
//
// Setup: a hungry baker autonomously generates a ProduceCommodity goal from
//   known recipe, workstation, and remote resource-source beliefs.
// Proves: generated candidate evidence reaches MethodSelector, so the
//   planning attempt records ProduceWithGather in MethodPlanAttemptTrace.
// Cross-system chain: candidate-generation evidence -> strategic HTN selector
//   -> method subgoal substitution -> decision trace method_trace.
#[test]
fn autonomous_produce_candidate_records_method_trace() {
    let observation = observe_htn_production(BTreeSet::new());

    assert_eq!(
        observation.selected_method,
        Some(PRODUCE_WITH_GATHER),
        "autonomous production should select ProduceWithGather: {observation:?}"
    );
    assert!(
        observation
            .subgoal_kinds
            .iter()
            .any(|subgoal| subgoal.contains("AcquireCommodity")),
        "method trace should record the ProduceWithGather acquisition subgoal: {observation:?}"
    );
    assert!(
        observation
            .rejected_methods
            .iter()
            .any(|(_, precondition)| precondition.contains("OwnsInputsForRecipe")),
        "method trace should record a rejected production method with its failing precondition: {observation:?}"
    );
    assert_eq!(observation.fallback_reason, None);
    assert!(
        observation
            .strategic_sub_goals
            .iter()
            .any(|sub_goal| sub_goal.contains("AcquirePrerequisite(Firewood)")),
        "method-selected strategic plan should include the firewood prerequisite stage: {observation:?}"
    );
}

#[test]
fn autonomous_produce_candidate_carries_source_evidence() {
    let observation = observe_generated_produce_offer();

    assert!(
        observation.evidence_places.contains(&ORCHARD_FARM),
        "generated ProduceCommodity offer should carry remote source place evidence: {observation:?}"
    );
    assert!(
        !observation.evidence_entities.is_empty(),
        "generated ProduceCommodity offer should carry source/workstation evidence entities: {observation:?}"
    );
}

#[test]
fn autonomous_produce_snapshot_selector_uses_source_evidence() {
    let observation = observe_snapshot_selector_for_generated_offer();

    assert_eq!(
        observation.selected_method,
        Some(PRODUCE_WITH_GATHER),
        "snapshot-backed selector should see generated source evidence: {observation:?}"
    );
}

#[test]
fn autonomous_produce_method_trace_replays_deterministically() {
    let first = observe_htn_production(BTreeSet::new());
    let second = observe_htn_production(BTreeSet::new());

    assert_eq!(first, second);
}

// ---------------------------------------------------------------------------
// Scenario 434: S147 FulfillBountyInvestigation Method Selection
// ---------------------------------------------------------------------------
//
// Systems: AI, Search, SocialArtifact
// GoalKinds: FulfillBounty
// ActionDomains: Social, Travel
// Places: Village Square, Orchard Farm
// Principles: 7, 14, 20, 26, 28, 29
//
// Setup: a hunter knows the same bounty through a reported source, making the
//   witness-report precondition available for the investigation method.
// Proves: generated FulfillBounty candidate evidence can select
//   FulfillBountyInvestigation without a hand-constructed GoalOffer.
// Cross-system chain: reported bounty artifact belief -> generated
//   FulfillBounty candidate evidence -> snapshot-backed MethodSelector ->
//   selected investigation method id.
#[test]
fn generated_bounty_candidate_selects_fulfill_bounty_investigation() {
    let observation = observe_generated_bounty_offer();

    assert_eq!(
        observation.selected_method,
        Some(FULFILL_BOUNTY_INVESTIGATION),
        "reported bounty candidate should select FulfillBountyInvestigation: {observation:?}"
    );
    assert!(
        !observation.evidence_entities.is_empty(),
        "generated bounty candidate should carry bounty and target evidence: {observation:?}"
    );
}

#[test]
fn generated_bounty_candidate_selector_replays_deterministically() {
    let first = observe_generated_bounty_offer();
    let second = observe_generated_bounty_offer();

    assert_eq!(first, second);
}

// ---------------------------------------------------------------------------
// Scenario 436: S147 FulfillBountyDirect Method Selection
// ---------------------------------------------------------------------------
//
// Systems: AI, Search, SocialArtifact, Combat
// GoalKinds: FulfillBounty
// ActionDomains: Social, Combat, Travel
// Places: Village Square, Orchard Farm
// Principles: 7, 14, 20, 26, 28, 29
//
// Setup: a hunter directly observes a posted bounty and its target-location
//   evidence, so no witness-report precondition is available.
// Proves: generated FulfillBounty candidate evidence can select
//   FulfillBountyDirect through TargetLastSeenKnown without a hand-built offer.
// Cross-system chain: direct bounty artifact belief -> generated FulfillBounty
//   candidate evidence -> snapshot-backed MethodSelector -> selected direct
//   method id.
#[test]
fn generated_direct_bounty_candidate_selects_fulfill_bounty_direct() {
    let observation = observe_generated_direct_bounty_offer();

    assert_eq!(
        observation.selected_method,
        Some(FULFILL_BOUNTY_DIRECT),
        "direct bounty candidate should select FulfillBountyDirect: {observation:?}"
    );
    assert!(
        !observation.evidence_entities.is_empty(),
        "generated direct bounty candidate should carry bounty and target evidence: {observation:?}"
    );
    assert!(
        observation.evidence_places.contains(&ORCHARD_FARM),
        "generated direct bounty candidate should carry target-location evidence: {observation:?}"
    );
}

#[test]
fn generated_direct_bounty_candidate_selector_replays_deterministically() {
    let first = observe_generated_direct_bounty_offer();
    let second = observe_generated_direct_bounty_offer();

    assert_eq!(first, second);
}

// ---------------------------------------------------------------------------
// Scenario 437: S147 EscortToHome Method Selection
// ---------------------------------------------------------------------------
//
// Systems: AI, Search, Care, Travel
// GoalKinds: EscortToSafety
// ActionDomains: Care, Travel
// Places: Village Square, Orchard Farm
// Principles: 7, 14, 20, 22, 26, 28, 29
//
// Setup: a caretaker directly observes a wounded co-located ward and a
//   reachable adjacent destination.
// Proves: generated EscortToSafety candidate evidence reaches MethodSelector
//   and selects EscortToHome through the escortee-location belief path.
// Cross-system chain: local wound observation -> generated EscortToSafety
//   candidate evidence -> snapshot-backed MethodSelector -> selected escort
//   method id.
#[test]
fn generated_escort_candidate_selects_escort_to_home() {
    let observation = observe_generated_escort_offer();

    assert_eq!(
        observation.selected_method,
        Some(ESCORT_TO_HOME),
        "escort candidate should select EscortToHome: {observation:?}"
    );
    assert!(
        !observation.evidence_entities.is_empty(),
        "generated escort candidate should carry escortee evidence: {observation:?}"
    );
    assert!(
        observation
            .evidence_places
            .iter()
            .any(|place| *place != VILLAGE_SQUARE),
        "generated escort candidate should carry destination evidence: {observation:?}"
    );
}

#[test]
fn generated_escort_candidate_selector_replays_deterministically() {
    let first = observe_generated_escort_offer();
    let second = observe_generated_escort_offer();

    assert_eq!(first, second);
}

// ---------------------------------------------------------------------------
// Scenario 438: S147 Method Failure Producer
// ---------------------------------------------------------------------------
//
// Systems: AI, Search, Care, FailureHandling
// GoalKinds: EscortToSafety
// ActionDomains: Care
// Places: Village Square
// Principles: 14, 20, 26, 28, 29
//
// Setup: a generated EscortToSafety candidate selects EscortToHome, then the
//   selected method id is carried on the active plan into the normal plan
//   failure handler.
// Proves: method-selected failures that are not classified by a stronger
//   blocker/discrepancy emit Discrepancy::MethodFailure through the runtime
//   failure producer instead of through fabricated traces.
// Cross-system chain: generated escort candidate -> snapshot-backed
//   MethodSelector -> active PlannedPlan.method_id -> handle_plan_failure ->
//   DiscrepancyMemory.
#[test]
fn method_selected_failure_records_method_failure_discrepancy() {
    let observation = observe_escort_method_failure_producer();

    assert_eq!(
        observation.selected_method,
        Some(ESCORT_TO_HOME),
        "hybrid method-failure proof should start from a generated escort method selection: {observation:?}"
    );
    assert_eq!(
        observation.recorded_discrepancy,
        Some(Discrepancy::MethodFailure(MethodFailureContext {
            method_id: ESCORT_TO_HOME,
            kind: MethodFailureKind::SubgoalUnachievable,
            subgoal_index: None,
        })),
        "method-selected plan failure should record typed method failure: {observation:?}"
    );
    assert!(
        observation.runtime_plan_cleared,
        "handle_plan_failure should clear the failed active plan: {observation:?}"
    );
}

// ---------------------------------------------------------------------------
// Scenario 435: S147 Autonomous FulfillBountyInvestigation Method Trace
// Propagation
// ---------------------------------------------------------------------------
//
// Systems: AI, Search, SocialArtifact, Combat
// GoalKinds: FulfillBounty
// ActionDomains: Social, Combat, Travel
// Places: Village Square, Orchard Farm
// Principles: 7, 14, 20, 26, 28, 29
//
// Setup: a hunter autonomously generates a FulfillBounty goal from a reported
//   bounty artifact and target-location report.
// Proves: the generated bounty candidate records FulfillBountyInvestigation
//   in MethodPlanAttemptTrace during planning.
// Cross-system chain: generated bounty candidate -> strategic HTN selector ->
//   method subgoal substitution -> decision trace method_trace.
#[test]
fn autonomous_bounty_candidate_records_method_trace() {
    let observation = observe_htn_bounty();

    assert_eq!(
        observation.selected_method,
        Some(FULFILL_BOUNTY_INVESTIGATION),
        "autonomous bounty planning should select FulfillBountyInvestigation: {observation:?}"
    );
    assert!(
        observation
            .subgoal_kinds
            .iter()
            .any(|subgoal| subgoal.contains("AskWitness")),
        "method trace should record the investigation witness subgoal: {observation:?}"
    );
    assert!(
        observation
            .strategic_sub_goals
            .iter()
            .any(|sub_goal| sub_goal.contains("SatisfyGoal")),
        "method-selected strategic plan should include investigation goal stages: {observation:?}"
    );
}

#[test]
fn autonomous_bounty_method_trace_replays_deterministically() {
    let first = observe_htn_bounty();
    let second = observe_htn_bounty();

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
    assert_eq!(
        observation.fallback_reason,
        Some(StrategicFallbackReason::NoViableMethod)
    );
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
