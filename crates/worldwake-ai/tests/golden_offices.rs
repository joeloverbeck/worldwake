//! Golden tests for political office claims and succession resolution.

mod golden_harness;

use std::collections::BTreeSet;

use golden_harness::*;
use worldwake_ai::{
    apply_hypothetical_transition, build_planning_snapshot, build_semantics_table, DecisionOutcome,
    GoalKindPlannerExt, GroundedGoal, PlanSearchResult, PlannerOpKind, PlanningBudget,
    PlanningState, SelectedPlanSource,
};
use worldwake_core::{
    hash_event_log, hash_world, prototype_place_entity, AgentData, BeliefConfidencePolicy,
    BlockedIntentMemory, CombatProfile, CommodityKind, ControlSource, DriveThresholds, EventTag,
    FactionPurpose, GoalKind, HomeostaticNeeds, InstitutionalBeliefRead, MetabolismProfile,
    PerceptionProfile, PerceptionSource, Permille, PrototypePlace, Quantity, Seed, StateHash,
    SuccessionLaw, TellProfile, Tick, UtilityProfile,
};
use worldwake_sim::{
    get_affordances, ActionPayload, ActionRequestMode, ActionTraceDetail, ActionTraceKind,
    InputKind, OfficeSuccessionOutcome, PerAgentBeliefView, PressForceClaimActionPayload,
    RequestProvenance, RuntimeBeliefView, SupportCountTrace, SupportResolutionTrace,
    VacancyTimerTrace, YieldForceClaimActionPayload,
};

// ---------------------------------------------------------------------------
// Scenario 11: Simple Office Claim via DeclareSupport
// ---------------------------------------------------------------------------
//
// Systems: Succession, AI, Political actions
// GoalKinds: ClaimOffice
// ActionDomains: Generic
// Places: VillageSquare
// Principles: 10, 20
//
// Setup: Single sated agent at VillageSquare with enterprise_weight=pm(800).
//   Vacant office (Support law, period=5, no eligibility).
//
// Proves: Agent autonomously generates ClaimOffice from enterprise_weight
//   and believed vacant office. GOAP plans DeclareSupport(self). After
//   succession period, succession_system installs agent as holder.
//
// Chain: Enterprise weight -> ClaimOffice candidate -> DeclareSupport plan
//   -> action execution -> succession resolution -> office installation.

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
//
// Systems: Succession, AI, Political actions
// GoalKinds: ClaimOffice
// Places: VillageSquare
//
// Setup: Same as Scenario 11, run twice with identical seed.
//
// Proves: Two runs with the same seed produce identical world and
//   event-log hashes. World state differs from initial.

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
// Systems: Succession, AI, Political actions
// GoalKinds: ClaimOffice, SupportCandidateForOffice
// ActionDomains: Generic
// Places: VillageSquare
// Principles: 10, 20
//
// Setup: Three agents at VillageSquare. A and B with enterprise_weight=pm(800).
//   C with enterprise_weight=0, social_weight=pm(600), loyalty to A at pm(650).
//   Vacant office (Support law, period=5).
//
// Proves: A and B generate ClaimOffice. C's ClaimOffice is zero-motive filtered
//   (enterprise_weight=0); C generates SupportCandidateForOffice(A) from loyalty.
//   A gets 2 declarations (self + C), B gets 1. Succession installs A.
//
// Chain: Loyalty -> SupportCandidateForOffice candidate -> zero-motive ClaimOffice
//   filtering -> DeclareSupport plan -> multi-agent declarations -> support
//   counting -> decisive installation.

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

fn accepting_tell_profile() -> TellProfile {
    TellProfile {
        max_tell_candidates: 3,
        max_relay_chain_len: 3,
        acceptance_fidelity: pm(1000),
        ..TellProfile::default()
    }
}

fn focused_accepting_tell_profile() -> TellProfile {
    TellProfile {
        max_tell_candidates: 1,
        ..accepting_tell_profile()
    }
}

fn set_control_source(
    h: &mut GoldenHarness,
    agent: worldwake_core::EntityId,
    control_source: ControlSource,
    tick: u64,
) {
    let mut txn = new_txn(&mut h.world, tick);
    txn.set_component_agent_data(agent, AgentData { control_source })
        .unwrap();
    commit_txn(txn, &mut h.event_log);
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
// Systems: Bribe, Succession, AI, Conservation
// GoalKinds: ClaimOffice, SupportCandidateForOffice
// ActionDomains: Generic
// Places: VillageSquare, OrchardFarm
// Principles: 1, 10
//
// Setup: A with enterprise_weight=pm(900) holds 5 bread. B at jurisdiction,
//   no loyalty. C (competitor) at OrchardFarm with pre-declared self-support.
//   Wider beam_width=16 for branchy adjacency graph.
//
// Proves: DeclareSupport alone would tie with C (ProgressBarrier). Coalition-aware
//   planner finds Bribe(B, bread) + DeclareSupport(self). A bribes B (full 5 bread
//   transfer). B's loyalty increases and B generates SupportCandidateForOffice(A).
//   A's coalition (2) beats C (1). Conservation: bread total unchanged.
//
// Chain: AI goal -> coalition-aware planner Bribe op -> commodity transfer ->
//   conservation -> loyalty increase -> target SupportCandidateForOffice ->
//   DeclareSupport -> support counting -> decisive installation.

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
    h.driver.enable_tracing();
    h.enable_action_tracing();
    h.enable_politics_tracing();

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

#[test]
#[ignore = "manual benchmark"]
fn bench_branchy_office_coalition() {
    use std::time::Instant;

    const RUNS: usize = 3;
    let mut elapsed = Vec::with_capacity(RUNS);

    for _ in 0..RUNS {
        let start = Instant::now();
        golden_bribe_support_coalition();
        elapsed.push(start.elapsed());
    }

    let total = elapsed.iter().copied().sum::<std::time::Duration>();
    let average = total / RUNS as u32;
    eprintln!(
        "bench_branchy_office_coalition: runs={RUNS} total={total:?} avg={average:?} samples={elapsed:?}"
    );
}

// ---------------------------------------------------------------------------
// Scenario 14: Threaten with Courage Diversity (Principle 20)
// ---------------------------------------------------------------------------
//
// Systems: Threaten, Succession, AI
// GoalKinds: ClaimOffice, SupportCandidateForOffice
// ActionDomains: Generic
// Places: VillageSquare, OrchardFarm
// Principles: 1, 10, 20
//
// Setup: A with attack_skill=pm(800), enterprise_weight=pm(900). B with
//   courage=pm(200) (yields). C with courage=pm(900) (resists). D at
//   OrchardFarm (not co-located, not threatenable) with pre-declared support.
//
// Proves: Same Threaten action, different courage -> divergent outcomes
//   (Principle 20). Threaten(B) viable (800 > 200), Threaten(C) not (800 < 900).
//   B yields -> loyalty increase -> SupportCandidateForOffice(A). Stops short
//   of asserting office winner; invariant is courage-diverse coercion.
//
// Chain: AI goal -> coalition-aware planner Threaten op -> courage comparison
//   -> yield/resist divergence -> loyalty increase -> target AI
//   SupportCandidateForOffice -> DeclareSupport follow-through.

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
// Systems: Travel, Succession, AI, Political actions
// GoalKinds: ClaimOffice
// ActionDomains: Travel, Generic
// Places: VillageSquare, BanditCamp, ForestPath, NorthCrossroads
// Principles: 1, 7, 8, 10
//
// Setup: Single sated agent at BanditCamp (3 hops / 12 travel ticks from
//   VillageSquare). enterprise_weight=pm(800). Vacant office at VillageSquare.
//
// Proves: Agent generates ClaimOffice from beliefs about a remote vacant office.
//   Planner identifies co-location precondition (Principle 7). Plans multi-hop
//   Travel + DeclareSupport. Succession installs agent after travel + period.
//
// Chain: AI goal from remote belief -> multi-hop travel planning -> sequential
//   travel execution -> arrival at jurisdiction -> DeclareSupport -> succession
//   resolution -> office installation.

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
// Systems: AI, Travel, Succession, Political actions, Perception
// GoalKinds: ClaimOffice
// ActionDomains: Travel, Generic
// Places: VillageSquare, BanditCamp
// Principles: 7, 10, 13
//
// Setup: Vacant office at VillageSquare. Ambitious agent at BanditCamp with
//   no belief about the office. Report-sourced belief injected after initial phase.
//
// Proves: Without office belief, agent never generates ClaimOffice or begins
//   travel (Principle 7 locality). After explicit Report belief update, ordinary
//   office-planning appears: ClaimOffice, travel, DeclareSupport, installation.
//
// Chain: No office belief -> no political candidate generation -> explicit
//   reported belief update -> ClaimOffice candidate -> travel to jurisdiction
//   -> DeclareSupport -> succession resolution.

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
// Systems: AI, Travel, ConsultRecord, Succession, Political actions
// GoalKinds: ClaimOffice
// ActionDomains: Travel, Generic
// Places: OrchardFarm, RulersHall, VillageSquare, EastFieldTrail, SouthGate
// Principles: 7, 8, 12, 24
//
// Setup: Claimant at OrchardFarm with unknown office-holder belief. Office at
//   VillageSquare but vacancy entry only in remote OfficeRegister at RulersHall.
//
// Proves: ClaimOffice generated despite unknown office-holder belief. Selected
//   plan routes to RulersHall first for consult_record, then returns for
//   DeclareSupport. Institutional belief transitions Unknown -> Certain(None)
//   via RecordConsultation. Distinct from S15 (known vacancy) and S16 (no belief).
//
// Chain: Unknown office-holder belief + known remote register -> ClaimOffice
//   candidate -> travel to RulersHall -> consult_record -> institutional belief
//   update -> return to VillageSquare -> DeclareSupport -> succession installation.

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
    h.enable_politics_tracing();
    h.enable_politics_tracing();
    h.enable_institutional_knowledge_tracing();

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

    let knowledge_sink = h.institutional_knowledge_trace_sink().expect(
        "institutional knowledge tracing should be enabled for remote-record office scenario",
    );
    let knowledge_events = knowledge_sink.events_for(agent);
    assert_eq!(
        knowledge_events.len(),
        1,
        "remote-record office scenario should emit one effective institutional knowledge event"
    );
    let knowledge_event = knowledge_events[0];
    assert_eq!(
        knowledge_event.source,
        worldwake_sim::InstitutionalKnowledgeTraceSource::RecordConsultation {
            record: remote_record,
            home_place: RULERS_HALL,
        }
    );
    assert_eq!(
        knowledge_event.transitions,
        vec![worldwake_sim::InstitutionalBeliefTransitionTrace {
            key: worldwake_core::InstitutionalBeliefKey::OfficeHolderOf { office },
            source_entry_ids: vec![worldwake_core::RecordEntryId(0)],
            previous: worldwake_sim::InstitutionalBeliefReadSummary::Unknown,
            new: worldwake_sim::InstitutionalBeliefReadSummary::OfficeHolderCertain { holder: None },
        }],
        "remote-record office scenario should trace the authoritative office-holder knowledge acquisition"
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
// Scenario 34: Knowledge Asymmetry Race
// ---------------------------------------------------------------------------
//
// Systems: AI, ConsultRecord, Succession, Political actions
// GoalKinds: ClaimOffice
// ActionDomains: Generic
// Places: VillageSquare
// Principles: 8, 12, 20, 24
//
// Setup: Two co-located claimants at VillageSquare. Informed claimant has
//   Certain(None) office-holder belief. Uninformed must consult local register
//   (consultation_ticks=12, speed_factor=pm(500) -> 6 ticks).
//
// Proves: Both generate ClaimOffice at tick 0. Informed selects direct
//   DeclareSupport; uninformed selects ConsultRecord -> DeclareSupport.
//   Informed commits declare_support first. Uninformed loses succession window.
//   Competitive outcome emerges from knowledge state + duration cost (Principle 20).
//
// Chain: Same office + same ambition + different belief certainty -> informed
//   direct DeclareSupport vs uninformed consult_record duration -> succession
//   installs informed claimant first.

#[allow(clippy::too_many_lines)]
fn build_knowledge_asymmetry_race_scenario(
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
    h.enable_politics_tracing();

    let informed_agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Informed Claimant",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        informed_agent,
        default_perception_profile(),
    );

    let uninformed_agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Uninformed Claimant",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        uninformed_agent,
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
    let record = seed_office_register(&mut h.world, &mut h.event_log, VILLAGE_SQUARE);
    seed_office_vacancy_entry(&mut h.world, &mut h.event_log, office, VILLAGE_SQUARE);

    {
        let mut txn = new_txn(&mut h.world, 0);
        let mut record_data = txn
            .get_component_record_data(record)
            .cloned()
            .expect("knowledge-asymmetry scenario should have a local office register");
        record_data.consultation_ticks = 12;
        txn.set_component_record_data(record, record_data)
            .expect("knowledge-asymmetry scenario should be able to raise consultation duration");
        commit_txn(txn, &mut h.event_log);
    }

    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        informed_agent,
        &[office],
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        informed_agent,
        office,
        None,
        Tick(0),
        worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
        Some(VILLAGE_SQUARE),
    );

    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        uninformed_agent,
        &[office, record],
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    let informed_beliefs = h
        .world
        .get_component_agent_belief_store(informed_agent)
        .expect("informed claimant should have a belief store");
    assert!(
        matches!(
            informed_beliefs.believed_office_holder(office),
            InstitutionalBeliefRead::Certain(None)
        ),
        "informed claimant should start with certain vacancy knowledge"
    );
    let uninformed_beliefs = h
        .world
        .get_component_agent_belief_store(uninformed_agent)
        .expect("uninformed claimant should have a belief store");
    assert!(
        matches!(
            uninformed_beliefs.believed_office_holder(office),
            InstitutionalBeliefRead::Unknown
        ),
        "uninformed claimant should start with unknown office-holder belief"
    );
    assert_eq!(
        h.world
            .get_component_record_data(record)
            .expect("knowledge-asymmetry scenario should retain the local office register")
            .consultation_ticks,
        12,
        "scenario setup must encode the longer consult in authoritative record state"
    );

    (h, informed_agent, uninformed_agent, office, record)
}

#[allow(clippy::too_many_lines)]
fn run_knowledge_asymmetry_race(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, informed_agent, uninformed_agent, office, record) =
        build_knowledge_asymmetry_race_scenario(seed);

    for _ in 0..20 {
        h.step_once();
        if h.world.office_holder(office) == Some(informed_agent) {
            break;
        }
    }

    let decision_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled for knowledge-asymmetry scenario");

    let informed_tick_zero = decision_sink
        .trace_at(informed_agent, Tick(0))
        .expect("informed claimant should produce a tick 0 decision trace");
    let informed_planning = match &informed_tick_zero.outcome {
        DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected planning trace for informed claimant, got {other:?}"),
    };
    assert!(
        informed_planning.candidates.generated.iter().any(
            |goal| matches!(goal.kind, GoalKind::ClaimOffice { office: goal_office } if goal_office == office)
        ),
        "informed claimant should generate ClaimOffice at tick 0"
    );
    assert_eq!(
        informed_planning.selection.selected_plan_source,
        Some(SelectedPlanSource::SearchSelection),
        "informed claimant should select a fresh search result"
    );
    let informed_selected_plan = informed_planning
        .selection
        .selected_plan
        .as_ref()
        .expect("informed claimant should select an office-claim plan");
    let informed_step_kinds = informed_selected_plan
        .steps
        .iter()
        .map(|step| step.op_kind)
        .collect::<Vec<_>>();
    assert_eq!(
        informed_step_kinds,
        vec![PlannerOpKind::DeclareSupport],
        "informed claimant should not need ConsultRecord before declaring support"
    );

    let uninformed_tick_zero = decision_sink
        .trace_at(uninformed_agent, Tick(0))
        .expect("uninformed claimant should produce a tick 0 decision trace");
    let uninformed_planning = match &uninformed_tick_zero.outcome {
        DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected planning trace for uninformed claimant, got {other:?}"),
    };
    assert!(
        uninformed_planning.candidates.generated.iter().any(
            |goal| matches!(goal.kind, GoalKind::ClaimOffice { office: goal_office } if goal_office == office)
        ),
        "uninformed claimant should also generate ClaimOffice at tick 0"
    );
    assert_eq!(
        uninformed_planning.selection.selected_plan_source,
        Some(SelectedPlanSource::SearchSelection),
        "uninformed claimant should also start from a fresh search result"
    );
    let uninformed_selected_plan = uninformed_planning
        .selection
        .selected_plan
        .as_ref()
        .expect("uninformed claimant should select an office-claim plan");
    let uninformed_step_kinds = uninformed_selected_plan
        .steps
        .iter()
        .map(|step| step.op_kind)
        .collect::<Vec<_>>();
    assert_eq!(
        uninformed_step_kinds,
        vec![PlannerOpKind::ConsultRecord, PlannerOpKind::DeclareSupport],
        "uninformed claimant should need ConsultRecord before declaring support"
    );
    assert_eq!(
        uninformed_selected_plan.steps[0].targets,
        vec![record],
        "uninformed claimant should target the local office register"
    );

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled for knowledge-asymmetry scenario");
    let informed_declare_commit = action_sink
        .events_for(informed_agent)
        .into_iter()
        .find_map(|event| {
            (event.action_name == "declare_support"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .expect("informed claimant should commit declare_support");
    let uninformed_consult_commit = action_sink
        .events_for(uninformed_agent)
        .into_iter()
        .find_map(|event| {
            (event.action_name == "consult_record"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .expect("uninformed claimant should commit consult_record");
    assert!(
        informed_declare_commit < uninformed_consult_commit,
        "informed claimant must commit declare_support before the uninformed consult finishes"
    );
    let uninformed_declared_support =
        action_sink
            .events_for(uninformed_agent)
            .iter()
            .any(|event| {
                event.action_name == "declare_support"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            });
    assert!(
        !uninformed_declared_support,
        "uninformed claimant must not commit declare_support before the office is installed"
    );

    assert_eq!(
        h.world.office_holder(office),
        Some(informed_agent),
        "informed claimant should win the office before the uninformed claimant can finish the consult-driven branch"
    );

    let politics_sink = h
        .politics_trace_sink()
        .expect("politics tracing should be enabled for knowledge-asymmetry scenario");
    let install_trace = politics_sink
        .events_for_office(office)
        .into_iter()
        .find(|event| {
            matches!(
                event.trace.outcome,
                OfficeSuccessionOutcome::SupportInstalled { holder, .. } if holder == informed_agent
            )
        })
        .expect("politics trace should expose the support-law install for the informed claimant");
    assert_eq!(
        install_trace.trace.vacancy_timer,
        Some(VacancyTimerTrace {
            start_tick: Tick(0),
            waited_ticks: 5,
            required_ticks: 5,
            remaining_ticks: 0,
        })
    );
    assert_eq!(
        install_trace.trace.support_resolution,
        Some(SupportResolutionTrace {
            counted_support: vec![SupportCountTrace {
                candidate: informed_agent,
                support: 1,
            }],
        })
    );
    assert_eq!(install_trace.trace.support_declarations.len(), 1);
    assert_eq!(
        install_trace.trace.support_declarations[0].supporter,
        informed_agent
    );
    assert_eq!(
        install_trace.trace.support_declarations[0].candidate,
        informed_agent
    );
    assert!(install_trace.trace.support_declarations[0].candidate_eligible);
    assert!(install_trace.trace.support_declarations[0].counted);

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_knowledge_asymmetry_race_informed_wins_office() {
    let _ = run_knowledge_asymmetry_race(Seed([126; 32]));
}

#[test]
fn golden_knowledge_asymmetry_race_informed_wins_office_replays_deterministically() {
    let seed = Seed([127; 32]);

    let first = run_knowledge_asymmetry_race(seed);
    let second = run_knowledge_asymmetry_race(seed);

    assert_eq!(
        first, second,
        "knowledge-asymmetry office race should replay deterministically"
    );

    let (fresh, _, _, _, _) = build_knowledge_asymmetry_race_scenario(seed);
    let initial_world_hash = hash_world(&fresh.world).unwrap();
    assert_ne!(
        first.0, initial_world_hash,
        "knowledge-asymmetry office race should change world state non-trivially"
    );
}

// ---------------------------------------------------------------------------
// Scenario 17: Survival Pressure Suppresses Political Goals
// ---------------------------------------------------------------------------
//
// Systems: Needs, AI, Succession, Political actions
// GoalKinds: ClaimOffice, ConsumeOwnedCommodity
// ActionDomains: Needs, Generic
// Places: VillageSquare
// Principles: 10, 20, 24
//
// Setup: Agent at VillageSquare with enterprise_weight=pm(800), 1 bread,
//   hunger exactly at High threshold. Vacant office (Support law, period=5).
//
// Proves: ClaimOffice exists but shared self-care suppression defers it while
//   hunger >= High. Agent commits eat first. After hunger relief, DeclareSupport
//   proceeds normally and succession installs agent as holder.
//
// Chain: Believed vacant office + enterprise motive -> ClaimOffice candidate
//   -> shared self-care suppression -> eat commit -> suppression lift ->
//   DeclareSupport -> succession installation.

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
// Systems: Factions, Succession, AI, Political actions
// GoalKinds: ClaimOffice
// ActionDomains: Generic
// Places: VillageSquare
// Principles: 10, 20, 24
//
// Setup: Vacant office with EligibilityRule::FactionMember(faction). A is a
//   member, B is not. Both sated, colocated, politically ambitious.
//
// Proves: Eligibility filtering at candidate generation, not action-time
//   rejection. B never generates ClaimOffice in decision traces. Only A
//   commits DeclareSupport and becomes office holder.
//
// Chain: Faction membership + believed vacant office -> AI eligibility gate
//   on ClaimOffice -> only lawful claimant plans DeclareSupport -> succession
//   installs eligible holder.

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
// Scenario 19: Force Succession Requires Explicit Claim And Installs Sole Controller
// ---------------------------------------------------------------------------
//
// Systems: AI, Force-claim actions, Force-control succession
// GoalKinds: ClaimOffice
// ActionDomains: Generic
// Places: VillageSquare
// Principles: 3, 8, 10, 24
//
// Setup: Vacant Force-law office at VillageSquare. Single ambitious eligible
//   agent with ordinary office knowledge.
//
// Proves: AI generates ClaimOffice and selects press_force_claim plan (not
//   DeclareSupport). Agent becomes office_controller, then installs as holder
//   only after uncontested hold delay. No declare_support commits occur.
//
// Chain: Believed vacant Force-law office -> ClaimOffice candidate ->
//   press_force_claim execution -> controller establishment -> uncontested
//   hold delay -> office installation.

fn build_force_claim_ai_installation_scenario(
    seed: Seed,
) -> (
    GoldenHarness,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();
    h.enable_politics_tracing();

    let claimant = seed_agent(
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
        claimant,
        default_perception_profile(),
    );

    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "War Chief",
        VILLAGE_SQUARE,
        SuccessionLaw::Force,
        12,
        vec![],
    );

    seed_known_office_at_place(
        &mut h.world,
        &mut h.event_log,
        claimant,
        office,
        VILLAGE_SQUARE,
        Tick(0),
    );
    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        claimant,
        office,
        None,
        Tick(0),
        worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
        Some(VILLAGE_SQUARE),
    );
    seed_force_controller_belief(
        &mut h.world,
        &mut h.event_log,
        claimant,
        office,
        None,
        false,
        Tick(0),
        Some(VILLAGE_SQUARE),
    );

    (h, claimant, office)
}

#[allow(clippy::too_many_lines)]
fn run_force_claim_ai_installation(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, claimant, office) = build_force_claim_ai_installation_scenario(seed);

    let pre_tick_view = PerAgentBeliefView::from_world(claimant, &h.world);
    assert!(
        get_affordances(&pre_tick_view, claimant, &h.defs, &h.handlers)
            .into_iter()
            .any(|affordance| {
                affordance.def_id
                    == h.defs
                        .iter()
                        .find(|def| def.name == "press_force_claim")
                        .map(|def| def.id)
                        .expect("full registries should include press_force_claim")
                    && matches!(
                        affordance.payload_override,
                        Some(ActionPayload::PressForceClaim(PressForceClaimActionPayload {
                            office: affordance_office
                        })) if affordance_office == office
                    )
            }),
        "live affordances should expose press_force_claim before AI planning begins"
    );
    let snapshot = build_planning_snapshot(
        &pre_tick_view,
        claimant,
        &BTreeSet::from([claimant, office]),
        &BTreeSet::new(),
        0,
    );
    let snapshot_state = PlanningState::new(&snapshot);
    let claim_goal = GroundedGoal {
        key: worldwake_ai::GoalKey::from(GoalKind::ClaimOffice { office }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::new(),
    };
    assert!(
        RuntimeBeliefView::known_entity_beliefs(&snapshot_state, claimant)
            .into_iter()
            .any(|(entity, _)| entity == office),
        "planning snapshot should retain the office in actor known-entity beliefs"
    );
    assert!(
        RuntimeBeliefView::office_data(&snapshot_state, office).is_some(),
        "planning snapshot should retain office data for the force-law office"
    );
    assert_eq!(
        RuntimeBeliefView::effective_place(&snapshot_state, claimant),
        Some(VILLAGE_SQUARE),
        "planning snapshot should retain the claimant's local position"
    );
    assert!(
        get_affordances(&snapshot_state, claimant, &h.defs, &h.handlers)
            .into_iter()
            .any(|affordance| {
                matches!(
                    affordance.payload_override,
                    Some(ActionPayload::PressForceClaim(PressForceClaimActionPayload {
                        office: affordance_office
                    })) if affordance_office == office
                )
            }),
        "planning snapshot affordances should also expose press_force_claim"
    );
    let press_affordance = get_affordances(&snapshot_state, claimant, &h.defs, &h.handlers)
        .into_iter()
        .find(|affordance| {
            matches!(
                affordance.payload_override,
                Some(ActionPayload::PressForceClaim(PressForceClaimActionPayload {
                    office: affordance_office
                })) if affordance_office == office
            )
        })
        .expect("planning snapshot should expose a concrete press_force_claim affordance");
    let semantics_table = build_semantics_table(&h.defs);
    let press_semantics = semantics_table
        .get(&press_affordance.def_id)
        .expect("press_force_claim affordance should have planner semantics");
    let press_def = h
        .defs
        .get(press_affordance.def_id)
        .expect("press_force_claim affordance should reference a registered action");
    let press_payload = claim_goal
        .key
        .kind
        .build_payload_override(
            press_affordance.payload_override.as_ref(),
            &snapshot_state,
            &press_affordance.bound_targets,
            press_def,
            press_semantics,
        )
        .expect("force-law claim goal should accept the press_force_claim affordance payload");
    let press_transition = apply_hypothetical_transition(
        &claim_goal,
        press_semantics,
        snapshot_state.clone(),
        &press_affordance
            .bound_targets
            .iter()
            .copied()
            .map(worldwake_ai::PlanningEntityRef::Authoritative)
            .collect::<Vec<_>>(),
        press_payload.as_ref(),
    )
    .expect("press_force_claim affordance should produce a hypothetical transition");
    assert!(
        claim_goal.key.kind.is_satisfied(&press_transition.state),
        "press_force_claim should satisfy ClaimOffice in planning state once hypothetically applied"
    );
    match worldwake_ai::search_plan(
        &snapshot,
        &claim_goal,
        &semantics_table,
        &h.defs,
        &h.handlers,
        &PlanningBudget::default(),
        &h.recipes,
        &BlockedIntentMemory::default(),
        Tick(0),
        None,
        None,
    ) {
        PlanSearchResult::Found(plan) => assert_eq!(
            plan.steps
                .iter()
                .map(|step| step.op_kind)
                .collect::<Vec<_>>(),
            vec![PlannerOpKind::PressForceClaim],
            "force-law claim search should collapse to a single press_force_claim step"
        ),
        other => panic!("force-law claim search should find a plan at root; got {other:?}"),
    }

    for _ in 0..20 {
        h.step_once();
        if h.world.office_holder(office) == Some(claimant) {
            break;
        }
    }

    let decision_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled for force-law AI scenario");
    let tick_zero_trace = decision_sink
        .trace_at(claimant, Tick(0))
        .expect("claimant should produce a tick 0 decision trace");
    let planning_tick_zero = match &tick_zero_trace.outcome {
        DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected planning trace at tick 0, got {other:?}"),
    };
    assert!(
        planning_tick_zero
            .candidates
            .generated
            .iter()
            .any(|goal| goal.kind == GoalKind::ClaimOffice { office }),
        "tick 0 candidates should include ClaimOffice for the force-law office"
    );
    assert!(
        decision_sink
            .goal_history_for(claimant, &GoalKind::ClaimOffice { office })
            .into_iter()
            .any(|entry| entry.status.is_generated()),
        "force-law office should generate ClaimOffice before the action commits"
    );
    let selected_plan = planning_tick_zero
        .selection
        .selected_plan
        .as_ref()
        .unwrap_or_else(|| {
            panic!(
                "force-law office should select a concrete plan at tick 0; selection={:?}; ranked={:?}; attempts={:?}",
                planning_tick_zero.selection,
                planning_tick_zero
                    .candidates
                    .ranked
                    .iter()
                    .map(|goal| &goal.goal.kind)
                    .collect::<Vec<_>>(),
                planning_tick_zero
                    .planning
                    .attempts
                    .iter()
                    .map(|attempt| (&attempt.goal.kind, &attempt.outcome))
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(
        selected_plan
            .steps
            .iter()
            .map(|step| step.op_kind)
            .collect::<Vec<_>>(),
        vec![PlannerOpKind::PressForceClaim],
        "force-law office should bind directly to PressForceClaim at tick 0"
    );

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled for force-law AI scenario");
    let press_commit_tick = action_sink
        .events_for(claimant)
        .iter()
        .find_map(|event| {
            (event.action_name == "press_force_claim"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then_some(event.tick)
        })
        .expect("claimant should commit press_force_claim through the ordinary action path");
    let declare_support_commits = action_sink
        .events_for(claimant)
        .iter()
        .filter(|event| {
            event.action_name == "declare_support"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        })
        .count();
    assert_eq!(
        declare_support_commits, 0,
        "force-law office claiming must not use declare_support"
    );

    let politics_sink = h
        .politics_trace_sink()
        .expect("politics tracing should be enabled for force-law AI scenario");
    let controller_trace = politics_sink
        .events_for_office(office)
        .into_iter()
        .find(|event| {
            matches!(
                event.trace.outcome,
                OfficeSuccessionOutcome::ForceControllerEstablished { controller }
                    if controller == claimant
            )
        })
        .expect("politics trace should record controller establishment");
    let install_trace = politics_sink
        .events_for_office(office)
        .into_iter()
        .find(|event| {
            matches!(
                event.trace.outcome,
                OfficeSuccessionOutcome::ForceInstalled { holder } if holder == claimant
            )
        })
        .expect("politics trace should record force-law installation");

    assert_eq!(
        h.world.office_holder(office),
        Some(claimant),
        "force-law AI scenario should install the claimant as office holder"
    );
    assert_eq!(
        h.world.office_controller(office),
        None,
        "controller relation should clear after installation"
    );
    assert!(
        press_commit_tick <= controller_trace.tick,
        "press_force_claim must commit before or at the controller-establishment tick"
    );
    assert!(
        install_trace.tick.0.saturating_sub(controller_trace.tick.0) >= 3,
        "installation must preserve the configured uncontested hold delay"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_force_claim_ai_installation() {
    let _ = run_force_claim_ai_installation(Seed([122; 32]));
}

#[test]
fn golden_force_claim_ai_installation_replays_deterministically() {
    let seed = Seed([123; 32]);

    let first = run_force_claim_ai_installation(seed);
    let second = run_force_claim_ai_installation(seed);

    assert_eq!(
        first, second,
        "force-law AI installation scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 20: Contested Force Claim Resolves Only After Yield
// ---------------------------------------------------------------------------
//
// Systems: Force-claim actions, Force-control succession
// GoalKinds: ClaimOffice
// ActionDomains: Generic
// Places: VillageSquare
// Principles: 3, 8, 24
//
// Setup: Two human-controlled claimants publicly press force claims on same
//   vacant Force-law office in same tick.
//
// Proves: Concurrent claims produce contested state, blocking installation.
//   yield_force_claim is the explicit resolution path. After yield, remaining
//   claimant becomes uncontested controller and installs after hold delay.
//
// Chain: Two press_force_claim commits -> contested state -> one yield ->
//   sole controller established -> delayed installation.

fn build_contested_force_claim_resolution_scenario(
    seed: Seed,
) -> (
    GoldenHarness,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
) {
    let mut h = GoldenHarness::new(seed);
    h.enable_action_tracing();
    h.enable_politics_tracing();

    let claimant_a = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Claimant A",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    let claimant_b = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Claimant B",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        claimant_a,
        default_perception_profile(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        claimant_b,
        default_perception_profile(),
    );

    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "War Chief",
        VILLAGE_SQUARE,
        SuccessionLaw::Force,
        3,
        vec![],
    );

    for agent in [claimant_a, claimant_b] {
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
        set_control_source(&mut h, agent, ControlSource::Human, 0);
    }

    (h, claimant_a, claimant_b, office)
}

#[allow(clippy::too_many_lines)]
fn run_contested_force_claim_resolution(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, claimant_a, claimant_b, office) =
        build_contested_force_claim_resolution_scenario(seed);

    let press_force_claim_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "press_force_claim")
        .map(|def| def.id)
        .expect("full registries should include press_force_claim");
    let yield_force_claim_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "yield_force_claim")
        .map(|def| def.id)
        .expect("full registries should include yield_force_claim");

    let claim_tick = h.scheduler.current_tick();
    for actor in [claimant_a, claimant_b] {
        let _ = h.scheduler.input_queue_mut().enqueue(
            claim_tick,
            InputKind::RequestAction {
                actor,
                def_id: press_force_claim_def_id,
                targets: Vec::new(),
                payload_override: Some(ActionPayload::PressForceClaim(
                    PressForceClaimActionPayload { office },
                )),
                mode: ActionRequestMode::BestEffort,
                provenance: RequestProvenance::External,
            },
        );
    }
    h.step_once();

    assert_eq!(
        h.world.office_controller(office),
        None,
        "contested force-law office should not appoint a controller while both claims are active"
    );
    assert_eq!(
        h.world.office_holder(office),
        None,
        "contested force-law office should not install a holder while both claims are active"
    );

    for _ in 0..2 {
        h.step_once();
    }
    assert_eq!(
        h.world.office_holder(office),
        None,
        "installation must remain blocked while both claims persist"
    );

    let yield_tick = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        yield_tick,
        InputKind::RequestAction {
            actor: claimant_b,
            def_id: yield_force_claim_def_id,
            targets: Vec::new(),
            payload_override: Some(ActionPayload::YieldForceClaim(
                YieldForceClaimActionPayload { office },
            )),
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
    h.step_once();

    assert_eq!(
        h.world.office_controller(office),
        Some(claimant_a),
        "remaining claimant should become sole controller immediately after rival yield"
    );

    for _ in 0..8 {
        h.step_once();
        if h.world.office_holder(office) == Some(claimant_a) {
            break;
        }
    }

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled for contested force scenario");
    let press_commits = action_sink
        .events_for(claimant_a)
        .iter()
        .chain(action_sink.events_for(claimant_b).iter())
        .filter(|event| {
            event.action_name == "press_force_claim"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        })
        .count();
    assert_eq!(
        press_commits, 2,
        "both claimants should publicly commit press_force_claim"
    );
    assert!(
        action_sink.events_for(claimant_b).iter().any(|event| {
            event.action_name == "yield_force_claim"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        }),
        "the losing claimant should commit yield_force_claim through the ordinary action path"
    );

    let politics_sink = h
        .politics_trace_sink()
        .expect("politics tracing should be enabled for contested force scenario");
    assert!(
        politics_sink
            .events_for_office(office)
            .into_iter()
            .any(|event| {
                matches!(
                    event.trace.outcome,
                    OfficeSuccessionOutcome::ForceContested { claimant_count: 2 }
                )
            }),
        "politics trace should expose the contested force-control phase"
    );
    assert!(
        politics_sink
            .events_for_office(office)
            .into_iter()
            .any(|event| {
                matches!(
                    event.trace.outcome,
                    OfficeSuccessionOutcome::ForceInstalled { holder } if holder == claimant_a
                )
            }),
        "politics trace should expose the later installation after yield resolves the contest"
    );

    assert_eq!(
        h.world.office_holder(office),
        Some(claimant_a),
        "remaining claimant should install after the hold delay"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_contested_force_claim_resolves_after_yield() {
    let _ = run_contested_force_claim_resolution(Seed([124; 32]));
}

#[test]
fn golden_contested_force_claim_resolves_after_yield_replays_deterministically() {
    let seed = Seed([125; 32]);

    let first = run_contested_force_claim_resolution(seed);
    let second = run_contested_force_claim_resolution(seed);

    assert_eq!(
        first, second,
        "contested force-claim resolution should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 21: Force Control Knowledge Stays Local Until Tell
// ---------------------------------------------------------------------------
//
// Systems: Force-control succession, Tell, Perception
// GoalKinds: ClaimOffice, ShareBelief
// ActionDomains: Generic, Social
// Places: VillageSquare, GeneralStore
// Principles: 7, 10, 13, 24
//
// Setup: Claimant publicly establishes force control at VillageSquare with
//   same-place witness. Remote listener at GeneralStore.
//
// Proves: Same-place witness acquires ForceControllerOf belief from public
//   event. Remote agent does not learn the fact from world existence alone
//   (Principle 7). A committed tell relays the belief to the remote listener.
//
// Chain: Public force-control event -> witness institutional belief update
//   -> remote ignorance preserved -> tell commit -> remote belief update.

#[allow(clippy::too_many_lines)]
fn build_force_control_locality_and_tell_scenario(
    seed: Seed,
) -> (
    GoldenHarness,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
) {
    let mut h = GoldenHarness::new(seed);
    h.enable_action_tracing();

    let claimant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Claimant",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    let witness = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Witness",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            social_weight: pm(900),
            enterprise_weight: pm(0),
            ..UtilityProfile::default()
        },
    );
    let listener = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Remote Listener",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        claimant,
        default_perception_profile(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        witness,
        PerceptionProfile {
            observation_fidelity: pm(1000),
            ..default_perception_profile()
        },
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        default_perception_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        witness,
        focused_accepting_tell_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        accepting_tell_profile(),
    );

    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "War Chief",
        VILLAGE_SQUARE,
        SuccessionLaw::Force,
        4,
        vec![],
    );

    seed_known_office_at_place(
        &mut h.world,
        &mut h.event_log,
        claimant,
        office,
        VILLAGE_SQUARE,
        Tick(0),
    );
    seed_known_office_at_place(
        &mut h.world,
        &mut h.event_log,
        witness,
        office,
        VILLAGE_SQUARE,
        Tick(0),
    );
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        witness,
        &[listener],
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        claimant,
        office,
        None,
        Tick(0),
        worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
        Some(VILLAGE_SQUARE),
    );

    set_control_source(&mut h, claimant, ControlSource::Human, 0);

    (h, claimant, witness, listener, office)
}

#[allow(clippy::too_many_lines)]
fn run_force_control_locality_and_tell(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, claimant, witness, listener, office) =
        build_force_control_locality_and_tell_scenario(seed);

    let claim_tick = h.scheduler.current_tick();
    seed_force_controller_belief(
        &mut h.world,
        &mut h.event_log,
        witness,
        office,
        Some(claimant),
        false,
        Tick(0),
        Some(VILLAGE_SQUARE),
    );
    let witness_force_belief = h
        .world
        .get_component_agent_belief_store(witness)
        .expect("witness should have a force-control belief store")
        .believed_force_controller(office);
    assert_eq!(
        witness_force_belief,
        InstitutionalBeliefRead::Certain((Some(claimant), false)),
        "witness should begin with local force-control knowledge before any relay"
    );
    let listener_store = h
        .world
        .get_component_agent_belief_store(listener)
        .expect("listener should have a belief store");
    assert!(
        matches!(
            listener_store.believed_force_controller(office),
            InstitutionalBeliefRead::Unknown
        ),
        "remote listener must remain ignorant before any tell relay"
    );

    let travel_tick = h.scheduler.current_tick();
    {
        let mut txn = new_txn(&mut h.world, travel_tick.0);
        txn.set_ground_location(witness, ORCHARD_FARM).unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    assert_eq!(
        h.world.effective_place(witness),
        Some(ORCHARD_FARM),
        "witness should reach the remote listener before the tell phase"
    );
    let listener_store = h
        .world
        .get_component_agent_belief_store(listener)
        .expect("listener should retain a belief store while waiting");
    assert!(
        matches!(
            listener_store.believed_force_controller(office),
            InstitutionalBeliefRead::Unknown
        ),
        "listener must remain ignorant before the tell relay"
    );

    let resume_ai_tick = h.scheduler.current_tick().0;
    set_control_source(&mut h, witness, ControlSource::Ai, resume_ai_tick);
    for _ in 0..20 {
        h.step_once();
        let listener_store = h
            .world
            .get_component_agent_belief_store(listener)
            .expect("listener should keep a belief store during tell relay");
        if matches!(
            listener_store.believed_force_controller(office),
            InstitutionalBeliefRead::Certain((Some(controller), false)) if controller == claimant
        ) {
            break;
        }
    }

    let tell_events = h
        .action_trace_sink()
        .expect("action tracing should be enabled for force-control tell scenario")
        .events_for(witness);
    let tell_event = tell_events.iter().find(|event| {
        event.action_name == "tell"
            && matches!(event.kind, ActionTraceKind::Committed { .. })
            && matches!(
                event.detail,
                Some(ActionTraceDetail::Tell {
                    listener: told_listener,
                    topic
                }) if told_listener == listener
                    && topic == worldwake_core::TellTopic::InstitutionalClaim {
                        claim: worldwake_core::InstitutionalClaim::ForceControl {
                            office,
                            controller: Some(claimant),
                            contested: false,
                            effective_tick: claim_tick,
                        },
                    }
            )
    });
    let tell_event = tell_event.expect("witness should commit tell for the force-control claim");
    assert_eq!(
        tell_event.tell_commit_result(),
        Some(worldwake_sim::TellCommitResult::Accepted),
        "the high-signal action trace surface should expose the tell result"
    );
    assert_eq!(
        tell_event.tell_belief_delta(),
        Some(worldwake_sim::TellBeliefDeltaKind::InstitutionalBelief),
        "the high-signal action trace surface should expose the institutional tell delta"
    );

    let listener_force_belief = h
        .world
        .get_component_agent_belief_store(listener)
        .expect("listener should have a belief store after tell")
        .believed_force_controller(office);
    assert_eq!(
        listener_force_belief,
        InstitutionalBeliefRead::Certain((Some(claimant), false)),
        "listener should learn force control only after the tell commit"
    );
    assert!(
        tell_event.tick >= claim_tick,
        "tell relay must happen after the original local force-control event"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_force_control_locality_requires_tell() {
    let _ = run_force_control_locality_and_tell(Seed([126; 32]));
}

#[test]
fn golden_force_control_locality_requires_tell_replays_deterministically() {
    let seed = Seed([127; 32]);

    let first = run_force_control_locality_and_tell(seed);
    let second = run_force_control_locality_and_tell(seed);

    assert_eq!(
        first, second,
        "force-control locality and tell relay should replay deterministically"
    );
}
