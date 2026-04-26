//! Golden tests for exploration fallback behavior.

mod golden_harness;

use golden_harness::*;
use worldwake_ai::{
    CognitiveProfile, CommodityPurpose, DecisionOutcome, GoalKey, GoalKind, GoalTraceStatus,
    PlanSearchOutcome, PlannerOpKind, PlanningPipelineTrace,
};
use worldwake_core::{
    AcquisitionQuantity, CommodityKind, DiversificationProfile, EntityId, EventLog,
    ExplorationMotivation, ExplorationProfile, HomeostaticNeedId, HomeostaticNeeds, KnownRecipes,
    MetabolismProfile, PerceptionProfile, PerceptionSource, Place, PlaceTag, Quantity,
    ResourceSource, Seed, Tick, Topology, TravelEdge, TravelEdgeId, UtilityProfile, WorkstationTag,
    World,
};
use worldwake_sim::{ActionTraceKind, ControllerState, Scheduler, SystemManifest};

const PLACE_START: EntityId = entity(900);
const PLACE_FRONTIER: EntityId = entity(901);
const PLACE_TRAIL: EntityId = entity(902);
const PLACE_VILLAGE: EntityId = entity(903);
const PLACE_FOREST: EntityId = entity(904);
const PLACE_FIELDS: EntityId = entity(905);
const PLACE_INN: EntityId = entity(906);
const PLACE_PROACTIVE_HOME: EntityId = entity(920);
const PLACE_PROACTIVE_EAST: EntityId = entity(921);
const PLACE_PROACTIVE_NORTH: EntityId = entity(922);
const PLACE_PROACTIVE_SOUTH: EntityId = entity(923);

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
    build_harness_with_topology(seed, build_exploration_topology())
}

fn build_harness_with_topology(seed: Seed, topology: Topology) -> GoldenHarness {
    let mut h = GoldenHarness::with_recipes(seed, build_multi_recipe_registry());
    h.world = World::new(topology).unwrap();
    h.event_log = EventLog::new();
    h.scheduler = Scheduler::new(SystemManifest::canonical());
    h.controller = ControllerState::new();
    h
}

fn add_bidirectional_edge(
    topology: &mut Topology,
    forward: TravelEdgeId,
    reverse: TravelEdgeId,
    origin: EntityId,
    destination: EntityId,
) {
    topology
        .add_edge(TravelEdge::new(forward, origin, destination, 1, None).unwrap())
        .unwrap();
    topology
        .add_edge(TravelEdge::new(reverse, destination, origin, 1, None).unwrap())
        .unwrap();
}

fn build_gate_unlock_topology() -> Topology {
    let mut topology = Topology::new();
    topology
        .add_place(PLACE_TRAIL, place("Trail", &[PlaceTag::Trail]))
        .unwrap();
    topology
        .add_place(PLACE_VILLAGE, place("Village", &[PlaceTag::Village]))
        .unwrap();
    add_bidirectional_edge(
        &mut topology,
        TravelEdgeId(902),
        TravelEdgeId(903),
        PLACE_TRAIL,
        PLACE_VILLAGE,
    );
    topology
}

fn build_multi_hop_topology() -> Topology {
    let mut topology = Topology::new();
    topology
        .add_place(PLACE_FOREST, place("Forest", &[PlaceTag::Forest]))
        .unwrap();
    topology
        .add_place(PLACE_VILLAGE, place("Village", &[PlaceTag::Village]))
        .unwrap();
    topology
        .add_place(PLACE_FIELDS, place("Fields", &[PlaceTag::Field]))
        .unwrap();
    add_bidirectional_edge(
        &mut topology,
        TravelEdgeId(904),
        TravelEdgeId(905),
        PLACE_FOREST,
        PLACE_VILLAGE,
    );
    add_bidirectional_edge(
        &mut topology,
        TravelEdgeId(906),
        TravelEdgeId(907),
        PLACE_VILLAGE,
        PLACE_FIELDS,
    );
    topology
}

fn build_persistence_topology() -> Topology {
    let mut topology = Topology::new();
    topology
        .add_place(PLACE_FOREST, place("Forest", &[PlaceTag::Forest]))
        .unwrap();
    topology
        .add_place(PLACE_VILLAGE, place("Village", &[PlaceTag::Village]))
        .unwrap();
    topology
        .add_place(PLACE_INN, place("Inn", &[PlaceTag::Inn]))
        .unwrap();
    add_bidirectional_edge(
        &mut topology,
        TravelEdgeId(908),
        TravelEdgeId(909),
        PLACE_FOREST,
        PLACE_VILLAGE,
    );
    add_bidirectional_edge(
        &mut topology,
        TravelEdgeId(910),
        TravelEdgeId(911),
        PLACE_VILLAGE,
        PLACE_INN,
    );
    topology
}

fn build_proactive_branch_topology() -> Topology {
    let mut topology = Topology::new();
    topology
        .add_place(
            PLACE_PROACTIVE_HOME,
            place("ProactiveHome", &[PlaceTag::Village]),
        )
        .unwrap();
    topology
        .add_place(
            PLACE_PROACTIVE_EAST,
            place("ProactiveEast", &[PlaceTag::Field]),
        )
        .unwrap();
    topology
        .add_place(
            PLACE_PROACTIVE_NORTH,
            place("ProactiveNorth", &[PlaceTag::Forest]),
        )
        .unwrap();
    topology
        .add_place(
            PLACE_PROACTIVE_SOUTH,
            place("ProactiveSouth", &[PlaceTag::Farm]),
        )
        .unwrap();
    add_bidirectional_edge(
        &mut topology,
        TravelEdgeId(920),
        TravelEdgeId(921),
        PLACE_PROACTIVE_HOME,
        PLACE_PROACTIVE_EAST,
    );
    add_bidirectional_edge(
        &mut topology,
        TravelEdgeId(922),
        TravelEdgeId(923),
        PLACE_PROACTIVE_HOME,
        PLACE_PROACTIVE_NORTH,
    );
    add_bidirectional_edge(
        &mut topology,
        TravelEdgeId(924),
        TravelEdgeId(925),
        PLACE_PROACTIVE_HOME,
        PLACE_PROACTIVE_SOUTH,
    );
    topology
}

fn exploration_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        entity_activation_threshold: pm(125),
        claim_confidence_threshold: pm(50),
        observation_buffer_capacity: 64,
        observation_budget: 24,
        need_salience_boost: pm(500),
        need_salience_urgency_threshold: pm(500),
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
    hunger_exploration_agent(
        h,
        name,
        PLACE_START,
        KnownRecipes::with([recipe_id(h, "Harvest Apples")]),
    )
}

fn recipe_id(h: &GoldenHarness, name: &str) -> worldwake_core::RecipeId {
    h.recipes
        .recipe_by_name(name)
        .map_or_else(|| panic!("missing recipe {name}"), |(id, _)| id)
}

fn hunger_exploration_agent(
    h: &mut GoldenHarness,
    name: &str,
    start_place: EntityId,
    known_recipes: KnownRecipes,
) -> EntityId {
    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        name,
        start_place,
        HomeostaticNeeds::new(pm(900), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            hunger_weight: pm(950),
            thirst_weight: pm(100),
            fatigue_weight: pm(0),
            bladder_weight: pm(0),
            ..UtilityProfile::default()
        },
        known_recipes,
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
            visit_lookback_ticks: 200,
            ..ExplorationProfile::default()
        },
    );
    agent
}

fn dirtiness_exploration_agent(
    h: &mut GoldenHarness,
    name: &str,
    start_place: EntityId,
    known_recipes: KnownRecipes,
) -> EntityId {
    let apple_recipe = h
        .recipes
        .recipe_by_name("Harvest Water")
        .map(|(id, _)| id)
        .expect("exploration scenarios require the harvest-water recipe");
    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        name,
        start_place,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(900)),
        MetabolismProfile::default(),
        UtilityProfile {
            hunger_weight: pm(100),
            thirst_weight: pm(100),
            fatigue_weight: pm(0),
            bladder_weight: pm(0),
            dirtiness_weight: pm(950),
            ..UtilityProfile::default()
        },
        if known_recipes.recipes.is_empty() {
            KnownRecipes::with([apple_recipe])
        } else {
            known_recipes
        },
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
            visit_lookback_ticks: 200,
            ..ExplorationProfile::default()
        },
    );
    agent
}

fn calm_metabolism_profile() -> MetabolismProfile {
    MetabolismProfile {
        hunger_rate: pm(0),
        thirst_rate: pm(0),
        fatigue_rate: pm(0),
        bladder_rate: pm(0),
        dirtiness_rate: pm(0),
        ..MetabolismProfile::default()
    }
}

fn isolated_utility_profile() -> UtilityProfile {
    UtilityProfile {
        social_weight: pm(0),
        activity_awareness_weight: pm(0),
        care_weight: pm(0),
        enterprise_weight: pm(0),
        side_benefit_weight: pm(0),
        ..UtilityProfile::default()
    }
}

fn set_agent_diversification_profile(
    h: &mut GoldenHarness,
    agent: EntityId,
    profile: DiversificationProfile,
) {
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_diversification_profile(agent, profile)
        .expect("golden harness should keep diversification profiles writable");
    commit_txn(txn, &mut h.event_log);
}

fn comfortable_proactive_agent(h: &mut GoldenHarness, name: &str) -> EntityId {
    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        name,
        PLACE_PROACTIVE_HOME,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(0)),
        calm_metabolism_profile(),
        isolated_utility_profile(),
        KnownRecipes::new(),
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
            visit_lookback_ticks: 200,
            ..ExplorationProfile::default()
        },
    );
    agent
}

fn processed_tick(h: &GoldenHarness) -> Tick {
    Tick(h.scheduler.current_tick().0.saturating_sub(1))
}

fn exploration_goal(target_place: EntityId, motivating_need: HomeostaticNeedId) -> GoalKey {
    GoalKey::from(GoalKind::ExploreLocation {
        target_place,
        motivating_need: ExplorationMotivation::NeedDriven(motivating_need),
    })
}

fn acquire_goal(commodity: CommodityKind) -> GoalKey {
    GoalKey::from(GoalKind::AcquireCommodity {
        commodity,
        purpose: CommodityPurpose::SelfConsume,
        quantity: AcquisitionQuantity::single(),
    })
}

fn planning_trace_at(
    h: &GoldenHarness,
    agent: EntityId,
    tick: Tick,
) -> Option<&PlanningPipelineTrace> {
    match &h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(agent, tick)?
        .outcome
    {
        DecisionOutcome::Planning(planning) => Some(planning.as_ref()),
        _ => None,
    }
}

fn planning_trace_selected_goal(
    h: &GoldenHarness,
    agent: EntityId,
    tick: Tick,
    expected_goal: GoalKey,
) -> bool {
    planning_trace_at(h, agent, tick)
        .is_some_and(|planning| planning.selection.selected_goal_is(expected_goal))
}

fn planning_trace_has_budget_exhausted_attempt(
    h: &GoldenHarness,
    agent: EntityId,
    tick: Tick,
    expected_goal: GoalKey,
) -> bool {
    planning_trace_at(h, agent, tick).is_some_and(|planning| {
        planning.planning.attempts.iter().any(|attempt| {
            attempt.goal == expected_goal
                && matches!(attempt.outcome, PlanSearchOutcome::BudgetExhausted { .. })
        })
    })
}

fn planning_trace_has_budget_exhausted_attempt_for_any(
    h: &GoldenHarness,
    agent: EntityId,
    tick: Tick,
    expected_goals: &[GoalKey],
) -> bool {
    expected_goals
        .iter()
        .copied()
        .any(|goal| planning_trace_has_budget_exhausted_attempt(h, agent, tick, goal))
}

fn planning_trace_selected_goal_status(
    h: &GoldenHarness,
    agent: EntityId,
    tick: Tick,
    goal: &GoalKind,
) -> GoalTraceStatus {
    h.driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .goal_status_at(agent, tick, goal)
}

fn any_committed_action_named(h: &GoldenHarness, agent: EntityId, action_name: &str) -> bool {
    h.action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(agent)
        .iter()
        .any(|event| {
            event.action_name == action_name
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        })
}

fn acquisition_exhaustion_count(h: &GoldenHarness, agent: EntityId, need: HomeostaticNeedId) -> u8 {
    h.world
        .get_component_acquisition_exhaustion_tracker(agent)
        .map_or(0, |tracker| tracker.count(need))
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

fn planning_trace_has_generated_proactive_exploration(
    h: &GoldenHarness,
    agent: EntityId,
    tick: Tick,
) -> bool {
    h.driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(agent, tick)
        .is_some_and(|trace| match &trace.outcome {
            DecisionOutcome::Planning(planning) => {
                planning.candidates.generated.iter().any(|goal| {
                    matches!(
                        goal.goal_key.kind,
                        GoalKind::ExploreLocation {
                            motivating_need: ExplorationMotivation::Proactive,
                            ..
                        }
                    )
                })
            }
            _ => false,
        })
}

fn selected_proactive_exploration_goals(
    h: &GoldenHarness,
    agent: EntityId,
) -> Vec<(Tick, EntityId)> {
    h.driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .traces_for(agent)
        .iter()
        .filter_map(|trace| match &trace.outcome {
            DecisionOutcome::Planning(planning) => {
                let GoalKind::ExploreLocation {
                    target_place,
                    motivating_need: ExplorationMotivation::Proactive,
                } = planning.selection.selected_goal()?.kind
                else {
                    return None;
                };
                Some((trace.tick, target_place))
            }
            _ => None,
        })
        .collect()
}

fn any_generated_need_driven_exploration(h: &GoldenHarness, agent: EntityId) -> bool {
    h.driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .traces_for(agent)
        .iter()
        .any(|trace| match &trace.outcome {
            DecisionOutcome::Planning(planning) => {
                planning.candidates.generated.iter().any(|goal| {
                    matches!(
                        goal.goal_key.kind,
                        GoalKind::ExploreLocation {
                            motivating_need: ExplorationMotivation::NeedDriven(_),
                            ..
                        }
                    )
                })
            }
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
                        quantity: AcquisitionQuantity::single(),
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
        motivating_need: ExplorationMotivation::NeedDriven(HomeostaticNeedId::Hunger),
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
            extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
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
            extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
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
        motivating_need: ExplorationMotivation::NeedDriven(HomeostaticNeedId::Hunger),
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

// ---------------------------------------------------------------------------
// Scenario 337: Budget Exhaustion Unlocks Frontier Exploration
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Production
// GoalKinds: AcquireCommodity(SelfConsume), ExploreLocation
// ActionDomains: Travel, Production
// Places: Trail, Village
// Principles: 14, 20, 22
//
// Setup: A hungry agent at Trail already believes there is a lawful apple
// source at Village, but its low planning budget repeatedly exhausts the
// remote acquire path. The exploration profile uses
// `acquisition_failure_threshold = 3` and a short lookback so Village can be
// revisited once the path becomes unreliable.
//
// Proves: repeated `BudgetExhausted` acquire attempts increment the
// authoritative exhaustion tracker, then exploration bypasses known-path
// suppression and selects travel to Village.
//
// Chain: known remote relief path -> repeated budget exhaustion -> stored
// failure count reaches threshold -> exploration emitted/selected -> travel
// commit.
#[test]
fn golden_s102_gate_unlock_after_budget_exhaustion() {
    let mut h = build_harness_with_topology(Seed([214; 32]), build_gate_unlock_topology());
    h.driver.enable_tracing();
    h.enable_action_tracing();
    let harvest_apple = recipe_id(&h, "Harvest Apples");

    let agent = hunger_exploration_agent(
        &mut h,
        "Budget-Limited Scout",
        PLACE_TRAIL,
        KnownRecipes::with([harvest_apple]),
    );
    set_agent_cognitive_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        CognitiveProfile {
            max_node_expansions: 1,
            ..CognitiveProfile::default()
        },
    );
    set_agent_exploration_profile(
        &mut h,
        agent,
        ExplorationProfile {
            curiosity_weight: pm(500),
            need_activation_threshold: pm(400),
            visit_lookback_ticks: 1,
            acquisition_failure_threshold: 3,
            ..ExplorationProfile::default()
        },
    );

    let village_source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        PLACE_VILLAGE,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(5),
            max_quantity: Quantity(5),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
        },
        ProductionOutputOwner::Actor,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        PLACE_TRAIL,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        PLACE_VILLAGE,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        village_source,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    let acquire_apple = acquire_goal(CommodityKind::Apple);
    let produce_apples = GoalKey::from(GoalKind::ProduceCommodity {
        recipe_id: harvest_apple,
    });
    let explore_village = exploration_goal(PLACE_VILLAGE, HomeostaticNeedId::Hunger);
    let mut budget_exhaustion_ticks = Vec::new();
    let mut unlock_tick = None;
    let mut reached_village = false;

    for _ in 0..20 {
        h.step_once();
        let tick = processed_tick(&h);

        if planning_trace_has_budget_exhausted_attempt_for_any(
            &h,
            agent,
            tick,
            &[acquire_apple, produce_apples],
        ) {
            budget_exhaustion_ticks.push(tick);
        }

        if unlock_tick.is_none() && planning_trace_selected_goal(&h, agent, tick, explore_village) {
            unlock_tick = Some(tick);
        }

        reached_village |= h.world.effective_place(agent) == Some(PLACE_VILLAGE);

        if unlock_tick.is_some() && reached_village {
            break;
        }
    }

    let unlock_tick = unlock_tick.unwrap_or_else(|| {
        panic!(
            "exploration should unlock after repeated budget exhaustion; budget_ticks={budget_exhaustion_ticks:?}; tracker={}; traces={:#?}; actions={:#?}",
            acquisition_exhaustion_count(&h, agent, HomeostaticNeedId::Hunger),
            h.driver
                .trace_sink()
                .expect("decision tracing should be enabled")
                .traces_for(agent),
            h.action_trace_sink()
                .expect("action tracing should be enabled")
                .events_for(agent)
        )
    });
    assert!(
        budget_exhaustion_ticks.len() >= 3,
        "the remote acquire path should budget-exhaust at least three times before exploration unlocks; ticks={budget_exhaustion_ticks:?}"
    );
    assert!(
        acquisition_exhaustion_count(&h, agent, HomeostaticNeedId::Hunger) >= 3,
        "budget exhaustion should persist on the authoritative hunger tracker before satisfaction"
    );
    assert!(
        planning_trace_has_generated_goal(&h, agent, unlock_tick, explore_village),
        "the unlock tick should emit ExploreLocation toward Village"
    );
    assert!(
        reached_village,
        "once the known path is marked unreliable, the agent should commit travel to Village; traces={:#?}; actions={:#?}",
        h.driver
            .trace_sink()
            .expect("decision tracing should be enabled")
            .traces_for(agent),
        h.action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(agent)
    );
    assert!(
        any_committed_action_named(&h, agent, "travel"),
        "unlocking exploration should commit a travel action"
    );
}

// ---------------------------------------------------------------------------
// Scenario 338: Multi-Hop Frontier Discovery Composes Across Rounds
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Production, Perception
// GoalKinds: ExploreLocation, AcquireCommodity(SelfConsume)
// ActionDomains: Travel, Production
// Places: Forest, Village, Fields
// Principles: 7, 14, 20, 22
//
// Setup: A hungry agent starts in Forest, knows only Forest, and has
// `frontier_depth = 2`. The only food source is a grain field at Fields.
//
// Proves: exploration first selects Village, later selects Fields, and arrival
// perception at Fields unlocks a concrete grain acquisition path.
//
// Chain: start-place belief -> ranked multi-hop frontier search -> first-hop
// travel -> second-hop travel -> source belief acquisition -> concrete acquire
// selection.
#[test]
fn golden_s102_multi_hop_frontier_discovery() {
    let mut h = build_harness_with_topology(Seed([215; 32]), build_multi_hop_topology());
    h.driver.enable_tracing();
    h.enable_action_tracing();
    let harvest_grain = recipe_id(&h, "Harvest Grain");

    let agent = hunger_exploration_agent(
        &mut h,
        "Frontier Walker",
        PLACE_FOREST,
        KnownRecipes::with([harvest_grain]),
    );
    set_agent_exploration_profile(
        &mut h,
        agent,
        ExplorationProfile {
            curiosity_weight: pm(500),
            need_activation_threshold: pm(400),
            frontier_depth: 2,
            ..ExplorationProfile::default()
        },
    );

    let field_source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        PLACE_FIELDS,
        WorkstationTag::FieldPlot,
        ResourceSource {
            commodity: CommodityKind::Grain,
            available_quantity: Quantity(5),
            max_quantity: Quantity(5),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
        },
        ProductionOutputOwner::Actor,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        PLACE_FOREST,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    let explore_village = exploration_goal(PLACE_VILLAGE, HomeostaticNeedId::Hunger);
    let explore_fields = exploration_goal(PLACE_FIELDS, HomeostaticNeedId::Hunger);
    let acquire_grain = acquire_goal(CommodityKind::Grain);
    let mut selected_village_tick = None;
    let mut selected_fields_tick = None;
    let mut reached_fields = false;
    let mut selected_acquire_grain = false;

    for _ in 0..36 {
        h.step_once();
        let tick = processed_tick(&h);

        if selected_village_tick.is_none()
            && planning_trace_selected_goal(&h, agent, tick, explore_village)
        {
            selected_village_tick = Some(tick);
        }
        if selected_fields_tick.is_none()
            && planning_trace_selected_goal(&h, agent, tick, explore_fields)
        {
            selected_fields_tick = Some(tick);
        }
        if planning_trace_selected_goal(&h, agent, tick, acquire_grain) {
            selected_acquire_grain = true;
        }

        reached_fields |= h.world.effective_place(agent) == Some(PLACE_FIELDS);

        if reached_fields
            && selected_acquire_grain
            && agent_belief_about(&h.world, agent, field_source)
                .is_some_and(|belief| belief.resource_source.is_some())
        {
            break;
        }
    }

    let selected_village_tick =
        selected_village_tick.expect("the first exploration round should select Village");
    let selected_fields_tick =
        selected_fields_tick.expect("a later exploration round should select Fields");
    assert!(
        selected_fields_tick.0 > selected_village_tick.0,
        "Fields should be selected only after the first-hop Village round; village={selected_village_tick:?}, fields={selected_fields_tick:?}"
    );
    assert!(
        reached_fields,
        "multi-hop exploration should physically reach the second-hop Fields place"
    );
    assert!(
        agent_belief_about(&h.world, agent, field_source)
            .is_some_and(|belief| belief.resource_source.is_some()),
        "arrival perception at Fields should add a belief about the grain field source"
    );
    assert!(
        selected_acquire_grain,
        "once the Fields source is discovered, planning should shift to AcquireCommodity(Grain)"
    );
}

#[derive(Debug)]
struct PersistenceOutcome {
    village_belief_after_arrival: bool,
    village_presentation_tick_count: u8,
    reached_inn: bool,
    selected_inn_tick: Option<Tick>,
    discovered_basin: bool,
}

fn run_persistence_scenario(seed: Seed, exploration_arrival_boost: u16) -> PersistenceOutcome {
    let mut h = build_harness_with_topology(seed, build_persistence_topology());
    h.driver.enable_tracing();
    h.enable_action_tracing();
    let agent =
        dirtiness_exploration_agent(&mut h, "Dusty Traveler", PLACE_FOREST, KnownRecipes::new());
    let mut perception = exploration_perception_profile();
    perception.entity_activation_threshold = pm(900);
    set_agent_perception_profile(&mut h.world, &mut h.event_log, agent, perception);
    set_agent_exploration_profile(
        &mut h,
        agent,
        ExplorationProfile {
            curiosity_weight: pm(500),
            need_activation_threshold: pm(400),
            frontier_depth: 1,
            exploration_arrival_boost: pm(exploration_arrival_boost),
            ..ExplorationProfile::default()
        },
    );

    let inn_basin = place_workstation(
        &mut h.world,
        &mut h.event_log,
        PLACE_INN,
        WorkstationTag::WashBasin,
        ProductionOutputOwner::Actor,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        PLACE_FOREST,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    let explore_inn = exploration_goal(PLACE_INN, HomeostaticNeedId::Dirtiness);
    let mut village_belief_after_arrival = false;
    let mut village_presentation_tick_count = 0;
    let mut reached_inn = false;
    let mut selected_inn_tick = None;
    let mut discovered_basin = false;

    for _ in 0..40 {
        h.step_once();
        let tick = processed_tick(&h);

        if h.world.effective_place(agent) == Some(PLACE_VILLAGE)
            && let Some(belief) = agent_belief_about(&h.world, agent, PLACE_VILLAGE)
        {
            village_belief_after_arrival = true;
            village_presentation_tick_count =
                village_presentation_tick_count.max(belief.presentation_tick_count);
        }
        if selected_inn_tick.is_none() && planning_trace_selected_goal(&h, agent, tick, explore_inn)
        {
            selected_inn_tick = Some(tick);
        }
        reached_inn |= h.world.effective_place(agent) == Some(PLACE_INN);
        discovered_basin |= agent_belief_about(&h.world, agent, inn_basin)
            .is_some_and(|belief| belief.workstation_tag == Some(WorkstationTag::WashBasin));

        if selected_inn_tick.is_some() && discovered_basin {
            break;
        }
    }

    PersistenceOutcome {
        village_belief_after_arrival,
        village_presentation_tick_count,
        reached_inn,
        selected_inn_tick,
        discovered_basin,
    }
}

// ---------------------------------------------------------------------------
// Scenario 339: Arrival Boost Preserves The Exploration Chain
// ---------------------------------------------------------------------------
//
// Systems: AI, Travel, Perception, Needs, Production
// GoalKinds: ExploreLocation
// ActionDomains: Travel, Perception
// Places: Forest, Village, Inn
// Principles: 7, 14, 15, 22
//
// Setup: Two otherwise identical dirtiness-driven runs differ only in
// `exploration_arrival_boost`. The world is a 1-hop chain Forest -> Village ->
// Inn, and the Inn contains a wash basin that starts unknown.
//
// Proves: the boosted run records a stronger Village place belief on arrival,
// then continues the chain to Inn where arrival perception discovers the wash
// basin. The zero-boost control remains a comparison run for the belief-state
// reinforcement itself.
//
// Chain: first-hop exploration arrival -> boosted place belief retention ->
// second-hop exploration -> arrival perception at Inn -> facility discovery.
#[test]
fn golden_s102_exploration_chain_belief_persistence() {
    let boosted = run_persistence_scenario(Seed([216; 32]), 500);
    let unboosted = run_persistence_scenario(Seed([217; 32]), 0);

    assert!(
        boosted.village_belief_after_arrival,
        "the boosted run should retain a Village belief after first arrival; boosted={boosted:?}; unboosted={unboosted:?}"
    );
    assert!(
        boosted.village_presentation_tick_count > unboosted.village_presentation_tick_count,
        "arrival boost should increase the retained Village presentation history; boosted={boosted:?}; unboosted={unboosted:?}"
    );
    assert!(
        boosted.selected_inn_tick.is_some() && boosted.reached_inn,
        "the boosted run should continue exploration to Inn; boosted={boosted:?}; unboosted={unboosted:?}"
    );
    assert!(
        boosted.discovered_basin,
        "the boosted run should discover the Inn wash basin"
    );

    assert!(
        unboosted.village_belief_after_arrival,
        "the control run should still record the baseline Village belief for comparison; outcome={unboosted:?}"
    );
}

// ---------------------------------------------------------------------------
// Scenario 340: Need Satisfaction Lazily Resets Exhaustion State
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Production
// GoalKinds: AcquireCommodity(SelfConsume), ExploreLocation
// ActionDomains: Travel, Production, Needs
// Places: Trail, Village
// Principles: 3, 20, 22
//
// Setup: Same as Scenario 337, but the run continues through local apple
// harvest and eating after exploration reaches Village.
//
// Proves: the authoritative hunger exhaustion counter stays non-zero while the
// need is active, then clears on the next candidate-generation pass after
// hunger relief drops below the exploration activation threshold.
//
// Chain: repeated budget exhaustion -> exploration unlock -> local relief ->
// satisfied hunger -> lazy tracker reset on next planning tick.
#[test]
fn golden_s102_counter_reset_on_need_satisfaction() {
    let mut h = build_harness_with_topology(Seed([218; 32]), build_gate_unlock_topology());
    h.driver.enable_tracing();
    let harvest_apple = recipe_id(&h, "Harvest Apples");

    let agent = hunger_exploration_agent(
        &mut h,
        "Resetting Scout",
        PLACE_TRAIL,
        KnownRecipes::with([harvest_apple]),
    );
    set_agent_cognitive_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        CognitiveProfile {
            max_node_expansions: 1,
            ..CognitiveProfile::default()
        },
    );
    set_agent_exploration_profile(
        &mut h,
        agent,
        ExplorationProfile {
            curiosity_weight: pm(500),
            need_activation_threshold: pm(400),
            visit_lookback_ticks: 1,
            acquisition_failure_threshold: 3,
            ..ExplorationProfile::default()
        },
    );

    let village_source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        PLACE_VILLAGE,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(5),
            max_quantity: Quantity(5),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
        },
        ProductionOutputOwner::Actor,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        PLACE_TRAIL,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        PLACE_VILLAGE,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        village_source,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    let explore_village_kind = GoalKind::ExploreLocation {
        target_place: PLACE_VILLAGE,
        motivating_need: ExplorationMotivation::NeedDriven(HomeostaticNeedId::Hunger),
    };
    let mut saw_nonzero_tracker = false;
    let mut reset_tick = None;

    for _ in 0..48 {
        h.step_once();
        let tick = processed_tick(&h);
        let hunger = h
            .world
            .get_component_homeostatic_needs(agent)
            .map_or(pm(0), |needs| needs.hunger);
        let tracker_count = acquisition_exhaustion_count(&h, agent, HomeostaticNeedId::Hunger);
        saw_nonzero_tracker |= tracker_count > 0;

        if saw_nonzero_tracker
            && hunger < pm(400)
            && tracker_count == 0
            && matches!(
                planning_trace_selected_goal_status(&h, agent, tick, &explore_village_kind),
                GoalTraceStatus::NotGenerated | GoalTraceStatus::NoTrace
            )
        {
            reset_tick = Some(tick);
            break;
        }
    }

    let reset_tick = reset_tick.expect(
        "hunger relief should lazily clear the exhaustion tracker on a later planning tick",
    );
    assert!(
        saw_nonzero_tracker,
        "the scenario should accumulate non-zero hunger exhaustion before relief"
    );
    assert_eq!(
        acquisition_exhaustion_count(&h, agent, HomeostaticNeedId::Hunger),
        0,
        "the authoritative hunger exhaustion tracker should reset after satisfaction"
    );
    assert!(
        matches!(
            planning_trace_selected_goal_status(&h, agent, reset_tick, &explore_village_kind),
            GoalTraceStatus::NotGenerated | GoalTraceStatus::NoTrace
        ),
        "once hunger is satisfied, the next planning pass should not re-emit hunger-motivated exploration"
    );
}

#[derive(Debug)]
struct ProactiveDiscoveryOutcome {
    selected_proactive_goals: Vec<(Tick, EntityId)>,
    reached_south: bool,
}

fn run_proactive_discovery_scenario(seed: Seed, with_profile: bool) -> ProactiveDiscoveryOutcome {
    let mut h = build_harness_with_topology(seed, build_proactive_branch_topology());
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let agent = comfortable_proactive_agent(&mut h, "Calm Scout");
    if with_profile {
        set_agent_diversification_profile(
            &mut h,
            agent,
            DiversificationProfile {
                base_curiosity: pm(900),
                comfort_threshold: pm(450),
                curiosity_buildup_rate: pm(250),
                exploration_cooldown_ticks: 6,
                familiarity_per_visit: pm(150),
                familiarity_recovery_per_tick: pm(2),
                familiarity_floor: pm(50),
                max_exploration_hops: 2,
            },
        );
    }
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        PLACE_PROACTIVE_HOME,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    let mut reached_south = false;
    for _ in 0..16 {
        h.step_once();
        reached_south |= h.world.effective_place(agent) == Some(PLACE_PROACTIVE_SOUTH);
        if reached_south {
            break;
        }
    }

    ProactiveDiscoveryOutcome {
        selected_proactive_goals: selected_proactive_exploration_goals(&h, agent),
        reached_south,
    }
}

// ---------------------------------------------------------------------------
// Scenario 343: Diversification Profile Unlocks Proactive Discovery
// ---------------------------------------------------------------------------
//
// Systems: AI, Travel, Perception
// GoalKinds: ExploreLocation
// ActionDomains: Travel
// Places: ProactiveHome, ProactiveEast, ProactiveNorth, ProactiveSouth
// Principles: 7, 14, 22
//
// Setup: Two otherwise identical calm runs start with only a belief about ProactiveHome. The diversified run has a `DiversificationProfile`; the control run does not. No survival-pressure goals are active.
//
// Proves: the diversified run selects and completes proactive exploration to the novel branch place, while the matched control never selects proactive exploration and never reaches that branch.
//
// Chain: calm needs + diversification profile -> proactive ExploreLocation emission/selection -> travel commit -> arrival at novel branch.
#[test]
fn golden_s107_proactive_diversification_discovers_novel_place() {
    let diversified = run_proactive_discovery_scenario(Seed([219; 32]), true);
    let control = run_proactive_discovery_scenario(Seed([220; 32]), false);

    assert_eq!(
        diversified.selected_proactive_goals.first().copied(),
        Some((Tick(1), PLACE_PROACTIVE_SOUTH)),
        "the diversified run should first select proactive exploration toward the highest-novelty branch; outcome={diversified:?}"
    );
    assert!(
        diversified.reached_south,
        "the diversified run should complete travel to the novel branch; outcome={diversified:?}"
    );
    assert!(
        control.selected_proactive_goals.is_empty(),
        "the control run should never select proactive exploration without the profile; outcome={control:?}"
    );
    assert!(
        !control.reached_south,
        "without the diversification profile, the matched control should not reach the novel branch; outcome={control:?}"
    );
}

// ---------------------------------------------------------------------------
// Scenario 344: Need Pressure Vetoes Proactive Motivation
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Travel, Perception
// GoalKinds: ExploreLocation
// ActionDomains: Travel
// Places: ExplorationStart, ExplorationFrontier
// Principles: 7, 14, 22
//
// Setup: A hungry exploration run reuses the standard need-driven frontier setup, but also adds a `DiversificationProfile`. The frontier remains the only lawful exploration path.
//
// Proves: high need pressure suppresses proactive exploration specifically, while lawful need-driven exploration still appears.
//
// Chain: high hunger pressure + diversification profile -> proactive veto -> need-driven ExploreLocation remains generated/selected.
#[test]
fn golden_s107_need_slack_veto_suppresses_proactive_exploration() {
    let mut h = build_exploration_harness(Seed([221; 32]));
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let agent = exploration_agent(&mut h, "Hungry Diversifier");
    set_agent_diversification_profile(
        &mut h,
        agent,
        DiversificationProfile {
            base_curiosity: pm(900),
            comfort_threshold: pm(450),
            curiosity_buildup_rate: pm(250),
            exploration_cooldown_ticks: 6,
            familiarity_per_visit: pm(150),
            familiarity_recovery_per_tick: pm(2),
            familiarity_floor: pm(50),
            max_exploration_hops: 2,
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

    let mut saw_need_driven_exploration = false;
    for _ in 0..8 {
        h.step_once();
        let tick = processed_tick(&h);
        assert!(
            !planning_trace_has_generated_proactive_exploration(&h, agent, tick),
            "high need pressure should veto proactive exploration emission on every planning tick; tick={tick:?}; traces={:#?}",
            h.driver
                .trace_sink()
                .expect("decision tracing should be enabled")
                .traces_for(agent)
        );
        saw_need_driven_exploration |= any_generated_need_driven_exploration(&h, agent);
    }

    assert!(
        saw_need_driven_exploration,
        "the veto scenario should still exercise lawful need-driven exploration rather than removing exploration entirely"
    );
    assert!(
        selected_proactive_exploration_goals(&h, agent).is_empty(),
        "high need pressure should prevent proactive exploration selection as well as emission"
    );
}

// ---------------------------------------------------------------------------
// Scenario 345: Proactive Cooldown Spaces Repeated Exploration
// ---------------------------------------------------------------------------
//
// Systems: AI, Travel, Perception
// GoalKinds: ExploreLocation
// ActionDomains: Travel
// Places: ProactiveHome, ProactiveEast, ProactiveNorth, ProactiveSouth
// Principles: 7, 14, 22
//
// Setup: A calm agent with a short proactive cooldown starts knowing only ProactiveHome in a branching topology with several novel targets.
//
// Proves: the run produces repeated proactive exploration selections, and the selection ticks stay spaced by at least the configured cooldown.
//
// Chain: calm needs + diversification profile -> proactive selection -> authoritative cooldown stamp -> later proactive selection after cooldown.
#[test]
fn golden_s107_cooldown_spaces_proactive_exploration_attempts() {
    let mut h = build_harness_with_topology(Seed([222; 32]), build_proactive_branch_topology());
    h.driver.enable_tracing();

    let agent = comfortable_proactive_agent(&mut h, "Cooldown Scout");
    let cooldown_ticks = 4;
    set_agent_diversification_profile(
        &mut h,
        agent,
        DiversificationProfile {
            base_curiosity: pm(900),
            comfort_threshold: pm(450),
            curiosity_buildup_rate: pm(250),
            exploration_cooldown_ticks: cooldown_ticks,
            familiarity_per_visit: pm(150),
            familiarity_recovery_per_tick: pm(2),
            familiarity_floor: pm(50),
            max_exploration_hops: 2,
        },
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        agent,
        PLACE_PROACTIVE_HOME,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    for _ in 0..18 {
        h.step_once();
    }

    let proactive_ticks = selected_proactive_exploration_goals(&h, agent);
    assert!(
        proactive_ticks.len() >= 2,
        "the cooldown scenario should produce repeated proactive exploration selections; traces={:#?}",
        h.driver
            .trace_sink()
            .expect("decision tracing should be enabled")
            .traces_for(agent)
    );
    for pair in proactive_ticks.windows(2) {
        let [(first_tick, _), (second_tick, _)] = pair else {
            continue;
        };
        assert!(
            second_tick.0.saturating_sub(first_tick.0) >= u64::from(cooldown_ticks),
            "consecutive proactive selections should respect the configured cooldown; proactive_ticks={proactive_ticks:?}"
        );
    }
}
