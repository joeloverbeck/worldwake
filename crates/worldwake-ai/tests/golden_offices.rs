//! Golden tests for political office claims and succession resolution.

mod golden_harness;

use golden_harness::*;
use worldwake_core::{
    hash_event_log, hash_world, BeliefConfidencePolicy, CombatProfile, CommodityKind, EventTag,
    HomeostaticNeeds, MetabolismProfile, Permille, PerceptionProfile, PerceptionSource, Quantity,
    Seed, StateHash, SuccessionLaw, Tick, UtilityProfile,
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

// ---------------------------------------------------------------------------
// Scenario 13: Bribe -> Support Coalition (Full-Quantity Transfer)
// ---------------------------------------------------------------------------
//
// Setup: Vacant office (Support law, period=5). Agent A eligible with high
// enterprise_weight, holds 5 bread. Agent B at jurisdiction, no initial
// loyalty to A. Agent C (competitor) at jurisdiction has already self-declared
// support for own office claim.
//
// The competitor ensures that DeclareSupport alone from A would produce a tie
// (ProgressBarrier), motivating the planner to select Bribe to build a
// winning coalition (GoalSatisfied).
//
// Expected: A generates ClaimOffice. Planner finds Bribe(B, bread) +
// DeclareSupport(self) because DeclareSupport alone ties with competitor C.
// A bribes B (all 5 bread transfer). B's loyalty increases. B generates
// SupportCandidateForOffice(A) and declares support. A's coalition
// (self + B = 2) exceeds C's (self = 1). Politics system installs A.

#[test]
fn golden_bribe_support_coalition() {
    // The bribe scenario requires a wider beam than the default (8) because
    // the prototype world's adjacency graph creates many travel candidates
    // at equal cost that can push Bribe nodes past the beam cutoff.
    let mut h = GoldenHarness::new(Seed([114; 32]));
    h.driver = worldwake_ai::AgentTickDriver::new(worldwake_ai::PlanningBudget {
        beam_width: 16,
        ..worldwake_ai::PlanningBudget::default()
    });

    // Agent A — claimant with high enterprise weight, holds 5 bread.
    let agent_a = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Briber Alpha",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(900)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent_a,
        default_perception_profile(),
    );
    let _bread_lot = give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent_a,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(5),
    );

    // Agent B — bribe target. social_weight > 0 so SupportCandidateForOffice
    // is viable after loyalty increases from the bribe. enterprise_weight=0
    // so B won't try to ClaimOffice itself.
    let agent_b = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Bribe Target",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        social_supporter_utility(pm(600)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent_b,
        default_perception_profile(),
    );

    // Agent C — competitor at a DIFFERENT place. High enterprise weight,
    // already self-declared support. Placed at ORCHARD_FARM so the planner
    // cannot target C with Bribe (not co-located), forcing the planner to
    // select B as the bribe target. C's pre-declared support still counts
    // for succession (declarations are relation-based, not positional).
    let agent_c = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Competitor",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent_c,
        default_perception_profile(),
    );

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

    // Pre-declare C's self-support — this creates the tie scenario.
    declare_support(&mut h.world, &mut h.event_log, agent_c, office, agent_c);

    // All agents need beliefs about the office and each other for political
    // goal generation and bribe targeting.
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
    // A needs to know about B (bribe target) and C (competitor).
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent_a,
        &[agent_b, agent_c],
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    // B needs to know about A (to generate SupportCandidateForOffice(A)
    // after loyalty increases from the bribe).
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent_b,
        &[agent_a],
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    // Record initial total bread for conservation check.
    let initial_bread_a = h.agent_commodity_qty(agent_a, CommodityKind::Bread);
    let initial_bread_b = h.agent_commodity_qty(agent_b, CommodityKind::Bread);
    let initial_total_bread = initial_bread_a.0 + initial_bread_b.0;
    assert_eq!(initial_bread_a, Quantity(5), "A starts with 5 bread");
    assert_eq!(initial_bread_b, Quantity(0), "B starts with 0 bread");

    // Run simulation — enough ticks for bribe, support declaration, and succession.
    for _ in 0..40 {
        h.step_once();
    }

    // Assertion 1: A is installed as office holder.
    assert_eq!(
        h.world.office_holder(office),
        Some(agent_a),
        "Agent A should be installed as office holder after bribe coalition"
    );

    // Assertion 2: Full commodity transfer — A's bread is 0 after bribe.
    let final_bread_a = h.agent_commodity_qty(agent_a, CommodityKind::Bread);
    assert_eq!(
        final_bread_a,
        Quantity(0),
        "Agent A should have 0 bread after full-stock bribe transfer"
    );

    // Assertion 3: B received all of A's former bread.
    let final_bread_b = h.agent_commodity_qty(agent_b, CommodityKind::Bread);
    assert_eq!(
        final_bread_b,
        Quantity(5),
        "Agent B should have received all 5 bread from the bribe"
    );

    // Assertion 4: Conservation — total bread unchanged.
    let final_total_bread = final_bread_a.0 + final_bread_b.0;
    assert_eq!(
        initial_total_bread, final_total_bread,
        "Bread conservation violated: initial={initial_total_bread}, final={final_total_bread}"
    );

    // Assertion 5: Event log contains Political events.
    let political_events = h.event_log.events_by_tag(EventTag::Political);
    assert!(
        !political_events.is_empty(),
        "Event log should contain Political events from bribe, support, and installation"
    );
}

// ---------------------------------------------------------------------------
// Scenario 14: Threaten with Courage Diversity (Principle 20)
// ---------------------------------------------------------------------------
//
// Setup: Vacant office (Support law, period=5). Agent A eligible with high
// enterprise_weight and attack_skill=pm(800). Agent B at jurisdiction with
// courage=pm(200) (should yield — 800 > 200). Agent C at jurisdiction with
// courage=pm(900) (should resist — 800 < 900). Agent D (competitor) at
// jurisdiction, has already self-declared support for own office claim.
//
// The competitor ensures DeclareSupport alone from A would produce a tie,
// motivating the planner to select Threaten to build a winning coalition.
//
// Expected: A generates ClaimOffice. Planner finds Threaten(B) viable
// (800 > 200) but not Threaten(C) (800 < 900). A threatens B -> B yields ->
// loyalty increase. B generates SupportCandidateForOffice(A). A declares
// for self. A's coalition (self + B = 2) exceeds D's (self = 1).
// C has hostility toward A or is unaffected.

fn combat_profile_with_attack_skill(attack_skill: Permille) -> CombatProfile {
    CombatProfile::new(
        pm(1000), // wound_capacity
        pm(700),  // incapacitation_threshold
        attack_skill,
        pm(500),  // guard_skill
        pm(80),   // defend_bonus
        pm(25),   // natural_clot_resistance
        pm(18),   // natural_recovery_rate
        pm(120),  // unarmed_wound_severity
        pm(35),   // unarmed_bleed_rate
        nz(6),    // unarmed_attack_ticks
    )
}

#[test]
#[ignore = "blocked on E16DPOLPLAN-028: courage not yet in belief pipeline"]
fn golden_threaten_with_courage_diversity() {
    // Wider beam — same rationale as bribe scenario: many equal-cost travel
    // candidates can push Threaten nodes past the default beam cutoff.
    let mut h = GoldenHarness::new(Seed([115; 32]));
    h.driver = worldwake_ai::AgentTickDriver::new(worldwake_ai::PlanningBudget {
        beam_width: 16,
        ..worldwake_ai::PlanningBudget::default()
    });

    // Agent A — claimant with high enterprise weight and high attack_skill.
    let agent_a = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Threatener Alpha",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(900)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent_a,
        default_perception_profile(),
    );
    // Override combat profile to set attack_skill=pm(800) (threat pressure).
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_combat_profile(agent_a, combat_profile_with_attack_skill(pm(800)))
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Agent B — low courage (pm(200)), should yield to threat (800 > 200).
    // social_weight > 0 so SupportCandidateForOffice is viable after loyalty
    // increases. enterprise_weight=0 so B won't try to ClaimOffice itself.
    let agent_b = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Timid Target",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        social_supporter_utility(pm(600)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent_b,
        default_perception_profile(),
    );
    set_courage(&mut h.world, &mut h.event_log, agent_b, pm(200));

    // Agent C — high courage (pm(900)), should resist threat (800 < 900).
    // social_weight > 0, enterprise_weight=0. C exists to prove agent
    // diversity: same Threaten action, different courage → different outcome.
    let agent_c = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Brave Resister",
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
    set_courage(&mut h.world, &mut h.event_log, agent_c, pm(900));

    // Agent D — competitor at jurisdiction. High enterprise weight, already
    // self-declared support. Creates the contested scenario where Threaten
    // is rational for building a winning coalition.
    let agent_d = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Competitor",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent_d,
        default_perception_profile(),
    );

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

    // Pre-declare D's self-support — creates the tie scenario.
    declare_support(&mut h.world, &mut h.event_log, agent_d, office, agent_d);

    // Seed beliefs: all agents need to know about the office.
    for agent in [agent_a, agent_b, agent_c, agent_d] {
        seed_actor_beliefs(
            &mut h.world,
            &mut h.event_log,
            agent,
            &[office],
            Tick(0),
            PerceptionSource::DirectObservation,
        );
    }
    // A needs to know about B, C (threaten targets) and D (competitor).
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent_a,
        &[agent_b, agent_c, agent_d],
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    // B needs to know about A (to generate SupportCandidateForOffice(A)
    // after loyalty increases from the threat yield).
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent_b,
        &[agent_a],
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    // Record initial loyalty state for delta assertions.
    let initial_b_loyalty_to_a = h.world.loyalty_to(agent_b, agent_a);
    assert_eq!(
        initial_b_loyalty_to_a, None,
        "B should have no initial loyalty to A"
    );

    // Run simulation — enough ticks for threat, support declaration, and succession.
    for _ in 0..40 {
        h.step_once();
    }

    // Assertion 1: B has increased loyalty to A (yield outcome from threat).
    let final_b_loyalty = h.world.loyalty_to(agent_b, agent_a);
    assert!(
        final_b_loyalty.is_some() && final_b_loyalty.unwrap() > pm(0),
        "Agent B (low courage) should have gained loyalty to A after yielding to threat, \
         got {:?}",
        final_b_loyalty
    );

    // Assertion 2: C has hostility toward A (resist outcome) or is unaffected.
    // The planner should not even select Threaten(C) since 800 < 900,
    // so C may have no interaction at all. But if A does threaten C,
    // the resist outcome produces hostility.
    let c_hostile_to_a = h.world.hostile_targets_of(agent_c).contains(&agent_a);
    let c_loyalty_to_a = h.world.loyalty_to(agent_c, agent_a);
    // C must NOT have gained loyalty (would mean the threat yielded, violating
    // the courage check).
    assert!(
        c_loyalty_to_a.is_none() || c_loyalty_to_a == Some(pm(0)),
        "Agent C (high courage) must not gain loyalty to A from threat, got {:?}",
        c_loyalty_to_a
    );
    // If threatened, C should be hostile. If not threatened, that's fine too
    // (planner correctly filtered it out).
    if c_hostile_to_a {
        // Resist outcome confirmed — C was threatened and resisted.
    }
    // Either way, the diversity assertion holds: B yielded, C did not.

    // Assertion 3: A is installed as office holder.
    assert_eq!(
        h.world.office_holder(office),
        Some(agent_a),
        "Agent A should be installed as office holder after threat coalition \
         (A self-support + B threat-yield support = 2 > D's 1)"
    );

    // Assertion 4: Agent diversity (Principle 20) — same action type,
    // different courage values produced divergent outcomes.
    // B gained loyalty (yield), C did not (resist or not threatened).
    assert_ne!(
        final_b_loyalty, c_loyalty_to_a,
        "Principle 20: same Threaten action must produce divergent outcomes \
         for agents with different courage values"
    );

    // Assertion 5: Event log contains Political events.
    let political_events = h.event_log.events_by_tag(EventTag::Political);
    assert!(
        !political_events.is_empty(),
        "Event log should contain Political events from threat, support, and installation"
    );
}
