//! Golden tests for political office claims and succession resolution.

mod golden_harness;

use golden_harness::*;
use worldwake_core::{
    hash_event_log, hash_world, prototype_place_entity, BeliefConfidencePolicy, CombatProfile,
    CommodityKind, DriveThresholds, EventTag, FactionPurpose, GoalKind, HomeostaticNeeds,
    MetabolismProfile, Permille, PerceptionProfile, PerceptionSource, PrototypePlace, Quantity,
    Seed, StateHash, SuccessionLaw, Tick, UtilityProfile,
};
use worldwake_ai::DecisionOutcome;
use worldwake_sim::ActionTraceKind;

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
#[allow(clippy::too_many_lines)]
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
// ORCHARD_FARM (not co-located), has already self-declared support for own
// office claim.
//
// D is placed at a different location so the planner cannot target D with
// Threaten (not co-located), forcing the planner to consider B and C only.
// D's pre-declared support still counts (declarations are relation-based).
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
#[allow(clippy::too_many_lines)]
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

    // Agent D — competitor at a DIFFERENT place. High enterprise weight,
    // already self-declared support. Placed at ORCHARD_FARM so the planner
    // cannot target D with Threaten (not co-located), forcing the planner to
    // select B as the threat target. D's pre-declared support still counts
    // for succession (declarations are relation-based, not positional).
    let agent_d = seed_agent(
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
         got {final_b_loyalty:?}"
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
        "Agent C (high courage) must not gain loyalty to A from threat, got {c_loyalty_to_a:?}"
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

// ---------------------------------------------------------------------------
// Scenario 15: Travel to Distant Jurisdiction for Office Claim
// ---------------------------------------------------------------------------
//
// Setup: Vacant office at VillageSquare (Support law, period=5, no eligibility).
// Single sated agent starts at BanditCamp (3 hops away: BanditCamp → ForestPath
// → NorthCrossroads → VillageSquare, 12 travel ticks total).
// Agent has beliefs about the vacant office. enterprise_weight=pm(800).
//
// Expected: Agent generates ClaimOffice → plans multi-hop Travel + DeclareSupport
// → traverses the 3-hop route → arrives at VillageSquare → declares support →
// installed as holder after succession period.

#[test]
fn golden_travel_to_distant_jurisdiction_for_claim() {
    let bandit_camp = prototype_place_entity(PrototypePlace::BanditCamp);
    let mut h = GoldenHarness::new(Seed([116; 32]));

    // Sated agent at BanditCamp with high enterprise weight — political goals dominate.
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Distant Claimant",
        bandit_camp,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );

    // Perception profile so the agent can observe post-action results.
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        default_perception_profile(),
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

    // Verify starting position.
    assert_eq!(
        h.world.effective_place(agent),
        Some(bandit_camp),
        "Agent should start at Bandit Camp"
    );

    // Run simulation — 12 travel ticks + planning + DeclareSupport + 5-tick
    // succession period + margin. 40 ticks is generous.
    for _ in 0..40 {
        h.step_once();
    }

    // Assertion 1: Agent arrived at VillageSquare (the office jurisdiction).
    assert_eq!(
        h.world.effective_place(agent),
        Some(VILLAGE_SQUARE),
        "Agent should have traveled from Bandit Camp to Village Square"
    );

    // Assertion 2: Agent is installed as office holder.
    assert_eq!(
        h.world.office_holder(office),
        Some(agent),
        "Agent should be installed as office holder after traveling to jurisdiction and declaring support"
    );

    // Assertion 3: Event log contains Political events from DeclareSupport
    // and succession installation.
    let political_events = h.event_log.events_by_tag(EventTag::Political);
    assert!(
        !political_events.is_empty(),
        "Event log should contain Political events from support declaration and installation"
    );
}

// ---------------------------------------------------------------------------
// Scenario 16: Survival Pressure Suppresses Political Goals
// ---------------------------------------------------------------------------
//
// Setup: Vacant office at VillageSquare (Support law, period=5). Single agent
// at VillageSquare has high enterprise_weight, a belief about the vacant
// office, and 1 owned bread. Hunger starts exactly at the agent's High
// threshold, so ClaimOffice is generated but suppressed by shared goal-policy
// evaluation until self-care pressure is relieved.
//
// Expected: Agent commits eat before any declare_support commit. Hunger drops
// below the High threshold before declare_support commits. Once suppression
// lifts, the agent declares support for self and is later installed as holder.

fn build_survival_pressure_suppresses_political_goals_scenario(
    seed: Seed,
) -> (
    GoldenHarness,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
    Permille,
) {
    let mut h = GoldenHarness::new(seed);
    let hunger_high = DriveThresholds::default().hunger.high();

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Hungry Claimant",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(hunger_high, pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        default_perception_profile(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
    );

    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "Village Elder",
        VILLAGE_SQUARE,
        SuccessionLaw::Support,
        5,
        vec![],
    );
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        &[office],
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    (h, agent, office, hunger_high)
}

fn run_survival_pressure_suppresses_political_goals(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, agent, office, hunger_high) =
        build_survival_pressure_suppresses_political_goals_scenario(seed);
    h.enable_action_tracing();

    let mut first_eat_commit_tick = None;
    let mut first_hunger_below_high_tick = None;
    let mut first_declare_commit_tick = None;
    let mut hunger_below_high_when_declare_committed = None;

    for _ in 0..30 {
        h.step_once();

        let current_tick = h.scheduler.current_tick();
        let sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled for suppression scenario");

        if first_eat_commit_tick.is_none() {
            first_eat_commit_tick = sink.events_for(agent).iter().find_map(|event| {
                if event.action_name == "eat"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
                {
                    Some(event.tick)
                } else {
                    None
                }
            });
        }

        if first_declare_commit_tick.is_none() {
            first_declare_commit_tick = sink.events_for(agent).iter().find_map(|event| {
                if event.action_name == "declare_support"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
                {
                    Some(event.tick)
                } else {
                    None
                }
            });
        }

        let hunger = h.agent_hunger(agent);
        if first_hunger_below_high_tick.is_none() && hunger < hunger_high {
            first_hunger_below_high_tick = Some(current_tick);
        }
        if hunger_below_high_when_declare_committed.is_none() && first_declare_commit_tick.is_some() {
            hunger_below_high_when_declare_committed = Some(hunger < hunger_high);
        }

        if hunger >= hunger_high {
            assert!(
                first_declare_commit_tick.is_none(),
                "Political declaration must remain suppressed while hunger is at or above the High threshold"
            );
        }

        if first_eat_commit_tick.is_some()
            && first_hunger_below_high_tick.is_some()
            && first_declare_commit_tick.is_some()
            && h.world.office_holder(office) == Some(agent)
        {
            break;
        }
    }

    let eat_tick = first_eat_commit_tick.expect("Claimant should commit eat before politics");
    first_hunger_below_high_tick
        .expect("Claimant hunger should fall below the High threshold after eating");
    let declare_tick = first_declare_commit_tick
        .expect("Claimant should commit declare_support after suppression lifts");

    assert!(
        eat_tick < declare_tick,
        "Claimant should commit eat before declare_support"
    );
    assert!(
        hunger_below_high_when_declare_committed == Some(true),
        "declare_support must not commit while hunger remains at or above the High threshold"
    );
    assert_eq!(
        h.agent_commodity_qty(agent, CommodityKind::Bread),
        Quantity(0),
        "Owned bread should be consumed during self-care resolution"
    );
    assert_eq!(
        h.world.office_holder(office),
        Some(agent),
        "Claimant should be installed as office holder after suppression lifts"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_survival_pressure_suppresses_political_goals() {
    let _ = run_survival_pressure_suppresses_political_goals(Seed([117; 32]));
}

#[test]
fn golden_survival_pressure_suppresses_political_goals_replays_deterministically() {
    let seed = Seed([118; 32]);
    let first = run_survival_pressure_suppresses_political_goals(seed);
    let second = run_survival_pressure_suppresses_political_goals(seed);

    assert_eq!(
        first, second,
        "survival-pressure office suppression scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 17: Faction Eligibility Filters Office Claim
// ---------------------------------------------------------------------------
//
// Setup: Vacant office at VillageSquare (Support law, period=5) restricted by
// EligibilityRule::FactionMember(faction). Agent A belongs to the faction and
// Agent B does not. Both are sated, colocated, politically ambitious, and
// have direct beliefs about the office.
//
// Expected: A generates ClaimOffice and is installed. B never generates
// ClaimOffice and never commits declare_support.

#[test]
#[allow(clippy::too_many_lines)]
fn golden_faction_eligibility_filters_office_claim() {
    let mut h = GoldenHarness::new(Seed([119; 32]));
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let eligible_agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Faction Claimant",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        eligible_agent,
        default_perception_profile(),
    );

    let ineligible_agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Unaffiliated Rival",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        ineligible_agent,
        default_perception_profile(),
    );

    let faction = seed_faction(
        &mut h.world,
        &mut h.event_log,
        "Council Circle",
        FactionPurpose::Political,
    );
    add_faction_membership(&mut h.world, &mut h.event_log, eligible_agent, faction);

    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "Village Elder",
        VILLAGE_SQUARE,
        SuccessionLaw::Support,
        5,
        vec![worldwake_core::EligibilityRule::FactionMember(faction)],
    );

    for agent in [eligible_agent, ineligible_agent] {
        seed_actor_beliefs(
            &mut h.world,
            &mut h.event_log,
            agent,
            &[office],
            Tick(0),
            PerceptionSource::DirectObservation,
        );
    }

    for _ in 0..30 {
        h.step_once();
    }

    assert_eq!(
        h.world.office_holder(office),
        Some(eligible_agent),
        "eligible faction member should be installed as office holder"
    );

    let decision_sink = h.driver.trace_sink().expect("decision tracing should be enabled");
    let eligible_generated_claim = (0u64..=30).any(|tick| {
        decision_sink
            .trace_at(eligible_agent, Tick(tick))
            .is_some_and(|trace| match &trace.outcome {
                DecisionOutcome::Planning(planning) => planning
                    .candidates
                    .generated
                    .iter()
                    .any(|goal| goal.kind == GoalKind::ClaimOffice { office }),
                _ => false,
            })
    });
    assert!(
        eligible_generated_claim,
        "eligible agent should generate ClaimOffice while the office is visibly vacant"
    );

    let ineligible_generated_claim = (0u64..=30).any(|tick| {
        decision_sink
            .trace_at(ineligible_agent, Tick(tick))
            .is_some_and(|trace| match &trace.outcome {
                DecisionOutcome::Planning(planning) => planning
                    .candidates
                    .generated
                    .iter()
                    .any(|goal| goal.kind == GoalKind::ClaimOffice { office }),
                _ => false,
            })
    });
    assert!(
        !ineligible_generated_claim,
        "ineligible agent must never generate ClaimOffice for a faction-restricted office"
    );

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let ineligible_declared_support = action_sink.events_for(ineligible_agent).iter().any(|event| {
        event.action_name == "declare_support"
            && matches!(event.kind, ActionTraceKind::Committed { .. })
    });
    assert!(
        !ineligible_declared_support,
        "ineligible agent must never commit declare_support for the restricted office"
    );
}
