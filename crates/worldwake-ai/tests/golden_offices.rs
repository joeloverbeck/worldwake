//! Golden tests for political office claims and succession resolution.

mod golden_harness;

use golden_harness::*;
use worldwake_core::{
    hash_event_log, hash_world, BeliefConfidencePolicy, EventTag, HomeostaticNeeds,
    MetabolismProfile, Permille, PerceptionProfile, PerceptionSource, Seed, StateHash,
    SuccessionLaw, Tick, UtilityProfile,
};

// ---------------------------------------------------------------------------
// Scenario 11: Simple Office Claim via DeclareSupport
// ---------------------------------------------------------------------------
//
// Setup: Single sated agent at VillageSquare with high enterprise_weight.
// Vacant office (Support law, period=5, no eligibility rules) at VillageSquare.
// Agent generates ClaimOffice -> plans DeclareSupport(self) -> executes ->
// after succession period, succession_system installs agent as holder.

fn build_simple_office_claim_scenario(seed: Seed) -> (GoldenHarness, worldwake_core::EntityId, worldwake_core::EntityId) {
    let mut h = GoldenHarness::new(seed);

    // Sated agent with high enterprise weight — political goals dominate.
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Claimant",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );

    // Perception profile so the agent can observe post-action results.
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        PerceptionProfile {
            memory_capacity: 32,
            memory_retention_ticks: 240,
            observation_fidelity: pm(875),
            confidence_policy: BeliefConfidencePolicy::default(),
        },
    );

    // Vacant office at VillageSquare — Support law, 5-tick succession period,
    // no eligibility rules (any agent can claim).
    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "Village Elder",
        VILLAGE_SQUARE,
        SuccessionLaw::Support,
        5,
        vec![],
    );

    // Seed the agent's beliefs about the office so candidate generation sees it.
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        &[office],
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    (h, agent, office)
}

fn run_simple_office_claim(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, agent, office) = build_simple_office_claim_scenario(seed);

    for _ in 0..20 {
        h.step_once();
    }

    // Assertion 1: Agent is now the office holder.
    assert_eq!(
        h.world.office_holder(office),
        Some(agent),
        "Agent should be installed as office holder after succession resolution"
    );

    // Assertion 2: Event log contains Political events (from DeclareSupport
    // and/or succession installation).
    let political_events = h.event_log.events_by_tag(EventTag::Political);
    assert!(
        !political_events.is_empty(),
        "Event log should contain Political events from support declaration and installation"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_simple_office_claim_via_declare_support() {
    let _ = run_simple_office_claim(Seed([111; 32]));
}

// ---------------------------------------------------------------------------
// Scenario 11b: Deterministic Replay
// ---------------------------------------------------------------------------

#[test]
fn golden_simple_office_claim_deterministic_replay() {
    let seed = Seed([112; 32]);

    let (world_hash_1, log_hash_1) = run_simple_office_claim(seed);
    let (world_hash_2, log_hash_2) = run_simple_office_claim(seed);

    assert_eq!(
        world_hash_1, world_hash_2,
        "Two runs with the same seed must produce identical world hashes"
    );
    assert_eq!(
        log_hash_1, log_hash_2,
        "Two runs with the same seed must produce identical event log hashes"
    );

    // Verify non-trivial simulation occurred.
    let (fresh, _, _) = build_simple_office_claim_scenario(seed);
    let initial_world_hash = hash_world(&fresh.world).unwrap();
    assert_ne!(
        world_hash_1, initial_world_hash,
        "World should have changed from initial state (non-trivial simulation)"
    );
}

// ---------------------------------------------------------------------------
// Scenario 12: Competing Claims with Loyal Supporter
// ---------------------------------------------------------------------------
//
// Setup: Vacant office (Support law, period=5). Agents A and B both eligible
// with high enterprise_weight. Agent C has loyalty to A and social_weight > 0
// but enterprise_weight=0 (so ClaimOffice gets zero-motive filtered and C
// generates SupportCandidateForOffice(A) instead).
//
// Expected: A declares for self, B declares for self, C supports A.
// A gets 2 declarations (self + C), B gets 1. succession_system installs A.

fn social_supporter_utility(social: Permille) -> UtilityProfile {
    UtilityProfile {
        enterprise_weight: Permille::new_unchecked(0),
        social_weight: social,
        ..UtilityProfile::default()
    }
}

fn default_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        memory_capacity: 32,
        memory_retention_ticks: 240,
        observation_fidelity: pm(875),
        confidence_policy: BeliefConfidencePolicy::default(),
    }
}

#[test]
fn golden_competing_claims_with_loyal_supporter() {
    let mut h = GoldenHarness::new(Seed([113; 32]));

    // Agent A — claimant with high enterprise weight.
    let agent_a = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Claimant Alpha",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent_a,
        default_perception_profile(),
    );

    // Agent B — rival claimant with high enterprise weight.
    let agent_b = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Claimant Beta",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent_b,
        default_perception_profile(),
    );

    // Agent C — loyal supporter of A. enterprise_weight=0 so ClaimOffice gets
    // zero-motive filtered; social_weight=600 with loyalty to A drives
    // SupportCandidateForOffice(A).
    let agent_c = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Loyal Supporter",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        social_supporter_utility(pm(600)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent_c,
        default_perception_profile(),
    );

    // Loyalty from C to A — drives SupportCandidateForOffice candidate generation.
    set_loyalty(&mut h.world, &mut h.event_log, agent_c, agent_a, pm(650));

    // Vacant office at VillageSquare — Support law, 5-tick succession period.
    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "Village Elder",
        VILLAGE_SQUARE,
        SuccessionLaw::Support,
        5,
        vec![],
    );

    // All three agents need beliefs about the office for political goal generation.
    // C also needs beliefs about A (to iterate as support candidate).
    for agent in [agent_a, agent_b, agent_c] {
        seed_actor_beliefs(
            &mut h.world,
            &mut h.event_log,
            agent,
            &[office],
            Tick(0),
            PerceptionSource::DirectObservation,
        );
    }
    // C needs to know about A as a candidate to support.
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent_c,
        &[agent_a],
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    // Run simulation — enough ticks for all agents to act and succession to resolve.
    for _ in 0..30 {
        h.step_once();
    }

    // Assertion 1: A is installed as office holder.
    // Without C's loyalty-driven support, A and B would tie 1-1 and the
    // succession system resets the vacancy clock (no winner on tie).
    // C's SupportCandidateForOffice(A) gives A 2 declarations vs B's 1,
    // making A the unique winner.
    assert_eq!(
        h.world.office_holder(office),
        Some(agent_a),
        "Agent A should be installed as office holder (2 support declarations vs B's 1)"
    );

    // Assertion 2: Event log contains Political events from declarations
    // and installation. Succession system clears declarations after
    // installing the holder, so we verify via event log, not world query.
    let political_events = h.event_log.events_by_tag(EventTag::Political);
    assert!(
        political_events.len() >= 3,
        "Expected at least 3 Political events (A declares for self, B declares for self, \
         C declares for A, installation), got {}",
        political_events.len()
    );
}
