//! Golden coverage for S152 cognitive archetype spawn contracts.

use std::collections::BTreeMap;

use crate::golden_harness::GoldenHarness;
use worldwake_cli::scenario::{
    lints::LintRule,
    spawn_scenario,
    types::{AgentDef, PlaceDef, ScenarioDef},
};
use worldwake_core::{
    ArchetypeAssignmentPolicy, ArchetypeAssignmentSource, CognitiveArchetype,
    CognitiveArchetypeComponent, ControlSource, EntityId, EventTag, EventView,
    PortfolioWeightsProfile, SlotKind, topology::PlaceTag,
};

fn minimal_agent(name: &str, archetype: Option<CognitiveArchetype>) -> AgentDef {
    AgentDef {
        name: name.into(),
        location: "Village".into(),
        control: ControlSource::Ai,
        needs: None,
        combat_profile: None,
        utility_profile: None,
        artifact_posting_profile: None,
        merchandise_profile: None,
        trade_disposition: None,
        perception_profile: None,
        tell_profile: None,
        cognitive_profile: None,
        portfolio_weights_profile: None,
        agent_schema_context_profile: None,
        risk_weight_profile: None,
        law_abiding_profile: None,
        agenda_profile: None,
        execution_budget: None,
        epistemic_disposition: None,
        intention_disposition: None,
        communication_profile: None,
        preference_profile: None,
        expectation_store: None,
        last_seen_memory: None,
        social_observations: None,
        obligation_satiation_profile: None,
        drive_thresholds: None,
        drive_escalation_profile: None,
        metabolism_profile: None,
        water_tolerance_profile: None,
        disposal_profile: None,
        exploration_profile: None,
        diversification_profile: None,
        carry_capacity: None,
        theft_disposition: None,
        justice_disposition: None,
        violation_disposition: None,
        patrol_profile: None,
        patrol_route: None,
        pursuit_profile: None,
        contention_disposition: None,
        commodity_valuation: None,
        substitute_preferences: None,
        testimony_trust_profile: None,
        route_preference_profile: None,
        archetype,
        known_recipes: None,
    }
}

fn scenario_with_agents(agents: Vec<AgentDef>) -> ScenarioDef {
    ScenarioDef {
        seed: 152,
        places: vec![PlaceDef {
            name: "Village".into(),
            tags: vec![PlaceTag::Village],
            visibility_profile: None,
            rest_capacity: None,
            sleep_quality: None,
            place_dirtiness: None,
            latrine_fullness: None,
            contention_policy: None,
        }],
        edges: vec![],
        agents,
        bandit_camps: Vec::new(),
        offices: Vec::new(),
        artifacts: Vec::new(),
        items: Vec::new(),
        facilities: Vec::new(),
        resource_sources: Vec::new(),
        hostilities: Vec::new(),
        commodity_decay: None,
        survival_health_contract: None,
        compaction_interval: 0,
        scenario_lint_overrides: BTreeMap::new(),
        harvest_trace_retention_ticks: None,
        archetype_assignment_policy: None,
    }
}

fn two_agent_scenario(
    left_name: &str,
    left: CognitiveArchetype,
    right_name: &str,
    right: CognitiveArchetype,
) -> ScenarioDef {
    scenario_with_agents(vec![
        minimal_agent(left_name, Some(left)),
        minimal_agent(right_name, Some(right)),
    ])
}

fn agent_by_name(world: &worldwake_core::World, name: &str) -> EntityId {
    world
        .entities_with_name_and_agent_data()
        .find(|entity| {
            world
                .get_component_name(*entity)
                .is_some_and(|component| component.0 == name)
        })
        .expect("scenario should contain named agent")
}

fn archetype(world: &worldwake_core::World, agent: EntityId) -> CognitiveArchetype {
    world
        .get_component_cognitive_archetype_component(agent)
        .expect("spawned agent should have archetype component")
        .archetype
}

// Scenario 447: S152 Seeded Assignment Is Deterministic
// Systems: Scenario, AI
// GoalKinds: None
// ActionDomains: Scenario
// Principles: P2, P22, P31
// Setup: the same minimal two-agent scenario is spawned twice with the same
//        seed and no explicit archetype overrides.
// Proves: archetype assignment and the resolved profile-bearing world state
//         are identical for identical scenario input.
// Cross-system chain: ScenarioDef.seed -> archetype assignment -> stored
//                     profile/component state.
#[test]
fn cognitive_archetypes_seeded_assignment_is_deterministic() {
    let def = scenario_with_agents(vec![
        minimal_agent("Aster", None),
        minimal_agent("Borin", None),
    ]);

    let first = spawn_scenario(&def).expect("scenario should spawn");
    let second = spawn_scenario(&def).expect("scenario should spawn");

    assert_eq!(
        first.state.hash().expect("first state should hash"),
        second.state.hash().expect("second state should hash")
    );
    for name in ["Aster", "Borin"] {
        let first_agent = agent_by_name(first.state.world(), name);
        let second_agent = agent_by_name(second.state.world(), name);
        assert_eq!(
            first
                .state
                .world()
                .get_component_cognitive_archetype_component(first_agent),
            second
                .state
                .world()
                .get_component_cognitive_archetype_component(second_agent)
        );
        assert_eq!(
            first
                .state
                .world()
                .get_component_cognitive_profile(first_agent),
            second
                .state
                .world()
                .get_component_cognitive_profile(second_agent)
        );
    }
}

// Scenario 448: S152 Cautious Backoff Exceeds Bold Backoff
// Systems: Scenario, AI
// GoalKinds: None
// ActionDomains: Scenario
// Principles: P3, P22, P31
// Setup: two otherwise identical agents differ only by explicit archetype:
//        Cautious versus Bold.
// Proves: the resolved CognitiveProfile backoff fields consumed by
//         failure_handling.rs are longer for Cautious than Bold.
// Cross-system chain: archetype template -> CognitiveProfile backoff fields ->
//                     failure-handling TTL input.
#[test]
fn cognitive_archetypes_cautious_resolves_longer_backoff_than_bold() {
    let spawned = spawn_scenario(&two_agent_scenario(
        "Cautious",
        CognitiveArchetype::Cautious,
        "Bold",
        CognitiveArchetype::Bold,
    ))
    .expect("scenario should spawn");
    let world = spawned.state.world();
    let cautious = world
        .get_component_cognitive_profile(agent_by_name(world, "Cautious"))
        .expect("cautious agent should have cognitive profile");
    let bold = world
        .get_component_cognitive_profile(agent_by_name(world, "Bold"))
        .expect("bold agent should have cognitive profile");

    assert!(cautious.stale_belief_backoff_ticks > bold.stale_belief_backoff_ticks);
    assert!(cautious.improper_state_backoff_ticks > bold.improper_state_backoff_ticks);
    assert!(cautious.transient_block_ticks > bold.transient_block_ticks);
    assert!(cautious.search_exhaustion_backoff_ticks > bold.search_exhaustion_backoff_ticks);
}

// Scenario 449: S152 Sociable Reasks Sooner Than Skeptical
// Systems: Scenario, AI
// GoalKinds: AskWitness
// ActionDomains: Social
// Principles: P3, P15, P22
// Setup: two otherwise identical agents differ only by explicit archetype:
//        Sociable versus Skeptical.
// Proves: the resolved EpistemicDispositionProfile ask-memory retention field
//         consumed by the AskWitness emitter is lower for Sociable.
// Cross-system chain: archetype template -> ask_memory_retention_ticks ->
//                     AskWitness re-ask cadence input.
#[test]
fn cognitive_archetypes_sociable_resolves_shorter_ask_retention_than_skeptical() {
    let spawned = spawn_scenario(&two_agent_scenario(
        "Sociable",
        CognitiveArchetype::Sociable,
        "Skeptical",
        CognitiveArchetype::Skeptical,
    ))
    .expect("scenario should spawn");
    let world = spawned.state.world();
    let sociable = world
        .get_component_epistemic_disposition_profile(agent_by_name(world, "Sociable"))
        .expect("sociable agent should have epistemic profile");
    let skeptical = world
        .get_component_epistemic_disposition_profile(agent_by_name(world, "Skeptical"))
        .expect("skeptical agent should have epistemic profile");

    assert!(sociable.ask_memory_retention_ticks < skeptical.ask_memory_retention_ticks);
}

// Scenario 450: S152 Greedy Boosts Economic Portfolio Weight
// Systems: Scenario, AI
// GoalKinds: ProduceCommodity, Trade
// ActionDomains: Portfolio
// Principles: P3, P20, P22
// Setup: two otherwise identical agents differ only by explicit archetype:
//        Greedy versus Cautious.
// Proves: the resolved PortfolioWeightsProfile ranks EconomicOpportunity
//         higher for Greedy than Cautious, without adding a new slot.
// Cross-system chain: archetype template -> PortfolioWeightsProfile ->
//                     portfolio slot scoring input.
#[test]
fn cognitive_archetypes_greedy_resolves_higher_economic_weight_than_cautious() {
    let spawned = spawn_scenario(&two_agent_scenario(
        "Greedy",
        CognitiveArchetype::Greedy,
        "Cautious",
        CognitiveArchetype::Cautious,
    ))
    .expect("scenario should spawn");
    let world = spawned.state.world();
    let greedy = world
        .get_component_portfolio_weights_profile(agent_by_name(world, "Greedy"))
        .expect("greedy agent should have portfolio profile");
    let cautious = world
        .get_component_portfolio_weights_profile(agent_by_name(world, "Cautious"))
        .expect("cautious agent should have portfolio profile");

    assert!(
        greedy.weight_for(SlotKind::EconomicOpportunity)
            > cautious.weight_for(SlotKind::EconomicOpportunity)
    );
    assert!(
        greedy.weight_for(SlotKind::EconomicOpportunity)
            > PortfolioWeightsProfile::default().weight_for(SlotKind::EconomicOpportunity)
    );
    assert_eq!(
        greedy.weight_for(SlotKind::EconomicOpportunity),
        greedy.economic_opportunity
    );
}

// Scenario 451: S152 PersonalityAssigned Event Per Agent
// Systems: Scenario, EventLog
// GoalKinds: None
// ActionDomains: Scenario
// Principles: P4, P22A, P29
// Setup: three agents spawn with explicit archetypes and no other differing
//        authored profile state.
// Proves: the append-only event log records exactly one PersonalityAssigned
//         payload per spawned agent.
// Cross-system chain: spawn assignment -> EventTag::PersonalityAssigned ->
//                     inspectable payload.
#[test]
fn cognitive_archetypes_emit_one_personality_assigned_event_per_agent() {
    let def = scenario_with_agents(vec![
        minimal_agent("Cautious", Some(CognitiveArchetype::Cautious)),
        minimal_agent("Bold", Some(CognitiveArchetype::Bold)),
        minimal_agent("Greedy", Some(CognitiveArchetype::Greedy)),
    ]);
    let mut def = def;
    def.scenario_lint_overrides.insert(
        LintRule::ProfileHomogeneity,
        "fixture isolates spawn-time archetype events; explicit archetype overrides diversify resolved runtime profiles after lint-time schema validation".into(),
    );
    let spawned = spawn_scenario(&def).expect("scenario should spawn");
    let world = spawned.state.world();
    let event_ids = spawned
        .state
        .event_log()
        .events_by_tag(EventTag::PersonalityAssigned);

    assert_eq!(event_ids.len(), def.agents.len());
    for event_id in event_ids {
        let payload = spawned
            .state
            .event_log()
            .get(*event_id)
            .and_then(EventView::personality_assigned_payload)
            .expect("personality assignment event should carry payload");
        assert_eq!(archetype(world, payload.agent), payload.archetype);
    }
}

// Scenario 452: S152 Explicit Override Pins Archetype
// Systems: Scenario, AI
// GoalKinds: None
// ActionDomains: Scenario
// Principles: P2, P22, P31
// Setup: the scenario policy can only draw Cautious, while the agent pins
//        Greedy explicitly.
// Proves: AgentDef.archetype takes precedence over policy assignment and the
//         event payload records Explicit as the source.
// Cross-system chain: AgentDef.archetype -> CognitiveArchetypeComponent ->
//                     PersonalityAssigned source.
#[test]
fn cognitive_archetypes_explicit_override_pins_archetype_over_policy() {
    let mut weights = BTreeMap::new();
    weights.insert(CognitiveArchetype::Cautious, 100);
    let mut def = scenario_with_agents(vec![minimal_agent(
        "Pinned",
        Some(CognitiveArchetype::Greedy),
    )]);
    def.archetype_assignment_policy = Some(ArchetypeAssignmentPolicy::Weighted(weights));

    let spawned = spawn_scenario(&def).expect("scenario should spawn");
    let world = spawned.state.world();
    let agent = agent_by_name(world, "Pinned");
    let event_id = spawned
        .state
        .event_log()
        .events_by_tag(EventTag::PersonalityAssigned)[0];
    let payload = spawned
        .state
        .event_log()
        .get(event_id)
        .and_then(EventView::personality_assigned_payload)
        .expect("personality assignment event should carry payload");

    assert_eq!(archetype(world, agent), CognitiveArchetype::Greedy);
    assert_eq!(payload.archetype, CognitiveArchetype::Greedy);
    assert_eq!(payload.source, ArchetypeAssignmentSource::Explicit);
}

// Scenario 453: S152 Save Load Preserves Archetype State
// Systems: Scenario, SaveLoad
// GoalKinds: None
// ActionDomains: Scenario
// Principles: P4, P12, P22A
// Setup: an explicit Fearful agent is spawned and immediately save/load
//        round-tripped through the golden harness.
// Proves: the stored CognitiveArchetypeComponent and resolved profile values
//         survive the save boundary.
// Cross-system chain: spawn assignment -> stored components -> save/load
//                     round-trip.
#[test]
fn cognitive_archetypes_save_load_preserves_component_and_resolved_profiles() {
    let def = scenario_with_agents(vec![minimal_agent(
        "Fearful",
        Some(CognitiveArchetype::Fearful),
    )]);
    let spawned = spawn_scenario(&def).expect("scenario should spawn");
    let harness = GoldenHarness::from_simulation_state(&spawned.state);
    let roundtripped = harness.save_load_roundtrip();
    let agent = agent_by_name(&harness.world, "Fearful");
    let roundtripped_agent = agent_by_name(&roundtripped.world, "Fearful");

    assert_eq!(
        roundtripped
            .world
            .get_component_cognitive_archetype_component(roundtripped_agent),
        Some(&CognitiveArchetypeComponent {
            archetype: CognitiveArchetype::Fearful,
        })
    );
    assert_eq!(
        harness.world.get_component_cognitive_profile(agent),
        roundtripped
            .world
            .get_component_cognitive_profile(roundtripped_agent)
    );
    assert_eq!(
        harness.world.get_component_portfolio_weights_profile(agent),
        roundtripped
            .world
            .get_component_portfolio_weights_profile(roundtripped_agent)
    );
    assert_eq!(
        harness
            .world
            .get_component_epistemic_disposition_profile(agent),
        roundtripped
            .world
            .get_component_epistemic_disposition_profile(roundtripped_agent)
    );
}
