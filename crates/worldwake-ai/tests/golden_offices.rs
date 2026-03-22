//! Golden tests for political office claims and succession resolution.

mod golden_harness;

use golden_harness::*;
use worldwake_ai::{DecisionOutcome, PlannerOpKind, SelectedPlanSource};
use worldwake_core::{
    hash_event_log, hash_world, prototype_place_entity, BeliefConfidencePolicy, CombatProfile,
    CommodityKind, DeadAt, DriveThresholds, EventTag, FactionPurpose, GoalKind, HomeostaticNeeds,
    InstitutionalBeliefRead, MetabolismProfile, PerceptionProfile, PerceptionSource, Permille,
    PrototypePlace, Quantity, Seed, StateHash, SuccessionLaw, Tick, UtilityProfile,
};
use worldwake_sim::ActionTraceKind;

// ---------------------------------------------------------------------------
// Scenario 11: Simple Office Claim via DeclareSupport
// ---------------------------------------------------------------------------
//
// Setup: Single sated agent at VillageSquare with high enterprise_weight.
// Vacant office (Support law, period=5, no eligibility rules) at VillageSquare.
// Agent generates ClaimOffice -> plans DeclareSupport(self) -> executes ->
// after succession period, succession_system installs agent as holder.

fn build_simple_office_claim_scenario(
    seed: Seed,
) -> (
    GoldenHarness,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
) {
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
            institutional_memory_capacity: 20,
            consultation_speed_factor: pm(500),
            contradiction_tolerance: pm(300),
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
    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        agent,
        office,
        None,
        Tick(0),
        worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
        Some(VILLAGE_SQUARE),
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
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
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
        seed_office_holder_belief(
            &mut h.world,
            &mut h.event_log,
            agent,
            office,
            None,
            Tick(0),
            worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
            Some(VILLAGE_SQUARE),
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
        seed_office_holder_belief(
            &mut h.world,
            &mut h.event_log,
            agent,
            office,
            None,
            Tick(0),
            worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
            Some(VILLAGE_SQUARE),
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
    seed_support_declaration_belief(
        &mut h.world,
        &mut h.event_log,
        agent_a,
        office,
        agent_c,
        Some(agent_c),
        Tick(0),
        worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
        Some(VILLAGE_SQUARE),
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
// loyalty increase and B starts generating SupportCandidateForOffice(A).
// A declares for self. C has hostility toward A or is unaffected.
//
// This scenario intentionally stops short of asserting the final office winner.
// Coalition winner selection is a separate ranking concern once multiple
// support paths remain live; the invariant here is courage-diverse coercion and
// the downstream opening of a support path for the yielding target.

fn combat_profile_with_attack_skill(attack_skill: Permille) -> CombatProfile {
    CombatProfile::new(
        pm(1000), // wound_capacity
        pm(700),  // incapacitation_threshold
        attack_skill,
        pm(500), // guard_skill
        pm(80),  // defend_bonus
        pm(25),  // natural_clot_resistance
        pm(18),  // natural_recovery_rate
        pm(120), // unarmed_wound_severity
        pm(35),  // unarmed_bleed_rate
        nz(6),   // unarmed_attack_ticks
        nz(10),  // defend_stance_ticks
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
    h.driver.enable_tracing();
    h.enable_action_tracing();

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
        seed_office_holder_belief(
            &mut h.world,
            &mut h.event_log,
            agent,
            office,
            None,
            Tick(0),
            worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
            Some(VILLAGE_SQUARE),
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
    seed_support_declaration_belief(
        &mut h.world,
        &mut h.event_log,
        agent_a,
        office,
        agent_d,
        Some(agent_d),
        Tick(0),
        worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
        Some(VILLAGE_SQUARE),
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

    let decision_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled");
    let b_generated_support_for_a = decision_sink
        .goal_history_for(
            agent_b,
            &GoalKind::SupportCandidateForOffice {
                office,
                candidate: agent_a,
            },
        )
        .into_iter()
        .any(|entry| entry.status.is_generated());
    assert!(
        b_generated_support_for_a,
        "B should generate SupportCandidateForOffice(A) after yielding to threat"
    );

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let a_declared_self_support = action_sink.events_for(agent_a).iter().any(|event| {
        event.action_name == "declare_support"
            && matches!(event.kind, ActionTraceKind::Committed { .. })
    });
    assert!(
        a_declared_self_support,
        "A should still commit declare_support after the threat path opens"
    );

    // Assertion 4: Agent diversity (Principle 20) — same action type,
    // different courage values produced divergent outcomes.
    // B gained loyalty (yield), C did not (resist or not threatened).
    assert_ne!(
        final_b_loyalty, c_loyalty_to_a,
        "Principle 20: same Threaten action must produce divergent outcomes \
         for agents with different courage values"
    );

    // Assertion 5: Event log contains coercive and political follow-through.
    let political_events = h.event_log.events_by_tag(EventTag::Political);
    let coercion_events = h.event_log.events_by_tag(EventTag::Coercion);
    assert!(
        !political_events.is_empty(),
        "Event log should contain Political events from support follow-through"
    );
    assert!(
        !coercion_events.is_empty(),
        "Event log should contain Coercion events from the threat interaction"
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
    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        agent,
        office,
        None,
        Tick(0),
        worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
        Some(VILLAGE_SQUARE),
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
// Scenario 16: Political Office Facts Remain Local Until Belief Update
// ---------------------------------------------------------------------------
//
// Setup: Vacant office at VillageSquare (Support law, period=5, no eligibility).
// Single politically ambitious agent starts at BanditCamp with no belief about
// the office. After an explicit reported belief update, the agent should begin
// the normal ClaimOffice -> travel -> declare_support -> succession path.

#[allow(clippy::too_many_lines)]
fn run_information_locality_for_political_facts(seed: Seed) -> (StateHash, StateHash) {
    let bandit_camp = prototype_place_entity(PrototypePlace::BanditCamp);
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();

    let informant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Informant",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        informant,
        default_perception_profile(),
    );

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Remote Claimant",
        bandit_camp,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        default_perception_profile(),
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

    assert!(
        agent_belief_about(&h.world, agent, office).is_none(),
        "agent should start without an office belief"
    );
    assert_eq!(
        h.world.effective_place(agent),
        Some(bandit_camp),
        "agent should start at Bandit Camp"
    );

    for _ in 0..8 {
        h.step_once();
    }

    let phase_one_end = h.scheduler.current_tick().0;
    let generated_before_update = {
        let decision_sink = h
            .driver
            .trace_sink()
            .expect("decision tracing should be enabled");
        decision_sink
            .goal_history_for(agent, &GoalKind::ClaimOffice { office })
            .into_iter()
            .filter(|entry| entry.tick.0 <= phase_one_end)
            .any(|entry| entry.status.is_generated())
            || decision_sink
                .goal_history_for(
                    agent,
                    &GoalKind::SupportCandidateForOffice {
                        office,
                        candidate: agent,
                    },
                )
                .into_iter()
                .filter(|entry| entry.tick.0 <= phase_one_end)
                .any(|entry| entry.status.is_generated())
    };
    assert!(
        !generated_before_update,
        "agent must not generate political goals for an unknown remote office"
    );
    assert_eq!(
        h.world.effective_place(agent),
        Some(bandit_camp),
        "agent should remain at Bandit Camp before learning about the office"
    );
    assert_eq!(
        h.world.office_holder(office),
        None,
        "office should remain vacant before the remote claimant learns about it"
    );

    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        &[office],
        Tick(phase_one_end),
        PerceptionSource::Report {
            from: informant,
            chain_len: 1,
        },
    );
    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        agent,
        office,
        None,
        Tick(phase_one_end),
        worldwake_core::InstitutionalKnowledgeSource::Report {
            from: informant,
            chain_len: 1,
        },
        Some(bandit_camp),
    );
    let seeded_belief = agent_belief_about(&h.world, agent, office)
        .expect("agent should immediately receive the explicit office belief update");
    assert!(
        matches!(
            seeded_belief.source,
            PerceptionSource::Report {
                from,
                chain_len: 1
            } if from == informant
        ),
        "office belief update should enter as an explicit report"
    );

    for _ in 0..40 {
        h.step_once();
        if h.world.office_holder(office) == Some(agent) {
            break;
        }
    }

    let generated_after_update = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .goal_history_for(agent, &GoalKind::ClaimOffice { office })
        .into_iter()
        .filter(|entry| entry.tick.0 > phase_one_end)
        .any(|entry| entry.status.is_generated());

    assert!(
        agent_belief_about(&h.world, agent, office).is_some(),
        "agent should retain some belief about the office after acting on it"
    );
    assert!(
        generated_after_update,
        "agent should generate ClaimOffice after receiving the office belief"
    );
    assert_eq!(
        h.world.effective_place(agent),
        Some(VILLAGE_SQUARE),
        "agent should travel to the office jurisdiction only after the belief update"
    );
    assert_eq!(
        h.world.office_holder(office),
        Some(agent),
        "agent should become office holder after learning about the remote office"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_information_locality_for_political_facts() {
    let _ = run_information_locality_for_political_facts(Seed([117; 32]));
}

#[test]
fn golden_information_locality_for_political_facts_replays_deterministically() {
    let seed = Seed([118; 32]);
    let first = run_information_locality_for_political_facts(seed);
    let second = run_information_locality_for_political_facts(seed);

    assert_eq!(
        first, second,
        "political locality scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 33: Remote Record Travel + Consultation + Political Action
// ---------------------------------------------------------------------------
//
// Setup: Single sated claimant starts at OrchardFarm with high enterprise
// weight. The office is vacant at VillageSquare, but the vacancy entry exists
// only in a remote OfficeRegister at RulersHall. The claimant knows about the
// office and the remote record entity, but has no seeded institutional belief
// about the office holder.
//
// Expected: ClaimOffice remains the selected goal, the initial selected plan is
// Travel(RulersHall) -> ConsultRecord(remote record) -> Travel(VillageSquare)
// -> DeclareSupport(self), consult_record commits before declare_support, and
// succession installs the claimant as office holder.

fn build_remote_record_consultation_political_action_scenario(
    seed: Seed,
) -> (
    GoldenHarness,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Archive Claimant",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        default_perception_profile(),
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
    let remote_record = seed_office_register(&mut h.world, &mut h.event_log, RULERS_HALL);
    seed_office_vacancy_entry(&mut h.world, &mut h.event_log, office, RULERS_HALL);

    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        &[office, remote_record],
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    let initial_beliefs = h
        .world
        .get_component_agent_belief_store(agent)
        .expect("claimant should have a belief store after entity belief seeding");
    assert!(
        matches!(
            initial_beliefs.believed_office_holder(office),
            InstitutionalBeliefRead::Unknown
        ),
        "claimant should start with unknown office-holder belief so ConsultRecord owns the prerequisite"
    );

    let local_record = h
        .world
        .query_record_data()
        .find_map(|(entity, record)| {
            (record.record_kind == worldwake_core::RecordKind::OfficeRegister
                && record.home_place == VILLAGE_SQUARE)
                .then_some((entity, record.entries.len()))
        })
        .expect("seed_office should create the jurisdiction-local office register");
    assert_eq!(
        local_record.1, 0,
        "the jurisdiction-local office register should remain empty in the remote-record scenario"
    );

    let remote_record_data = h
        .world
        .get_component_record_data(remote_record)
        .expect("remote office register should exist");
    assert_eq!(
        remote_record_data.entries.len(),
        1,
        "remote office register should hold the vacancy entry for the scenario"
    );

    (h, agent, office, remote_record, local_record.0)
}

#[allow(clippy::too_many_lines)]
fn run_remote_record_consultation_political_action(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, agent, office, remote_record, _) =
        build_remote_record_consultation_political_action_scenario(seed);

    for _ in 0..30 {
        h.step_once();
        if h.world.office_holder(office) == Some(agent) {
            break;
        }
    }

    let decision_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled for remote-record office scenario");
    let tick_zero_trace = decision_sink
        .trace_at(agent, Tick(0))
        .expect("claimant should produce a tick 0 decision trace");
    let planning_tick_zero = match &tick_zero_trace.outcome {
        DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected planning trace at tick 0, got {other:?}"),
    };
    let selected_plan = planning_tick_zero
        .selection
        .selected_plan
        .as_ref()
        .expect("claimant should select a remote-record office plan at tick 0");
    assert_eq!(
        planning_tick_zero.selection.selected_plan_source,
        Some(SelectedPlanSource::SearchSelection),
        "remote-record office scenario should start from a fresh search result"
    );
    assert!(
        planning_tick_zero.candidates.generated.iter().any(
            |goal| matches!(goal.kind, GoalKind::ClaimOffice { office: goal_office } if goal_office == office)
        ),
        "tick 0 candidates should include ClaimOffice for the vacant office"
    );
    let step_kinds = selected_plan
        .steps
        .iter()
        .map(|step| step.op_kind)
        .collect::<Vec<_>>();
    assert_eq!(
        step_kinds,
        vec![
            PlannerOpKind::Travel,
            PlannerOpKind::Travel,
            PlannerOpKind::Travel,
            PlannerOpKind::Travel,
            PlannerOpKind::ConsultRecord,
            PlannerOpKind::Travel,
            PlannerOpKind::DeclareSupport,
        ],
        "selected plan should expose the concrete multi-hop route to the remote record before the political terminal step"
    );
    assert_eq!(
        selected_plan.steps[0].targets,
        vec![prototype_place_entity(PrototypePlace::EastFieldTrail)],
        "the first step should leave Orchard Farm toward East Field Trail"
    );
    assert_eq!(
        selected_plan.steps[1].targets,
        vec![prototype_place_entity(PrototypePlace::SouthGate)],
        "the second step should continue toward South Gate"
    );
    assert_eq!(
        selected_plan.steps[2].targets,
        vec![VILLAGE_SQUARE],
        "the third step should bring the claimant back to Village Square on the way to the archive"
    );
    assert_eq!(
        selected_plan.steps[3].targets,
        vec![RULERS_HALL],
        "the fourth step should reach the remote record location"
    );
    assert_eq!(
        selected_plan.steps[4].targets,
        vec![remote_record],
        "the consult step should target the remote office register"
    );
    assert_eq!(
        selected_plan.steps[5].targets,
        vec![VILLAGE_SQUARE],
        "the return travel step should target the office jurisdiction"
    );

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled for remote-record office scenario");
    let consult_commit = action_sink
        .events_for(agent)
        .into_iter()
        .find_map(|event| {
            (event.action_name == "consult_record"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .expect("claimant should commit consult_record before acting politically");
    let declare_support_commit = action_sink
        .events_for(agent)
        .into_iter()
        .find_map(|event| {
            (event.action_name == "declare_support"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .expect("claimant should commit declare_support after consulting the record");
    assert!(
        consult_commit < declare_support_commit,
        "consult_record must commit before declare_support in the remote-record path"
    );

    assert_eq!(
        h.world.effective_place(agent),
        Some(VILLAGE_SQUARE),
        "claimant should finish at the office jurisdiction after the return leg"
    );
    assert_eq!(
        h.world.office_holder(office),
        Some(agent),
        "claimant should become office holder after remote consultation and succession"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_remote_record_consultation_political_action() {
    let _ = run_remote_record_consultation_political_action(Seed([124; 32]));
}

#[test]
fn golden_remote_record_consultation_political_action_replays_deterministically() {
    let seed = Seed([125; 32]);

    let first = run_remote_record_consultation_political_action(seed);
    let second = run_remote_record_consultation_political_action(seed);

    assert_eq!(
        first, second,
        "remote-record office scenario should replay deterministically"
    );

    let (fresh, _, _, _, _) = build_remote_record_consultation_political_action_scenario(seed);
    let initial_world_hash = hash_world(&fresh.world).unwrap();
    assert_ne!(
        first.0, initial_world_hash,
        "remote-record office scenario should change world state non-trivially"
    );
}

// ---------------------------------------------------------------------------
// Scenario 17: Survival Pressure Suppresses Political Goals
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
    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        agent,
        office,
        None,
        Tick(0),
        worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
        Some(VILLAGE_SQUARE),
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
        if hunger_below_high_when_declare_committed.is_none() && first_declare_commit_tick.is_some()
        {
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
    let _ = run_survival_pressure_suppresses_political_goals(Seed([119; 32]));
}

#[test]
fn golden_survival_pressure_suppresses_political_goals_replays_deterministically() {
    let seed = Seed([120; 32]);
    let first = run_survival_pressure_suppresses_political_goals(seed);
    let second = run_survival_pressure_suppresses_political_goals(seed);

    assert_eq!(
        first, second,
        "survival-pressure office suppression scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 18: Faction Eligibility Filters Office Claim
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
    let mut h = GoldenHarness::new(Seed([121; 32]));
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
        seed_office_holder_belief(
            &mut h.world,
            &mut h.event_log,
            agent,
            office,
            None,
            Tick(0),
            worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
            Some(VILLAGE_SQUARE),
        );
    }
    seed_faction_membership_belief(
        &mut h.world,
        &mut h.event_log,
        eligible_agent,
        faction,
        eligible_agent,
        true,
        Tick(0),
        worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
        Some(VILLAGE_SQUARE),
    );

    for _ in 0..30 {
        h.step_once();
    }

    assert_eq!(
        h.world.office_holder(office),
        Some(eligible_agent),
        "eligible faction member should be installed as office holder"
    );

    let decision_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled");
    let eligible_generated_claim = decision_sink
        .goal_history_for(eligible_agent, &GoalKind::ClaimOffice { office })
        .into_iter()
        .filter(|entry| entry.tick.0 <= 30)
        .any(|entry| entry.status.is_generated());
    assert!(
        eligible_generated_claim,
        "eligible agent should generate ClaimOffice while the office is visibly vacant"
    );

    let ineligible_generated_claim = decision_sink
        .goal_history_for(ineligible_agent, &GoalKind::ClaimOffice { office })
        .into_iter()
        .filter(|entry| entry.tick.0 <= 30)
        .any(|entry| entry.status.is_generated());
    assert!(
        !ineligible_generated_claim,
        "ineligible agent must never generate ClaimOffice for a faction-restricted office"
    );

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let ineligible_declared_support =
        action_sink
            .events_for(ineligible_agent)
            .iter()
            .any(|event| {
                event.action_name == "declare_support"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            });
    assert!(
        !ineligible_declared_support,
        "ineligible agent must never commit declare_support for the restricted office"
    );
}

// ---------------------------------------------------------------------------
// Scenario 19: Force Succession Installs Sole Living Eligible Contender
// ---------------------------------------------------------------------------
//
// Setup: Vacant office at VillageSquare using SuccessionLaw::Force. Agent A is
// politically ambitious, informed about the office, and alive at the
// jurisdiction. Agent B is colocated and otherwise eligible but has
// DeadAt(Tick(0)).
//
// Expected: Force-law succession installs A after the succession period. Since
// Force offices do not use support-based political actions, no declare_support
// action commits occur.

fn build_force_succession_scenario(
    seed: Seed,
) -> (
    GoldenHarness,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
) {
    let mut h = GoldenHarness::new(seed);
    h.enable_action_tracing();

    let living_claimant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Force Claimant",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        living_claimant,
        default_perception_profile(),
    );

    let dead_rival = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Dead Rival",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        dead_rival,
        default_perception_profile(),
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_dead_at(dead_rival, DeadAt(Tick(0)))
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "War Chief",
        VILLAGE_SQUARE,
        SuccessionLaw::Force,
        5,
        vec![],
    );

    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        living_claimant,
        &[office],
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        living_claimant,
        office,
        None,
        Tick(0),
        worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
        Some(VILLAGE_SQUARE),
    );

    (h, living_claimant, dead_rival, office)
}

fn run_force_succession(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, living_claimant, dead_rival, office) = build_force_succession_scenario(seed);

    for _ in 0..12 {
        h.step_once();
    }

    assert_eq!(
        h.world.office_holder(office),
        Some(living_claimant),
        "Force-law succession should install the sole living eligible contender"
    );
    assert_eq!(
        h.world.get_component_dead_at(dead_rival),
        Some(&DeadAt(Tick(0))),
        "dead rival should remain dead and excluded from eligibility"
    );

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled for force succession scenario");
    let declare_support_commits = action_sink
        .events_for(living_claimant)
        .iter()
        .chain(action_sink.events_for(dead_rival).iter())
        .filter(|event| {
            event.action_name == "declare_support"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        })
        .count();
    assert_eq!(
        declare_support_commits, 0,
        "Force-law offices must not produce declare_support commits"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_force_succession_sole_eligible() {
    let _ = run_force_succession(Seed([122; 32]));
}

#[test]
fn golden_force_succession_deterministic_replay() {
    let seed = Seed([123; 32]);

    let first = run_force_succession(seed);
    let second = run_force_succession(seed);

    assert_eq!(
        first, second,
        "force succession scenario should replay deterministically"
    );
}
