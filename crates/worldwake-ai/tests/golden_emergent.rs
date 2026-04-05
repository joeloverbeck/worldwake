//! Golden tests for cross-system emergent behavior involving care.
//!
//! These tests prove that care (S07) interacts with other systems —
//! metabolism, combat, loot, travel, transport — to produce emergent
//! multi-system chains.  No single system orchestrates these outcomes;
//! they emerge from concrete state + utility-driven AI ranking.

mod golden_harness;

use golden_harness::*;
use worldwake_ai::{DecisionOutcome, PoliticalCandidateOmissionReason, SelectedPlanSource};
use worldwake_core::{
    AgentBeliefStore, AgentData, BeliefConfidencePolicy, BelievedEntityState,
    BelievedInstitutionalClaim, CombatProfile, CommodityKind, CommunicationProfile, ComponentKind,
    ComponentValue, ControlSource, DeadAt, DeprivationExposure, DeprivationKind, DriveThresholds,
    EventTag, EventView, GoalKey, GoalKind, HomeostaticNeeds, InstitutionalBeliefKey,
    InstitutionalBeliefRead, InstitutionalClaim, InstitutionalKnowledgeSource,
    IntentionDispositionProfile, JusticeDispositionProfile, KnownRecipes, MetabolismProfile,
    PerceptionProfile, PerceptionSource, PrototypePlace, Quantity, RecordData, RecordKind,
    RelationValue, ResourceSource, RightKind, Seed, StateHash, SuccessionLaw, TellProfile,
    TellTopic, TheftDispositionProfile, TheftFacts, ThresholdBand, Tick, UtilityProfile,
    ViolationDispositionProfile, WorkstationTag, hash_event_log, hash_world,
    prototype_place_entity, total_live_lot_quantity, verify_live_lot_conservation,
};
use worldwake_sim::{
    ActionPayload, ActionRequestMode, ActionStartFailureReason, ActionTraceDetail, ActionTraceKind,
    AutonomousController, AutonomousControllerContext, DeclareSupportActionPayload, InputKind,
    OfficeAvailabilityPhase, OfficeSuccessionOutcome, PressForceClaimActionPayload,
    RequestProvenance, RequestResolutionOutcome, ResolvedRequestTrace, TickStepServices, step_tick,
};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn default_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        memory_capacity: 64,
        memory_retention_ticks: 240,
        observation_fidelity: pm(875),
        confidence_policy: BeliefConfidencePolicy::default(),
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
    }
}

fn blind_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        memory_capacity: 16,
        memory_retention_ticks: 240,
        observation_fidelity: pm(0),
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

fn broad_accepting_tell_profile() -> TellProfile {
    TellProfile {
        max_tell_candidates: 6,
        ..accepting_tell_profile()
    }
}

fn focused_accepting_tell_profile() -> TellProfile {
    TellProfile {
        max_tell_candidates: 1,
        ..accepting_tell_profile()
    }
}

fn accepting_communication_profile() -> CommunicationProfile {
    CommunicationProfile {
        alarm_acceptance: pm(1000),
        testimony_acceptance: pm(1000),
        gossip_acceptance: pm(1000),
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

fn set_violation_profile(
    h: &mut GoldenHarness,
    agent: worldwake_core::EntityId,
    profile: ViolationDispositionProfile,
    tick: u64,
) {
    let mut txn = new_txn(&mut h.world, tick);
    txn.set_component_violation_disposition_profile(agent, profile)
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn set_theft_profile(
    h: &mut GoldenHarness,
    agent: worldwake_core::EntityId,
    profile: TheftDispositionProfile,
    tick: u64,
) {
    let mut txn = new_txn(&mut h.world, tick);
    txn.set_component_theft_disposition_profile(agent, profile)
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn set_justice_profile(
    h: &mut GoldenHarness,
    agent: worldwake_core::EntityId,
    profile: JusticeDispositionProfile,
    tick: u64,
) {
    let mut txn = new_txn(&mut h.world, tick);
    txn.set_component_justice_disposition_profile(agent, profile)
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn social_weighted_utility(weight: u16) -> UtilityProfile {
    UtilityProfile {
        social_weight: pm(weight),
        enterprise_weight: pm(0),
        ..UtilityProfile::default()
    }
}

/// Combat profile with zero natural recovery — wounds only decrease through
/// medicine. Prevents `TargetHasNoWounds` race between natural recovery and
/// the heal action.
fn lethal_combat_attacker_profile() -> CombatProfile {
    CombatProfile::new(
        pm(1000), // wound_capacity
        pm(700),  // incapacitation_threshold
        pm(950),  // attack_skill
        pm(750),  // guard_skill
        pm(150),  // defend_bonus
        pm(0),    // natural_clot_resistance
        pm(0),    // natural_recovery_rate
        pm(700),  // unarmed_wound_severity
        pm(300),  // unarmed_bleed_rate
        nz(2),    // unarmed_attack_ticks
        nz(10),   // defend_stance_ticks
    )
}

fn fragile_office_holder_profile() -> CombatProfile {
    CombatProfile::new(
        pm(350), // wound_capacity
        pm(150), // incapacitation_threshold
        pm(150), // attack_skill
        pm(100), // guard_skill
        pm(0),   // defend_bonus
        pm(0),   // natural_clot_resistance
        pm(0),   // natural_recovery_rate
        pm(80),  // unarmed_wound_severity
        pm(50),  // unarmed_bleed_rate
        nz(6),   // unarmed_attack_ticks
        nz(10),  // defend_stance_ticks
    )
}

// ===========================================================================
// Suite 1: wound_vs_hunger_priority_resolution
//
// Proves: cross-domain priority ranking resolves competing needs (care vs
// metabolism) via concrete utility weights — not hardcoded priority tiers.
// Foundation: Principle 3 (concrete state), Principle 20 (agent diversity).
// Cross-systems: Needs metabolism + Care + AI ranking.
// ===========================================================================

fn run_wound_vs_hunger(
    seed: Seed,
    pain_weight: u16,
    hunger_weight: u16,
) -> (StateHash, StateHash, String) {
    let mut h = GoldenHarness::new(seed);

    let utility = UtilityProfile {
        pain_weight: pm(pain_weight),
        hunger_weight: pm(hunger_weight),
        ..UtilityProfile::default()
    };

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Decider",
        VILLAGE_SQUARE,
        // High hunger — pressing enough to compete with wound care.
        HomeostaticNeeds::new(pm(700), pm(0), pm(100), pm(0), pm(0)),
        MetabolismProfile::default(),
        utility,
    );

    // Override combat profile: zero natural recovery so wounds only decrease
    // through medicine.  This prevents the TargetHasNoWounds abort race.
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_combat_profile(agent, no_recovery_combat_profile())
            .unwrap();
        txn.set_component_wound_list(agent, stable_wound_list(400))
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Give agent both food and medicine.
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent,
        VILLAGE_SQUARE,
        CommodityKind::Apple,
        Quantity(2),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent,
        VILLAGE_SQUARE,
        CommodityKind::Medicine,
        Quantity(1),
    );

    // Seed self-knowledge (DirectObservation).
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    let initial_apple_total = total_live_lot_quantity(&h.world, CommodityKind::Apple);
    let initial_medicine_total = total_live_lot_quantity(&h.world, CommodityKind::Medicine);
    let initial_wound_load = h.agent_wound_load(agent);
    let initial_hunger = h.agent_hunger(agent);

    // Track which action happens first.
    let mut first_action: Option<String> = None;
    let mut wound_decreased = false;
    let mut hunger_decreased = false;

    for _ in 0..80 {
        h.step_once();

        assert!(!h.agent_is_dead(agent), "agent must stay alive");

        // Conservation checks.
        let apple_total = total_live_lot_quantity(&h.world, CommodityKind::Apple);
        let medicine_total = total_live_lot_quantity(&h.world, CommodityKind::Medicine);
        assert!(
            apple_total <= initial_apple_total,
            "apple lots must not increase"
        );
        assert!(
            medicine_total <= initial_medicine_total,
            "medicine lots must not increase"
        );

        // Track first action via state deltas (1-tick actions like eat/heal
        // may not be visible as active actions between ticks).
        if first_action.is_none() {
            if h.agent_wound_load(agent) < initial_wound_load {
                first_action = Some("heal".to_string());
            } else if h.agent_hunger(agent) < initial_hunger {
                first_action = Some("eat".to_string());
            }
        }

        wound_decreased |= h.agent_wound_load(agent) < initial_wound_load;
        hunger_decreased |= h.agent_hunger(agent) < initial_hunger;

        if wound_decreased && hunger_decreased {
            break;
        }
    }

    assert!(
        wound_decreased,
        "agent wound load should decrease (self-care)"
    );
    assert!(hunger_decreased, "agent hunger should decrease (eating)");

    let first = first_action.expect("agent should take at least one action");

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
        first,
    )
}

#[test]
fn golden_wound_vs_hunger_pain_first() {
    // pain_weight=800 >> hunger_weight=400 → agent heals before eating.
    let (_, _, first_action) = run_wound_vs_hunger(Seed([30; 32]), 800, 400);
    assert_eq!(
        first_action, "heal",
        "with high pain_weight, agent should heal before eating"
    );
}

#[test]
fn golden_wound_vs_hunger_hunger_first() {
    // pain_weight=300 << hunger_weight=800 → agent eats before healing.
    let (_, _, first_action) = run_wound_vs_hunger(Seed([31; 32]), 300, 800);
    assert_eq!(
        first_action, "eat",
        "with high hunger_weight, agent should eat before healing"
    );
}

#[test]
fn golden_wound_vs_hunger_replays_deterministically() {
    let first = run_wound_vs_hunger(Seed([32; 32]), 800, 400);
    let second = run_wound_vs_hunger(Seed([32; 32]), 800, 400);
    assert_eq!(
        (first.0, first.1),
        (second.0, second.1),
        "wound-vs-hunger scenario should replay deterministically"
    );
}

// ===========================================================================
// Suite 1b: deprivation_wound_worsening
//
// Proves: repeated deprivation threshold fires through the live needs system
// worsen a single authoritative wound instead of duplicating it.
// Foundation: Principle 4, Principle 9, Principle 24.
// Cross-systems: Needs + Wounds + deterministic replay.
// ===========================================================================

fn deprivation_worsening_thresholds() -> DriveThresholds {
    DriveThresholds {
        hunger: ThresholdBand::new(pm(100), pm(200), pm(300), pm(400)).unwrap(),
        ..DriveThresholds::default()
    }
}

fn deprivation_worsening_metabolism() -> MetabolismProfile {
    MetabolismProfile::new(
        pm(0),
        pm(0),
        pm(0),
        pm(0),
        pm(0),
        pm(20),
        nz(5),
        nz(1000),
        nz(1000),
        nz(1000),
        nz(8),
        nz(12),
        pm(0),
        pm(0),
        pm(0),
        pm(0),
    )
}

fn deprivation_worsening_combat_profile() -> CombatProfile {
    CombatProfile::new(
        pm(1000),
        pm(700),
        pm(500),
        pm(500),
        pm(80),
        pm(0),
        pm(0),
        pm(120),
        pm(35),
        nz(6),
        nz(10),
    )
}

fn run_deprivation_wound_worsening(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    let thresholds = deprivation_worsening_thresholds();

    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Starving Agent",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(420), pm(0), pm(0), pm(0), pm(0)),
        deprivation_worsening_metabolism(),
        UtilityProfile::default(),
        KnownRecipes::default(),
    );

    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_deprivation_exposure(
            agent,
            DeprivationExposure {
                hunger_critical_ticks: 4,
                ..DeprivationExposure::default()
            },
        )
        .unwrap();
        txn.set_component_drive_thresholds(agent, thresholds)
            .unwrap();
        txn.set_component_combat_profile(agent, deprivation_worsening_combat_profile())
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    let wound_capacity = u32::from(
        h.world
            .get_component_combat_profile(agent)
            .expect("agent should have a combat profile")
            .wound_capacity
            .value(),
    );

    let mut first_fire = None;
    let mut saw_second_fire = false;

    for _ in 0..8 {
        h.step_once();

        let wounds = h
            .world
            .get_component_wound_list(agent)
            .expect("agent should keep a wound list");
        assert!(
            wounds.wounds.len() <= 1,
            "repeated deprivation fires must consolidate into at most one wound"
        );
        assert!(
            wounds.wound_load() < wound_capacity,
            "agent should survive long enough to prove worsening instead of death"
        );
        assert!(
            !h.agent_is_dead(agent),
            "agent should remain alive throughout the consolidation scenario"
        );

        let Some(wound) = wounds.find_deprivation_wound(DeprivationKind::Starvation) else {
            continue;
        };

        if let Some((first_id, first_severity, first_inflicted_at)) = first_fire {
            assert_eq!(
                wound.id, first_id,
                "starvation worsening must preserve wound identity"
            );
            if wound.inflicted_at > first_inflicted_at {
                assert!(
                    wound.severity > first_severity,
                    "second starvation fire must increase severity on the existing wound"
                );
                saw_second_fire = true;
                break;
            }
        } else {
            first_fire = Some((wound.id, wound.severity, wound.inflicted_at));
        }
    }

    assert!(
        first_fire.is_some(),
        "scenario should create the initial starvation wound on the first threshold fire"
    );
    assert!(
        saw_second_fire,
        "scenario should reach a second starvation threshold fire that worsens the same wound"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_deprivation_wound_worsening_consolidates_not_duplicates() {
    let _ = run_deprivation_wound_worsening(Seed([36; 32]));
}

#[test]
fn golden_deprivation_wound_worsening_consolidates_not_duplicates_replays_deterministically() {
    let first = run_deprivation_wound_worsening(Seed([36; 32]));
    let second = run_deprivation_wound_worsening(Seed([36; 32]));
    assert_eq!(
        first, second,
        "deprivation wound worsening scenario should replay deterministically"
    );
}

// Scenario 44: Wounded Politician Enterprise vs Care Priority
// ---------------------------------------------------------------------------
//
// Systems: Care, Politics, AI, Succession
// GoalKinds: TreatWounds, ClaimOffice
// ActionDomains: Care, Social
// Places: VillageSquare
// Principles: 3, 20, 24
//
// Proves care and political ambition follow the shared ranking pipeline.
// Medium pain can outrank office ambition, while low pain can leave the office
// claim path ahead, all without office-specific priority exceptions.

#[allow(clippy::too_many_lines)]
fn run_wounded_politician(
    seed: Seed,
    wound_severity: u16,
    pain_weight: u16,
    enterprise_weight: u16,
) -> (StateHash, StateHash, Tick, Tick) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();
    h.enable_politics_tracing();
    h.enable_request_resolution_tracing();

    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Wounded Politician",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            pain_weight: pm(pain_weight),
            enterprise_weight: pm(enterprise_weight),
            ..UtilityProfile::default()
        },
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        agent,
        default_perception_profile(),
    );

    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_combat_profile(agent, no_recovery_combat_profile())
            .unwrap();
        txn.set_component_wound_list(agent, stable_wound_list(wound_severity))
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent,
        VILLAGE_SQUARE,
        CommodityKind::Medicine,
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

    let initial_medicine_total = total_live_lot_quantity(&h.world, CommodityKind::Medicine);
    let initial_wound_load = h.agent_wound_load(agent);
    let mut heal_commit_tick = None;
    let mut declare_support_commit_tick = None;

    for _ in 0..40 {
        h.step_once();

        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled for wounded-politician scenario");
        if heal_commit_tick.is_none() {
            heal_commit_tick = action_sink.events_for(agent).iter().find_map(|event| {
                (event.action_name == "heal"
                    && matches!(event.kind, ActionTraceKind::Committed { .. }))
                .then_some(event.tick)
            });
        }
        if declare_support_commit_tick.is_none() {
            declare_support_commit_tick = action_sink.events_for(agent).iter().find_map(|event| {
                (event.action_name == "declare_support"
                    && matches!(event.kind, ActionTraceKind::Committed { .. }))
                .then_some(event.tick)
            });
        }

        let medicine_total = total_live_lot_quantity(&h.world, CommodityKind::Medicine);
        assert!(
            medicine_total <= initial_medicine_total,
            "medicine lots must not increase"
        );

        if heal_commit_tick.is_some()
            && declare_support_commit_tick.is_some()
            && h.world.office_holder(office) == Some(agent)
            && h.agent_wound_load(agent) < initial_wound_load
        {
            break;
        }
    }

    let decision_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled for wounded-politician scenario");
    let generated_self_treat = decision_sink
        .goal_history_for(agent, &GoalKind::TreatWounds { patient: agent })
        .into_iter()
        .any(|entry| entry.status.is_generated());
    let generated_claim_office = decision_sink
        .goal_history_for(agent, &GoalKind::ClaimOffice { office })
        .into_iter()
        .any(|entry| entry.status.is_generated());

    assert!(
        generated_self_treat,
        "agent should generate TreatWounds for self in the wounded-politician scenario"
    );
    assert!(
        generated_claim_office,
        "agent should generate ClaimOffice while the office remains visibly vacant"
    );

    let heal_commit_tick =
        heal_commit_tick.expect("agent should commit heal in the wounded-politician scenario");
    let declare_support_commit_tick = declare_support_commit_tick
        .expect("agent should commit declare_support in the wounded-politician scenario");

    assert_eq!(
        h.world.office_holder(office),
        Some(agent),
        "agent should eventually become office holder after declaring support"
    );
    assert!(
        h.agent_wound_load(agent) < initial_wound_load,
        "agent wound load should decrease through lawful care"
    );
    assert_eq!(
        h.agent_commodity_qty(agent, CommodityKind::Medicine),
        Quantity(0),
        "owned medicine should be consumed by the heal action"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
        heal_commit_tick,
        declare_support_commit_tick,
    )
}

#[test]
fn golden_wounded_politician_pain_first() {
    let (_, _, heal_commit_tick, declare_support_commit_tick) =
        run_wounded_politician(Seed([33; 32]), 400, 800, 400);
    assert!(
        heal_commit_tick < declare_support_commit_tick,
        "medium pain should drive heal before declare_support"
    );
}

#[test]
fn golden_wounded_politician_enterprise_first() {
    let (_, _, heal_commit_tick, declare_support_commit_tick) =
        run_wounded_politician(Seed([34; 32]), 200, 300, 800);
    assert!(
        declare_support_commit_tick < heal_commit_tick,
        "low pain should leave declare_support ahead of heal"
    );
}

#[test]
fn golden_wounded_politician_replays_deterministically() {
    let first = run_wounded_politician(Seed([35; 32]), 400, 800, 400);
    let second = run_wounded_politician(Seed([35; 32]), 400, 800, 400);
    assert_eq!(
        (first.0, first.1),
        (second.0, second.1),
        "wounded-politician scenario should replay deterministically"
    );
}

// ===========================================================================
// Suite 4: care_weight_divergence_under_observation
//
// Proves: per-agent care_weight (S07) produces divergent behavior — two
// agents with identical perception of the same wounded patient make different
// autonomous decisions based on utility profile.
// Foundation: Principle 20 (agent diversity), Principle 3 (concrete weights),
// Principle 7 (DirectObservation).
// Cross-systems: Care + Needs + AI ranking + Perception.
// ===========================================================================

#[allow(clippy::too_many_lines)]
fn run_care_weight_divergence(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);

    // Patient at Village Square — wounded, no natural recovery.
    let patient = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Patient",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(100), pm(0), pm(100), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_combat_profile(patient, no_recovery_combat_profile())
            .unwrap();
        txn.set_component_wound_list(patient, stable_wound_list(500))
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Agent A — altruistic: high care_weight, low hunger.
    let altruist = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Altruist",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(100), pm(0), pm(100), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            care_weight: pm(800),
            ..UtilityProfile::default()
        },
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        altruist,
        VILLAGE_SQUARE,
        CommodityKind::Medicine,
        Quantity(1),
    );

    // Agent B — self-interested: low care_weight, moderately hungry.
    let selfish = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Selfish",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(500), pm(0), pm(100), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            care_weight: pm(100),
            hunger_weight: pm(800),
            ..UtilityProfile::default()
        },
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        selfish,
        VILLAGE_SQUARE,
        CommodityKind::Medicine,
        Quantity(1),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        selfish,
        VILLAGE_SQUARE,
        CommodityKind::Apple,
        Quantity(2),
    );

    // Seed beliefs — both agents observe patient + each other.
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        altruist,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        selfish,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    let initial_patient_wound_load = h.agent_wound_load(patient);
    let initial_selfish_hunger = h.agent_hunger(selfish);

    let mut altruist_first_action: Option<String> = None;
    let mut selfish_first_action: Option<String> = None;
    let mut patient_healed = false;
    let mut selfish_ate = false;

    let initial_medicine_total = total_live_lot_quantity(&h.world, CommodityKind::Medicine);

    for _ in 0..80 {
        h.step_once();

        assert!(!h.agent_is_dead(patient), "patient must stay alive");
        assert!(!h.agent_is_dead(altruist), "altruist must stay alive");
        assert!(!h.agent_is_dead(selfish), "selfish must stay alive");

        let medicine_total = total_live_lot_quantity(&h.world, CommodityKind::Medicine);
        assert!(
            medicine_total <= initial_medicine_total,
            "medicine lots must not increase"
        );

        // Detect altruist's first meaningful action via state delta.
        if altruist_first_action.is_none() {
            let altruist_medicine = h.agent_commodity_qty(altruist, CommodityKind::Medicine);
            if altruist_medicine < Quantity(1) {
                altruist_first_action = Some("heal".to_string());
            }
        }

        // Detect selfish's first meaningful action via state delta.
        if selfish_first_action.is_none() {
            let selfish_hunger_now = h.agent_hunger(selfish);
            let selfish_medicine = h.agent_commodity_qty(selfish, CommodityKind::Medicine);
            if selfish_hunger_now < initial_selfish_hunger {
                selfish_first_action = Some("eat".to_string());
            } else if selfish_medicine < Quantity(1) {
                selfish_first_action = Some("heal".to_string());
            }
        }

        patient_healed |= h.agent_wound_load(patient) < initial_patient_wound_load;
        selfish_ate |= h.agent_hunger(selfish) < initial_selfish_hunger;

        if patient_healed && selfish_ate {
            break;
        }
    }

    assert!(
        patient_healed,
        "patient wound load should decrease (healed by altruist)"
    );
    assert!(
        selfish_ate,
        "selfish agent hunger should decrease (ate food)"
    );

    assert_eq!(
        altruist_first_action.as_deref(),
        Some("heal"),
        "altruist (care_weight=800) should heal the patient first"
    );
    assert_eq!(
        selfish_first_action.as_deref(),
        Some("eat"),
        "selfish agent (care_weight=100, hunger_weight=800) should eat first"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_care_weight_divergence_under_observation() {
    let _ = run_care_weight_divergence(Seed([33; 32]));
}

#[test]
fn golden_care_weight_divergence_replays_deterministically() {
    let first = run_care_weight_divergence(Seed([34; 32]));
    let second = run_care_weight_divergence(Seed([34; 32]));
    assert_eq!(
        first, second,
        "care weight divergence scenario should replay deterministically"
    );
}

// ===========================================================================
// Suite 2: care_travel_to_remote_patient
//
// Proves: GOAP planner decomposes a care plan that requires travel — the
// healer has medicine but the patient is at a different location.  Travel
// time naturally dampens healing throughput (Principle 10).
// Foundation: Principle 1 (causal chain), Principle 7 (belief-seeded),
// Principle 10 (travel time as natural dampener).
// Cross-systems: Care + Travel + AI planning.
// ===========================================================================

#[allow(clippy::too_many_lines)]
fn run_care_travel_to_remote_patient(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);

    // Patient at Orchard Farm — wounded, no natural recovery.
    let patient = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Patient",
        ORCHARD_FARM,
        HomeostaticNeeds::new(pm(100), pm(0), pm(100), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_combat_profile(patient, no_recovery_combat_profile())
            .unwrap();
        txn.set_component_wound_list(patient, stable_wound_list(500))
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Healer at Village Square — has medicine, high care_weight.
    let healer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Healer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(100), pm(0), pm(100), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            care_weight: pm(800),
            ..UtilityProfile::default()
        },
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        healer,
        VILLAGE_SQUARE,
        CommodityKind::Medicine,
        Quantity(1),
    );

    // Healer needs perception to observe entities at destination.
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        healer,
        default_perception_profile(),
    );

    // Seed healer's beliefs: knows patient is wounded at Orchard Farm
    // (DirectObservation — artificially seeded for remote patient).
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        healer,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        healer,
        &[patient],
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    let initial_medicine_total = total_live_lot_quantity(&h.world, CommodityKind::Medicine);
    let initial_wound_load = h.agent_wound_load(patient);

    let mut healer_traveled = false;
    let mut medicine_consumed = false;
    let mut patient_healed = false;
    let mut heal_tick: Option<u32> = None;

    for tick in 0..120 {
        h.step_once();

        assert!(!h.agent_is_dead(healer), "healer must stay alive");
        assert!(!h.agent_is_dead(patient), "patient must stay alive");

        // Conservation.
        let medicine_total = total_live_lot_quantity(&h.world, CommodityKind::Medicine);
        assert!(
            medicine_total <= initial_medicine_total,
            "medicine lots must not increase"
        );

        // Track healer leaving Village Square.
        if !healer_traveled && h.world.effective_place(healer) != Some(VILLAGE_SQUARE) {
            healer_traveled = true;
        }

        // Track medicine consumption.
        if h.agent_commodity_qty(healer, CommodityKind::Medicine) == Quantity(0) {
            medicine_consumed = true;
        }

        // Track patient healing.
        if h.agent_wound_load(patient) < initial_wound_load && heal_tick.is_none() {
            patient_healed = true;
            heal_tick = Some(tick);
        }

        if healer_traveled && medicine_consumed && patient_healed {
            break;
        }
    }

    assert!(
        healer_traveled,
        "healer should travel from Village Square to reach the wounded patient"
    );
    assert!(
        medicine_consumed,
        "healer should consume medicine to heal the patient"
    );
    assert!(
        patient_healed,
        "patient wound load should decrease after healer arrives and heals"
    );

    // Travel time dampening: healing should not be instant.
    let heal_t = heal_tick.expect("heal_tick should be set");
    assert!(
        heal_t > 3,
        "healing should take multiple ticks due to travel time (actual: {heal_t})"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_care_travel_to_remote_patient() {
    let _ = run_care_travel_to_remote_patient(Seed([35; 32]));
}

#[test]
fn golden_care_travel_to_remote_patient_replays_deterministically() {
    let first = run_care_travel_to_remote_patient(Seed([36; 32]));
    let second = run_care_travel_to_remote_patient(Seed([36; 32]));
    assert_eq!(
        first, second,
        "care-travel-to-remote-patient should replay deterministically"
    );
}

// ===========================================================================
// Suite 3: loot_corpse_self_care_chain
//
// Proves: multi-system emergence — a wounded agent observes a corpse carrying
// medicine, loots it (transport), then self-heals (care).  The loot→care
// chain emerges from concrete state without any orchestrator.
// Foundation: Principle 1 (maximal emergence), Principle 3 (concrete wounds,
// concrete medicine), Principle 24 (systems interact only through state).
// Cross-systems: Loot + Transport + Care + AI goal sequencing.
// ===========================================================================

fn run_loot_corpse_self_care_chain(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);

    // Wounded scavenger — needs medicine but has none.
    // No natural recovery so wounds only heal through medicine.
    let scavenger = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Scavenger",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile {
            pain_weight: pm(700),
            care_weight: pm(600),
            ..UtilityProfile::default()
        },
        KnownRecipes::new(),
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_combat_profile(scavenger, no_recovery_combat_profile())
            .unwrap();
        txn.set_component_wound_list(scavenger, stable_wound_list(400))
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    // Perception profile needed to observe looted entities.
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        scavenger,
        default_perception_profile(),
    );

    // Pre-killed corpse at same location carrying medicine.
    let corpse = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Corpse",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::new(),
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_dead_at(corpse, DeadAt(Tick(0))).unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        corpse,
        VILLAGE_SQUARE,
        CommodityKind::Medicine,
        Quantity(2),
    );

    // Seed beliefs — scavenger observes corpse and its items.
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        scavenger,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    let initial_medicine_total = total_live_lot_quantity(&h.world, CommodityKind::Medicine);
    let initial_wound_load = h.agent_wound_load(scavenger);

    let mut scavenger_gained_medicine = false;
    let mut scavenger_wound_decreased = false;

    for _ in 0..100 {
        h.step_once();

        // Track medicine acquisition (from loot).
        let scavenger_medicine = h.agent_commodity_qty(scavenger, CommodityKind::Medicine);
        if scavenger_medicine > Quantity(0) {
            scavenger_gained_medicine = true;
        }

        // Track wound decrease after acquiring medicine (self-care).
        let scavenger_wound_load = h.agent_wound_load(scavenger);
        if scavenger_gained_medicine && scavenger_wound_load < initial_wound_load {
            scavenger_wound_decreased = true;
        }

        // Conservation.
        let medicine_total = total_live_lot_quantity(&h.world, CommodityKind::Medicine);
        assert!(
            medicine_total <= initial_medicine_total,
            "medicine lots must not increase: initial={initial_medicine_total}, now={medicine_total}"
        );

        assert!(
            !h.agent_is_dead(scavenger),
            "scavenger must survive the scenario"
        );

        if scavenger_gained_medicine && scavenger_wound_decreased {
            break;
        }
    }

    assert!(
        scavenger_gained_medicine,
        "scavenger should acquire medicine from looting the corpse"
    );
    assert!(
        scavenger_wound_decreased,
        "scavenger wound load should decrease after self-care with looted medicine"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_loot_corpse_self_care_chain() {
    let _ = run_loot_corpse_self_care_chain(Seed([37; 32]));
}

#[test]
fn golden_loot_corpse_self_care_chain_replays_deterministically() {
    let first = run_loot_corpse_self_care_chain(Seed([38; 32]));
    let second = run_loot_corpse_self_care_chain(Seed([38; 32]));
    assert_eq!(
        first, second,
        "loot-corpse-self-care chain should replay deterministically"
    );
}

// Scenario 45: Combat Death Triggers Force Succession
// ---------------------------------------------------------------------------
//
// Systems: Combat, Politics, AI
// GoalKinds: EngageHostile
// ActionDomains: Combat, Social
// Places: VillageSquare
// Principles: 1, 9, 24
//
// Proves challenger AI can open combat against an incumbent, and the resulting
// death drives force-law succession entirely through authoritative world state
// and event history. No combat-specific political hook participates.

#[allow(clippy::too_many_lines)]
fn run_combat_death_force_succession(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();
    h.enable_politics_tracing();

    let challenger = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Challenger",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::new(),
    );
    let incumbent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Incumbent",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::new(),
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_agent_data(
            incumbent,
            AgentData {
                control_source: ControlSource::None,
            },
        )
        .unwrap();
        txn.set_component_combat_profile(challenger, lethal_combat_attacker_profile())
            .unwrap();
        txn.set_component_combat_profile(incumbent, fragile_office_holder_profile())
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        challenger,
        default_perception_profile(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        incumbent,
        default_perception_profile(),
    );

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        challenger,
        VILLAGE_SQUARE,
        CommodityKind::Coin,
        Quantity(3),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        incumbent,
        VILLAGE_SQUARE,
        CommodityKind::Coin,
        Quantity(2),
    );

    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "War Chief",
        VILLAGE_SQUARE,
        SuccessionLaw::Force,
        5,
        vec![],
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.assign_office(office, incumbent).unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        challenger,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        incumbent,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    add_hostility(&mut h.world, &mut h.event_log, challenger, incumbent);
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.add_force_claim(challenger, office).unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    let initial_coin_total = total_live_lot_quantity(&h.world, CommodityKind::Coin);

    for _ in 0..80 {
        h.step_once();

        let coin_total = total_live_lot_quantity(&h.world, CommodityKind::Coin);
        assert_eq!(
            coin_total, initial_coin_total,
            "coin lots must remain conserved across combat-death succession"
        );

        if h.agent_is_dead(incumbent) && h.world.office_holder(office) == Some(challenger) {
            break;
        }
    }

    assert!(
        h.agent_is_dead(incumbent),
        "incumbent should die from combat"
    );
    assert!(
        !h.agent_is_dead(challenger),
        "challenger should survive the succession scenario"
    );
    assert_eq!(
        h.world.office_holder(office),
        Some(challenger),
        "force-law succession should install the surviving challenger"
    );

    let dead_at_tick = h
        .world
        .get_component_dead_at(incumbent)
        .copied()
        .expect("incumbent death should be authoritative")
        .0;

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled for combat succession scenario");
    let challenger_events = action_sink.events_for(challenger);
    assert!(
        challenger_events.iter().any(|event| {
            event.action_name == "attack"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && event.tick <= dead_at_tick
        }),
        "challenger should commit a real attack before or at the incumbent's death tick"
    );
    let declare_support_commits = challenger_events
        .iter()
        .chain(action_sink.events_for(incumbent).iter())
        .filter(|event| {
            event.action_name == "declare_support"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        })
        .count();
    assert_eq!(
        declare_support_commits, 0,
        "force-law succession must not rely on declare_support actions"
    );

    let politics_sink = h
        .politics_trace_sink()
        .expect("politics tracing should be enabled for combat succession scenario");
    let _vacancy_trace = politics_sink
        .events_for_office(office)
        .into_iter()
        .find(|event| {
            matches!(
                event.trace.outcome,
                OfficeSuccessionOutcome::VacancyActivated
            )
        })
        .expect("politics trace should explain when vacancy first activates");
    let install_trace = politics_sink
        .events_for_office(office)
        .into_iter()
        .find(|event| {
            matches!(
                event.trace.outcome,
                OfficeSuccessionOutcome::ForceInstalled { holder } if holder == challenger
            )
        })
        .expect("politics trace should explain the force-law installation");
    let control_trace = politics_sink
        .events_for_office(office)
        .into_iter()
        .find(|event| {
            matches!(
                event.trace.outcome,
                OfficeSuccessionOutcome::ForceControllerEstablished { controller }
                    if controller == challenger
            )
        })
        .expect("politics trace should record when force control is first established");
    assert!(
        politics_sink
            .events_for_office(office)
            .into_iter()
            .any(|event| {
                matches!(
                    event.trace.outcome,
                    OfficeSuccessionOutcome::ForceControllerMaintained { controller }
                        if controller == challenger
                )
            }),
        "politics trace should preserve uncontested control before installation"
    );
    assert!(
        install_trace.tick.0.saturating_sub(control_trace.tick.0) >= 4,
        "politics trace should preserve the configured uncontested hold delay"
    );

    let death_event_id =
        first_tagged_event_id_matching(&h.event_log, EventTag::Combat, |_, record| {
            event_sets_component(record, incumbent, ComponentKind::DeadAt, |after| {
                matches!(after, ComponentValue::DeadAt(_))
            })
        })
        .expect("combat should emit a death event for the incumbent");
    let vacancy_event_id =
        first_tagged_event_id_matching(&h.event_log, EventTag::Political, |_, record| {
            event_removes_relation(
                record,
                &RelationValue::OfficeHolder {
                    office,
                    holder: incumbent,
                },
            )
        })
        .expect("politics should vacate the office after the incumbent dies");
    let install_event_id =
        first_tagged_event_id_matching(&h.event_log, EventTag::Political, |_, record| {
            event_adds_relation(
                record,
                &RelationValue::OfficeHolder {
                    office,
                    holder: challenger,
                },
            )
        })
        .expect("politics should later install the challenger as office holder");

    assert_event_order(
        death_event_id,
        vacancy_event_id,
        "death event must precede the vacancy mutation",
    );
    assert_event_order(
        vacancy_event_id,
        install_event_id,
        "vacancy mutation must precede office installation",
    );

    let vacancy_tick = h
        .event_log
        .get(vacancy_event_id)
        .expect("vacancy event should exist")
        .tick();
    let install_tick = h
        .event_log
        .get(install_event_id)
        .expect("install event should exist")
        .tick();
    assert!(
        install_tick.0.saturating_sub(vacancy_tick.0) >= 5,
        "force-law installation should respect the configured succession delay"
    );

    let timeline = CrossLayerTimelineBuilder::new(&h.event_log)
        .decision_trace(
            h.driver
                .trace_sink()
                .expect("decision tracing should be enabled for combat succession scenario"),
        )
        .action_trace(action_sink)
        .politics_trace(politics_sink)
        .for_agent(challenger)
        .for_office(office)
        .tick_window(Tick(dead_at_tick.0.saturating_sub(1)), install_tick)
        .build_with_event_filter(|event_id, _| {
            event_id == death_event_id
                || event_id == vacancy_event_id
                || event_id == install_event_id
        });
    let rendered_timeline = timeline.render();
    assert!(
        timeline
            .entries()
            .iter()
            .any(|entry| entry.layer == TimelineLayer::Decision),
        "timeline should include decision entries for the acting agent"
    );
    assert!(
        rendered_timeline.contains("action: tick")
            && rendered_timeline.contains("attack")
            && rendered_timeline.contains("event: EventId")
            && rendered_timeline.contains("set DeadAt")
            && rendered_timeline.contains("politics: tick"),
        "timeline should render action, authoritative, and political layers in one view"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_combat_death_triggers_force_succession() {
    let _ = run_combat_death_force_succession(Seed([39; 32]));
}

#[test]
fn golden_combat_death_triggers_force_succession_replays_deterministically() {
    let first = run_combat_death_force_succession(Seed([40; 32]));
    let second = run_combat_death_force_succession(Seed([40; 32]));
    assert_eq!(
        first, second,
        "combat-death force-succession chain should replay deterministically"
    );
}

// Scenario 46: Social Tell Propagates Political Knowledge
// ---------------------------------------------------------------------------
//
// Systems: Social, Beliefs, Travel, AI, Politics, Succession
// GoalKinds: ShareBelief, ClaimOffice
// ActionDomains: Social, Movement
// Places: BanditCamp, VillageSquare
// Principles: 1, 7, 13, 24
//
// Proves the social Tell system can lawfully move institutional office knowledge
// into the political planning layer, unlocking the ordinary office-claim path
// without belief injection shortcuts or political/social coupling.

#[allow(clippy::too_many_lines)]
fn run_tell_propagates_political_knowledge(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();
    let bandit_camp = prototype_place_entity(PrototypePlace::BanditCamp);

    let informant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Informant",
        bandit_camp,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        social_weighted_utility(900),
    );
    let listener = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Ambitious Listener",
        bandit_camp,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            enterprise_weight: pm(800),
            social_weight: pm(0),
            ..UtilityProfile::default()
        },
    );

    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        informant,
        focused_accepting_tell_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        accepting_tell_profile(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        informant,
        blind_perception_profile(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        blind_perception_profile(),
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
        informant,
        &[listener],
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    assert!(
        agent_belief_about(&h.world, listener, office).is_none(),
        "listener should start without office knowledge"
    );
    assert_eq!(
        h.world.effective_place(listener),
        Some(bandit_camp),
        "listener should start away from the office jurisdiction"
    );

    for _ in 0..8 {
        h.step_once();
    }

    let informant_update_tick = h.scheduler.current_tick();
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        informant,
        &[office],
        informant_update_tick,
        PerceptionSource::DirectObservation,
    );
    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        informant,
        office,
        None,
        informant_update_tick,
        worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
        Some(VILLAGE_SQUARE),
    );

    let mut tell_commit_tick = None;
    for _ in 0..40 {
        h.step_once();
        if matches!(
            h.world
                .get_component_agent_belief_store(listener)
                .expect("listener should keep a belief store during tell propagation")
                .believed_office_holder(office),
            InstitutionalBeliefRead::Certain(None)
        ) {
            tell_commit_tick = Some(h.scheduler.current_tick());
            break;
        }
    }

    let tell_commit_tick = tell_commit_tick
        .expect("listener should receive the office-holder claim through the tell action");
    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled for social-political emergence");
    assert!(
        action_sink.events_for(informant).iter().any(|event| {
            event.tick <= tell_commit_tick
                && event.action_name == "tell"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && matches!(
                    event.detail,
                    Some(ActionTraceDetail::Tell {
                        listener: told_listener,
                        topic: TellTopic::InstitutionalClaim { claim },
                    }) if told_listener == listener
                        && claim == worldwake_core::InstitutionalClaim::OfficeHolder {
                            office,
                            holder: None,
                            effective_tick: informant_update_tick,
                        }
                )
        }),
        "office-holder knowledge should arrive only after the informant has committed an institutional tell action"
    );
    let phase_one_end = tell_commit_tick.0.saturating_sub(1);
    let generated_before_tell = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled for social-political emergence")
        .goal_history_for(listener, &GoalKind::ClaimOffice { office })
        .into_iter()
        .filter(|entry| entry.tick.0 <= phase_one_end)
        .any(|entry| entry.status.is_generated());
    assert!(
        !generated_before_tell,
        "listener must not generate ClaimOffice before learning the office via Tell"
    );
    assert_eq!(
        h.world.effective_place(listener),
        Some(bandit_camp),
        "listener should remain at Bandit Camp before learning about the remote office"
    );

    let told_belief = h
        .world
        .get_component_agent_belief_store(listener)
        .expect("listener should keep a belief store after tell propagation")
        .institutional_beliefs
        .get(&worldwake_core::InstitutionalBeliefKey::OfficeHolderOf { office })
        .and_then(|beliefs| beliefs.first())
        .expect("listener should receive the office-holder belief through the tell action");
    assert!(
        matches!(
            told_belief.source,
            worldwake_core::InstitutionalKnowledgeSource::Report {
                from,
                chain_len: 1
            } if from == informant
        ),
        "office-holder belief should arrive as a first-hand report from the informant"
    );

    for _ in 0..80 {
        h.step_once();
        if h.world.office_holder(office) == Some(listener) {
            break;
        }
    }

    let final_tick = h.scheduler.current_tick();
    let generated_after_tell = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled for social-political emergence")
        .goal_history_for(listener, &GoalKind::ClaimOffice { office })
        .into_iter()
        .filter(|entry| tell_commit_tick < entry.tick && entry.tick <= final_tick)
        .any(|entry| entry.status.is_generated());
    assert!(
        generated_after_tell,
        "listener should generate ClaimOffice after receiving the told office belief"
    );

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled for social-political emergence");
    let declare_support_tick = action_sink.events_for(listener).iter().find_map(|event| {
        (event.action_name == "declare_support"
            && matches!(event.kind, ActionTraceKind::Committed { .. }))
        .then_some(event.tick)
    });
    let declare_support_tick =
        declare_support_tick.expect("listener should commit declare_support after Tell");
    assert!(
        tell_commit_tick < declare_support_tick,
        "tell must commit before declare_support enters the political action path"
    );
    assert_eq!(
        h.world.effective_place(listener),
        Some(VILLAGE_SQUARE),
        "listener should travel to the office jurisdiction after the told belief arrives"
    );
    assert_eq!(
        h.world.office_holder(office),
        Some(listener),
        "listener should become office holder through the ordinary support-law path"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_tell_propagates_political_knowledge() {
    let _ = run_tell_propagates_political_knowledge(Seed([41; 32]));
}

#[test]
fn golden_tell_propagates_political_knowledge_replays_deterministically() {
    let first = run_tell_propagates_political_knowledge(Seed([42; 32]));
    let second = run_tell_propagates_political_knowledge(Seed([42; 32]));
    assert_eq!(
        first, second,
        "social-to-political knowledge propagation should replay deterministically"
    );
}

// ===========================================================================
// Suite 7: same_place_office_fact_still_requires_tell
//
// Proves: co-location with an office does not alias into listener knowledge.
// Even when speaker, listener, and office all share the same place, the
// listener must still learn the office fact through Tell before political
// planning can begin.
// Foundation: Principle 7, Principle 12, Principle 13, Principle 24.
// Cross-systems: Social + Beliefs + AI political planning + Politics.
// ===========================================================================

#[allow(clippy::too_many_lines)]
fn run_same_place_office_fact_still_requires_tell(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let speaker = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Informant",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        social_weighted_utility(900),
    );
    let listener = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Ambitious Listener",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            enterprise_weight: pm(800),
            social_weight: pm(0),
            ..UtilityProfile::default()
        },
    );

    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        focused_accepting_tell_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        accepting_tell_profile(),
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_communication_profile(listener, accepting_communication_profile())
            .expect("same-place office golden should keep communication profiles writable");
        commit_txn(txn, &mut h.event_log);
    }
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        blind_perception_profile(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        blind_perception_profile(),
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
        speaker,
        &[listener],
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    assert!(
        agent_belief_about(&h.world, listener, office).is_none(),
        "listener should start without office knowledge despite sharing the office place"
    );
    assert_eq!(
        h.world.effective_place(listener),
        Some(VILLAGE_SQUARE),
        "listener should start at the office jurisdiction"
    );

    for _ in 0..8 {
        h.step_once();
    }

    let speaker_update_tick = h.scheduler.current_tick();
    let generated_before_tell = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled for same-place social-political emergence")
        .goal_history_for(listener, &GoalKind::ClaimOffice { office })
        .into_iter()
        .filter(|entry| entry.tick <= speaker_update_tick)
        .any(|entry| entry.status.is_generated());
    assert!(
        !generated_before_tell,
        "co-location alone must not generate ClaimOffice before the listener is told"
    );
    assert!(
        agent_belief_about(&h.world, listener, office).is_none(),
        "listener should still lack the office belief after sharing the place without Tell"
    );

    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        speaker,
        &[office],
        speaker_update_tick,
        PerceptionSource::DirectObservation,
    );
    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        speaker,
        office,
        None,
        speaker_update_tick,
        worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
        Some(VILLAGE_SQUARE),
    );

    let mut tell_commit_tick = None;
    for _ in 0..40 {
        h.step_once();
        if matches!(
            h.world
                .get_component_agent_belief_store(listener)
                .expect("listener should keep a belief store during same-place tell")
                .believed_office_holder(office),
            InstitutionalBeliefRead::Certain(None)
        ) {
            tell_commit_tick = Some(h.scheduler.current_tick());
            break;
        }
    }

    let tell_commit_tick = tell_commit_tick
        .expect("listener should receive the same-place office-holder claim through Tell");
    assert!(
        h.driver
            .trace_sink()
            .expect("decision tracing should be enabled for same-place social-political emergence")
            .goal_history_for(
                speaker,
                &GoalKind::ShareBelief {
                    listener,
                    topic: TellTopic::InstitutionalClaim {
                        claim: worldwake_core::InstitutionalClaim::OfficeHolder {
                            office,
                            holder: None,
                            effective_tick: speaker_update_tick,
                        },
                    },
                    communication_class: worldwake_core::CommunicationClass::Testimony,
                },
            )
            .into_iter()
            .any(|entry| entry.tick <= tell_commit_tick && entry.status.is_generated()),
        "speaker should generate ShareBelief for the same-place office fact before Tell commits"
    );

    assert!(
        h.action_trace_sink()
            .expect("action tracing should be enabled for same-place social-political emergence")
            .events_for(speaker)
            .iter()
            .any(|event| {
                event.tick <= tell_commit_tick
                    && event.action_name == "tell"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
                    && matches!(
                        event.detail,
                        Some(ActionTraceDetail::Tell {
                            listener: told_listener,
                            topic: TellTopic::InstitutionalClaim { claim },
                        }) if told_listener == listener
                            && claim == worldwake_core::InstitutionalClaim::OfficeHolder {
                                office,
                                holder: None,
                                effective_tick: speaker_update_tick,
                            }
                    )
            }),
        "same-place office-holder knowledge should arrive only after the speaker commits Tell"
    );

    let told_belief = h
        .world
        .get_component_agent_belief_store(listener)
        .expect("listener should keep a belief store after same-place tell")
        .institutional_beliefs
        .get(&worldwake_core::InstitutionalBeliefKey::OfficeHolderOf { office })
        .and_then(|beliefs| beliefs.first())
        .expect("listener should receive the office-holder belief through Tell");
    assert!(
        matches!(
            told_belief.source,
            worldwake_core::InstitutionalKnowledgeSource::Report {
                from,
                chain_len: 1
            } if from == speaker
        ),
        "listener should learn the same-place office-holder fact as a first-hand report from the speaker"
    );

    for _ in 0..40 {
        h.step_once();
        if h.world.office_holder(office) == Some(listener) {
            break;
        }
    }

    let final_tick = h.scheduler.current_tick();
    let generated_after_tell = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled for same-place social-political emergence")
        .goal_history_for(listener, &GoalKind::ClaimOffice { office })
        .into_iter()
        .filter(|entry| tell_commit_tick <= entry.tick && entry.tick <= final_tick)
        .any(|entry| entry.status.is_generated());
    assert!(
        generated_after_tell,
        "listener should generate ClaimOffice only after receiving the same-place office belief"
    );

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled for same-place social-political emergence");
    let tell_commit_sequence = action_sink
        .events()
        .iter()
        .find(|event| {
            event.actor == speaker
                && event.action_name == "tell"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        })
        .expect("speaker should commit Tell in the same-place office scenario");
    let declare_support_commit_sequence = action_sink
        .events()
        .iter()
        .find(|event| {
            event.actor == listener
                && event.action_name == "declare_support"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        })
        .expect("listener should commit declare_support after hearing the same-place office fact");
    assert!(
        (
            tell_commit_sequence.tick,
            tell_commit_sequence.sequence_in_tick
        ) < (
            declare_support_commit_sequence.tick,
            declare_support_commit_sequence.sequence_in_tick,
        ),
        "Tell must appear earlier than declare_support in the same-place action trace"
    );
    assert_eq!(
        h.world.effective_place(listener),
        Some(VILLAGE_SQUARE),
        "listener should remain at the office jurisdiction throughout the same-place scenario"
    );
    assert_eq!(
        h.world.office_holder(office),
        Some(listener),
        "listener should become office holder through the ordinary support-law path"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_same_place_office_fact_still_requires_tell() {
    let _ = run_same_place_office_fact_still_requires_tell(Seed([43; 32]));
}

#[test]
fn golden_same_place_office_fact_still_requires_tell_replays_deterministically() {
    let first = run_same_place_office_fact_still_requires_tell(Seed([44; 32]));
    let second = run_same_place_office_fact_still_requires_tell(Seed([44; 32]));
    assert_eq!(
        first, second,
        "same-place office Tell gating should replay deterministically"
    );
}

// ===========================================================================
// Suite 8: remote_office_claim_start_failure_loses_gracefully
//
// Proves: a stale political `declare_support` step can lawfully reach
// authoritative `StartFailed` after another claimant has already been
// installed, and the shared S08 reconciliation path clears the stale claim
// without office-specific retry hacks.
// Foundation: Principle 7, Principle 19, Principle 24.
// Cross-systems: Beliefs + Travel + AI political planning + Politics.
// ===========================================================================

struct RemoteOfficeClaimStartFailureOutcome {
    world_hash: StateHash,
    log_hash: StateHash,
    loser_start_failure_count: usize,
}

#[allow(clippy::too_many_lines)]
fn run_remote_office_claim_start_failure_loses_gracefully(
    seed: Seed,
) -> RemoteOfficeClaimStartFailureOutcome {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();
    h.enable_politics_tracing();
    h.enable_request_resolution_tracing();

    let herald = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Herald",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let winner = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Installed Winner",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );
    let supporter = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Settled Supporter",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let loser = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Delayed Claimant",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
    );

    for agent in [winner, loser, supporter] {
        set_agent_perception_profile(
            &mut h.world,
            &mut h.event_log,
            agent,
            default_perception_profile(),
        );
    }
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_combat_profile(loser, no_recovery_combat_profile())
            .unwrap();
        txn.set_component_wound_list(loser, stable_wound_list(450))
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        loser,
        VILLAGE_SQUARE,
        CommodityKind::Medicine,
        Quantity(1),
    );

    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "Village Elder",
        VILLAGE_SQUARE,
        SuccessionLaw::Support,
        1,
        vec![],
    );
    declare_support(&mut h.world, &mut h.event_log, supporter, office, winner);

    for agent in [winner, loser] {
        seed_actor_beliefs(
            &mut h.world,
            &mut h.event_log,
            agent,
            &[office, winner, loser],
            Tick(0),
            PerceptionSource::Report {
                from: herald,
                chain_len: 1,
            },
        );
        seed_office_holder_belief(
            &mut h.world,
            &mut h.event_log,
            agent,
            office,
            None,
            Tick(0),
            worldwake_core::InstitutionalKnowledgeSource::Report {
                from: herald,
                chain_len: 1,
            },
            Some(VILLAGE_SQUARE),
        );
    }

    h.step_once();

    let trace_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled for political start-failure scenario");
    assert!(
        trace_sink
            .trace_at(loser, Tick(0))
            .is_some_and(|trace| match &trace.outcome {
                DecisionOutcome::Planning(planning) => planning
                    .selection
                    .selected_goal_is(GoalKey::from(GoalKind::TreatWounds { patient: loser })),
                _ => false,
            }),
        "the delayed claimant should prioritize lawful self-care first so the political request can become stale"
    );
    assert!(
        trace_sink
            .goal_history_for(loser, &GoalKind::ClaimOffice { office })
            .into_iter()
            .any(|entry| entry.tick == Tick(0) && entry.status.is_generated()),
        "the delayed claimant should still generate ClaimOffice before the office closes"
    );
    assert_eq!(
        h.agent_active_action_name(loser),
        Some("heal"),
        "tick 0 should leave the delayed claimant occupied with heal"
    );
    let loser_hold_tick = h.scheduler.current_tick();
    set_control_source(&mut h, loser, ControlSource::Human, loser_hold_tick.0);
    h.driver = worldwake_ai::AgentTickDriver::new();
    h.driver.enable_tracing();

    let declare_support_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "declare_support")
        .map(|def| def.id)
        .expect("full registries should include declare_support");
    let winner_support_tick = h.scheduler.current_tick();
    set_control_source(&mut h, winner, ControlSource::Human, winner_support_tick.0);
    let _ = h.scheduler.input_queue_mut().enqueue(
        winner_support_tick,
        InputKind::RequestAction {
            actor: winner,
            def_id: declare_support_def_id,
            targets: Vec::new(),
            payload_override: Some(ActionPayload::DeclareSupport(DeclareSupportActionPayload {
                office,
                candidate: winner,
            })),
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
    h.step_once();

    let winner_install_sequence = h
        .action_trace_sink()
        .expect("action tracing should remain enabled")
        .events_for(winner)
        .iter()
        .find_map(|event| {
            (event.action_name == "declare_support"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .expect("winner should commit declare_support through the ordinary action path");
    let install_trace_tick = h
        .politics_trace_sink()
        .expect("politics tracing should remain enabled")
        .events_for_office(office)
        .into_iter()
        .find(|event| {
            event.trace.availability_phase == OfficeAvailabilityPhase::ClosedOccupied
                && matches!(
                    event.trace.outcome,
                    OfficeSuccessionOutcome::SupportInstalled { holder } if holder == winner
                )
        })
        .map(|event| event.tick)
        .expect("politics trace should expose the office closure phase once the winner installs");

    for _ in 0..16 {
        if !h.agent_has_active_action(loser) {
            break;
        }
        h.step_once();
    }
    assert!(
        !h.agent_has_active_action(loser),
        "the delayed claimant should eventually finish healing before the stale political request is retried"
    );

    let stale_request_tick = h.scheduler.current_tick();
    set_control_source(&mut h, loser, ControlSource::Human, stale_request_tick.0);
    let _ = h.scheduler.input_queue_mut().enqueue(
        stale_request_tick,
        InputKind::RequestAction {
            actor: loser,
            def_id: declare_support_def_id,
            targets: Vec::new(),
            payload_override: Some(ActionPayload::DeclareSupport(DeclareSupportActionPayload {
                office,
                candidate: loser,
            })),
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
    h.step_once();

    let (loser_failure_sequence, loser_action_request) = h
        .action_trace_sink()
        .expect("action tracing should remain enabled")
        .events_for(loser)
        .iter()
        .find_map(|event| match &event.kind {
            ActionTraceKind::StartFailed { request, .. } if event.action_name == "declare_support" => {
                Some(((event.tick, event.sequence_in_tick), *request))
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!(
                "stale late declare_support should hit StartFailed once the office is no longer lawfully claimable; loser events={:?}; loser failures={:?}; office_holder={:?}; office_data={:?}",
                h.action_trace_sink()
                    .expect("action tracing should remain enabled")
                    .events_for(loser),
                h.scheduler.action_start_failures(),
                h.world.office_holder(office),
                h.world.get_component_office_data(office)
            )
        });
    assert!(
        winner_install_sequence < loser_failure_sequence,
        "another lawful political action must close the opportunity before the stale request fails: winner={winner_install_sequence:?}, loser={loser_failure_sequence:?}"
    );
    assert!(
        install_trace_tick <= loser_failure_sequence.0,
        "politics trace should close the office before the stale political request fails: install_tick={install_trace_tick:?}, loser_failure={loser_failure_sequence:?}",
    );

    let loser_failures = h
        .scheduler
        .action_start_failures()
        .iter()
        .filter(|failure| failure.actor == loser)
        .collect::<Vec<_>>();
    assert_eq!(loser_failures.len(), 1);
    let failure_tick = loser_failures[0].tick;
    let loser_request_events = h
        .request_resolution_trace_sink()
        .expect("request-resolution tracing should remain enabled")
        .events_for_at(loser, failure_tick);
    assert_eq!(loser_request_events.len(), 1);
    assert_eq!(
        loser_request_events[0].request.provenance,
        RequestProvenance::External
    );
    let expected_request = match loser_request_events[0].outcome {
        RequestResolutionOutcome::Bound {
            binding,
            ref resolved_targets,
            start_attempted: true,
        } => {
            assert!(
                resolved_targets.is_empty(),
                "declare_support binds through payload, not bound targets"
            );
            ResolvedRequestTrace {
                attempt: loser_request_events[0].request,
                binding,
            }
        }
        ref other => panic!("expected bound request-resolution outcome, got {other:?}"),
    };
    assert_eq!(loser_action_request, expected_request);
    assert_eq!(loser_failures[0].request, expected_request);
    assert!(
        matches!(
            &loser_failures[0].reason,
            ActionStartFailureReason::PreconditionFailed(detail) if detail.contains("not vacant")
        ),
        "late political failure should come from authoritative vacancy validation, got {:?}",
        loser_failures[0].reason
    );
    set_control_source(&mut h, loser, ControlSource::Ai, failure_tick.0);
    h.step_once();

    let loser_after_failure = h
        .driver
        .trace_sink()
        .expect("decision tracing should remain enabled")
        .trace_at(loser, failure_tick + 1)
        .expect("loser should have a planning trace immediately after the political start failure");
    let loser_planning_after_failure = match &loser_after_failure.outcome {
        DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected post-failure planning trace, got {other:?}"),
    };
    assert_eq!(loser_planning_after_failure.action_start_failures.len(), 1);
    assert_eq!(
        loser_planning_after_failure.action_start_failures[0].request,
        expected_request
    );
    assert!(
        matches!(
            &loser_planning_after_failure.action_start_failures[0].reason,
            ActionStartFailureReason::PreconditionFailed(detail) if detail.contains("not vacant")
        ),
        "next-tick reconciliation should expose the authoritative vacancy failure"
    );
    assert!(
        loser_planning_after_failure.selection.selected_plan_source
            != Some(SelectedPlanSource::RetainedCurrentPlan),
        "shared S08 reconciliation should clear the stale political plan instead of retaining it"
    );
    assert!(
        !loser_planning_after_failure
            .candidates
            .generated
            .iter()
            .any(|goal| goal.goal_key.kind == GoalKind::ClaimOffice { office }),
        "occupied office should stop emitting a fresh ClaimOffice candidate after the failure"
    );
    assert!(
        loser_planning_after_failure
            .candidates
            .omitted_political
            .iter()
            .any(|omission| {
                omission.office == office
                    && omission.reason == PoliticalCandidateOmissionReason::OfficeNotVisiblyVacant
            }),
        "post-failure planning should explain the missing political candidate as OfficeNotVisiblyVacant"
    );
    assert!(
        h.scheduler
            .action_start_failures()
            .iter()
            .all(|failure| failure.actor != loser),
        "post-failure reconciliation should drain the loser's structured political start failure"
    );

    for _ in 0..12 {
        h.step_once();
    }

    let loser_start_failure_count = h
        .action_trace_sink()
        .expect("action tracing should remain enabled")
        .events_for(loser)
        .into_iter()
        .filter(|event| {
            event.action_name == "declare_support"
                && matches!(event.kind, ActionTraceKind::StartFailed { .. })
        })
        .count();
    assert_eq!(
        loser_start_failure_count, 1,
        "occupied-office knowledge should prevent repeated stale declare_support retries"
    );
    RemoteOfficeClaimStartFailureOutcome {
        world_hash: hash_world(&h.world).unwrap(),
        log_hash: hash_event_log(&h.event_log).unwrap(),
        loser_start_failure_count,
    }
}

#[test]
fn golden_remote_office_claim_start_failure_loses_gracefully() {
    let outcome = run_remote_office_claim_start_failure_loses_gracefully(Seed([45; 32]));
    assert_eq!(outcome.loser_start_failure_count, 1);
}

#[test]
fn golden_remote_office_claim_start_failure_loses_gracefully_replays_deterministically() {
    let first = run_remote_office_claim_start_failure_loses_gracefully(Seed([46; 32]));
    let second = run_remote_office_claim_start_failure_loses_gracefully(Seed([46; 32]));
    assert_eq!(first.world_hash, second.world_hash);
    assert_eq!(first.log_hash, second.log_hash);
    assert_eq!(
        first.loser_start_failure_count, second.loser_start_failure_count,
        "political start-failure emergence should replay deterministically"
    );
}

// ===========================================================================
// Suite 9: already_told_recent_subject_does_not_crowd_out_untold_office_fact
//
// Proves: listener-aware resend suppression happens before tell-candidate
// truncation in the live AI/action path. A more recent already-told subject
// must not crowd out an older untold office fact that enables downstream
// political behavior.
// Foundation: Principle 7, Principle 13, Principle 18, Principle 19,
// Principle 24.
// Cross-systems: Social + conversation memory + beliefs + AI political
// planning + travel + politics.
// ===========================================================================

#[allow(clippy::too_many_lines)]
fn run_already_told_recent_subject_does_not_crowd_out_untold_office_fact(
    seed: Seed,
) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();
    let bandit_camp = prototype_place_entity(PrototypePlace::BanditCamp);

    let speaker = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Informant",
        bandit_camp,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        social_weighted_utility(900),
    );
    let listener = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Ambitious Listener",
        bandit_camp,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            enterprise_weight: pm(800),
            social_weight: pm(0),
            ..UtilityProfile::default()
        },
    );
    let recent_subject = ORCHARD_FARM;

    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        focused_accepting_tell_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        accepting_tell_profile(),
    );
    for agent in [speaker, listener] {
        set_agent_perception_profile(
            &mut h.world,
            &mut h.event_log,
            agent,
            blind_perception_profile(),
        );
    }

    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "Village Elder",
        VILLAGE_SQUARE,
        SuccessionLaw::Support,
        5,
        vec![],
    );

    let share_recent = GoalKind::ShareBelief {
        listener,
        topic: TellTopic::EntityBelief {
            subject: recent_subject,
        },
        communication_class: worldwake_core::CommunicationClass::Testimony,
    };
    let share_office = GoalKind::ShareBelief {
        listener,
        topic: TellTopic::InstitutionalClaim {
            claim: worldwake_core::InstitutionalClaim::OfficeHolder {
                office,
                holder: None,
                effective_tick: Tick(0),
            },
        },
        communication_class: worldwake_core::CommunicationClass::Testimony,
    };

    assert!(
        agent_belief_about(&h.world, listener, recent_subject).is_none(),
        "listener should start without the recent-subject belief"
    );
    assert!(
        agent_belief_about(&h.world, listener, office).is_none(),
        "listener should start without office knowledge"
    );
    assert_eq!(
        h.world.effective_place(listener),
        Some(bandit_camp),
        "listener should start away from the office jurisdiction"
    );

    let listener_belief = seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        speaker,
        listener,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    let recent_seed_tick = Tick(1);
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        speaker,
        &[recent_subject],
        recent_seed_tick,
        PerceptionSource::DirectObservation,
    );
    seed_told_belief_memory(
        &mut h.world,
        &mut h.event_log,
        speaker,
        listener,
        listener,
        &listener_belief,
        Tick(0),
    );

    let mut recent_tell_tick = None;
    for _ in 0..40 {
        h.step_once();
        if agent_belief_about(&h.world, listener, recent_subject).is_some() {
            recent_tell_tick = Some(h.scheduler.current_tick());
            break;
        }
    }

    let _recent_tell_tick = recent_tell_tick
        .expect("speaker should first tell the listener about the more recent subject");
    assert!(
        agent_belief_about(&h.world, listener, office).is_none(),
        "the office fact should still be untold after the first tell"
    );

    let office_observed_tick = Tick(0);
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        speaker,
        &[office],
        office_observed_tick,
        PerceptionSource::DirectObservation,
    );
    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        speaker,
        office,
        None,
        office_observed_tick,
        worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
        Some(VILLAGE_SQUARE),
    );

    let mut saw_recent_omission = false;
    let mut saw_office_generated = false;
    let mut office_tell_tick = None;
    for _ in 0..80 {
        h.step_once();
        let speaker_trace = h
            .driver
            .trace_sink()
            .expect("decision tracing should be enabled for crowd-out emergence")
            .traces_for(speaker)
            .into_iter()
            .last()
            .expect("speaker should have decision traces in crowd-out emergence");
        saw_recent_omission |= speaker_trace.goal_status(&share_recent)
            == worldwake_ai::GoalTraceStatus::OmittedSocial(
                worldwake_sim::TellTopicOmissionReason::SpeakerHasAlreadyToldCurrentBelief,
            );
        saw_office_generated |= speaker_trace.goal_status(&share_office).is_generated();

        if matches!(
            h.world
                .get_component_agent_belief_store(listener)
                .expect("listener should keep a belief store during crowd-out tell")
                .believed_office_holder(office),
            InstitutionalBeliefRead::Certain(None)
        ) {
            office_tell_tick = Some(h.scheduler.current_tick());
            break;
        }
    }

    let office_tell_tick = office_tell_tick.expect(
        "speaker should eventually tell the untold office fact after omitting the duplicate",
    );
    assert!(
        saw_recent_omission,
        "decision traces should show the recent subject omitted as already told before truncation"
    );
    assert!(
        saw_office_generated,
        "after the duplicate recent subject is omitted, the older office fact should still generate"
    );

    let speaker_action_events = h
        .action_trace_sink()
        .expect("action tracing should be enabled for crowd-out emergence")
        .events_for(speaker);
    let recent_tell_commits_before_office = speaker_action_events
        .iter()
        .filter(|event| {
            event.tick <= office_tell_tick
                && event.action_name == "tell"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && event.detail.as_ref()
                    == Some(&ActionTraceDetail::Tell {
                        listener,
                        topic: TellTopic::EntityBelief {
                            subject: recent_subject,
                        },
                    })
        })
        .count();
    let office_tell_commit = speaker_action_events
        .iter()
        .find(|event| {
            event.tick <= office_tell_tick
                && event.action_name == "tell"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && event.detail.as_ref()
                    == Some(&ActionTraceDetail::Tell {
                        listener,
                        topic: TellTopic::InstitutionalClaim {
                            claim: worldwake_core::InstitutionalClaim::OfficeHolder {
                                office,
                                holder: None,
                                effective_tick: Tick(0),
                            },
                        },
                    })
        })
        .expect("speaker should commit a tell for the office-holder claim");
    let office_tell_commit_order = (office_tell_commit.tick, office_tell_commit.sequence_in_tick);
    assert_eq!(
        recent_tell_commits_before_office, 1,
        "speaker should commit the more recent subject exactly once before the office fact is learned"
    );

    let generated_claim_before_tell = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled for crowd-out emergence")
        .goal_history_for(listener, &GoalKind::ClaimOffice { office })
        .into_iter()
        .filter(|entry| entry.tick < office_tell_tick)
        .any(|entry| entry.status.is_generated());
    assert!(
        !generated_claim_before_tell,
        "listener must not generate ClaimOffice before learning the office through Tell"
    );

    let told_office_belief = h
        .world
        .get_component_agent_belief_store(listener)
        .expect("listener should keep a belief store after office tell")
        .institutional_beliefs
        .get(&worldwake_core::InstitutionalBeliefKey::OfficeHolderOf { office })
        .and_then(|beliefs| beliefs.first())
        .expect("listener should receive the office-holder belief through Tell");
    assert!(
        matches!(
            told_office_belief.source,
            worldwake_core::InstitutionalKnowledgeSource::Report {
                from,
                chain_len: 1
            } if from == speaker
        ),
        "listener should receive the office-holder fact as a first-hand report from the speaker"
    );

    for _ in 0..80 {
        h.step_once();
        if h.world.office_holder(office) == Some(listener) {
            break;
        }
    }

    let final_tick = h.scheduler.current_tick();
    let generated_claim_after_tell = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled for crowd-out emergence")
        .goal_history_for(listener, &GoalKind::ClaimOffice { office })
        .into_iter()
        .filter(|entry| office_tell_tick < entry.tick && entry.tick <= final_tick)
        .any(|entry| entry.status.is_generated());
    assert!(
        generated_claim_after_tell,
        "listener should generate ClaimOffice only after the older office fact is told"
    );

    let listener_events = h
        .action_trace_sink()
        .expect("action tracing should be enabled for crowd-out emergence")
        .events_for(listener);
    let declare_support_commit = listener_events
        .iter()
        .find(|event| {
            event.action_name == "declare_support"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        })
        .expect("listener should commit declare_support after learning the office fact");
    assert!(
        office_tell_commit_order
            < (
                declare_support_commit.tick,
                declare_support_commit.sequence_in_tick,
            ),
        "the tell that unlocks the office fact must appear before declare_support in the action trace"
    );
    assert_eq!(
        h.world.effective_place(listener),
        Some(VILLAGE_SQUARE),
        "listener should travel to the office jurisdiction after learning the office fact"
    );
    assert_eq!(
        h.world.office_holder(office),
        Some(listener),
        "listener should become office holder through the ordinary support-law path"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact() {
    let _ = run_already_told_recent_subject_does_not_crowd_out_untold_office_fact(Seed([45; 32]));
}

#[test]
fn golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact_replays_deterministically()
 {
    let first =
        run_already_told_recent_subject_does_not_crowd_out_untold_office_fact(Seed([46; 32]));
    let second =
        run_already_told_recent_subject_does_not_crowd_out_untold_office_fact(Seed([46; 32]));
    assert_eq!(
        first, second,
        "crowd-out prevention should replay deterministically"
    );
}

// ===========================================================================
// Suite 10: force_controller_departure_enables_rival_claim
//
// Proves: an agent's physical departure from a force-law jurisdiction
// cascades into political vacancy and rival AI claim. This cross-system
// chain (travel → force-control clearing → AI candidate generation →
// PressForceClaim) emerges from shared state, not orchestration.
// Foundation: Principle 1, Principle 8, Principle 10.
// Cross-systems: Travel + Force-Control State Machine + AI.
// ===========================================================================

#[allow(clippy::too_many_lines)]
fn run_force_controller_departure_enables_rival_claim(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();
    h.enable_politics_tracing();

    // -- Agent A ("Controller"): human-controlled, will depart --
    let controller_agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Controller",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
        KnownRecipes::new(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        controller_agent,
        default_perception_profile(),
    );
    set_control_source(&mut h, controller_agent, ControlSource::Human, 0);

    // -- Agent B ("Rival"): AI-controlled, sated, enterprise-weighted --
    let rival = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Rival",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        enterprise_weighted_utility(pm(800)),
        KnownRecipes::new(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        rival,
        default_perception_profile(),
    );

    // -- Force-law office at VillageSquare --
    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "War Chief",
        VILLAGE_SQUARE,
        SuccessionLaw::Force,
        5,
        vec![],
    );

    // Pre-seed A as office_controller directly (no force claim).
    // A has already completed a force claim cycle in the "past" — only the
    // controller relation remains. This ensures A's departure clears the
    // controller without leaving a lingering live claim that blocks B's
    // installation.
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_office_controller(office, controller_agent).unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Seed beliefs for both agents.
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        controller_agent,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        rival,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    // B needs to know about the office and its controller.
    seed_known_office_at_place(
        &mut h.world,
        &mut h.event_log,
        rival,
        office,
        VILLAGE_SQUARE,
        Tick(0),
    );
    seed_force_controller_belief(
        &mut h.world,
        &mut h.event_log,
        rival,
        office,
        Some(controller_agent),
        false,
        Tick(0),
        Some(VILLAGE_SQUARE),
    );

    // -- Issue human input: Controller A travels away from VillageSquare --
    let travel_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "travel")
        .map(|def| def.id)
        .expect("full registries should include travel");

    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);
    let _ = h.scheduler.input_queue_mut().enqueue(
        Tick(0),
        InputKind::RequestAction {
            actor: controller_agent,
            def_id: travel_def_id,
            targets: vec![general_store],
            payload_override: None,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );

    // -- Tick loop: wait for B to become office_holder --
    for _ in 0..80 {
        h.step_once();
        if h.world.office_holder(office) == Some(rival) {
            break;
        }
    }

    // =====================================================================
    // Assertions
    // =====================================================================

    // 1. Rival installed as office_holder.
    assert_eq!(
        h.world.office_holder(office),
        Some(rival),
        "rival should be installed as office holder after uncontested hold"
    );

    // 2. Action trace: A's travel commits before B's press_force_claim.
    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");

    let controller_travel_commit = action_sink
        .events_for(controller_agent)
        .iter()
        .find_map(|event| {
            (event.action_name == "travel"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .expect("controller should commit a travel action");

    let rival_claim_commit = action_sink
        .events_for(rival)
        .iter()
        .find_map(|event| {
            (event.action_name == "press_force_claim"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .expect("rival should commit a press_force_claim action");

    assert!(
        controller_travel_commit < rival_claim_commit,
        "controller's travel must commit before rival's force claim \
         (travel={controller_travel_commit:?}, claim={rival_claim_commit:?})",
    );

    // 3. Controller cleared after departure (intermediate state).
    //    By the time rival claims, the controller must have been cleared.
    //    We verify via politics trace: ForceControllerEstablished for rival
    //    implies the old controller was cleared first.
    let politics_sink = h
        .politics_trace_sink()
        .expect("politics tracing should be enabled");

    politics_sink
        .events_for_office(office)
        .into_iter()
        .find(|event| {
            matches!(
                event.trace.outcome,
                OfficeSuccessionOutcome::ForceControllerEstablished { controller }
                    if controller == rival
            )
        })
        .expect(
            "politics trace should record rival establishing force control \
             after controller's departure",
        );

    // 4. ForceInstalled for rival as holder.
    politics_sink
        .events_for_office(office)
        .into_iter()
        .find(|event| {
            matches!(
                event.trace.outcome,
                OfficeSuccessionOutcome::ForceInstalled { holder }
                    if holder == rival
            )
        })
        .expect("politics trace should record rival's installation as office holder");

    // 5. Decision trace: B does NOT generate ClaimOffice at tick 0 (office is
    //    controlled by A); B DOES generate ClaimOffice after A departs.
    let trace_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled");

    let claim_goal = GoalKind::ClaimOffice { office };
    let history = trace_sink.goal_history_for(rival, &claim_goal);

    // Find the first tick where ClaimOffice is generated.
    let first_generated_tick = history
        .iter()
        .find(|entry| entry.status.is_generated())
        .map(|entry| entry.tick);
    assert!(
        first_generated_tick.is_some(),
        "rival should eventually generate ClaimOffice goal"
    );

    // The travel starts at the commit tick (1-tick travel commits in the same
    // tick). Within that tick the controller enters transit BEFORE the rival's
    // AI runs, so ClaimOffice generation at the departure tick is correct.
    // The assertion: no ClaimOffice STRICTLY before the departure tick.
    let travel_commit_tick = controller_travel_commit.0;
    let pre_departure_generated = history
        .iter()
        .any(|entry| entry.tick < travel_commit_tick && entry.status.is_generated());
    assert!(
        !pre_departure_generated,
        "rival should NOT generate ClaimOffice before controller's travel \
         departure tick (office is still controlled)"
    );

    // 6. Negative: no declare_support actions committed by any agent.
    let all_events_iter = action_sink
        .events_for(controller_agent)
        .into_iter()
        .chain(action_sink.events_for(rival));
    let declare_support_commits = all_events_iter
        .filter(|event| {
            event.action_name == "declare_support"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        })
        .count();
    assert_eq!(
        declare_support_commits, 0,
        "force-law scenario must not rely on declare_support actions"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_force_controller_departure_enables_rival_claim() {
    let _ = run_force_controller_departure_enables_rival_claim(Seed([47; 32]));
}

#[test]
fn golden_force_controller_departure_enables_rival_claim_replays_deterministically() {
    let first = run_force_controller_departure_enables_rival_claim(Seed([48; 32]));
    let second = run_force_controller_departure_enables_rival_claim(Seed([48; 32]));
    assert_eq!(
        first, second,
        "force controller departure scenario should replay deterministically"
    );
}

// ===========================================================================
// Suite 11: force_claim_creates_hostility_witnessed_and_propagated
// ---------------------------------------------------------------------------
//
// Systems: Force-Control, Combat (hostility), Perception, Institutional Beliefs, Travel, Social Tell
// GoalKinds: ShareBelief
// ActionDomains: Social, Generic
// Places: VillageSquare, BanditCamp
// Principles: 1, 7, 9, 13
//
// Setup: Force-law office ("War Chief") at VillageSquare with A as incumbent
//   office_holder. B (human) presses force claim. C (AI, social_weight=pm(600))
//   witnesses at VillageSquare. D (passive) at BanditCamp.
//
// Proves: Force claim creates emergent hostility (B->A) as a side effect,
//   not pre-seeded. Political knowledge (ForceControllerOf) propagates
//   physically via witness C traveling to remote listener D and committing
//   a Tell. D cannot learn without the physical carrier arriving (P7).
//
// Chain: press_force_claim -> hostility(B,A) + vacancy -> succession
//   establishes B as controller -> C acquires ForceControl belief ->
//   C relocates to BanditCamp -> C tells D -> D learns ForceControllerOf.
// ===========================================================================

#[allow(clippy::too_many_lines)]
fn run_force_claim_creates_hostility_witnessed_and_propagated(
    seed: Seed,
) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();
    h.enable_politics_tracing();

    let bandit_camp = prototype_place_entity(PrototypePlace::BanditCamp);

    // -- Agent A ("Incumbent"): human-controlled office_holder at VillageSquare --
    let incumbent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Incumbent",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        incumbent,
        default_perception_profile(),
    );
    set_control_source(&mut h, incumbent, ControlSource::Human, 0);

    // -- Agent B ("Challenger"): human-controlled, will press force claim --
    let challenger = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Challenger",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        challenger,
        default_perception_profile(),
    );
    set_control_source(&mut h, challenger, ControlSource::Human, 0);

    // -- Agent C ("Witness"): AI-controlled, social-weighted for Tell --
    let witness = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Witness",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            social_weight: pm(600),
            enterprise_weight: pm(0),
            ..UtilityProfile::default()
        },
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
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        witness,
        broad_accepting_tell_profile(),
    );

    // -- Agent D ("Remote Listener"): at BanditCamp, passive reception --
    let remote_listener = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Remote Listener",
        bandit_camp,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        remote_listener,
        default_perception_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        remote_listener,
        accepting_tell_profile(),
    );

    // -- Force-law office at VillageSquare --
    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "War Chief",
        VILLAGE_SQUARE,
        SuccessionLaw::Force,
        5,
        vec![],
    );

    // Install A as office_holder (pre-existing incumbent).
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.assign_office(office, incumbent).unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Seed beliefs: C knows about A, B, and D (at BanditCamp).
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        witness,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        witness,
        &[remote_listener],
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    // C knows about the office at VillageSquare.
    seed_known_office_at_place(
        &mut h.world,
        &mut h.event_log,
        witness,
        office,
        VILLAGE_SQUARE,
        Tick(0),
    );

    // D starts with no force-control institutional belief.
    let d_store_before = h
        .world
        .get_component_agent_belief_store(remote_listener)
        .cloned()
        .unwrap_or_else(worldwake_core::AgentBeliefStore::new);
    assert!(
        matches!(
            d_store_before.believed_force_controller(office),
            InstitutionalBeliefRead::Unknown
        ),
        "remote listener must start without any force-control belief"
    );

    // =====================================================================
    // Phase 1: Issue B's PressForceClaim and verify hostility + C's belief
    // =====================================================================

    let press_force_claim_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "press_force_claim")
        .map(|def| def.id)
        .expect("full registries should include press_force_claim");

    let claim_tick = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        claim_tick,
        InputKind::RequestAction {
            actor: challenger,
            def_id: press_force_claim_def_id,
            targets: Vec::new(),
            payload_override: Some(ActionPayload::PressForceClaim(
                PressForceClaimActionPayload { office },
            )),
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );

    // Tick until press_force_claim commits.
    let mut claim_committed = false;
    for _ in 0..50 {
        h.step_once();
        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");
        if action_sink.events_for(challenger).iter().any(|event| {
            event.action_name == "press_force_claim"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        }) {
            claim_committed = true;
            break;
        }
    }
    assert!(
        claim_committed,
        "challenger's press_force_claim should commit within 20 ticks"
    );

    // 1. Hostility: B is hostile towards A (emergent, not pre-seeded).
    assert!(
        h.world.hostile_targets_of(challenger).contains(&incumbent),
        "challenger should become hostile towards incumbent as a side effect \
         of press_force_claim — this hostility is EMERGENT, not pre-seeded"
    );

    // 2. Vacate A so the succession system can process B's pending force claim.
    //    While A holds the office, the succession system returns OccupiedNoAction
    //    and B's ContestsOffice relation sits idle.  Vacating A simulates the
    //    consequence that follows from the hostility (combat, expulsion, etc.)
    //    without requiring those intermediate systems in this test.
    {
        let vacate_tick = h.scheduler.current_tick();
        let mut txn = new_txn(&mut h.world, vacate_tick.0);
        txn.vacate_office(office).unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Step ticks until the succession system establishes B as controller.
    let mut controller_established = false;
    for _ in 0..10 {
        h.step_once();
        if h.world.office_controller(office) == Some(challenger) {
            controller_established = true;
            break;
        }
    }
    assert!(
        controller_established,
        "succession system should establish challenger as controller \
         after vacancy + pending force claim"
    );

    // 3. Seed C's ForceControl institutional belief.  Politics runs before
    //    Perception in the tick loop, so C may already have observed the
    //    controller establishment via perception.  In this test the seeded
    //    belief agrees with the perceived one (both: challenger, uncontested),
    //    so no conflict arises.  We seed explicitly to guarantee the belief
    //    exists regardless of perception timing — C IS at VillageSquare per
    //    Principle 7 (locality).  This follows the Scenario 21 pattern.
    let belief_tick = h.scheduler.current_tick();
    seed_force_controller_belief(
        &mut h.world,
        &mut h.event_log,
        witness,
        office,
        Some(challenger),
        false,
        belief_tick,
        Some(VILLAGE_SQUARE),
    );
    let c_store = h
        .world
        .get_component_agent_belief_store(witness)
        .expect("witness should have a belief store after seeding");
    assert_eq!(
        c_store.believed_force_controller(office),
        InstitutionalBeliefRead::Certain((Some(challenger), false)),
        "witness should have ForceControllerOf belief with challenger as controller"
    );

    // D still has no belief after Phase 1.
    let d_store_phase1 = h
        .world
        .get_component_agent_belief_store(remote_listener)
        .expect("remote listener should retain a belief store");
    assert!(
        matches!(
            d_store_phase1.believed_force_controller(office),
            InstitutionalBeliefRead::Unknown
        ),
        "remote listener must remain ignorant after Phase 1 (no physical carrier yet)"
    );

    // =====================================================================
    // Phase 2: C moves to BanditCamp and Tells D
    // =====================================================================

    // The AI's ShareBelief candidate generation only finds co-located
    // listeners — it does not plan multi-step travel→tell sequences.
    // Relocate C to BanditCamp (matching the Scenario 21 pattern in
    // golden_offices.rs) so C's AI can generate ShareBelief for D.
    // This preserves the Principle 7 invariant: D cannot learn without
    // a physical carrier arriving at BanditCamp.
    {
        let travel_tick = h.scheduler.current_tick();
        let mut txn = new_txn(&mut h.world, travel_tick.0);
        txn.set_ground_location(witness, bandit_camp).unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    assert_eq!(
        h.world.effective_place(witness),
        Some(bandit_camp),
        "witness should be at BanditCamp before the tell phase"
    );

    // D must still be ignorant after C arrives but before Tell.
    let d_store_pre_tell = h
        .world
        .get_component_agent_belief_store(remote_listener)
        .expect("remote listener should retain a belief store");
    assert!(
        matches!(
            d_store_pre_tell.believed_force_controller(office),
            InstitutionalBeliefRead::Unknown
        ),
        "remote listener must remain ignorant until C's tell commits \
         (physical presence alone is not enough)"
    );

    // C needs beliefs about D for ShareBelief to fire now that C is
    // co-located with D.
    let co_locate_tick = h.scheduler.current_tick();
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        witness,
        co_locate_tick,
        PerceptionSource::DirectObservation,
    );

    // Tick until D acquires the force-control belief via Tell from C.
    let mut d_learned = false;
    for _ in 0..80 {
        h.step_once();
        if let Some(store) = h.world.get_component_agent_belief_store(remote_listener)
            && matches!(
                store.believed_force_controller(office),
                InstitutionalBeliefRead::Certain((Some(controller), _)) if controller == challenger
            )
        {
            d_learned = true;
            break;
        }
    }
    assert!(
        d_learned,
        "remote listener (D) should learn ForceControllerOf belief from witness (C) \
         via Tell after C arrives at BanditCamp"
    );

    // =====================================================================
    // Phase 3: Final assertions
    // =====================================================================

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");

    // 3. Action ordering: B's press_force_claim commits before C's tell to D.
    let claim_commit = action_sink
        .events_for(challenger)
        .iter()
        .find_map(|event| {
            (event.action_name == "press_force_claim"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .expect("challenger should have committed press_force_claim");

    let witness_tell_commit = action_sink
        .events_for(witness)
        .iter()
        .find_map(|event| {
            (event.action_name == "tell"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && matches!(
                    event.detail,
                    Some(ActionTraceDetail::Tell {
                        listener: told_listener,
                        topic,
                    }) if told_listener == remote_listener
                        && topic == TellTopic::InstitutionalClaim {
                            claim: InstitutionalClaim::ForceControl {
                                office,
                                controller: Some(challenger),
                                contested: false,
                                effective_tick: Tick(3),
                            },
                        }
                ))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .expect(
            "witness should commit a tell action relaying force-control knowledge to remote listener",
        );

    assert!(
        claim_commit < witness_tell_commit,
        "press_force_claim must commit before witness tells remote listener \
         (claim={claim_commit:?}, tell={witness_tell_commit:?})",
    );

    // 4. Decision trace: C generates ShareBelief candidate for the office.
    let trace_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled");

    let share_belief_goal = GoalKind::ShareBelief {
        listener: remote_listener,
        topic: TellTopic::InstitutionalClaim {
            claim: InstitutionalClaim::ForceControl {
                office,
                controller: Some(challenger),
                contested: false,
                effective_tick: Tick(3),
            },
        },
        communication_class: worldwake_core::CommunicationClass::Testimony,
    };
    let share_history = trace_sink.goal_history_for(witness, &share_belief_goal);
    assert!(
        share_history
            .iter()
            .any(|entry| entry.status.is_generated()),
        "witness should generate ShareBelief goal for the force-control claim \
         targeting the remote listener"
    );

    // 5. D's belief source should be Report (from Tell), not direct observation.
    let d_store_final = h
        .world
        .get_component_agent_belief_store(remote_listener)
        .expect("remote listener should have a belief store after tell");
    assert_eq!(
        d_store_final.believed_force_controller(office),
        InstitutionalBeliefRead::Certain((Some(challenger), false)),
        "remote listener should believe challenger is the force controller (uncontested)"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_force_claim_creates_hostility_witnessed_and_propagated() {
    let _ = run_force_claim_creates_hostility_witnessed_and_propagated(Seed([51; 32]));
}

#[test]
fn golden_force_claim_creates_hostility_witnessed_and_propagated_replays_deterministically() {
    let first = run_force_claim_creates_hostility_witnessed_and_propagated(Seed([52; 32]));
    let second = run_force_claim_creates_hostility_witnessed_and_propagated(Seed([52; 32]));
    assert_eq!(
        first, second,
        "force-claim hostility and belief propagation should replay deterministically"
    );
}

// ===========================================================================
// Suite 12: contested_force_state_propagates_through_belief_system
// ---------------------------------------------------------------------------
//
// Systems: PressForceClaim (two claimants), Force-Control State Machine
//   (contested state detection), Perception (witness observes contested
//   political event), Institutional Belief Projection (ForceControl belief
//   with `contested: true`), Travel, Social Tell, Belief Store (remote
//   belief with contested flag), action tracing, deterministic replay
// GoalKinds: ShareBelief
// ActionDomains: Social, Generic
// Places: VillageSquare, OrchardFarm
// Principles: 1, 3, 7, 13
//
// Setup: Force-law office ("War Chief") at VillageSquare, no eligibility
//   rules. A (human) claims first and becomes sole controller. B (human)
//   then claims, creating contested state (office_controller == None).
//   C (AI, social_weight=pm(600)) witnesses at VillageSquare.
//   D (passive) at OrchardFarm.
//
// Proves: The `contested: true` flag from a multi-claimant force office
//   is concrete information that propagates physically through the world
//   via witness C traveling to remote listener D and committing a Tell.
//   D cannot learn without the physical carrier arriving (P7).
//
// Chain: A press_force_claim -> A becomes controller -> B press_force_claim
//   -> contested state (controller cleared) -> C acquires ForceControl
//   belief with contested=true -> C relocates to OrchardFarm -> C tells D
//   -> D learns ForceControllerOf with contested=true.
// ===========================================================================

#[allow(clippy::too_many_lines)]
fn run_contested_force_state_propagates_through_belief_system(
    seed: Seed,
) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();
    h.enable_politics_tracing();

    let orchard_farm = prototype_place_entity(PrototypePlace::OrchardFarm);

    // -- Agent A ("Claimant Alpha"): human-controlled at VillageSquare --
    let claimant_alpha = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Claimant Alpha",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        claimant_alpha,
        default_perception_profile(),
    );
    set_control_source(&mut h, claimant_alpha, ControlSource::Human, 0);

    // -- Agent B ("Claimant Beta"): human-controlled at VillageSquare --
    let claimant_beta = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Claimant Beta",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        claimant_beta,
        default_perception_profile(),
    );
    set_control_source(&mut h, claimant_beta, ControlSource::Human, 0);

    // -- Agent C ("Witness"): AI-controlled, social-weighted for Tell --
    let witness = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Witness",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            social_weight: pm(600),
            enterprise_weight: pm(0),
            ..UtilityProfile::default()
        },
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
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        witness,
        broad_accepting_tell_profile(),
    );

    // -- Agent D ("Remote Listener"): at OrchardFarm, passive reception --
    let remote_listener = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Remote Listener",
        orchard_farm,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        remote_listener,
        default_perception_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        remote_listener,
        accepting_tell_profile(),
    );

    // -- Force-law office at VillageSquare --
    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "War Chief",
        VILLAGE_SQUARE,
        SuccessionLaw::Force,
        5,
        vec![],
    );

    // Seed beliefs: C knows about A, B, and D (at OrchardFarm).
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        witness,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        witness,
        &[remote_listener],
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    // C knows about the office at VillageSquare.
    seed_known_office_at_place(
        &mut h.world,
        &mut h.event_log,
        witness,
        office,
        VILLAGE_SQUARE,
        Tick(0),
    );

    // D starts with no force-control institutional belief.
    let d_store_before = h
        .world
        .get_component_agent_belief_store(remote_listener)
        .cloned()
        .unwrap_or_else(worldwake_core::AgentBeliefStore::new);
    assert!(
        matches!(
            d_store_before.believed_force_controller(office),
            InstitutionalBeliefRead::Unknown
        ),
        "remote listener must start without any force-control belief"
    );

    // =====================================================================
    // Phase 1: A claims, becomes controller, then B claims → contested
    // =====================================================================

    let press_force_claim_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "press_force_claim")
        .map(|def| def.id)
        .expect("full registries should include press_force_claim");

    // A presses force claim on vacant office.
    let claim_tick_a = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        claim_tick_a,
        InputKind::RequestAction {
            actor: claimant_alpha,
            def_id: press_force_claim_def_id,
            targets: Vec::new(),
            payload_override: Some(ActionPayload::PressForceClaim(
                PressForceClaimActionPayload { office },
            )),
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );

    // Tick until A is established as sole controller.
    let mut alpha_controller = false;
    for _ in 0..10 {
        h.step_once();
        if h.world.office_controller(office) == Some(claimant_alpha) {
            alpha_controller = true;
            break;
        }
    }
    assert!(
        alpha_controller,
        "claimant alpha should become sole controller after pressing force claim on vacant office"
    );

    // B presses force claim, creating contested state.
    let claim_tick_b = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        claim_tick_b,
        InputKind::RequestAction {
            actor: claimant_beta,
            def_id: press_force_claim_def_id,
            targets: Vec::new(),
            payload_override: Some(ActionPayload::PressForceClaim(
                PressForceClaimActionPayload { office },
            )),
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );

    // Tick until B's claim commits and contested state activates.
    let mut beta_committed = false;
    for _ in 0..10 {
        h.step_once();
        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");
        if action_sink.events_for(claimant_beta).iter().any(|event| {
            event.action_name == "press_force_claim"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        }) {
            beta_committed = true;
            break;
        }
    }
    assert!(
        beta_committed,
        "claimant beta's press_force_claim should commit within 10 ticks"
    );

    // Run a few more ticks for the succession system to detect contested state.
    for _ in 0..5 {
        h.step_once();
    }

    // Verify contested state: office_controller == None.
    assert_eq!(
        h.world.office_controller(office),
        None,
        "office_controller must be None while the office is contested by two claimants"
    );

    // Verify contested_since is set on the force state.
    let force_state = h
        .world
        .get_component_office_force_state(office)
        .expect("force-law office should have OfficeForceState");
    assert!(
        force_state.contested_since.is_some(),
        "OfficeForceState.contested_since must be set when two claimants contest the office"
    );

    // Verify office_holder is NOT set during contested phase.
    assert_eq!(
        h.world.office_holder(office),
        None,
        "office_holder must NOT be set while the office is contested"
    );

    // Verify politics trace contains ForceContested { claimant_count: 2 }.
    let politics_sink = h
        .politics_trace_sink()
        .expect("politics tracing should be enabled");
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
        "politics trace should contain ForceContested with claimant_count == 2"
    );

    // =====================================================================
    // Phase 2: Verify C's belief includes contested == true
    // =====================================================================

    // Seed C's ForceControl institutional belief with contested=true.
    // Politics runs before Perception in the tick loop, so C may have
    // already observed A's controller establishment (uncontested).
    // Clear the stale entry before seeding the updated contested belief
    // to avoid a Conflicted read from two contradicting observations.
    let belief_tick = h.scheduler.current_tick();
    {
        let mut store = h
            .world
            .get_component_agent_belief_store(witness)
            .cloned()
            .unwrap_or_else(worldwake_core::AgentBeliefStore::new);
        store
            .institutional_beliefs
            .remove(&InstitutionalBeliefKey::ForceControllerOf { office });
        let mut txn = new_txn(&mut h.world, belief_tick.0);
        txn.set_component_agent_belief_store(witness, store)
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    seed_force_controller_belief(
        &mut h.world,
        &mut h.event_log,
        witness,
        office,
        None, // no sole controller during contested state
        true, // contested == true
        belief_tick,
        Some(VILLAGE_SQUARE),
    );
    let c_store = h
        .world
        .get_component_agent_belief_store(witness)
        .expect("witness should have a belief store after seeding");
    assert_eq!(
        c_store.believed_force_controller(office),
        InstitutionalBeliefRead::Certain((None, true)),
        "witness should have ForceControllerOf belief with contested == true and no controller"
    );

    // D still has no belief after Phase 2.
    let d_store_phase2 = h
        .world
        .get_component_agent_belief_store(remote_listener)
        .expect("remote listener should retain a belief store");
    assert!(
        matches!(
            d_store_phase2.believed_force_controller(office),
            InstitutionalBeliefRead::Unknown
        ),
        "remote listener must remain ignorant after Phase 2 (no physical carrier yet)"
    );

    // =====================================================================
    // Phase 3: C moves to OrchardFarm and Tells D
    // =====================================================================

    // Relocate C to OrchardFarm so C's AI can generate ShareBelief for D.
    {
        let travel_tick = h.scheduler.current_tick();
        let mut txn = new_txn(&mut h.world, travel_tick.0);
        txn.set_ground_location(witness, orchard_farm).unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    assert_eq!(
        h.world.effective_place(witness),
        Some(orchard_farm),
        "witness should be at OrchardFarm before the tell phase"
    );

    // D must still be ignorant after C arrives but before Tell.
    let d_store_pre_tell = h
        .world
        .get_component_agent_belief_store(remote_listener)
        .expect("remote listener should retain a belief store");
    assert!(
        matches!(
            d_store_pre_tell.believed_force_controller(office),
            InstitutionalBeliefRead::Unknown
        ),
        "remote listener must remain ignorant until C's tell commits \
         (physical presence alone is not enough)"
    );

    // C needs beliefs about D for ShareBelief to fire now that C is
    // co-located with D.
    let co_locate_tick = h.scheduler.current_tick();
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        witness,
        co_locate_tick,
        PerceptionSource::DirectObservation,
    );

    // Tick until D acquires the force-control belief via Tell from C.
    let mut d_learned = false;
    for _ in 0..80 {
        h.step_once();
        if let Some(store) = h.world.get_component_agent_belief_store(remote_listener)
            && !matches!(
                store.believed_force_controller(office),
                InstitutionalBeliefRead::Unknown
            )
        {
            d_learned = true;
            break;
        }
    }
    assert!(
        d_learned,
        "remote listener (D) should learn ForceControllerOf belief from witness (C) \
         via Tell after C arrives at OrchardFarm"
    );

    // =====================================================================
    // Final assertions
    // =====================================================================

    // D's belief must carry contested == true.
    let d_store_final = h
        .world
        .get_component_agent_belief_store(remote_listener)
        .expect("remote listener should have a belief store after tell");
    assert_eq!(
        d_store_final.believed_force_controller(office),
        InstitutionalBeliefRead::Certain((None, true)),
        "remote listener should believe the office is contested (controller=None, contested=true)"
    );

    // Action ordering: A's press_force_claim commits before C's tell to D.
    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");

    let alpha_claim_commit = action_sink
        .events_for(claimant_alpha)
        .iter()
        .find_map(|event| {
            (event.action_name == "press_force_claim"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .expect("claimant alpha should have committed press_force_claim");

    let witness_tell_commit = action_sink
        .events_for(witness)
        .iter()
        .find_map(|event| {
            (event.action_name == "tell"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && matches!(
                    event.detail,
                    Some(ActionTraceDetail::Tell {
                        listener: told_listener,
                        topic,
                    }) if told_listener == remote_listener
                        && topic == TellTopic::InstitutionalClaim {
                            claim: InstitutionalClaim::ForceControl {
                                office,
                                controller: None,
                                contested: true,
                                effective_tick: Tick(8),
                            },
                        }
                ))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .expect(
            "witness should commit a tell action relaying contested force-control knowledge to remote listener",
        );

    assert!(
        alpha_claim_commit < witness_tell_commit,
        "press_force_claim must commit before witness tells remote listener \
         (claim={alpha_claim_commit:?}, tell={witness_tell_commit:?})",
    );

    // Decision trace: C generates ShareBelief candidate for the office.
    let trace_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled");

    let share_belief_goal = GoalKind::ShareBelief {
        listener: remote_listener,
        topic: TellTopic::InstitutionalClaim {
            claim: InstitutionalClaim::ForceControl {
                office,
                controller: None,
                contested: true,
                effective_tick: Tick(8),
            },
        },
        communication_class: worldwake_core::CommunicationClass::Testimony,
    };
    let share_history = trace_sink.goal_history_for(witness, &share_belief_goal);
    assert!(
        share_history
            .iter()
            .any(|entry| entry.status.is_generated()),
        "witness should generate ShareBelief goal for the contested force-control claim \
         targeting the remote listener"
    );

    // office_holder must NOT be set at any point during the contested phase.
    assert_eq!(
        h.world.office_holder(office),
        None,
        "office_holder must remain None throughout the contested phase — \
         no holder can be installed while multiple force claims are active"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_contested_force_state_propagates_through_belief_system() {
    let _ = run_contested_force_state_propagates_through_belief_system(Seed([53; 32]));
}

#[test]
fn golden_contested_force_state_propagates_through_belief_system_replays_deterministically() {
    let first = run_contested_force_state_propagates_through_belief_system(Seed([54; 32]));
    let second = run_contested_force_state_propagates_through_belief_system(Seed([54; 32]));
    assert_eq!(
        first, second,
        "contested force state belief propagation should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 35: Same-Place Concurrent Violations Stay Distinct
// ---------------------------------------------------------------------------
//
// Systems: Perception, AI, Generic Actions
// GoalKinds: InvestigateViolation
// ActionDomains: Generic
// Places: VillageSquare, OrchardFarm
// Principles: 7, 9, 15
//
// Proves two same-place violated expectations remain distinct incidents through
// planning, start binding, first commit aftermath, and sibling follow-up.

#[allow(clippy::too_many_lines)]
fn run_same_place_concurrent_violation_lifecycle(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.enable_action_tracing();
    h.driver.enable_tracing();

    let investigator = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Investigator",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            social_weight: pm(0),
            ..UtilityProfile::default()
        },
    );
    let first_missing = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "FirstMissing",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let second_missing = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "SecondMissing",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, first_missing, ControlSource::None, 0);
    set_control_source(&mut h, second_missing, ControlSource::None, 0);

    let mut keen = default_perception_profile();
    keen.observation_fidelity = pm(1000);
    set_agent_perception_profile(&mut h.world, &mut h.event_log, investigator, keen);
    set_violation_profile(
        &mut h,
        investigator,
        ViolationDispositionProfile {
            investigation_duration_ticks: nz(2),
            violation_memory_retention_ticks: 50,
            investigation_motive_weight: pm(1000),
            ownership_motive_bonus: pm(0),
        },
        0,
    );

    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        investigator,
        first_missing,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        investigator,
        second_missing,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_ground_location(first_missing, ORCHARD_FARM)
            .unwrap();
        txn.set_ground_location(second_missing, ORCHARD_FARM)
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    h.step_once();

    let first_trace = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(investigator, Tick(0))
        .expect("tick 0 decision trace should exist");
    let first_planning = match &first_trace.outcome {
        DecisionOutcome::Planning(planning) => planning.as_ref(),
        other => panic!("expected planning trace at tick 0, got {other:?}"),
    };

    let generated_violation_ids = first_planning
        .candidates
        .generated
        .iter()
        .filter_map(|goal| match goal.goal_key.kind {
            GoalKind::InvestigateViolation {
                violation_id,
                place,
            } if place == VILLAGE_SQUARE => Some(violation_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        generated_violation_ids.len(),
        2,
        "tick 0 should generate two same-place investigate candidates"
    );
    assert_ne!(
        generated_violation_ids[0], generated_violation_ids[1],
        "same-place incidents must keep distinct violation ids"
    );

    let initial_selected_violation_id = match first_planning
        .selection
        .selected_goal()
        .expect("one investigate goal should be selected")
        .kind
    {
        GoalKind::InvestigateViolation {
            violation_id,
            place,
        } => {
            assert_eq!(place, VILLAGE_SQUARE);
            violation_id
        }
        other => panic!("expected InvestigateViolation selection, got {other:?}"),
    };
    assert_eq!(
        first_planning
            .selection
            .selected_plan
            .as_ref()
            .and_then(|plan| plan.next_step.as_ref())
            .map(|step| step.action_name.as_str()),
        Some("investigate"),
        "the winning plan should bind an investigate step for the selected violation",
    );
    assert!(
        generated_violation_ids.contains(&initial_selected_violation_id),
        "the initial selected investigate goal must come from the generated same-place incident set",
    );

    let mut first_committed_violation_id = None;
    for _ in 0..8 {
        h.step_once();
        let events = h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(investigator);
        first_committed_violation_id = events.iter().find_map(|event| {
            (event.action_name == "investigate"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then(|| match event.detail {
                Some(ActionTraceDetail::Investigate { violation_id }) => Some(violation_id),
                _ => None,
            })
            .flatten()
        });
        if first_committed_violation_id.is_some() {
            break;
        }
    }
    assert!(
        first_committed_violation_id.is_some(),
        "the first same-place violation should commit investigation",
    );
    let first_committed_violation_id =
        first_committed_violation_id.expect("first investigate commit id should be recorded");
    assert!(
        generated_violation_ids.contains(&first_committed_violation_id),
        "the first committed investigate must bind one of the generated same-place violation ids",
    );
    let sibling_violation_id = generated_violation_ids
        .iter()
        .copied()
        .find(|id| *id != first_committed_violation_id)
        .expect("one sibling violation id should remain after the first commit");

    let memory_after_first_commit = h
        .world
        .get_component_violation_memory(investigator)
        .expect("investigator should have violation memory");
    let selected_record = memory_after_first_commit
        .violations
        .iter()
        .find(|record| record.id == first_committed_violation_id)
        .expect("first committed violation should remain recorded");
    let sibling_record = memory_after_first_commit
        .violations
        .iter()
        .find(|record| record.id == sibling_violation_id)
        .expect("sibling violation should remain recorded");
    assert!(
        selected_record.resolved_tick.is_some(),
        "selected violation should be resolved after the first commit",
    );
    assert_eq!(
        sibling_record.resolved_tick, None,
        "sibling violation should remain unresolved after the first commit",
    );

    let selected_subject = match &selected_record.kind {
        worldwake_core::ViolationKind::EntityMissing { entity, .. } => *entity,
        other => panic!("expected entity-missing violation, got {other:?}"),
    };
    let sibling_subject = match &sibling_record.kind {
        worldwake_core::ViolationKind::EntityMissing { entity, .. } => *entity,
        other => panic!("expected entity-missing violation, got {other:?}"),
    };

    let store_after_first_commit = h
        .world
        .get_component_agent_belief_store(investigator)
        .expect("investigator should keep a belief store");
    let first_absences = store_after_first_commit
        .social_observations
        .iter()
        .filter(|observation| {
            observation.kind() == worldwake_core::SocialObservationKind::WitnessedAbsence
        })
        .collect::<Vec<_>>();
    assert_eq!(
        first_absences.len(),
        1,
        "only the selected violation should leave aftermath after the first commit",
    );
    assert_eq!(
        first_absences[0].detail,
        worldwake_core::SocialObservationDetail::WitnessedAbsence {
            missing_entity: selected_subject,
            expected_place: VILLAGE_SQUARE,
        },
        "the first aftermath artifact must belong to the selected violation",
    );

    let mut sibling_selected = false;
    let mut sibling_committed = false;
    for _ in 0..8 {
        h.step_once();
        let just_ran_tick = Tick(h.scheduler.current_tick().0.saturating_sub(1));

        let trace = h
            .driver
            .trace_sink()
            .expect("decision tracing should be enabled")
            .trace_at(investigator, just_ran_tick)
            .expect("decision trace should exist for each stepped tick");
        if let DecisionOutcome::Planning(planning) = &trace.outcome
            && planning.selection.selected_goal_is(
                GoalKind::InvestigateViolation {
                    violation_id: sibling_violation_id,
                    place: VILLAGE_SQUARE,
                }
                .into(),
            )
        {
            sibling_selected = true;
        }

        let committed_ids = h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(investigator)
            .into_iter()
            .filter_map(|event| {
                if event.action_name != "investigate"
                    || !matches!(event.kind, ActionTraceKind::Committed { .. })
                {
                    return None;
                }
                match event.detail {
                    Some(ActionTraceDetail::Investigate { violation_id }) => Some(violation_id),
                    _ => None,
                }
            })
            .collect::<Vec<_>>();
        sibling_committed = committed_ids.contains(&sibling_violation_id);
        if committed_ids.len() >= 2 && sibling_committed {
            break;
        }
    }

    assert!(
        sibling_selected,
        "the sibling same-place violation should later be selected by its own id",
    );
    assert!(
        sibling_committed,
        "the sibling same-place violation should later commit its own investigation",
    );

    let final_store = h
        .world
        .get_component_agent_belief_store(investigator)
        .expect("investigator should keep a belief store");
    let final_absence_subjects = final_store
        .social_observations
        .iter()
        .filter(|observation| {
            observation.kind() == worldwake_core::SocialObservationKind::WitnessedAbsence
        })
        .map(|observation| observation.detail)
        .collect::<Vec<_>>();
    assert_eq!(
        final_absence_subjects.len(),
        2,
        "both same-place incidents should leave distinct aftermath artifacts",
    );
    assert!(
        final_absence_subjects.contains(
            &worldwake_core::SocialObservationDetail::WitnessedAbsence {
                missing_entity: selected_subject,
                expected_place: VILLAGE_SQUARE,
            }
        ),
        "selected violation aftermath should persist",
    );
    assert!(
        final_absence_subjects.contains(
            &worldwake_core::SocialObservationDetail::WitnessedAbsence {
                missing_entity: sibling_subject,
                expected_place: VILLAGE_SQUARE,
            }
        ),
        "sibling violation aftermath should be recorded after its own commit",
    );

    let final_memory = h
        .world
        .get_component_violation_memory(investigator)
        .expect("investigator should have violation memory");
    assert!(
        final_memory
            .violations
            .iter()
            .filter(|record| record.resolved_tick.is_some())
            .count()
            >= 2,
        "both recorded violations should be resolved by the end of the scenario",
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_same_place_concurrent_violations_stay_distinct() {
    let _ = run_same_place_concurrent_violation_lifecycle(Seed([55; 32]));
}

#[test]
fn golden_same_place_concurrent_violations_stay_distinct_replays_deterministically() {
    let first = run_same_place_concurrent_violation_lifecycle(Seed([56; 32]));
    let second = run_same_place_concurrent_violation_lifecycle(Seed([56; 32]));
    assert_eq!(
        first, second,
        "same-place concurrent violation scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 36: Entity Missing Triggers Investigation
// ---------------------------------------------------------------------------
//
// Systems: Perception, AI, Generic Actions
// GoalKinds: InvestigateViolation
// ActionDomains: Generic
// Places: VillageSquare, OrchardFarm
// Principles: 7, 9, 15
//
// Proves the baseline single-incident violation pipeline: stale local belief,
// mismatch detection, investigate planning, committed investigate aftermath,
// and exact violation-id resolution.

#[allow(clippy::too_many_lines)]
fn run_entity_missing_triggers_investigation(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.enable_action_tracing();
    h.driver.enable_tracing();

    let investigator = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Investigator",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            social_weight: pm(0),
            ..UtilityProfile::default()
        },
    );
    let missing_subject = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "MissingSubject",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, missing_subject, ControlSource::None, 0);

    let mut keen = default_perception_profile();
    keen.observation_fidelity = pm(1000);
    set_agent_perception_profile(&mut h.world, &mut h.event_log, investigator, keen);
    set_violation_profile(
        &mut h,
        investigator,
        ViolationDispositionProfile {
            investigation_duration_ticks: nz(2),
            violation_memory_retention_ticks: 50,
            investigation_motive_weight: pm(1000),
            ownership_motive_bonus: pm(0),
        },
        0,
    );

    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        investigator,
        missing_subject,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_ground_location(missing_subject, ORCHARD_FARM)
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    h.step_once();

    let first_trace = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(investigator, Tick(0))
        .expect("tick 0 decision trace should exist");
    let first_planning = match &first_trace.outcome {
        DecisionOutcome::Planning(planning) => planning.as_ref(),
        other => panic!("expected planning trace at tick 0, got {other:?}"),
    };

    let generated_goal = first_planning
        .candidates
        .generated
        .iter()
        .find_map(|goal| match goal.goal_key.kind {
            GoalKind::InvestigateViolation {
                violation_id,
                place,
            } if place == VILLAGE_SQUARE => Some((violation_id, place)),
            _ => None,
        })
        .expect("tick 0 should generate an investigate goal for the missing entity");
    let generated_violation_id = generated_goal.0;

    assert!(
        first_planning.selection.selected_goal_is(
            GoalKind::InvestigateViolation {
                violation_id: generated_violation_id,
                place: VILLAGE_SQUARE,
            }
            .into()
        ),
        "the missing-entity violation should be selected for investigation",
    );
    assert_eq!(
        first_planning
            .selection
            .selected_plan
            .as_ref()
            .and_then(|plan| plan.next_step.as_ref())
            .map(|step| step.action_name.as_str()),
        Some("investigate"),
        "the selected violation should bind an investigate step",
    );

    let memory_after_detection = h
        .world
        .get_component_violation_memory(investigator)
        .expect("investigator should have violation memory");
    let detected_record = memory_after_detection
        .violations
        .iter()
        .find(|record| record.id == generated_violation_id)
        .expect("generated violation should be recorded in memory");
    match &detected_record.kind {
        worldwake_core::ViolationKind::EntityMissing {
            entity,
            expected_place,
        } => {
            assert_eq!(*entity, missing_subject);
            assert_eq!(*expected_place, VILLAGE_SQUARE);
        }
        other => panic!("expected entity-missing violation, got {other:?}"),
    }
    assert_eq!(
        detected_record.resolved_tick, None,
        "the violation should remain unresolved until investigate commits",
    );

    let mut committed_violation_id = None;
    for _ in 0..8 {
        h.step_once();
        let events = h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(investigator);
        committed_violation_id = events.iter().find_map(|event| {
            (event.action_name == "investigate"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then(|| match event.detail {
                Some(ActionTraceDetail::Investigate { violation_id }) => Some(violation_id),
                _ => None,
            })
            .flatten()
        });
        if committed_violation_id.is_some() {
            break;
        }
    }

    assert_eq!(
        committed_violation_id,
        Some(generated_violation_id),
        "the committed investigate action should resolve the same violation id that planning selected",
    );

    let final_store = h
        .world
        .get_component_agent_belief_store(investigator)
        .expect("investigator should keep a belief store");
    let witnessed_absences = final_store
        .social_observations
        .iter()
        .filter(|observation| {
            observation.kind() == worldwake_core::SocialObservationKind::WitnessedAbsence
        })
        .collect::<Vec<_>>();
    assert_eq!(
        witnessed_absences.len(),
        1,
        "the single committed investigate should leave one witnessed-absence artifact",
    );
    assert_eq!(
        witnessed_absences[0].detail,
        worldwake_core::SocialObservationDetail::WitnessedAbsence {
            missing_entity: missing_subject,
            expected_place: VILLAGE_SQUARE,
        },
        "the aftermath artifact should identify the missing subject and expected place",
    );

    let final_memory = h
        .world
        .get_component_violation_memory(investigator)
        .expect("investigator should have violation memory");
    let resolved_record = final_memory
        .violations
        .iter()
        .find(|record| record.id == generated_violation_id)
        .expect("the investigated violation should remain in memory after resolution");
    assert!(
        resolved_record.resolved_tick.is_some(),
        "investigate commit should resolve the selected violation record",
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_entity_missing_triggers_investigation() {
    let _ = run_entity_missing_triggers_investigation(Seed([57; 32]));
}

#[test]
fn golden_entity_missing_triggers_investigation_replays_deterministically() {
    let first = run_entity_missing_triggers_investigation(Seed([58; 32]));
    let second = run_entity_missing_triggers_investigation(Seed([58; 32]));
    assert_eq!(
        first, second,
        "entity-missing investigation scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 37: Theft Leads Owner To Local Suspected Theft Discovery
// ---------------------------------------------------------------------------
//
// Systems: Transport, Perception, AI, Generic Actions
// GoalKinds: StealItem, InvestigateViolation
// ActionDomains: Transport, Generic, Travel
// Places: VillageSquare, GeneralStore, CommonHouse
// Principles: 3, 7, 12, 15
//
// Proves the canonical owner-local theft-discovery path under the live
// architecture: thief steals, departs with the lot, owner later returns
// lawfully, mismatch detection records EntityMissing, and investigate upgrades
// the owner's discovery into typed SuspectedTheft evidence.

#[allow(clippy::too_many_lines)]
fn run_theft_leads_owner_to_local_suspected_theft_discovery(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.enable_action_tracing();
    h.driver.enable_tracing();

    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);
    let common_house = prototype_place_entity(PrototypePlace::CommonHouse);

    let thief = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Thief",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let owner = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Owner",
        general_store,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            social_weight: pm(0),
            enterprise_weight: pm(0),
            ..UtilityProfile::default()
        },
    );
    set_control_source(&mut h, owner, ControlSource::Human, 0);

    let mut sharp_perception = default_perception_profile();
    sharp_perception.observation_fidelity = pm(1000);
    set_agent_perception_profile(&mut h.world, &mut h.event_log, thief, sharp_perception);
    set_agent_perception_profile(&mut h.world, &mut h.event_log, owner, sharp_perception);

    set_theft_profile(
        &mut h,
        thief,
        TheftDispositionProfile {
            steal_duration_ticks: nz(2),
            theft_motive_weight: pm(900),
            witness_risk_penalty: pm(400),
        },
        0,
    );
    set_violation_profile(
        &mut h,
        owner,
        ViolationDispositionProfile {
            investigation_duration_ticks: nz(2),
            violation_memory_retention_ticks: 50,
            investigation_motive_weight: pm(1000),
            ownership_motive_bonus: pm(400),
        },
        0,
    );

    let stolen_lot = {
        let mut txn = new_txn(&mut h.world, 0);
        let lot = txn
            .create_item_lot(CommodityKind::Bread, Quantity(2))
            .unwrap();
        txn.set_ground_location(lot, VILLAGE_SQUARE).unwrap();
        txn.set_owner(lot, owner).unwrap();
        commit_txn(txn, &mut h.event_log);
        lot
    };
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        thief,
        stolen_lot,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief(
        &mut h.world,
        &mut h.event_log,
        owner,
        stolen_lot,
        BelievedEntityState {
            last_known_place: Some(VILLAGE_SQUARE),
            last_known_inventory: std::collections::BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            observed_tick: Tick(0),
            source: PerceptionSource::DirectObservation,
        },
    );

    verify_live_lot_conservation(&h.world, CommodityKind::Bread, 2).unwrap();

    let mut steal_committed = false;
    for _ in 0..12 {
        h.step_once();
        verify_live_lot_conservation(&h.world, CommodityKind::Bread, 2).unwrap();
        steal_committed = h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(thief)
            .iter()
            .any(|event| {
                event.action_name == "steal"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            });
        if steal_committed {
            break;
        }
    }

    assert!(steal_committed, "thief should commit a steal action");
    assert_eq!(
        h.world.possessor_of(stolen_lot),
        Some(thief),
        "steal should transfer possession to the thief",
    );
    assert_eq!(
        h.world.owner_of(stolen_lot),
        Some(owner),
        "steal should preserve ownership for the original owner",
    );

    let travel_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "travel")
        .map(|def| def.id)
        .expect("full registries should include travel");

    let thief_departure_tick = h.scheduler.current_tick();
    set_control_source(&mut h, thief, ControlSource::Human, thief_departure_tick.0);
    let _ = h.scheduler.input_queue_mut().enqueue(
        thief_departure_tick,
        InputKind::RequestAction {
            actor: thief,
            def_id: travel_def_id,
            targets: vec![common_house],
            payload_override: None,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
    h.step_once();
    verify_live_lot_conservation(&h.world, CommodityKind::Bread, 2).unwrap();

    assert_eq!(
        h.world.effective_place(thief),
        Some(common_house),
        "thief should depart the stash before the owner returns",
    );
    assert_eq!(
        h.world.effective_place(stolen_lot),
        Some(common_house),
        "the stolen lot should leave the stash with the thief",
    );
    assert!(
        h.action_trace_sink()
            .expect("action tracing should stay enabled")
            .events_for(thief)
            .iter()
            .any(|event| {
                event.action_name == "travel"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            }),
        "thief departure should commit as a lawful travel action",
    );

    let owner_return_tick = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        owner_return_tick,
        InputKind::RequestAction {
            actor: owner,
            def_id: travel_def_id,
            targets: vec![VILLAGE_SQUARE],
            payload_override: None,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
    h.step_once();
    verify_live_lot_conservation(&h.world, CommodityKind::Bread, 2).unwrap();

    assert_eq!(
        h.world.effective_place(owner),
        Some(VILLAGE_SQUARE),
        "owner should return to the expected stash place lawfully via travel",
    );
    assert!(
        h.action_trace_sink()
            .expect("action tracing should stay enabled")
            .events_for(owner)
            .iter()
            .any(|event| {
                event.action_name == "travel"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            }),
        "owner return should commit as a lawful travel action",
    );

    let detection_tick = h.scheduler.current_tick();
    set_control_source(&mut h, owner, ControlSource::Ai, detection_tick.0);
    h.step_once();
    verify_live_lot_conservation(&h.world, CommodityKind::Bread, 2).unwrap();

    let detection_trace = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(owner, detection_tick)
        .expect("owner should produce a decision trace on the arrival tick");
    let detection_planning = match &detection_trace.outcome {
        DecisionOutcome::Planning(planning) => planning.as_ref(),
        other => panic!("expected planning trace after owner arrival, got {other:?}"),
    };

    let generated_violation_id = detection_planning
        .candidates
        .generated
        .iter()
        .find_map(|goal| match goal.goal_key.kind {
            GoalKind::InvestigateViolation {
                violation_id,
                place,
            } if place == VILLAGE_SQUARE => Some(violation_id),
            _ => None,
        })
        .expect("owner arrival should generate an investigate goal for the missing owned lot");
    assert!(
        detection_planning.selection.selected_goal_is(
            GoalKind::InvestigateViolation {
                violation_id: generated_violation_id,
                place: VILLAGE_SQUARE,
            }
            .into()
        ),
        "owner should select the local investigate goal after discovering the missing lot",
    );

    let memory_after_detection = h
        .world
        .get_component_violation_memory(owner)
        .expect("owner should have violation memory");
    let detected_record = memory_after_detection
        .violations
        .iter()
        .find(|record| record.id == generated_violation_id)
        .expect("generated violation should be recorded");
    match &detected_record.kind {
        worldwake_core::ViolationKind::EntityMissing {
            entity,
            expected_place,
        } => {
            assert_eq!(*entity, stolen_lot);
            assert_eq!(*expected_place, VILLAGE_SQUARE);
        }
        other => panic!("expected EntityMissing record after owner arrival, got {other:?}"),
    }
    assert_eq!(
        detected_record.resolved_tick, None,
        "the arrival-triggered violation should remain unresolved until investigate commits",
    );

    let expected_theft = TheftFacts {
        missing_entity: stolen_lot,
        expected_place: VILLAGE_SQUARE,
        commodity: CommodityKind::Bread,
        quantity: Quantity(2),
    };

    let mut investigate_committed = false;
    for _ in 0..10 {
        h.step_once();
        verify_live_lot_conservation(&h.world, CommodityKind::Bread, 2).unwrap();
        investigate_committed = h
            .action_trace_sink()
            .expect("action tracing should remain enabled")
            .events_for(owner)
            .iter()
            .any(|event| {
                event.action_name == "investigate"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
                    && event.detail
                        == Some(ActionTraceDetail::Investigate {
                            violation_id: generated_violation_id,
                        })
            });
        if investigate_committed {
            break;
        }
    }

    assert!(
        investigate_committed,
        "owner should commit investigate for the recorded missing-entity violation",
    );

    let final_store = h
        .world
        .get_component_agent_belief_store(owner)
        .expect("owner should retain a belief store");
    assert!(
        final_store.social_observations.iter().any(|observation| {
            observation.detail
                == worldwake_core::SocialObservationDetail::SuspectedTheft {
                    theft: expected_theft,
                    suspect: None,
                }
                && observation.place == VILLAGE_SQUARE
        }),
        "owner investigate aftermath should record typed suspected-theft evidence",
    );

    let final_memory = h
        .world
        .get_component_violation_memory(owner)
        .expect("owner should retain violation memory");
    assert!(final_memory.violations.iter().any(|record| {
        record.id == generated_violation_id
            && record.kind
                == worldwake_core::ViolationKind::EntityMissing {
                    entity: stolen_lot,
                    expected_place: VILLAGE_SQUARE,
                }
            && record.resolved_tick.is_some()
    }));
    assert!(final_memory.violations.iter().any(|record| {
        record.kind
            == worldwake_core::ViolationKind::SuspectedTheft {
                theft: expected_theft,
                suspect: None,
            }
            && record.resolved_tick.is_none()
    }));

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_theft_leads_owner_to_local_suspected_theft_discovery() {
    let _ = run_theft_leads_owner_to_local_suspected_theft_discovery(Seed([61; 32]));
}

#[test]
fn golden_theft_leads_owner_to_local_suspected_theft_discovery_replays_deterministically() {
    let first = run_theft_leads_owner_to_local_suspected_theft_discovery(Seed([62; 32]));
    let second = run_theft_leads_owner_to_local_suspected_theft_discovery(Seed([62; 32]));
    assert_eq!(
        first, second,
        "theft-to-owner-discovery scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 38: Witnessed Theft Enables Accusation Chain
// ---------------------------------------------------------------------------
//
// Systems: Transport, Perception, Social Tell, AI, Institutions
// GoalKinds: StealItem, ShareBelief, Accuse, PunishAccused
// ActionDomains: Transport, Social, Travel
// Places: VillageSquare, RulersHall
// Principles: 1, 7, 13, 16, 21, 23
//
// Proves the intended justice architecture after the accusation refactor:
// witnessed theft becomes relayable social evidence, Tell creates a concrete
// local case for the authority, accusation is filed at the CrimeRegister, and
// punishment happens later through the recorded case rather than a fused
// same-place confrontation path.

#[allow(clippy::too_many_lines)]
fn run_witnessed_theft_accusation_chain(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let rulers_hall = prototype_place_entity(PrototypePlace::RulersHall);

    let thief = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Thief",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            social_weight: pm(0),
            enterprise_weight: pm(0),
            ..UtilityProfile::default()
        },
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
    let victim = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Victim",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let authority = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Magistrate",
        rulers_hall,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            social_weight: pm(0),
            enterprise_weight: pm(0),
            ..UtilityProfile::default()
        },
    );
    set_control_source(&mut h, victim, ControlSource::Human, 0);

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
        authority,
        default_perception_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        witness,
        broad_accepting_tell_profile(),
    );
    set_violation_profile(
        &mut h,
        authority,
        ViolationDispositionProfile {
            investigation_duration_ticks: nz(1),
            violation_memory_retention_ticks: 80,
            investigation_motive_weight: pm(600),
            ownership_motive_bonus: pm(0),
        },
        0,
    );
    set_theft_profile(
        &mut h,
        thief,
        TheftDispositionProfile {
            steal_duration_ticks: nz(2),
            theft_motive_weight: pm(1000),
            witness_risk_penalty: pm(0),
        },
        0,
    );
    set_justice_profile(
        &mut h,
        authority,
        JusticeDispositionProfile {
            accusation_motive_weight: pm(900),
            fine_severity: pm(500),
        },
        0,
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_intention_disposition_profile(
            authority,
            IntentionDispositionProfile {
                commitment_switch_margin: pm(100),
                ..IntentionDispositionProfile::default()
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    let faction = {
        let mut txn = new_txn(&mut h.world, 0);
        let faction = txn.create_faction("Town Ward").unwrap();
        commit_txn(txn, &mut h.event_log);
        faction
    };
    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "Magistrate",
        rulers_hall,
        SuccessionLaw::Support,
        8,
        vec![worldwake_core::EligibilityRule::FactionMember(faction)],
    );
    let (crime_register, stolen_lot) = {
        let mut txn = new_txn(&mut h.world, 0);
        txn.assign_office(office, authority).unwrap();
        txn.add_member(thief, faction).unwrap();
        let crime_register = txn
            .create_record(RecordData {
                record_kind: RecordKind::CrimeRegister,
                home_place: rulers_hall,
                issuer: office,
                consultation_ticks: 1,
                max_entries_per_consult: 8,
                entries: Vec::new(),
                next_entry_id: 0,
            })
            .unwrap();
        let stolen_lot = txn
            .create_item_lot(CommodityKind::Bread, Quantity(2))
            .unwrap();
        txn.set_ground_location(stolen_lot, VILLAGE_SQUARE).unwrap();
        txn.set_owner(stolen_lot, victim).unwrap();
        commit_txn(txn, &mut h.event_log);
        (crime_register, stolen_lot)
    };

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        witness,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        witness,
        &[authority],
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        authority,
        thief,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        authority,
        crime_register,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        authority,
        office,
        Some(authority),
        Tick(0),
        InstitutionalKnowledgeSource::SelfDeclaration,
        Some(rulers_hall),
    );
    seed_faction_membership_belief(
        &mut h.world,
        &mut h.event_log,
        authority,
        faction,
        thief,
        true,
        Tick(0),
        InstitutionalKnowledgeSource::SelfDeclaration,
        Some(rulers_hall),
    );

    let theft_facts = TheftFacts {
        missing_entity: stolen_lot,
        expected_place: VILLAGE_SQUARE,
        commodity: CommodityKind::Bread,
        quantity: Quantity(2),
    };

    let mut steal_commit_tick = None;
    for _ in 0..12 {
        h.step_once();
        if h.action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(thief)
            .iter()
            .any(|event| {
                event.action_name == "steal"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            })
        {
            steal_commit_tick = Some(h.scheduler.current_tick());
            break;
        }
    }
    let steal_commit_tick = steal_commit_tick.expect("thief should commit the witnessed theft");
    set_control_source(&mut h, thief, ControlSource::Human, steal_commit_tick.0);

    assert!(
        h.world
            .get_component_agent_belief_store(witness)
            .expect("witness should keep a belief store")
            .social_observations
            .iter()
            .any(|observation| {
                observation.detail
                    == worldwake_core::SocialObservationDetail::SuspectedTheft {
                        theft: theft_facts,
                        suspect: Some(thief),
                    }
            }),
        "witness should record direct suspected-theft evidence with the thief identity"
    );

    {
        let relocate_tick = h.scheduler.current_tick();
        let mut txn = new_txn(&mut h.world, relocate_tick.0);
        txn.set_ground_location(witness, rulers_hall).unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        witness,
        h.scheduler.current_tick(),
        PerceptionSource::DirectObservation,
    );

    let accuse_goal = GoalKind::Accuse {
        crime_register,
        accused: thief,
        violation_id: worldwake_core::ViolationId(0),
    };
    let pre_tell_tick = h.scheduler.current_tick();
    let generated_before_tell = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .goal_history_for(authority, &accuse_goal)
        .into_iter()
        .filter(|entry| entry.tick <= pre_tell_tick)
        .any(|entry| entry.status.is_generated());
    assert!(
        !generated_before_tell,
        "authority should not generate an accusation goal before receiving the witness report"
    );

    let mut tell_commit_order = None;
    // 80-tick window: the witness needs to plan and execute a Tell action.
    // Budget tuning (max_candidates=2, TTL-based exhaustion skip) may cause
    // the Tell goal to take longer to surface as a top candidate.
    for _ in 0..80 {
        h.step_once();
        let store = h
            .world
            .get_component_agent_belief_store(authority)
            .expect("authority should keep a belief store");
        if store.social_observations.iter().any(|observation| {
            observation.detail
                == worldwake_core::SocialObservationDetail::SuspectedTheft {
                    theft: theft_facts,
                    suspect: Some(thief),
                }
                && matches!(
                    observation.source,
                    PerceptionSource::Report { from, chain_len: 1 } if from == witness
                )
        }) {
            let tell_event = h
                .action_trace_sink()
                .expect("action tracing should be enabled")
                .events_for(witness)
                .iter()
                .find_map(|event| {
                    (event.action_name == "tell"
                        && matches!(event.kind, ActionTraceKind::Committed { .. })
                        && matches!(
                            event.detail,
                            Some(ActionTraceDetail::Tell {
                                listener,
                                topic: TellTopic::SocialObservation { observation },
                            }) if listener == authority
                                && observation.detail
                                    == worldwake_core::SocialObservationDetail::SuspectedTheft {
                                        theft: theft_facts,
                                        suspect: Some(thief),
                                    }
                        ))
                    .then_some((event.tick, event.sequence_in_tick))
                })
                .expect("witness should commit Tell for the theft observation");
            tell_commit_order = Some(tell_event);
            break;
        }
    }
    let _tell_commit_order =
        tell_commit_order.expect("authority should learn the theft through Tell");
    let authority_store = h
        .world
        .get_component_agent_belief_store(authority)
        .expect("authority should keep a belief store after tell");
    assert!(
        authority_store.get_entity(&crime_register).is_some(),
        "authority should still know the local crime register when the witness report arrives"
    );
    assert_eq!(
        authority_store.believed_office_holder(office),
        InstitutionalBeliefRead::Certain(Some(authority)),
        "authority should still know they hold the office that issued the crime register"
    );

    let authority_memory = h
        .world
        .get_component_violation_memory(authority)
        .expect("authority should receive violation memory from the told theft evidence");
    authority_memory
        .violations
        .iter()
        .find(|record| {
            record.kind
                == worldwake_core::ViolationKind::SuspectedTheft {
                    theft: theft_facts,
                    suspect: Some(thief),
                }
        })
        .map(|record| record.id)
        .expect("authority should record a local suspected-theft case");

    let mut generated_after_report = false;
    for _ in 0..50 {
        h.step_once();
        let accuse_history = h
            .driver
            .trace_sink()
            .expect("decision tracing should be enabled")
            .goal_history_for(authority, &accuse_goal);
        if accuse_history
            .iter()
            .any(|entry| entry.tick > pre_tell_tick && entry.status.is_generated())
        {
            generated_after_report = true;
            break;
        }
    }
    assert!(
        generated_after_report,
        "authority should generate the office-filed accusation goal after Tell"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_witnessed_theft_accusation_chain() {
    let _ = run_witnessed_theft_accusation_chain(Seed([63; 32]));
}

#[test]
fn golden_witnessed_theft_accusation_chain_replays_deterministically() {
    let first = run_witnessed_theft_accusation_chain(Seed([64; 32]));
    let second = run_witnessed_theft_accusation_chain(Seed([64; 32]));
    assert_eq!(
        first, second,
        "witnessed-theft accusation chain should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 39: Traceability Explains Stale Fine Branch
// ---------------------------------------------------------------------------
//
// Systems: AI, Institutions, Justice, Action Trace
// GoalKinds: PunishAccused
// ActionDomains: Social
// Places: RulersHall, GeneralStore
// Principles: 3, 12, 24, 27
//
// Proves the mixed-layer debugging contract for justice punishment:
// planner-side trace records why `Fine` was selected, then authoritative
// start-failure trace records the concrete contradiction after the goods move
// away before start. The mismatch is inspectable from traces alone.

#[test]
fn golden_traceability_explains_stale_fine_branch_without_source_diving() {
    let mut h = GoldenHarness::new(Seed([39; 32]));
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let rulers_hall = prototype_place_entity(PrototypePlace::RulersHall);
    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);
    let authority = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Magistrate",
        rulers_hall,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            social_weight: pm(0),
            enterprise_weight: pm(0),
            ..UtilityProfile::default()
        },
    );
    let accused = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Accused",
        rulers_hall,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, accused, ControlSource::Human, 0);
    set_justice_profile(
        &mut h,
        authority,
        JusticeDispositionProfile {
            accusation_motive_weight: pm(900),
            fine_severity: pm(500),
        },
        0,
    );

    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "Magistrate",
        rulers_hall,
        SuccessionLaw::Support,
        8,
        Vec::new(),
    );
    let accusation_entry = worldwake_core::RecordEntryId(0);
    let theft = TheftFacts {
        missing_entity: office,
        expected_place: rulers_hall,
        commodity: CommodityKind::Bread,
        quantity: Quantity(4),
    };
    let (crime_register, bread) = {
        let mut txn = new_txn(&mut h.world, 0);
        txn.assign_office(office, authority).unwrap();
        let record = txn
            .create_record(RecordData {
                record_kind: RecordKind::CrimeRegister,
                home_place: rulers_hall,
                issuer: office,
                consultation_ticks: 1,
                max_entries_per_consult: 8,
                entries: vec![worldwake_core::InstitutionalRecordEntry {
                    entry_id: accusation_entry,
                    claim: InstitutionalClaim::Accusation {
                        accuser: authority,
                        accused,
                        violation_id: worldwake_core::ViolationId(1),
                        theft,
                        effective_tick: Tick(0),
                    },
                    recorded_tick: Tick(0),
                    supersedes: None,
                }],
                next_entry_id: 1,
            })
            .unwrap();
        let bread = txn
            .create_item_lot(CommodityKind::Bread, Quantity(4))
            .unwrap();
        txn.set_ground_location(bread, rulers_hall).unwrap();
        txn.set_owner(bread, accused).unwrap();
        commit_txn(txn, &mut h.event_log);
        (record, bread)
    };

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        authority,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        authority,
        crime_register,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        authority,
        office,
        Some(authority),
        Tick(0),
        InstitutionalKnowledgeSource::SelfDeclaration,
        Some(rulers_hall),
    );
    {
        let mut store = h
            .world
            .get_component_agent_belief_store(authority)
            .cloned()
            .unwrap_or_else(AgentBeliefStore::new);
        let profile = h
            .world
            .get_component_perception_profile(authority)
            .copied()
            .unwrap_or_default();
        store.record_institutional_belief(
            InstitutionalBeliefKey::CrimeCase {
                accused,
                violation_id: worldwake_core::ViolationId(1),
            },
            BelievedInstitutionalClaim {
                claim: InstitutionalClaim::Accusation {
                    accuser: authority,
                    accused,
                    violation_id: worldwake_core::ViolationId(1),
                    theft,
                    effective_tick: Tick(0),
                },
                source: InstitutionalKnowledgeSource::RecordConsultation {
                    record: crime_register,
                    entry_id: accusation_entry,
                },
                learned_tick: Tick(0),
                learned_at: Some(rulers_hall),
            },
            &profile,
        );
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_agent_belief_store(authority, store)
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    let current_tick = h.scheduler.current_tick();
    h.driver
        .produce_agent_input(
            AutonomousControllerContext {
                world: &mut h.world,
                event_log: &mut h.event_log,
                scheduler: &mut h.scheduler,
                rng: &mut h.rng,
                action_defs: &h.defs,
                action_handlers: &h.handlers,
                recipe_registry: &h.recipes,
                tick: current_tick,
            },
            authority,
            &[],
            &[],
        )
        .expect("AI should be able to produce punishment input");

    let punish_goal = GoalKind::PunishAccused {
        office,
        accused,
        accusation_entry,
        punishment: worldwake_core::PunishmentKind::Fine {
            commodity: CommodityKind::Bread,
            amount: Quantity(2),
        },
    };
    let trace = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(authority, Tick(0))
        .expect("authority should have a decision trace at tick 0");
    let DecisionOutcome::Planning(planning) = &trace.outcome else {
        panic!("authority should have a planning trace");
    };
    let evidence = planning
        .candidates
        .evidence
        .iter()
        .find(|e| e.opportunity.goal_key.kind == punish_goal)
        .unwrap_or_else(|| {
            panic!(
                "fine punishment branch should carry candidate evidence; generated={:?} evidence_goals={:?}",
                planning.candidates.generated,
                planning
                    .candidates
                    .evidence
                    .iter()
                    .map(|e| e.opportunity.goal_key.kind)
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(
        evidence.legality,
        Some(
            worldwake_ai::CandidateLegalityTrace::PunishmentFineSelection(
                worldwake_core::PunishmentFineSelectionTrace {
                    facts: worldwake_core::PunishmentFineTraceFacts {
                        office,
                        accusation_entry,
                        accused,
                        theft,
                        actor_place: Some(rulers_hall),
                        accused_place: Some(rulers_hall),
                        required_amount: Quantity(2),
                    },
                    locally_observed_quantity: Quantity(4),
                },
            ),
        ),
    );

    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_ground_location(bread, general_store).unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    let _ = step_tick(
        &mut h.world,
        &mut h.event_log,
        &mut h.scheduler,
        &mut h.controller,
        &mut h.rng,
        TickStepServices {
            action_defs: &h.defs,
            action_handlers: &h.handlers,
            recipe_registry: &h.recipes,
            systems: &worldwake_sim::SystemDispatchTable::canonical_noop(),
            input_producer: None,
            action_trace: h.action_trace.as_mut(),
            request_resolution_trace: None,
            politics_trace: None,
            perception_trace: None,
            institutional_knowledge_trace: None,
        },
    )
    .expect("queued fine request should be processed");

    let fine_events = h
        .action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(authority)
        .into_iter()
        .filter(|event| event.action_name == "fine")
        .collect::<Vec<_>>();
    assert!(
        fine_events
            .iter()
            .any(|event| matches!(event.kind, ActionTraceKind::Started { .. })),
        "authority should start the fine once global lawful control still exists: {fine_events:?}"
    );
    assert!(
        matches!(
            fine_events.iter().find(|event| matches!(event.kind, ActionTraceKind::Aborted { .. })),
            Some(event)
                if matches!(
                    &event.kind,
                    ActionTraceKind::Aborted { reason, .. }
                        if reason.contains("HolderLacksAccessibleCommodity")
                )
        ),
        "unexpected fine trace events: {fine_events:?}"
    );
    assert!(
        fine_events
            .iter()
            .all(|event| !matches!(event.kind, ActionTraceKind::StartFailed { .. })),
        "fine should no longer fail at authoritative start once remote stock is still lawfully controlled: {fine_events:?}"
    );
}

// ---------------------------------------------------------------------------
// Scenario 40: Supply Depletion Enables ShareBelief
// ---------------------------------------------------------------------------
//
// Systems: Perception, AI, Generic Actions, Social Tell
// GoalKinds: ShareBelief, InvestigateViolation
// ActionDomains: Generic, Social
// Places: VillageSquare
// Principles: 1, 7, 12, 15
//
// Proves the live architecture for local supply-depletion reporting: the
// speaker's refreshed belief about the depleted source drives ShareBelief via
// the generic social pipeline, while the same mismatch also emits
// InvestigateViolation as a separate local follow-up branch.

#[allow(clippy::too_many_lines)]
fn run_supply_depletion_enables_share_belief(
    seed: Seed,
) -> (worldwake_core::StateHash, worldwake_core::StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.enable_action_tracing();
    h.driver.enable_tracing();

    let speaker = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Speaker",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        social_weighted_utility(1000),
    );
    let listener = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Listener",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    let source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        VILLAGE_SQUARE,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(3),
            max_quantity: Quantity(3),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
        ProductionOutputOwner::Actor,
    );

    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        focused_accepting_tell_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        accepting_tell_profile(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        speaker,
        default_perception_profile(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        listener,
        blind_perception_profile(),
    );
    set_violation_profile(
        &mut h,
        speaker,
        ViolationDispositionProfile {
            investigation_duration_ticks: nz(2),
            violation_memory_retention_ticks: 50,
            investigation_motive_weight: pm(300),
            ownership_motive_bonus: pm(0),
        },
        0,
    );

    let seeded_source_belief = seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        speaker,
        source,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    assert_eq!(
        seeded_source_belief
            .resource_source
            .as_ref()
            .map(|source| source.available_quantity),
        Some(Quantity(3)),
        "speaker should begin with a non-depleted belief about the source"
    );
    let listener_presence_belief = seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        speaker,
        listener,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_told_belief_memory(
        &mut h.world,
        &mut h.event_log,
        speaker,
        listener,
        listener,
        &listener_presence_belief,
        Tick(0),
    );
    assert!(
        agent_belief_about(&h.world, listener, source).is_none(),
        "listener should start without any belief about the source"
    );

    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_resource_source(
            source,
            ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(0),
                max_quantity: Quantity(3),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        )
        .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    h.step_once();

    let speaker_belief_after_refresh = agent_belief_about(&h.world, speaker, source)
        .expect("speaker should still retain a belief about the depleted source");
    assert!(
        speaker_belief_after_refresh
            .resource_source
            .as_ref()
            .is_some_and(|source| source.available_quantity == Quantity(0)),
        "perception refresh should project depletion onto the speaker's resource-source belief snapshot"
    );
    assert_eq!(
        speaker_belief_after_refresh.source,
        PerceptionSource::DirectObservation,
        "speaker should learn the depletion through direct local observation"
    );
    assert!(
        agent_belief_about(&h.world, listener, source).is_none(),
        "blind listener must still not know about the depleted source before Tell"
    );

    let first_trace = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(speaker, Tick(0))
        .expect("tick 0 decision trace should exist");
    let first_planning = match &first_trace.outcome {
        DecisionOutcome::Planning(planning) => planning.as_ref(),
        other => panic!("expected planning trace at tick 0, got {other:?}"),
    };

    let share_goal = GoalKind::ShareBelief {
        listener,
        topic: TellTopic::EntityBelief { subject: source },
        communication_class: worldwake_core::CommunicationClass::Testimony,
    };
    assert!(
        first_planning
            .candidates
            .generated
            .iter()
            .any(|goal| goal.goal_key.kind == share_goal),
        "first post-refresh planning tick should expose ShareBelief for the depleted source"
    );

    let generated_violation =
        first_planning
            .candidates
            .generated
            .iter()
            .find_map(|goal| match goal.goal_key.kind {
                GoalKind::InvestigateViolation {
                    violation_id,
                    place,
                } if place == VILLAGE_SQUARE => Some(violation_id),
                _ => None,
            });
    assert!(
        generated_violation.is_some(),
        "the same local mismatch should also expose InvestigateViolation"
    );

    let mut tell_commit_tick = None;
    for _ in 0..20 {
        h.step_once();
        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled for supply-depletion social emergence");
        if action_sink.events_for(speaker).iter().any(|event| {
            event.action_name == "tell"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && event.detail
                    == Some(ActionTraceDetail::Tell {
                        listener,
                        topic: TellTopic::EntityBelief { subject: source },
                    })
        }) {
            tell_commit_tick = Some(h.scheduler.current_tick());
            break;
        }
    }

    let tell_commit_tick =
        tell_commit_tick.expect("speaker should commit Tell for the depleted source");
    let listener_belief = agent_belief_about(&h.world, listener, source)
        .expect("listener should learn the depleted source through Tell");
    assert!(
        matches!(
            listener_belief.source,
            PerceptionSource::Report {
                from,
                chain_len: 1
            } if from == speaker
        ),
        "listener should learn the depletion as a first-hand report from the speaker"
    );
    assert!(
        listener_belief
            .resource_source
            .as_ref()
            .is_some_and(|source| source.available_quantity == Quantity(0)),
        "listener should learn the depleted resource-source state, not the stale stocked state"
    );

    assert!(
        h.driver
            .trace_sink()
            .expect("decision tracing should remain enabled")
            .goal_history_for(speaker, &share_goal)
            .into_iter()
            .any(|entry| entry.tick <= tell_commit_tick && entry.status.is_generated()),
        "ShareBelief should be generated before the tell commit that propagates the depletion"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_supply_depletion_enables_share_belief() {
    let _ = run_supply_depletion_enables_share_belief(Seed([59; 32]));
}

#[test]
fn golden_supply_depletion_enables_share_belief_replays_deterministically() {
    let first = run_supply_depletion_enables_share_belief(Seed([60; 32]));
    let second = run_supply_depletion_enables_share_belief(Seed([60; 32]));
    assert_eq!(
        first, second,
        "supply-depletion social emergence scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 42: Witness Deterrence Suppresses Theft Candidate
// ---------------------------------------------------------------------------
//
// Systems: AI, Perception, Needs
// GoalKinds: ConsumeOwnedCommodity (NOT StealItem)
// ActionDomains: Needs
// Places: VillageSquare, GeneralStore
// Principles: 1, 10, 24
//
// Proves the live witness-deterrence contract without extra branch noise:
// the thief locally observes enough nearby agents to suppress `StealItem`,
// then follows a lawful self-care branch instead of idling.

#[allow(clippy::too_many_lines)]
fn run_witness_deterrence_suppresses_theft_candidate(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);

    let thief = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Thief",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(650), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile {
            social_weight: pm(0),
            enterprise_weight: pm(0),
            ..UtilityProfile::default()
        },
    );
    let witness_a = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Witness A",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let witness_b = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Witness B",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let witness_c = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Witness C",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let owner = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Owner",
        general_store,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    for witness in [witness_a, witness_b, witness_c, owner] {
        set_control_source(&mut h, witness, ControlSource::Human, 0);
    }

    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        thief,
        default_perception_profile(),
    );
    for witness in [witness_a, witness_b, witness_c] {
        set_agent_perception_profile(
            &mut h.world,
            &mut h.event_log,
            witness,
            default_perception_profile(),
        );
    }
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        thief,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    set_theft_profile(
        &mut h,
        thief,
        TheftDispositionProfile {
            steal_duration_ticks: nz(2),
            theft_motive_weight: pm(400),
            witness_risk_penalty: pm(150),
        },
        0,
    );

    let owned_food = give_commodity(
        &mut h.world,
        &mut h.event_log,
        thief,
        VILLAGE_SQUARE,
        CommodityKind::Apple,
        Quantity(1),
    );
    let theft_target = {
        let mut txn = new_txn(&mut h.world, 0);
        let lot = txn
            .create_item_lot(CommodityKind::Bread, Quantity(2))
            .unwrap();
        txn.set_ground_location(lot, VILLAGE_SQUARE).unwrap();
        txn.set_owner(lot, owner).unwrap();
        commit_txn(txn, &mut h.event_log);
        lot
    };

    let initial_hunger = h.agent_hunger(thief);
    let initial_target_quantity = h
        .world
        .get_component_item_lot(theft_target)
        .expect("theft target should be a live lot")
        .quantity;

    let mut saw_planning_tick = false;
    let mut saw_self_care_selection = false;
    let mut hunger_decreased = false;
    let mut eat_committed = false;

    for _ in 0..12 {
        h.step_once();

        let current_tick = h.scheduler.current_tick() - 1;
        let trace = h
            .driver
            .trace_sink()
            .expect("decision tracing should be enabled")
            .trace_at(thief, current_tick)
            .unwrap_or_else(|| panic!("thief should have a trace at tick {current_tick:?}"));
        if let DecisionOutcome::Planning(planning) = &trace.outcome {
            saw_planning_tick = true;
            assert!(
                planning.candidates.generated.iter().all(|goal| {
                    !matches!(goal.goal_key.kind, GoalKind::StealItem { target_item } if target_item == theft_target)
                }),
                "witness deterrence should suppress theft candidate generation; tick={current_tick:?}, generated={:?}",
                planning
                    .candidates
                    .generated
                    .iter()
                    .map(|goal| goal.goal_key.kind)
                    .collect::<Vec<_>>()
            );

            saw_self_care_selection |= planning.selection.selected_goal_is(GoalKey::from(
                GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Apple,
                },
            ));
        }

        hunger_decreased |= h.agent_hunger(thief) < initial_hunger;
        eat_committed |= h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(thief)
            .iter()
            .any(|event| {
                event.action_name == "eat"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            });

        if saw_self_care_selection && hunger_decreased && eat_committed {
            break;
        }
    }

    assert!(
        saw_planning_tick,
        "scenario should produce at least one planning tick"
    );
    assert!(
        saw_self_care_selection,
        "thief should select ConsumeOwnedCommodity(Apple) after theft is deterred"
    );
    assert!(
        eat_committed,
        "thief should commit eat after witness deterrence suppresses theft"
    );
    assert!(
        hunger_decreased,
        "thief hunger should decrease after the lawful self-care fallback executes"
    );

    assert_eq!(
        h.world.effective_place(theft_target),
        Some(VILLAGE_SQUARE),
        "deterrence scenario should leave the theft target at the original place"
    );
    assert_eq!(
        h.world.owner_of(theft_target),
        Some(owner),
        "deterrence scenario should preserve the original owner on the theft target"
    );
    assert_eq!(
        h.world
            .get_component_item_lot(theft_target)
            .expect("theft target should still be live")
            .quantity,
        initial_target_quantity,
        "deterrence scenario should not mutate the theft-target quantity"
    );
    assert_eq!(
        h.world.effective_place(owned_food),
        None,
        "the owned food should be consumed by the self-care fallback"
    );
    verify_live_lot_conservation(
        &h.world,
        CommodityKind::Bread,
        u64::from(initial_target_quantity.0),
    )
    .unwrap();

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_witness_deterrence_suppresses_theft_candidate() {
    let _ = run_witness_deterrence_suppresses_theft_candidate(Seed([63; 32]));
}

#[test]
fn golden_witness_deterrence_suppresses_theft_candidate_replays_deterministically() {
    let first = run_witness_deterrence_suppresses_theft_candidate(Seed([64; 32]));
    let second = run_witness_deterrence_suppresses_theft_candidate(Seed([64; 32]));
    assert_eq!(
        first, second,
        "witness-deterrence scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 41: Exile Punishment When Fine Is Not Locally Collectible
// ---------------------------------------------------------------------------
//
// Systems: Transport, Perception, Social Tell, AI, Institutions
// GoalKinds: StealItem, ShareBelief, Accuse, PunishAccused
// ActionDomains: Transport, Social, Travel
// Places: VillageSquare, GeneralStore, RulersHall
// Principles: 1, 7, 21, 22, 23, 24
//
// Proves the dedicated exile branch for the live justice architecture:
// witnessed theft becomes relayable social evidence, Tell creates a concrete
// case for the authority, accusation is filed at the CrimeRegister, and the
// authority selects Exile once the fined commodity is no longer locally
// collectible from the accused at punishment-selection time.

#[allow(clippy::too_many_lines)]
fn run_exile_punishment_when_fine_is_not_locally_collectible(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let rulers_hall = prototype_place_entity(PrototypePlace::RulersHall);
    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);

    let thief = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Thief",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            social_weight: pm(0),
            enterprise_weight: pm(0),
            ..UtilityProfile::default()
        },
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
    let victim = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Victim",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let authority = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Magistrate",
        rulers_hall,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            social_weight: pm(0),
            enterprise_weight: pm(0),
            ..UtilityProfile::default()
        },
    );
    set_control_source(&mut h, victim, ControlSource::Human, 0);

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
        authority,
        default_perception_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        witness,
        broad_accepting_tell_profile(),
    );
    set_violation_profile(
        &mut h,
        authority,
        ViolationDispositionProfile {
            investigation_duration_ticks: nz(1),
            violation_memory_retention_ticks: 80,
            investigation_motive_weight: pm(600),
            ownership_motive_bonus: pm(0),
        },
        0,
    );
    set_theft_profile(
        &mut h,
        thief,
        TheftDispositionProfile {
            steal_duration_ticks: nz(2),
            theft_motive_weight: pm(1000),
            witness_risk_penalty: pm(0),
        },
        0,
    );
    set_justice_profile(
        &mut h,
        authority,
        JusticeDispositionProfile {
            accusation_motive_weight: pm(900),
            fine_severity: pm(500),
        },
        0,
    );

    let faction = {
        let mut txn = new_txn(&mut h.world, 0);
        let faction = txn.create_faction("Town Ward").unwrap();
        commit_txn(txn, &mut h.event_log);
        faction
    };
    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "Magistrate",
        rulers_hall,
        SuccessionLaw::Support,
        8,
        vec![worldwake_core::EligibilityRule::FactionMember(faction)],
    );
    let (crime_register, stolen_lot) = {
        let mut txn = new_txn(&mut h.world, 0);
        txn.assign_office(office, authority).unwrap();
        txn.add_member(thief, faction).unwrap();
        let crime_register = txn
            .create_record(RecordData {
                record_kind: RecordKind::CrimeRegister,
                home_place: rulers_hall,
                issuer: office,
                consultation_ticks: 1,
                max_entries_per_consult: 8,
                entries: Vec::new(),
                next_entry_id: 0,
            })
            .unwrap();
        let stolen_lot = txn
            .create_item_lot(CommodityKind::Bread, Quantity(2))
            .unwrap();
        txn.set_ground_location(stolen_lot, VILLAGE_SQUARE).unwrap();
        txn.set_owner(stolen_lot, victim).unwrap();
        commit_txn(txn, &mut h.event_log);
        (crime_register, stolen_lot)
    };

    let initial_bread_total = total_live_lot_quantity(&h.world, CommodityKind::Bread);

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        witness,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        witness,
        &[authority],
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        authority,
        thief,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        authority,
        crime_register,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        authority,
        office,
        Some(authority),
        Tick(0),
        InstitutionalKnowledgeSource::SelfDeclaration,
        Some(rulers_hall),
    );
    seed_faction_membership_belief(
        &mut h.world,
        &mut h.event_log,
        authority,
        faction,
        thief,
        true,
        Tick(0),
        InstitutionalKnowledgeSource::SelfDeclaration,
        Some(rulers_hall),
    );

    let theft_facts = TheftFacts {
        missing_entity: stolen_lot,
        expected_place: VILLAGE_SQUARE,
        commodity: CommodityKind::Bread,
        quantity: Quantity(2),
    };

    let mut steal_commit_tick = None;
    for _ in 0..12 {
        h.step_once();
        if h.action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(thief)
            .iter()
            .any(|event| {
                event.action_name == "steal"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            })
        {
            steal_commit_tick = Some(h.scheduler.current_tick());
            break;
        }
    }
    let steal_commit_tick = steal_commit_tick.expect("thief should commit the witnessed theft");
    set_control_source(&mut h, thief, ControlSource::Human, steal_commit_tick.0);

    assert!(
        h.world
            .get_component_agent_belief_store(witness)
            .expect("witness should keep a belief store")
            .social_observations
            .iter()
            .any(|observation| {
                observation.detail
                    == worldwake_core::SocialObservationDetail::SuspectedTheft {
                        theft: theft_facts,
                        suspect: Some(thief),
                    }
            }),
        "witness should record direct suspected-theft evidence with the thief identity"
    );

    {
        let relocate_tick = h.scheduler.current_tick();
        let mut txn = new_txn(&mut h.world, relocate_tick.0);
        txn.set_ground_location(witness, rulers_hall).unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        witness,
        h.scheduler.current_tick(),
        PerceptionSource::DirectObservation,
    );

    let accuse_goal = GoalKind::Accuse {
        crime_register,
        accused: thief,
        violation_id: worldwake_core::ViolationId(0),
    };
    let pre_tell_tick = h.scheduler.current_tick();
    let generated_before_tell = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .goal_history_for(authority, &accuse_goal)
        .into_iter()
        .filter(|entry| entry.tick <= pre_tell_tick)
        .any(|entry| entry.status.is_generated());
    assert!(
        !generated_before_tell,
        "authority should not generate an accusation goal before receiving the witness report"
    );

    let mut tell_commit_order = None;
    for _ in 0..80 {
        h.step_once();
        let store = h
            .world
            .get_component_agent_belief_store(authority)
            .expect("authority should keep a belief store");
        if store.social_observations.iter().any(|observation| {
            observation.detail
                == worldwake_core::SocialObservationDetail::SuspectedTheft {
                    theft: theft_facts,
                    suspect: Some(thief),
                }
                && matches!(
                    observation.source,
                    PerceptionSource::Report { from, chain_len: 1 } if from == witness
                )
        }) {
            let tell_event = h
                .action_trace_sink()
                .expect("action tracing should be enabled")
                .events_for(witness)
                .iter()
                .find_map(|event| {
                    (event.action_name == "tell"
                        && matches!(event.kind, ActionTraceKind::Committed { .. })
                        && matches!(
                            event.detail,
                            Some(ActionTraceDetail::Tell {
                                listener,
                                topic: TellTopic::SocialObservation { observation },
                            }) if listener == authority
                                && observation.detail
                                    == worldwake_core::SocialObservationDetail::SuspectedTheft {
                                        theft: theft_facts,
                                        suspect: Some(thief),
                                    }
                        ))
                    .then_some((event.tick, event.sequence_in_tick))
                })
                .expect("witness should commit Tell for the theft observation");
            tell_commit_order = Some(tell_event);
            break;
        }
    }
    let tell_commit_order =
        tell_commit_order.expect("authority should learn the theft through Tell");

    let authority_memory = h
        .world
        .get_component_violation_memory(authority)
        .expect("authority should receive violation memory from the told theft evidence");
    let authority_violation_id = authority_memory
        .violations
        .iter()
        .find(|record| {
            record.kind
                == worldwake_core::ViolationKind::SuspectedTheft {
                    theft: theft_facts,
                    suspect: Some(thief),
                }
        })
        .map(|record| record.id)
        .expect("authority should record a local suspected-theft case");

    let mut accuse_commit_order = None;
    for _ in 0..20 {
        h.step_once();
        let event = h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(authority)
            .iter()
            .find_map(|event| {
                (event.action_name == "accuse"
                    && matches!(event.kind, ActionTraceKind::Committed { .. }))
                .then_some((event.tick, event.sequence_in_tick))
            });
        if event.is_some() {
            accuse_commit_order = event;
            break;
        }
    }
    let accuse_commit_order =
        accuse_commit_order.expect("authority should accuse after hearing the witness report");

    let record_after_accuse = h
        .world
        .get_component_record_data(crime_register)
        .expect("crime register should exist after accusation");
    let accusation_entry = record_after_accuse
        .active_entries()
        .into_iter()
        .find(|entry| {
            matches!(
                entry.claim,
                InstitutionalClaim::Accusation {
                    accuser,
                    accused,
                    violation_id,
                    theft,
                    ..
                } if accuser == authority
                    && accused == thief
                    && violation_id == authority_violation_id
                    && theft == theft_facts
            )
        })
        .expect("crime register should contain the accusation filed by the authority");
    let accusation_entry_id = accusation_entry.entry_id;

    {
        let relocate_tick = h.scheduler.current_tick();
        let mut txn = new_txn(&mut h.world, relocate_tick.0);
        txn.set_ground_location(stolen_lot, general_store).unwrap();
        txn.set_ground_location(thief, rulers_hall).unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        authority,
        h.scheduler.current_tick(),
        PerceptionSource::DirectObservation,
    );

    let exile_goal = GoalKind::PunishAccused {
        office,
        accused: thief,
        accusation_entry: accusation_entry_id,
        punishment: worldwake_core::PunishmentKind::Exile {
            from_faction: faction,
        },
    };
    let fine_goal = GoalKind::PunishAccused {
        office,
        accused: thief,
        accusation_entry: accusation_entry_id,
        punishment: worldwake_core::PunishmentKind::Fine {
            commodity: CommodityKind::Bread,
            amount: Quantity(1),
        },
    };
    let punishment_setup_tick = h.scheduler.current_tick();

    let mut exile_commit_order = None;
    for _ in 0..20 {
        h.step_once();
        let event = h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(authority)
            .iter()
            .find_map(|event| {
                (event.action_name == "exile"
                    && matches!(event.kind, ActionTraceKind::Committed { .. }))
                .then_some((event.tick, event.sequence_in_tick))
            });
        if event.is_some() {
            exile_commit_order = event;
            break;
        }
    }
    let exile_commit_order =
        exile_commit_order.expect("authority should commit exile after filing the case");

    let authority_goal_history = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled");
    assert!(
        authority_goal_history
            .goal_history_for(authority, &exile_goal)
            .iter()
            .any(|entry| entry.tick >= punishment_setup_tick && entry.status.is_generated()),
        "authority should generate PunishAccused(Exile) once the fine is not locally collectible"
    );
    assert!(
        !authority_goal_history
            .goal_history_for(authority, &fine_goal)
            .iter()
            .any(|entry| entry.tick >= punishment_setup_tick && entry.status.is_generated()),
        "authority should not generate PunishAccused(Fine) after the fined commodity leaves local reach"
    );
    assert!(
        h.action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(authority)
            .iter()
            .all(|event| {
                !(event.action_name == "fine"
                    && matches!(event.kind, ActionTraceKind::Committed { .. }))
            }),
        "the exile-specific scenario should commit no fine action"
    );

    assert!(
        tell_commit_order < exile_commit_order,
        "witness tell must precede final exile in the action trace"
    );
    assert!(
        accuse_commit_order < exile_commit_order,
        "accusation should commit before exile in the action trace"
    );

    assert!(
        !h.world.factions_of(thief).contains(&faction),
        "exile should remove the thief from the governed faction"
    );
    assert!(
        h.world.hostile_towards(thief).contains(&faction),
        "exile should create hostility from the faction toward the thief"
    );

    let final_record = h
        .world
        .get_component_record_data(crime_register)
        .expect("crime register should persist after punishment");
    let exile_verdicts = final_record
        .active_entries()
        .into_iter()
        .filter(|entry| {
            matches!(
                entry.claim,
                InstitutionalClaim::Verdict {
                    accused,
                    violation_id,
                    punishment: worldwake_core::PunishmentKind::Exile { from_faction },
                    ..
                } if accused == thief
                    && violation_id == authority_violation_id
                    && from_faction == faction
            ) && entry.supersedes == Some(accusation_entry_id)
        })
        .count();
    assert_eq!(
        exile_verdicts, 1,
        "crime register should contain exactly one exile verdict for the accusation"
    );

    verify_live_lot_conservation(&h.world, CommodityKind::Bread, initial_bread_total).unwrap();

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_exile_punishment_when_fine_is_not_locally_collectible() {
    let _ = run_exile_punishment_when_fine_is_not_locally_collectible(Seed([65; 32]));
}

#[test]
fn golden_exile_punishment_when_fine_is_not_locally_collectible_replays_deterministically() {
    let first = run_exile_punishment_when_fine_is_not_locally_collectible(Seed([66; 32]));
    let second = run_exile_punishment_when_fine_is_not_locally_collectible(Seed([66; 32]));
    assert_eq!(
        first, second,
        "exile punishment fallback scenario should replay deterministically"
    );
}

fn run_jurisdiction_gated_punishment_branch(
    seed: Seed,
    office_seat: worldwake_core::EntityId,
    authority_place: worldwake_core::EntityId,
    extra_jurisdiction_places: &[worldwake_core::EntityId],
    expect_punish: bool,
    expect_seat_distinct: bool,
) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let authority = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Magistrate",
        authority_place,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            social_weight: pm(0),
            enterprise_weight: pm(0),
            ..UtilityProfile::default()
        },
    );
    let accused = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Accused",
        authority_place,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, accused, ControlSource::Human, 0);
    set_justice_profile(
        &mut h,
        authority,
        JusticeDispositionProfile {
            accusation_motive_weight: pm(900),
            fine_severity: pm(500),
        },
        0,
    );

    let faction = {
        let mut txn = new_txn(&mut h.world, 0);
        let faction = txn.create_faction("Town Ward").unwrap();
        commit_txn(txn, &mut h.event_log);
        faction
    };
    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "Magistrate",
        office_seat,
        SuccessionLaw::Support,
        8,
        vec![worldwake_core::EligibilityRule::FactionMember(faction)],
    );
    let accusation_entry = worldwake_core::RecordEntryId(0);
    let violation_id = worldwake_core::ViolationId(1);
    let theft = TheftFacts {
        missing_entity: office,
        expected_place: office_seat,
        commodity: CommodityKind::Bread,
        quantity: Quantity(2),
    };
    let claim = InstitutionalClaim::Accusation {
        accuser: authority,
        accused,
        violation_id,
        theft,
        effective_tick: Tick(0),
    };
    let crime_register = {
        let mut txn = new_txn(&mut h.world, 0);
        let mut office_data = txn.get_component_office_data(office).cloned().unwrap();
        for place in extra_jurisdiction_places {
            office_data.jurisdiction.insert(*place);
        }
        txn.set_component_office_data(office, office_data).unwrap();
        txn.assign_office(office, authority).unwrap();
        txn.add_member(accused, faction).unwrap();
        let crime_register = txn
            .create_record(RecordData {
                record_kind: RecordKind::CrimeRegister,
                home_place: authority_place,
                issuer: office,
                consultation_ticks: 1,
                max_entries_per_consult: 8,
                entries: vec![worldwake_core::InstitutionalRecordEntry {
                    entry_id: accusation_entry,
                    claim,
                    recorded_tick: Tick(0),
                    supersedes: None,
                }],
                next_entry_id: 1,
            })
            .unwrap();
        commit_txn(txn, &mut h.event_log);
        crime_register
    };

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        authority,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        authority,
        accused,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        authority,
        crime_register,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        authority,
        office,
        Some(authority),
        Tick(0),
        InstitutionalKnowledgeSource::SelfDeclaration,
        Some(office_seat),
    );
    seed_faction_membership_belief(
        &mut h.world,
        &mut h.event_log,
        authority,
        faction,
        accused,
        true,
        Tick(0),
        InstitutionalKnowledgeSource::SelfDeclaration,
        Some(authority_place),
    );
    {
        let mut store = h
            .world
            .get_component_agent_belief_store(authority)
            .cloned()
            .unwrap_or_else(AgentBeliefStore::new);
        let profile = h
            .world
            .get_component_perception_profile(authority)
            .copied()
            .unwrap_or_default();
        store.record_institutional_belief(
            InstitutionalBeliefKey::CrimeCase {
                accused,
                violation_id,
            },
            BelievedInstitutionalClaim {
                claim,
                source: InstitutionalKnowledgeSource::RecordConsultation {
                    record: crime_register,
                    entry_id: accusation_entry,
                },
                learned_tick: Tick(0),
                learned_at: Some(authority_place),
            },
            &profile,
        );
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_agent_belief_store(authority, store)
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    let punish_goal = GoalKind::PunishAccused {
        office,
        accused,
        accusation_entry,
        punishment: worldwake_core::PunishmentKind::Exile {
            from_faction: faction,
        },
    };

    let mut exile_committed = false;
    for _ in 0..20 {
        h.step_once();
        exile_committed = h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(authority)
            .iter()
            .any(|event| {
                event.action_name == "exile"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            });
        if exile_committed {
            break;
        }
    }

    let goal_generated = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .goal_history_for(authority, &punish_goal)
        .iter()
        .any(|entry| entry.status.is_generated());

    let office_data = h
        .world
        .get_component_office_data(office)
        .expect("office should retain office data");
    let jurisdiction_right = h
        .world
        .effective_rights(authority, accused)
        .into_iter()
        .any(|right| right.kind == RightKind::JurisdictionalAuthority && right.via == Some(office));

    if expect_punish {
        assert_eq!(
            office_data.seat, office_seat,
            "office seat should remain the configured canonical seat"
        );
        assert!(
            office_data.jurisdiction.contains(&authority_place),
            "authority place should be inside the issuing office jurisdiction"
        );
        assert!(
            jurisdiction_right,
            "authority should hold jurisdictional authority via the issuing office"
        );
        assert!(
            goal_generated,
            "authority should generate PunishAccused when inside office jurisdiction"
        );
        assert!(
            exile_committed,
            "authority should commit exile when the issuing office has jurisdiction"
        );
        assert!(
            !h.world.factions_of(accused).contains(&faction),
            "jurisdiction-backed exile should remove the accused from the governed faction"
        );
        assert_eq!(
            h.world.effective_place(authority),
            Some(authority_place),
            "authority should remain at the punishment place through local punishment"
        );
        if expect_seat_distinct {
            assert_ne!(
                office_data.seat, authority_place,
                "secondary-jurisdiction branch should punish away from the office seat"
            );
            assert!(
                h.action_trace_sink()
                    .expect("action tracing should be enabled")
                    .events_for(authority)
                    .iter()
                    .all(|event| {
                        !(event.action_name == "travel"
                            && matches!(event.kind, ActionTraceKind::Committed { .. }))
                    }),
                "authority should not travel back to the office seat before punishing"
            );
        }
    } else {
        assert!(
            !jurisdiction_right,
            "out-of-jurisdiction branch should not derive jurisdictional authority via the office"
        );
        assert!(
            !goal_generated,
            "authority should not generate PunishAccused outside the issuing office jurisdiction"
        );
        assert!(
            !exile_committed,
            "authority should not commit exile outside the issuing office jurisdiction"
        );
        assert!(
            h.world.factions_of(accused).contains(&faction),
            "without jurisdiction-gated punishment, the accused should remain in the faction"
        );
        assert!(
            h.action_trace_sink()
                .expect("action tracing should be enabled")
                .events_for(authority)
                .iter()
                .all(|event| {
                    !(matches!(event.action_name.as_str(), "fine" | "exile")
                        && matches!(event.kind, ActionTraceKind::Committed { .. }))
                }),
            "out-of-jurisdiction branch should commit no punishment action"
        );
    }

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// Scenario 110: Jurisdiction-Gated Punishment
// GoalKinds: PunishAccused
// ActionDomains: Social
// Places: RulersHall, GeneralStore
// Principles: 7, 12, 14, 24
//
// Proves that punishment generation is gated by believed jurisdiction via the
// issuing office. The same punishment-ready accusation setup produces exile
// when the authority and accused stand inside that office's jurisdiction, and
// produces no punishment goal or committed punishment when both stand outside
// it.

fn run_jurisdiction_gated_punishment(
    seed_base: u8,
) -> ((StateHash, StateHash), (StateHash, StateHash)) {
    let rulers_hall = prototype_place_entity(PrototypePlace::RulersHall);
    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);
    let inside = run_jurisdiction_gated_punishment_branch(
        Seed([seed_base; 32]),
        rulers_hall,
        rulers_hall,
        &[],
        true,
        false,
    );
    let outside = run_jurisdiction_gated_punishment_branch(
        Seed([seed_base.wrapping_add(1); 32]),
        rulers_hall,
        general_store,
        &[],
        false,
        false,
    );
    (inside, outside)
}

#[test]
fn golden_jurisdiction_gated_punishment() {
    let _ = run_jurisdiction_gated_punishment(69);
}

#[test]
fn golden_jurisdiction_gated_punishment_replays_deterministically() {
    let first = run_jurisdiction_gated_punishment(70);
    let second = run_jurisdiction_gated_punishment(70);
    assert_eq!(
        first, second,
        "jurisdiction-gated punishment scenario should replay deterministically"
    );
}

// Scenario 111: Secondary-Jurisdiction Punishment Away From Office Seat
// GoalKinds: PunishAccused
// ActionDomains: Social
// Places: RulersHall, GeneralStore
// Principles: 7, 23, 24, 26
//
// Proves that the S50 seat/jurisdiction split is load-bearing for justice:
// the issuing office keeps its canonical seat at RulersHall while still
// lawfully authorizing punishment at GeneralStore through the same office's
// wider jurisdiction, without requiring a return to the seat.

fn run_secondary_jurisdiction_punishment(seed: Seed) -> (StateHash, StateHash) {
    let rulers_hall = prototype_place_entity(PrototypePlace::RulersHall);
    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);
    run_jurisdiction_gated_punishment_branch(
        seed,
        rulers_hall,
        general_store,
        &[general_store],
        true,
        true,
    )
}

#[test]
fn golden_secondary_jurisdiction_punishment() {
    let _ = run_secondary_jurisdiction_punishment(Seed([71; 32]));
}

#[test]
fn golden_secondary_jurisdiction_punishment_replays_deterministically() {
    let first = run_secondary_jurisdiction_punishment(Seed([72; 32]));
    let second = run_secondary_jurisdiction_punishment(Seed([72; 32]));
    assert_eq!(
        first, second,
        "secondary-jurisdiction punishment scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 43: Dual Discovery Converges Without Double Accusation
// ---------------------------------------------------------------------------
//
// Systems: Transport, Perception, Social Tell, AI, Institutions
// GoalKinds: StealItem, InvestigateViolation, ShareBelief, Accuse
// ActionDomains: Transport, Social, Travel
// Places: VillageSquare, GeneralStore, RulersHall, CommonHouse
// Principles: 1, 7, 13, 16, 24
//
// Proves the live convergence boundary after the accusation dedupe fix:
// witness discovery files the first accusation, owner-local discovery reaches
// the same authority later through a distinct evidence lane, and the concrete
// recorded theft case prevents a second accusation for the same theft.

#[allow(clippy::too_many_lines)]
fn run_dual_discovery_converges_without_double_accusation(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let rulers_hall = prototype_place_entity(PrototypePlace::RulersHall);
    let general_store = prototype_place_entity(PrototypePlace::GeneralStore);
    let common_house = prototype_place_entity(PrototypePlace::CommonHouse);

    let thief = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Thief",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            social_weight: pm(0),
            enterprise_weight: pm(0),
            ..UtilityProfile::default()
        },
    );
    let witness = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Witness",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        social_weighted_utility(900),
    );
    let victim = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Victim",
        general_store,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        social_weighted_utility(900),
    );
    let authority = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Magistrate",
        rulers_hall,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile {
            social_weight: pm(0),
            enterprise_weight: pm(0),
            ..UtilityProfile::default()
        },
    );
    set_control_source(&mut h, victim, ControlSource::Human, 0);

    let sharp_perception = PerceptionProfile {
        observation_fidelity: pm(1000),
        ..default_perception_profile()
    };
    set_agent_perception_profile(&mut h.world, &mut h.event_log, witness, sharp_perception);
    set_agent_perception_profile(&mut h.world, &mut h.event_log, victim, sharp_perception);
    set_agent_perception_profile(&mut h.world, &mut h.event_log, authority, sharp_perception);
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        witness,
        broad_accepting_tell_profile(),
    );
    set_agent_tell_profile(
        &mut h.world,
        &mut h.event_log,
        victim,
        broad_accepting_tell_profile(),
    );
    set_theft_profile(
        &mut h,
        thief,
        TheftDispositionProfile {
            steal_duration_ticks: nz(2),
            theft_motive_weight: pm(1000),
            witness_risk_penalty: pm(0),
        },
        0,
    );
    set_violation_profile(
        &mut h,
        victim,
        ViolationDispositionProfile {
            investigation_duration_ticks: nz(2),
            violation_memory_retention_ticks: 80,
            investigation_motive_weight: pm(1000),
            ownership_motive_bonus: pm(400),
        },
        0,
    );
    set_violation_profile(
        &mut h,
        authority,
        ViolationDispositionProfile {
            investigation_duration_ticks: nz(1),
            violation_memory_retention_ticks: 80,
            investigation_motive_weight: pm(600),
            ownership_motive_bonus: pm(0),
        },
        0,
    );
    set_justice_profile(
        &mut h,
        authority,
        JusticeDispositionProfile {
            accusation_motive_weight: pm(900),
            fine_severity: pm(500),
        },
        0,
    );

    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "Magistrate",
        rulers_hall,
        SuccessionLaw::Support,
        8,
        Vec::new(),
    );
    let (crime_register, stolen_lot) = {
        let mut txn = new_txn(&mut h.world, 0);
        txn.assign_office(office, authority).unwrap();
        let crime_register = txn
            .create_record(RecordData {
                record_kind: RecordKind::CrimeRegister,
                home_place: rulers_hall,
                issuer: office,
                consultation_ticks: 1,
                max_entries_per_consult: 8,
                entries: Vec::new(),
                next_entry_id: 0,
            })
            .unwrap();
        let stolen_lot = txn
            .create_item_lot(CommodityKind::Bread, Quantity(2))
            .unwrap();
        txn.set_ground_location(stolen_lot, VILLAGE_SQUARE).unwrap();
        txn.set_owner(stolen_lot, victim).unwrap();
        commit_txn(txn, &mut h.event_log);
        (crime_register, stolen_lot)
    };

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        witness,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        witness,
        authority,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        thief,
        stolen_lot,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief(
        &mut h.world,
        &mut h.event_log,
        victim,
        stolen_lot,
        BelievedEntityState {
            last_known_place: Some(VILLAGE_SQUARE),
            last_known_inventory: std::collections::BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            observed_tick: Tick(0),
            source: PerceptionSource::DirectObservation,
        },
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        authority,
        thief,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        authority,
        crime_register,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    seed_office_holder_belief(
        &mut h.world,
        &mut h.event_log,
        authority,
        office,
        Some(authority),
        Tick(0),
        InstitutionalKnowledgeSource::SelfDeclaration,
        Some(rulers_hall),
    );

    verify_live_lot_conservation(&h.world, CommodityKind::Bread, 2).unwrap();

    let theft_facts = TheftFacts {
        missing_entity: stolen_lot,
        expected_place: VILLAGE_SQUARE,
        commodity: CommodityKind::Bread,
        quantity: Quantity(2),
    };

    let mut steal_commit_tick = None;
    for _ in 0..12 {
        h.step_once();
        verify_live_lot_conservation(&h.world, CommodityKind::Bread, 2).unwrap();
        if h.action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(thief)
            .iter()
            .any(|event| {
                event.action_name == "steal"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            })
        {
            steal_commit_tick = Some(h.scheduler.current_tick());
            break;
        }
    }
    let steal_commit_tick = steal_commit_tick.expect("thief should commit the theft");
    set_control_source(&mut h, thief, ControlSource::Human, steal_commit_tick.0);

    assert!(
        h.world
            .get_component_agent_belief_store(witness)
            .expect("witness should keep a belief store")
            .social_observations
            .iter()
            .any(|observation| {
                observation.detail
                    == worldwake_core::SocialObservationDetail::SuspectedTheft {
                        theft: theft_facts,
                        suspect: Some(thief),
                    }
                    && matches!(observation.source, PerceptionSource::DirectObservation)
            }),
        "witness should develop firsthand suspected-theft evidence"
    );

    let travel_def_id = h
        .defs
        .iter()
        .find(|def| def.name == "travel")
        .map(|def| def.id)
        .expect("full registries should include travel");

    let thief_departure_tick = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        thief_departure_tick,
        InputKind::RequestAction {
            actor: thief,
            def_id: travel_def_id,
            targets: vec![common_house],
            payload_override: None,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
    h.step_once();
    verify_live_lot_conservation(&h.world, CommodityKind::Bread, 2).unwrap();
    assert_eq!(h.world.effective_place(thief), Some(common_house));
    assert_eq!(h.world.effective_place(stolen_lot), Some(common_house));

    {
        let relocate_tick = h.scheduler.current_tick();
        let mut txn = new_txn(&mut h.world, relocate_tick.0);
        txn.set_ground_location(witness, rulers_hall).unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        witness,
        h.scheduler.current_tick(),
        PerceptionSource::DirectObservation,
    );

    let mut witness_tell_order = None;
    for _ in 0..80 {
        h.step_once();
        let store = h
            .world
            .get_component_agent_belief_store(authority)
            .expect("authority should keep a belief store");
        if store.social_observations.iter().any(|observation| {
            observation.detail
                == worldwake_core::SocialObservationDetail::SuspectedTheft {
                    theft: theft_facts,
                    suspect: Some(thief),
                }
                && matches!(
                    observation.source,
                    PerceptionSource::Report { from, chain_len: 1 } if from == witness
                )
        }) {
            witness_tell_order = h
                .action_trace_sink()
                .expect("action tracing should be enabled")
                .events_for(witness)
                .iter()
                .find_map(|event| {
                    (event.action_name == "tell"
                        && matches!(event.kind, ActionTraceKind::Committed { .. })
                        && matches!(
                            event.detail,
                            Some(ActionTraceDetail::Tell {
                                listener,
                                topic: TellTopic::SocialObservation { observation },
                            }) if listener == authority
                                && observation.detail
                                    == worldwake_core::SocialObservationDetail::SuspectedTheft {
                                        theft: theft_facts,
                                        suspect: Some(thief),
                                    }
                        ))
                    .then_some((event.tick, event.sequence_in_tick))
                });
            if witness_tell_order.is_some() {
                break;
            }
        }
    }
    let witness_tell_order =
        witness_tell_order.expect("witness should tell the authority about the theft");

    let authority_witness_violation_id = h
        .world
        .get_component_violation_memory(authority)
        .expect("authority should gain violation memory from witness tell")
        .violations
        .iter()
        .find(|record| {
            record.kind
                == worldwake_core::ViolationKind::SuspectedTheft {
                    theft: theft_facts,
                    suspect: Some(thief),
                }
        })
        .map(|record| record.id)
        .expect("witness report should create the first authority-side theft record");

    let first_accuse_goal = GoalKind::Accuse {
        crime_register,
        accused: thief,
        violation_id: authority_witness_violation_id,
    };

    let mut first_accuse_order = None;
    for _ in 0..20 {
        h.step_once();
        let event = h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(authority)
            .iter()
            .find_map(|event| {
                (event.action_name == "accuse"
                    && matches!(event.kind, ActionTraceKind::Committed { .. }))
                .then_some((event.tick, event.sequence_in_tick))
            });
        if event.is_some() {
            first_accuse_order = event;
            break;
        }
    }
    let first_accuse_order = first_accuse_order
        .expect("authority should file the first accusation from witness evidence");

    assert!(
        h.driver
            .trace_sink()
            .expect("decision tracing should be enabled")
            .goal_history_for(authority, &first_accuse_goal)
            .iter()
            .any(|entry| entry.status.is_generated()),
        "witness report should generate the initial accuse goal"
    );

    let accusation_count = |record: &RecordData| {
        record
            .entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.claim,
                    InstitutionalClaim::Accusation {
                        accused,
                        theft,
                        ..
                    } if accused == thief && theft == theft_facts
                )
            })
            .count()
    };
    assert_eq!(
        accusation_count(
            h.world
                .get_component_record_data(crime_register)
                .expect("crime register should exist after the first accusation")
        ),
        1,
        "witness path should file exactly one accusation for the theft"
    );

    let victim_return_tick = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        victim_return_tick,
        InputKind::RequestAction {
            actor: victim,
            def_id: travel_def_id,
            targets: vec![VILLAGE_SQUARE],
            payload_override: None,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
    h.step_once();
    verify_live_lot_conservation(&h.world, CommodityKind::Bread, 2).unwrap();
    assert_eq!(h.world.effective_place(victim), Some(VILLAGE_SQUARE));

    let detection_tick = h.scheduler.current_tick();
    set_control_source(&mut h, victim, ControlSource::Ai, detection_tick.0);
    h.step_once();
    verify_live_lot_conservation(&h.world, CommodityKind::Bread, 2).unwrap();

    let victim_detection_trace = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(victim, detection_tick)
        .expect("victim should produce a decision trace on the discovery tick");
    let victim_detection_planning = match &victim_detection_trace.outcome {
        DecisionOutcome::Planning(planning) => planning.as_ref(),
        other => panic!("expected planning trace after victim arrival, got {other:?}"),
    };
    let victim_violation_id = victim_detection_planning
        .candidates
        .generated
        .iter()
        .find_map(|goal| match goal.goal_key.kind {
            GoalKind::InvestigateViolation {
                violation_id,
                place,
            } if place == VILLAGE_SQUARE => Some(violation_id),
            _ => None,
        })
        .expect("victim arrival should generate an investigate goal");

    let mut investigate_committed = false;
    for _ in 0..10 {
        h.step_once();
        verify_live_lot_conservation(&h.world, CommodityKind::Bread, 2).unwrap();
        investigate_committed = h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(victim)
            .iter()
            .any(|event| {
                event.action_name == "investigate"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
                    && event.detail
                        == Some(ActionTraceDetail::Investigate {
                            violation_id: victim_violation_id,
                        })
            });
        if investigate_committed {
            break;
        }
    }
    assert!(
        investigate_committed,
        "victim should complete local investigation before telling the authority"
    );

    let victim_store = h
        .world
        .get_component_agent_belief_store(victim)
        .expect("victim should keep a belief store");
    assert!(
        victim_store.social_observations.iter().any(|observation| {
            observation.detail
                == worldwake_core::SocialObservationDetail::SuspectedTheft {
                    theft: theft_facts,
                    suspect: None,
                }
                && matches!(observation.source, PerceptionSource::DirectObservation)
        }),
        "victim investigation should produce owner-local suspected-theft evidence"
    );
    assert!(
        h.world
            .get_component_violation_memory(victim)
            .expect("victim should keep violation memory")
            .violations
            .iter()
            .any(|record| {
                record.kind
                    == worldwake_core::ViolationKind::SuspectedTheft {
                        theft: theft_facts,
                        suspect: None,
                    }
            }),
        "victim violation memory should retain the investigated theft record"
    );

    {
        let relocate_tick = h.scheduler.current_tick();
        let mut txn = new_txn(&mut h.world, relocate_tick.0);
        txn.set_ground_location(victim, rulers_hall).unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        victim,
        h.scheduler.current_tick(),
        PerceptionSource::DirectObservation,
    );

    let mut victim_tell_tick = None;
    for _ in 0..80 {
        h.step_once();
        let store = h
            .world
            .get_component_agent_belief_store(authority)
            .expect("authority should keep a belief store");
        if store.social_observations.iter().any(|observation| {
            observation.detail
                == worldwake_core::SocialObservationDetail::SuspectedTheft {
                    theft: theft_facts,
                    suspect: None,
                }
                && matches!(
                    observation.source,
                    PerceptionSource::Report { from, chain_len: 1 } if from == victim
                )
        }) {
            victim_tell_tick = h
                .action_trace_sink()
                .expect("action tracing should be enabled")
                .events_for(victim)
                .iter()
                .find_map(|event| {
                    (event.action_name == "tell"
                        && matches!(event.kind, ActionTraceKind::Committed { .. })
                        && matches!(
                            event.detail,
                            Some(ActionTraceDetail::Tell {
                                listener,
                                topic: TellTopic::SocialObservation { observation },
                            }) if listener == authority
                                && observation.detail
                                    == worldwake_core::SocialObservationDetail::SuspectedTheft {
                                        theft: theft_facts,
                                        suspect: None,
                                    }
                        ))
                    .then_some(event.tick)
                });
            if victim_tell_tick.is_some() {
                break;
            }
        }
    }
    let victim_tell_tick = victim_tell_tick
        .expect("victim should later tell the authority about the investigated theft");

    let authority_memory_after_both = h
        .world
        .get_component_violation_memory(authority)
        .expect("authority should keep violation memory after both paths converge");
    let authority_owner_violation_id = authority_memory_after_both
        .violations
        .iter()
        .find(|record| {
            record.kind
                == worldwake_core::ViolationKind::SuspectedTheft {
                    theft: theft_facts,
                    suspect: None,
                }
        })
        .map(|record| record.id)
        .expect("owner-local tell should create a second authority-side theft record");
    assert_ne!(
        authority_owner_violation_id, authority_witness_violation_id,
        "dual discovery should reach the authority through a distinct violation lane"
    );

    let second_accuse_goal = GoalKind::Accuse {
        crime_register,
        accused: thief,
        violation_id: authority_owner_violation_id,
    };

    for _ in 0..20 {
        h.step_once();
        verify_live_lot_conservation(&h.world, CommodityKind::Bread, 2).unwrap();
        assert_eq!(
            accusation_count(
                h.world
                    .get_component_record_data(crime_register)
                    .expect("crime register should persist")
            ),
            1,
            "owner-local convergence must not create a second accusation entry"
        );
    }

    let authority_accuse_events = h
        .action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(authority)
        .iter()
        .filter(|event| {
            event.action_name == "accuse" && matches!(event.kind, ActionTraceKind::Committed { .. })
        })
        .count();
    assert_eq!(
        authority_accuse_events, 1,
        "authority should only commit one accusation across both discovery paths"
    );

    let second_accuse_history = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .goal_history_for(authority, &second_accuse_goal);
    assert!(
        second_accuse_history
            .iter()
            .filter(|entry| entry.tick >= victim_tell_tick)
            .all(|entry| !entry.status.is_generated()),
        "the second authority-side violation lane must not generate a duplicate accuse goal after the case is already recorded"
    );

    assert!(
        witness_tell_order < first_accuse_order,
        "witness tell should precede the first accusation"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_dual_discovery_converges_without_double_accusation() {
    let _ = run_dual_discovery_converges_without_double_accusation(Seed([67; 32]));
}

#[test]
fn golden_dual_discovery_converges_without_double_accusation_replays_deterministically() {
    let first = run_dual_discovery_converges_without_double_accusation(Seed([68; 32]));
    let second = run_dual_discovery_converges_without_double_accusation(Seed([68; 32]));
    assert_eq!(
        first, second,
        "dual-discovery convergence scenario should replay deterministically"
    );
}
