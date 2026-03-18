//! Golden tests for political office claims and succession resolution.

mod golden_harness;

use golden_harness::*;
use worldwake_core::{
    hash_event_log, hash_world, BeliefConfidencePolicy, EventTag, HomeostaticNeeds,
    MetabolismProfile, PerceptionProfile, PerceptionSource, Seed, StateHash, SuccessionLaw, Tick,
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
