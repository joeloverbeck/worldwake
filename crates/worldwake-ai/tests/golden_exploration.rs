//! Golden tests for exploration fallback behavior.

mod golden_harness;

use golden_harness::*;
use worldwake_ai::{CommodityPurpose, DecisionOutcome, GoalKey, GoalKind, PlannerOpKind};
use worldwake_core::{
    CommodityKind, EntityId, EventLog, ExplorationProfile, HomeostaticNeedId, HomeostaticNeeds,
    KnownRecipes, MetabolismProfile, PerceptionProfile, PerceptionSource, Place, PlaceTag,
    Quantity, ResourceSource, Seed, Tick, Topology, TravelEdge, TravelEdgeId, UtilityProfile,
    WorkstationTag, World,
};
use worldwake_sim::{ActionTraceKind, ControllerState, Scheduler, SystemManifest};

const PLACE_START: EntityId = entity(900);
const PLACE_FRONTIER: EntityId = entity(901);

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

fn build_exploration_topology() -> Topology {
    let mut topology = Topology::new();
    topology
        .add_place(PLACE_START, place("ExplorationStart", &[PlaceTag::Village]))
        .unwrap();
    topology
        .add_place(
            PLACE_FRONTIER,
            place("ExplorationFrontier", &[PlaceTag::Farm]),
        )
        .unwrap();

    topology
        .add_edge(TravelEdge::new(TravelEdgeId(900), PLACE_START, PLACE_FRONTIER, 1, None).unwrap())
        .unwrap();
    topology
        .add_edge(TravelEdge::new(TravelEdgeId(901), PLACE_FRONTIER, PLACE_START, 1, None).unwrap())
        .unwrap();
    topology
}

fn build_exploration_harness(seed: Seed) -> GoldenHarness {
    let mut h = GoldenHarness::with_recipes(seed, build_multi_recipe_registry());
    h.world = World::new(build_exploration_topology()).unwrap();
    h.event_log = EventLog::new();
    h.scheduler = Scheduler::new(SystemManifest::canonical());
    h.controller = ControllerState::new();
    h
}

fn exploration_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        entity_memory_capacity: 64,
        entity_claim_capacity: 64,
        memory_retention_ticks: 64,
        observation_fidelity: pm(875),
        confidence_policy: worldwake_core::BeliefConfidencePolicy::default(),
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
    }
}

fn set_agent_exploration_profile(
    h: &mut GoldenHarness,
    agent: EntityId,
    profile: ExplorationProfile,
) {
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_exploration_profile(agent, profile)
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn exploration_agent(h: &mut GoldenHarness, name: &str) -> EntityId {
    let apple_recipe = h
        .recipes
        .recipe_by_name("Harvest Apples")
        .map(|(id, _)| id)
        .expect("exploration scenarios require the harvest-apple recipe");
    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        name,
        PLACE_START,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            hunger_weight: pm(950),
            thirst_weight: pm(100),
            fatigue_weight: pm(0),
            bladder_weight: pm(0),
            ..UtilityProfile::default()
        },
        KnownRecipes::with([apple_recipe]),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        exploration_perception_profile(),
    );
    set_agent_exploration_profile(
        h,
        agent,
        ExplorationProfile {
            curiosity_weight: pm(500),
            need_activation_threshold: pm(400),
            max_consecutive_explorations: 3,
            visit_lookback_ticks: 200,
            consecutive_exploration_count: 0,
        },
    );
    agent
}

fn planning_trace_has_generated_goal(
    h: &GoldenHarness,
    agent: EntityId,
    tick: Tick,
    expected_goal: GoalKey,
) -> bool {
    h.driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(agent, tick)
        .is_some_and(|trace| match &trace.outcome {
            DecisionOutcome::Planning(planning) => planning
                .candidates
                .generated
                .iter()
                .any(|goal| goal.goal_key == expected_goal),
            _ => false,
        })
}

fn any_trace_selected_travel_for_goal(
    h: &GoldenHarness,
    agent: EntityId,
    expected_goal: GoalKey,
) -> bool {
    h.driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .traces_for(agent)
        .iter()
        .any(|trace| match &trace.outcome {
            DecisionOutcome::Planning(planning) => {
                planning.selection.selected_goal_is(expected_goal)
                    && planning
                        .selection
                        .selected_plan
                        .as_ref()
                        .and_then(|plan| plan.next_step.as_ref())
                        .is_some_and(|step| step.op_kind == PlannerOpKind::Travel)
            }
            _ => false,
        })
}

fn planning_trace_has_no_exploration_candidate(
    h: &GoldenHarness,
    agent: EntityId,
    tick: Tick,
) -> bool {
    h.driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(agent, tick)
        .is_some_and(|trace| match &trace.outcome {
            DecisionOutcome::Planning(planning) => !planning
                .candidates
                .generated
                .iter()
                .any(|goal| matches!(goal.goal_key.kind, GoalKind::ExploreLocation { .. })),
            _ => false,
        })
}

fn planning_trace_selected_acquire_apple(h: &GoldenHarness, agent: EntityId, tick: Tick) -> bool {
    h.driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(agent, tick)
        .is_some_and(|trace| match &trace.outcome {
            DecisionOutcome::Planning(planning) => {
                planning
                    .selection
                    .selected_goal_is(GoalKey::from(GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Apple,
                        purpose: CommodityPurpose::SelfConsume,
                    }))
            }
            _ => false,
        })
}

// ---------------------------------------------------------------------------
// Scenario 133: Ignorance-Driven Frontier Exploration
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Perception
// GoalKinds: ExploreLocation
// ActionDomains: Travel
// Places: ExplorationStart, ExplorationFrontier
// Principles: 7, 14, 20
//
// Setup: A hungry agent at ExplorationStart knows only the start place and a frontier place belief for ExplorationFrontier. No believed food source exists, and no competing non-self-care goal families are seeded.
//
// Proves: opening planning emits `ExploreLocation` for the frontier place, and the live planner can turn that fallback into a lawful travel plan rather than skipping directly to concrete food acquisition.
//
// Chain: unmet self-care need + frontier place belief -> exploration candidate generation -> planning selection -> travel plan synthesis.
#[test]
fn golden_exploration_triggers_on_need_and_ignorance() {
    let mut h = build_exploration_harness(Seed([210; 32]));
    h.driver.enable_tracing();

    let agent = exploration_agent(&mut h, "Ignorant Scout");
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        PLACE_START,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    h.step_once();

    let exploration_goal = GoalKey::from(GoalKind::ExploreLocation {
        target_place: PLACE_FRONTIER,
        motivating_need: HomeostaticNeedId::Hunger,
    });

    assert!(
        planning_trace_has_generated_goal(&h, agent, Tick(0), exploration_goal),
        "opening planning trace should generate exploration toward the unknown frontier"
    );
    assert!(
        !planning_trace_selected_acquire_apple(&h, agent, Tick(0)),
        "opening planning trace should not jump directly to concrete apple acquisition while the frontier source is still unknown"
    );

    for _ in 0..6 {
        if any_trace_selected_travel_for_goal(&h, agent, exploration_goal) {
            break;
        }
        h.step_once();
    }

    assert!(
        any_trace_selected_travel_for_goal(&h, agent, exploration_goal),
        "once emitted, exploration should remain plannable as a travel goal toward the frontier place; traces={:#?}",
        h.driver
            .trace_sink()
            .expect("decision tracing should be enabled")
            .traces_for(agent)
    );
}

// ---------------------------------------------------------------------------
// Scenario 134: Known Satisfaction Path Suppresses Exploration
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Production, Perception
// GoalKinds: ExploreLocation, AcquireCommodity(SelfConsume)
// ActionDomains: Production
// Places: ExplorationStart
// Principles: 7, 14, 20
//
// Setup: Same hungry exploration setup as Scenario 133, except the agent also directly observes a lawful local apple source at ExplorationStart.
//
// Proves: once a concrete self-care path is believed, `ExploreLocation` is not generated and planning shifts to `AcquireCommodity(SelfConsume)` instead.
//
// Chain: local source observation -> belief update -> candidate generation -> exploration suppression -> concrete self-care selection.
#[test]
fn golden_exploration_is_suppressed_when_known_satisfaction_path_exists() {
    let mut h = build_exploration_harness(Seed([211; 32]));
    h.driver.enable_tracing();

    let agent = exploration_agent(&mut h, "Informed Scout");
    let orchard_source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        PLACE_START,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(5),
            max_quantity: Quantity(5),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        PLACE_START,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        orchard_source,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    h.step_once();

    assert!(
        planning_trace_has_no_exploration_candidate(&h, agent, Tick(0)),
        "known apple source should suppress exploration emission"
    );
    assert!(
        planning_trace_selected_acquire_apple(&h, agent, Tick(0)),
        "known apple source should shift planning to concrete self-consume acquisition"
    );
}

// ---------------------------------------------------------------------------
// Scenario 135: Consecutive Exploration Cap Suppresses Re-Emission
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Perception
// GoalKinds: ExploreLocation
// ActionDomains: N/A
// Places: ExplorationStart
// Principles: 20, 21, 22A
//
// Setup: A hungry agent at ExplorationStart has the normal frontier belief, but its `ExplorationProfile` is pre-seeded with `consecutive_exploration_count` already at `max_consecutive_explorations`.
//
// Proves: candidate generation honors the profile cap and omits `ExploreLocation` rather than emitting another exploration goal.
//
// Chain: stored exploration-profile counter -> candidate gating -> no exploration fallback emitted.
#[test]
fn golden_exploration_consecutive_cap_is_respected() {
    let mut h = build_exploration_harness(Seed([212; 32]));
    h.driver.enable_tracing();

    let agent = exploration_agent(&mut h, "Capped Scout");
    set_agent_exploration_profile(
        &mut h,
        agent,
        ExplorationProfile {
            max_consecutive_explorations: 1,
            consecutive_exploration_count: 1,
            ..ExplorationProfile::default()
        },
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        PLACE_START,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    h.step_once();

    assert!(
        planning_trace_has_no_exploration_candidate(&h, agent, Tick(0)),
        "reaching the consecutive exploration cap should suppress further ExploreLocation candidates"
    );
}

// ---------------------------------------------------------------------------
// Scenario 136: Arrival Perception Unlocks Concrete Relief
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Perception, Production
// GoalKinds: ExploreLocation, AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Production
// Places: ExplorationStart, ExplorationFrontier
// Principles: 7, 14, 15, 20
//
// Setup: A hungry agent knows only ExplorationStart and a frontier place belief for ExplorationFrontier. The frontier authoritatively contains an apple source, but that source starts unknown to the agent. The agent has an explicit `PerceptionProfile` so post-arrival observation is lawful.
//
// Proves: the agent first commits travel under `ExploreLocation`, then arrival perception adds a resource-source belief for the frontier, and later planning shifts to `AcquireCommodity(SelfConsume)`.
//
// Chain: unmet need + frontier belief -> exploration travel -> arrival perception -> source belief acquisition -> concrete self-care plan.
#[test]
fn golden_exploration_arrival_unlocks_beliefs_and_concrete_relief() {
    let mut h = build_exploration_harness(Seed([213; 32]));
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let agent = exploration_agent(&mut h, "Discovering Scout");
    let frontier_source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        PLACE_FRONTIER,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(5),
            max_quantity: Quantity(5),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        PLACE_START,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    assert!(
        agent_belief_about(&h.world, agent, frontier_source).is_none(),
        "frontier source should start unknown so the scenario proves belief acquisition on arrival"
    );

    let exploration_goal = GoalKey::from(GoalKind::ExploreLocation {
        target_place: PLACE_FRONTIER,
        motivating_need: HomeostaticNeedId::Hunger,
    });

    let mut travel_committed = false;
    let mut reached_frontier = false;
    let mut selected_concrete_relief = false;
    for _ in 0..24 {
        h.step_once();
        let processed_tick = Tick(h.scheduler.current_tick().0.saturating_sub(1));

        travel_committed |= h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(agent)
            .iter()
            .any(|event| {
                event.action_name == "travel"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            });

        reached_frontier |= h.world.effective_place(agent) == Some(PLACE_FRONTIER);

        if h.world.effective_place(agent) == Some(PLACE_FRONTIER)
            && planning_trace_selected_acquire_apple(&h, agent, processed_tick)
        {
            selected_concrete_relief = true;
        }

        if travel_committed
            && reached_frontier
            && selected_concrete_relief
            && agent_belief_about(&h.world, agent, frontier_source)
                .is_some_and(|belief| belief.resource_source.is_some())
        {
            break;
        }
    }

    assert!(
        planning_trace_has_generated_goal(&h, agent, Tick(0), exploration_goal),
        "the opening decision should emit exploration before post-arrival beliefs unlock a concrete relief path"
    );
    assert!(
        travel_committed,
        "exploration scenario should commit a travel action before belief unlock; traces={:#?}; actions={:#?}",
        h.driver
            .trace_sink()
            .expect("decision tracing should be enabled")
            .traces_for(agent),
        h.action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(agent)
    );
    assert!(
        reached_frontier,
        "the agent should reach the frontier place before concrete relief planning; traces={:#?}; actions={:#?}",
        h.driver
            .trace_sink()
            .expect("decision tracing should be enabled")
            .traces_for(agent),
        h.action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(agent)
    );
    assert!(
        agent_belief_about(&h.world, agent, frontier_source)
            .is_some_and(|belief| belief.resource_source.is_some()),
        "arrival and perception should add a belief about the frontier resource source"
    );
    assert!(
        selected_concrete_relief,
        "after the frontier source is perceived, planning should shift to AcquireCommodity for self-consumption"
    );
}
