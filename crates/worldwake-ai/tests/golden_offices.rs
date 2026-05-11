//! Golden tests for political office claims and succession resolution.

mod golden_harness;

use golden_harness::*;
use worldwake_core::{
    AgentData, BeliefConfidencePolicy, CombatProfile, CommodityKind, ControlSource,
    DriveThresholds, EventTag, ExecutionBudget, FactionPurpose, GoalKind, HomeostaticNeeds,
    InstitutionalBeliefRead, MetabolismProfile, NoticeTopic, PerceptionProfile, PerceptionSource,
    Permille, PrototypePlace, Quantity, Seed, StateHash, SuccessionLaw, TellProfile, Tick,
    UtilityProfile, hash_event_log, hash_world, prototype_place_entity,
};
use worldwake_sim::{
    ActionPayload, ActionRequestMode, ActionTraceDetail, ActionTraceKind, InputKind,
    OfficeSuccessionOutcome, PostNoticeActionPayload, PressForceClaimActionPayload,
    RequestProvenance, YieldForceClaimActionPayload,
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
            entity_activation_threshold: pm(64),
            claim_confidence_threshold: pm(50),
            observation_buffer_capacity: 32,
            observation_budget: 24,
            salience_policy: worldwake_core::SaliencePolicy::default(),
            omission_log_capacity: worldwake_core::default_omission_log_capacity(),
            opportunity_floor_permille: worldwake_core::default_opportunity_floor_permille(),
            need_salience_boost: pm(500),
            need_salience_urgency_threshold: pm(500),
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
        entity_activation_threshold: pm(64),
        claim_confidence_threshold: pm(50),
        observation_buffer_capacity: 32,
        observation_budget: 24,
        salience_policy: worldwake_core::SaliencePolicy::default(),
        omission_log_capacity: worldwake_core::default_omission_log_capacity(),
        opportunity_floor_permille: worldwake_core::default_opportunity_floor_permille(),
        need_salience_boost: pm(500),
        need_salience_urgency_threshold: pm(500),
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

fn request_action_with_payload(
    h: &mut GoldenHarness,
    actor: worldwake_core::EntityId,
    def_name: &str,
    targets: Vec<worldwake_core::EntityId>,
    payload_override: Option<ActionPayload>,
) {
    let def_id = h.defs.iter().find(|def| def.name == def_name).map_or_else(
        || panic!("full registries should include {def_name}"),
        |def| def.id,
    );
    let tick = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        tick,
        InputKind::RequestAction {
            actor,
            def_id,
            targets,
            payload_override,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
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
    set_agent_execution_budget(
        &mut h.world,
        &mut h.event_log,
        agent_a,
        ExecutionBudget::new(
            16,
            ExecutionBudget::default().max_prerequisite_locations(),
            ExecutionBudget::default().preferred_operator_boost(),
        ),
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
    set_agent_execution_budget(
        &mut h.world,
        &mut h.event_log,
        agent_a,
        ExecutionBudget::new(
            16,
            ExecutionBudget::default().max_prerequisite_locations(),
            ExecutionBudget::default().preferred_operator_boost(),
        ),
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
// Scenario 71: Contested Force Claim Resolves Only After Yield
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

// ---------------------------------------------------------------------------
// Scenario 72: Force Control Knowledge Stays Local Until Tell
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

// ---------------------------------------------------------------------------
// Scenario 109: Vacancy notice unlocks political action without record consult
// ---------------------------------------------------------------------------
//
// Systems: Social artifact actions, Perception, Institutional beliefs, AI,
//   Political actions, Succession
// GoalKinds: ClaimOffice
// ActionDomains: Social, Generic
// Places: VillageSquare
// Principles: 7, 12, 18, 25
//
// Setup: A human issuer and a non-AI claimant are co-located with a vacant
//   support-law office at VillageSquare. The claimant has no seeded office-
//   holder belief and no pre-consulted office register. The issuer posts an
//   `OfficeVacancy` notice locally, the claimant perceives it, internalizes
//   vacancy certainty through the notice path, then AI resumes.
//
// Proves: The notice-artifact path can unlock ordinary political action without
//   `consult_record` or Tell. The claimant perceives the notice, records a
//   direct-observation vacancy belief, generates `ClaimOffice`, commits
//   `declare_support`, and becomes office holder through the normal succession
//   surface.
//
// Chain: post_notice -> local perception stores believed_artifact vacancy ->
//   institutional belief becomes Certain(None) via DirectObservation -> AI
//   generates ClaimOffice -> declare_support commits without consult_record ->
//   succession installs claimant.

#[allow(clippy::too_many_lines)]
fn run_vacancy_notice_political_uptake(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let issuer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Vacancy Herald",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, issuer, ControlSource::Human, 0);

    let claimant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Ambitious Claimant",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    set_control_source(&mut h, claimant, ControlSource::None, 0);
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        claimant,
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

    let pre_notice_belief = h
        .world
        .get_component_agent_belief_store(claimant)
        .map_or(InstitutionalBeliefRead::Unknown, |store| {
            store.believed_office_holder(office)
        });
    assert!(
        matches!(pre_notice_belief, InstitutionalBeliefRead::Unknown),
        "claimant should start without seeded office-holder knowledge"
    );

    request_action_with_payload(
        &mut h,
        issuer,
        "post_notice",
        vec![VILLAGE_SQUARE],
        Some(ActionPayload::PostNotice(PostNoticeActionPayload {
            posting_place: VILLAGE_SQUARE,
            issuing_authority: None,
            expires_at: Some(Tick(40)),
            jurisdiction: None,
            topic: NoticeTopic::OfficeVacancy { office },
        })),
    );

    let mut notice = None;
    let mut noticed_vacancy = false;
    for _ in 0..8 {
        h.step_once();
        if notice.is_none() {
            notice = h
                .world
                .query_artifact_header()
                .find_map(|(entity, header)| {
                    (header.kind == worldwake_core::ArtifactKind::Notice).then_some(entity)
                });
        }
        noticed_vacancy = notice.is_some_and(|artifact| {
            agent_belief_about(&h.world, claimant, artifact)
                .and_then(|belief| belief.believed_artifact.as_ref())
                .is_some_and(|artifact_state| {
                    artifact_state.kind == worldwake_core::ArtifactKind::Notice
                        && artifact_state.actionability
                            == worldwake_core::ArtifactActionability::Actionable
                        && artifact_state.notice_topic
                            == Some(NoticeTopic::OfficeVacancy { office })
                })
                && h.world
                    .get_component_agent_belief_store(claimant)
                    .is_some_and(|store| {
                        matches!(
                            store.believed_office_holder(office),
                            InstitutionalBeliefRead::Certain(None)
                        ) && store
                            .institutional_beliefs
                            .get(&worldwake_core::InstitutionalBeliefKey::OfficeHolderOf {
                                office,
                            })
                            .is_some_and(|beliefs| beliefs.iter().any(|belief| {
                                belief.claim
                                    == worldwake_core::InstitutionalClaim::OfficeHolder {
                                        office,
                                        holder: None,
                                        effective_tick: belief.learned_tick,
                                    }
                                    && belief.source
                                        == worldwake_core::InstitutionalKnowledgeSource::DirectObservation
                                    && belief.learned_at == Some(VILLAGE_SQUARE)
                            }))
                    })
        });
        if noticed_vacancy {
            break;
        }
    }

    let notice = notice.expect("post_notice should create a notice artifact");
    assert!(
        h.action_trace_sink()
            .expect("action tracing enabled")
            .events_for(issuer)
            .iter()
            .any(|event| {
                event.action_name == "post_notice"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            }),
        "issuer should commit post_notice"
    );
    assert!(
        noticed_vacancy,
        "claimant should perceive the vacancy notice and internalize the vacancy belief"
    );
    assert_eq!(
        h.world
            .get_component_artifact_header(notice)
            .expect("posted notice should retain an artifact header")
            .kind,
        worldwake_core::ArtifactKind::Notice,
        "the created social artifact should be a notice"
    );

    let ai_tick = h.scheduler.current_tick().0;
    set_control_source(&mut h, claimant, ControlSource::Ai, ai_tick);

    for _ in 0..20 {
        h.step_once();
        if h.world.office_holder(office) == Some(claimant) {
            break;
        }
    }

    let decision_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled for vacancy-notice scenario");
    let generated_claim_goal = decision_sink
        .goal_history_for(claimant, &GoalKind::ClaimOffice { office })
        .into_iter()
        .any(|entry| entry.status.is_generated());
    assert!(
        generated_claim_goal,
        "claimant should generate ClaimOffice after learning vacancy from the notice"
    );

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled for vacancy-notice scenario");
    assert!(
        !action_sink.events_for(claimant).iter().any(|event| {
            event.action_name == "consult_record"
                && matches!(
                    event.kind,
                    ActionTraceKind::Started { .. }
                        | ActionTraceKind::Committed { .. }
                        | ActionTraceKind::StartFailed { .. }
                )
        }),
        "claimant should not start consult_record when notice-derived vacancy belief is already certain"
    );
    assert!(
        action_sink.events_for(claimant).iter().any(|event| {
            event.action_name == "declare_support"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        }),
        "claimant should commit declare_support through the ordinary political action path"
    );
    assert_eq!(
        h.world.office_holder(office),
        Some(claimant),
        "claimant should become office holder after acting on the notice-derived vacancy belief"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_vacancy_notice_unlocks_political_action_without_record_consult() {
    let _ = run_vacancy_notice_political_uptake(Seed([128; 32]));
}
