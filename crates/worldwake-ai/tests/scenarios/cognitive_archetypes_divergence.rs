//! Golden coverage for S167 cognitive archetype behavioral divergence.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_ai::{
    AgentDecisionRuntime, DecisionOutcome, PlannerOpKind, SelectedPlanSearchProvenance,
    TravelSuccessorTrace,
};
use worldwake_cli::scenario::{
    load_scenario_file, spawn_scenario,
    types::{AgentDef, ScenarioDef},
};
use worldwake_core::{
    AcquisitionQuantity, CognitiveArchetype, CommodityKind, CommodityPurpose, EntityId, EventId,
    GoalKey, GoalKind, PerceptionSource, Permille, RoutePreference, RoutePreferenceProfile,
    RouteSegment, StateHash, Tick,
};

const FORWARD_GREEDY_AGENT: &str = "Greedy Rowan";
const FORWARD_CAUTIOUS_AGENT: &str = "Cautious Rowan";
const START_PLACE: &str = "Market Green";
const DIRECT_DESTINATION: &str = "Risky Orchard";
const ALTERNATE_FIRST_HOP: &str = "Sheltered Cut";
const DIVERGENCE_TICK: Tick = Tick(0);

#[derive(Clone, Debug, Eq, PartialEq)]
struct RunObservation {
    world_hash_after_divergence: StateHash,
    greedy: AgentObservation,
    cautious: AgentObservation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentObservation {
    agent: EntityId,
    selected_goal: GoalKey,
    first_travel_target: EntityId,
    direct_successor: TravelSuccessorTrace,
    selected_search: SelectedPlanSearchProvenance,
    route_preference_profile: RoutePreferenceProfile,
    direct_route_preference: Permille,
    known_entities_hash: StateHash,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/cognitive-archetypes-divergence.ron")
}

fn load_s167_def() -> ScenarioDef {
    load_scenario_file(&scenario_path()).expect("cognitive archetypes scenario should parse")
}

fn swapped_archetype_def() -> ScenarioDef {
    let mut def = load_s167_def();
    set_agent_archetype(&mut def, FORWARD_GREEDY_AGENT, CognitiveArchetype::Cautious);
    set_agent_archetype(&mut def, FORWARD_CAUTIOUS_AGENT, CognitiveArchetype::Greedy);
    def
}

fn set_agent_archetype(def: &mut ScenarioDef, name: &str, archetype: CognitiveArchetype) {
    let agent = def
        .agents
        .iter_mut()
        .find(|agent| agent.name == name)
        .unwrap_or_else(|| panic!("scenario should contain agent '{name}'"));
    agent.archetype = Some(archetype);
}

fn observe_run(def: &ScenarioDef) -> RunObservation {
    let spawned = spawn_scenario(def).expect("cognitive archetypes scenario should spawn");
    let mut h = GoldenHarness::from_simulation_state(&spawned.state);
    h.driver.enable_tracing();

    let start = scenario_place_id(def, START_PLACE);
    let direct_destination = scenario_place_id(def, DIRECT_DESTINATION);
    let alternate_first_hop = scenario_place_id(def, ALTERNATE_FIRST_HOP);
    let direct_segment = RouteSegment::new(start, direct_destination);
    let agents = archetype_agents(&h);
    let resource_entities = resource_entities(&h);

    for agent in [agents.greedy, agents.cautious] {
        set_agent_hunger(&mut h, agent, Permille::new_unchecked(700));
        seed_actor_beliefs(
            &mut h.world,
            &mut h.event_log,
            agent,
            &resource_entities,
            Tick(0),
            PerceptionSource::Inference,
        );
        seed_mixed_direct_route_memory(&mut h, agent, direct_segment);
    }

    let greedy_known_entities_hash = known_entities_hash(&h, agents.greedy);
    let cautious_known_entities_hash = known_entities_hash(&h, agents.cautious);
    assert_eq!(
        greedy_known_entities_hash, cautious_known_entities_hash,
        "agents should enter the divergence tick with identical seeded decision-side entity beliefs"
    );

    h.step_once();
    assert_eq!(
        h.scheduler.current_tick(),
        Tick(1),
        "the pinned divergence check expects one planning tick to advance from tick 0"
    );

    let greedy = observe_agent(&h, agents.greedy, direct_segment);
    let cautious = observe_agent(&h, agents.cautious, direct_segment);

    assert_eq!(greedy.selected_goal, cautious.selected_goal);
    assert_eq!(
        greedy.selected_goal,
        GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        })
    );
    assert_eq!(
        greedy.first_travel_target, direct_destination,
        "Greedy should choose direct route; observation={greedy:?}"
    );
    assert_eq!(
        cautious.first_travel_target, alternate_first_hop,
        "Cautious should choose neutral first hop; observation={cautious:?}"
    );
    assert_ne!(greedy.first_travel_target, cautious.first_travel_target);

    assert!(
        greedy.route_preference_profile.dangerous_traversal_penalty
            < cautious
                .route_preference_profile
                .dangerous_traversal_penalty,
        "Greedy should resolve a lower dangerous-route penalty than Cautious"
    );
    assert!(
        greedy.direct_route_preference > Permille::new_unchecked(500),
        "Greedy mixed-route memory should price the direct route as preferred"
    );
    assert!(
        cautious.direct_route_preference < Permille::new_unchecked(500),
        "Cautious mixed-route memory should price the direct route as avoided"
    );
    assert!(
        greedy.direct_successor.projected_total_cost
            < cautious.direct_successor.projected_total_cost,
        "the resolved profile delta should make the direct branch cheaper for Greedy than for Cautious"
    );
    assert_eq!(
        greedy
            .selected_search
            .selected_root_travel_destination
            .expect("Greedy selected search should identify a root travel destination"),
        direct_destination
    );
    assert_eq!(
        cautious
            .selected_search
            .selected_root_travel_destination
            .expect("Cautious selected search should identify a root travel destination"),
        alternate_first_hop
    );

    RunObservation {
        world_hash_after_divergence: h
            .snapshot_state()
            .hash()
            .expect("post-divergence state should hash deterministically"),
        greedy,
        cautious,
    }
}

#[derive(Clone, Copy, Debug)]
struct ArchetypeAgents {
    greedy: EntityId,
    cautious: EntityId,
}

fn archetype_agents(h: &GoldenHarness) -> ArchetypeAgents {
    let mut greedy = None;
    let mut cautious = None;
    for agent in h.world.query_agent_data().map(|(agent, _)| agent) {
        match h
            .world
            .get_component_cognitive_archetype_component(agent)
            .expect("scenario agents should have archetype components")
            .archetype
        {
            CognitiveArchetype::Greedy => greedy = Some(agent),
            CognitiveArchetype::Cautious => cautious = Some(agent),
            other => panic!("unexpected archetype in S167 scenario: {other:?}"),
        }
    }
    ArchetypeAgents {
        greedy: greedy.expect("scenario should contain a Greedy agent"),
        cautious: cautious.expect("scenario should contain a Cautious agent"),
    }
}

fn scenario_place_id(def: &ScenarioDef, place_name: &str) -> EntityId {
    let slot = def
        .places
        .iter()
        .position(|place| place.name == place_name)
        .and_then(|index| u32::try_from(index).ok())
        .unwrap_or_else(|| panic!("scenario should contain place '{place_name}'"));
    EntityId {
        slot,
        generation: 0,
    }
}

fn resource_entities(h: &GoldenHarness) -> Vec<EntityId> {
    let entities = h
        .world
        .entities()
        .filter(|entity| h.world.get_component_resource_source(*entity).is_some())
        .collect::<Vec<_>>();
    assert!(
        !entities.is_empty(),
        "scenario should author at least one resource source"
    );
    entities
}

fn seed_mixed_direct_route_memory(h: &mut GoldenHarness, agent: EntityId, segment: RouteSegment) {
    let mut route_preference = RoutePreference::default();
    for _ in 0..3 {
        route_preference.record_safe(segment, Tick(0));
    }
    route_preference.record_dangerous(segment, EventId(0), Tick(0));

    let mut runtime = h
        .driver
        .runtime(agent)
        .cloned()
        .unwrap_or_else(AgentDecisionRuntime::default);
    runtime.route_preference = route_preference;
    h.driver.set_runtime(agent, runtime);
}

fn set_agent_hunger(h: &mut GoldenHarness, agent: EntityId, hunger: Permille) {
    let mut needs = *h
        .world
        .get_component_homeostatic_needs(agent)
        .expect("scenario agents should have needs");
    needs.hunger = hunger;
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_homeostatic_needs(agent, needs)
        .expect("golden harness should keep needs writable");
    commit_txn(txn, &mut h.event_log);
}

fn known_entities_hash(h: &GoldenHarness, agent: EntityId) -> StateHash {
    let known_entities = h
        .world
        .get_component_agent_belief_store(agent)
        .expect("agent should have a belief store")
        .known_entities
        .clone();
    worldwake_core::hash_serializable(&known_entities)
        .expect("seeded known-entity beliefs should hash canonically")
}

fn observe_agent(
    h: &GoldenHarness,
    agent: EntityId,
    direct_segment: RouteSegment,
) -> AgentObservation {
    let trace = h
        .driver
        .trace_sink()
        .and_then(|sink| sink.trace_at(agent, DIVERGENCE_TICK))
        .unwrap_or_else(|| panic!("agent {agent:?} should have a trace at {DIVERGENCE_TICK:?}"));
    let DecisionOutcome::Planning(planning) = &trace.outcome else {
        panic!("agent {agent:?} should be planning at the divergence tick");
    };
    let selected_plan = planning
        .selection
        .selected_plan
        .as_ref()
        .unwrap_or_else(|| {
            panic!(
                "divergence tick should select a plan; trace summary: {}",
                trace.outcome.summary()
            )
        });
    let first_travel_target = selected_plan
        .steps
        .iter()
        .find(|step| step.op_kind == PlannerOpKind::Travel)
        .and_then(|step| step.targets.first().copied())
        .expect("selected plan should start with a travel branch");
    let selected_search = selected_plan
        .search_provenance
        .clone()
        .expect("selected fresh search should carry route provenance");
    let direct_successor = root_successor_for(&selected_search, direct_segment.to);
    let route_preference_profile = h
        .world
        .get_component_route_preference_profile(agent)
        .cloned()
        .expect("agent should have a route preference profile");
    let direct_route_preference = h
        .driver
        .runtime(agent)
        .and_then(|runtime| runtime.route_preference.get(&direct_segment))
        .map(|entry| entry.preference(&route_preference_profile, DIVERGENCE_TICK))
        .expect("agent should have seeded mixed route memory for the direct segment");

    AgentObservation {
        agent,
        selected_goal: planning
            .selection
            .selected_goal()
            .expect("selected opportunity should expose a goal"),
        first_travel_target,
        direct_successor,
        selected_search,
        route_preference_profile,
        direct_route_preference,
        known_entities_hash: known_entities_hash(h, agent),
    }
}

fn root_successor_for(
    selected_search: &SelectedPlanSearchProvenance,
    destination: EntityId,
) -> TravelSuccessorTrace {
    let pruning = selected_search
        .root_travel_pruning
        .as_ref()
        .expect("selected route search should carry root travel pruning");
    pruning
        .retained
        .iter()
        .chain(pruning.pruned.iter())
        .find(|successor| successor.destination == destination)
        .cloned()
        .unwrap_or_else(|| {
            panic!("root travel pruning should include destination {destination:?}: {pruning:?}")
        })
}

fn agent_name_map(def: &ScenarioDef) -> BTreeMap<&str, &AgentDef> {
    def.agents
        .iter()
        .map(|agent| (agent.name.as_str(), agent))
        .collect()
}

// Scenario 454: S167 Archetype Route Preference Drives Plan Divergence
// Systems: Scenario, AI, Travel
// GoalKinds: AcquireCommodity
// ActionDomains: Travel, Production
// Principles: P14B, P20, P22, P31
// Setup: two same-role agents load the canonical cognitive-archetypes
//        scenario with identical seeded resource beliefs and identical mixed
//        route memory on the direct road.
// Proves: Greedy and Cautious select the same apple-acquisition goal but
//         different first travel targets because their resolved
//         RoutePreferenceProfile.dangerous_traversal_penalty values price the
//         same direct route differently through selected-plan search
//         provenance.
// Cross-system chain: AgentDef.archetype -> RoutePreferenceProfile ->
//                     route-aware search cost -> SelectedPlanTrace.
#[test]
#[ignore = "CI-only: archetype divergence golden; run via golden-cognitive-archetypes workflow"]
fn forward() {
    let def = load_s167_def();
    let agents = agent_name_map(&def);
    assert_eq!(
        agents[FORWARD_GREEDY_AGENT].archetype,
        Some(CognitiveArchetype::Greedy)
    );
    assert_eq!(
        agents[FORWARD_CAUTIOUS_AGENT].archetype,
        Some(CognitiveArchetype::Cautious)
    );

    let first = observe_run(&def);
    let second = observe_run(&def);
    assert_eq!(
        first, second,
        "same seed, scenario, seeded beliefs, and route memory should replay deterministically"
    );
}

// Scenario 455: S167 Archetype Swap Reverses The Route Decision
// Systems: Scenario, AI, Travel
// GoalKinds: AcquireCommodity
// ActionDomains: Travel, Production
// Principles: P14B, P20, P22, P31
// Setup: the same canonical scenario is loaded with only the two explicit
//        AgentDef.archetype assignments swapped in test setup.
// Proves: the direct-vs-neutral first travel target follows the Greedy versus
//         Cautious archetype rather than a fixed authored agent name or
//         scenario rail.
// Cross-system chain: swapped AgentDef.archetype -> swapped
//                     RoutePreferenceProfile -> reversed selected route.
#[test]
#[ignore = "CI-only: archetype divergence golden; run via golden-cognitive-archetypes workflow"]
fn counterfactual_symmetry() {
    let forward = observe_run(&load_s167_def());
    let swapped = observe_run(&swapped_archetype_def());

    assert_eq!(
        swapped.greedy.first_travel_target, forward.greedy.first_travel_target,
        "the Greedy decision should follow the archetype after swapping authored agents"
    );
    assert_eq!(
        swapped.cautious.first_travel_target, forward.cautious.first_travel_target,
        "the Cautious decision should follow the archetype after swapping authored agents"
    );
    assert_eq!(swapped.greedy.selected_goal, forward.greedy.selected_goal);
    assert_eq!(
        swapped.cautious.selected_goal,
        forward.cautious.selected_goal
    );
    assert_eq!(
        swapped.greedy.direct_route_preference,
        forward.greedy.direct_route_preference
    );
    assert_eq!(
        swapped.cautious.direct_route_preference,
        forward.cautious.direct_route_preference
    );
}
