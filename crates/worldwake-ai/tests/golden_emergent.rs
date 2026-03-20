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
    hash_event_log, hash_world, prototype_place_entity, total_live_lot_quantity, AgentData,
    BeliefConfidencePolicy, BodyPart, CombatProfile, CommodityKind, ComponentKind, ComponentValue,
    ControlSource, DeadAt, EventTag, EventView, GoalKind, HomeostaticNeeds, KnownRecipes,
    MetabolismProfile, PerceptionProfile, PerceptionSource, PrototypePlace, Quantity,
    RecipientKnowledgeStatus, RelationValue, Seed, StateHash, SuccessionLaw, TellProfile, Tick,
    UtilityProfile, Wound, WoundCause, WoundId, WoundList,
};
use worldwake_sim::{
    ActionPayload, ActionRequestMode, ActionStartFailureReason, ActionTraceDetail, ActionTraceKind,
    DeclareSupportActionPayload, InputKind, OfficeAvailabilityPhase, OfficeSuccessionOutcome,
    RequestProvenance, RequestResolutionOutcome, ResolvedRequestTrace,
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
    }
}

fn blind_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        memory_capacity: 16,
        memory_retention_ticks: 240,
        observation_fidelity: pm(0),
        confidence_policy: BeliefConfidencePolicy::default(),
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
fn no_recovery_combat_profile() -> CombatProfile {
    CombatProfile::new(
        pm(1000), // wound_capacity
        pm(700),  // incapacitation_threshold
        pm(500),  // attack_skill
        pm(500),  // guard_skill
        pm(80),   // defend_bonus
        pm(25),   // natural_clot_resistance
        pm(0),    // natural_recovery_rate — ZERO: wounds stay until healed
        pm(120),  // unarmed_wound_severity
        pm(35),   // unarmed_bleed_rate
        nz(6),    // unarmed_attack_ticks
    )
}

/// Create a wound list with a single clotted wound at given severity.
fn stable_wound_list(severity: u16) -> WoundList {
    WoundList {
        wounds: vec![Wound {
            id: WoundId(1),
            body_part: BodyPart::Torso,
            cause: WoundCause::Deprivation(worldwake_core::DeprivationKind::Starvation),
            severity: pm(severity),
            inflicted_at: Tick(0),
            bleed_rate_per_tick: pm(0), // clotted — won't escalate
        }],
    }
}

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
// Suite 2: wounded_politician_priority_resolution
//
// Proves: care and political ambition follow the shared ranking pipeline.
// Medium pain can outrank office ambition, while low pain can leave the office
// claim path ahead, all without office-specific priority exceptions.
// Foundation: Principle 3, Principle 20, Principle 24.
// Cross-systems: Care + AI ranking + Political planning + Succession.
// ===========================================================================

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

// ===========================================================================
// Suite 5: combat_death_triggers_force_succession
//
// Proves: combat, death, and political succession interact only through
// authoritative world state and event history. No combat-specific political
// hook or political action alias is involved.
// Foundation: Principle 1, Principle 9, Principle 24.
// Cross-systems: Combat + Politics + AI combat goal generation.
// ===========================================================================

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
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_combat_profile(challenger, lethal_combat_attacker_profile())
            .unwrap();
        txn.set_component_combat_profile(incumbent, fragile_office_holder_profile())
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

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
    let vacancy_trace = politics_sink
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
    assert!(
        politics_sink
            .events_for_office(office)
            .into_iter()
            .any(|event| matches!(
                event.trace.outcome,
                OfficeSuccessionOutcome::WaitingForTimer { .. }
            )),
        "politics trace should include timer-blocked waiting before installation"
    );
    assert!(
        install_trace.tick.0.saturating_sub(vacancy_trace.tick.0) >= 5,
        "politics trace should preserve the configured succession delay"
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

// ===========================================================================
// Suite 6: social_tell_propagates_political_knowledge
//
// Proves: the social Tell system can lawfully move office knowledge into the
// political planning layer, unlocking the ordinary office-claim path without
// belief injection shortcuts or political/social coupling.
// Foundation: Principle 1, Principle 7, Principle 13, Principle 24.
// Cross-systems: Social + Beliefs + Travel + AI political planning + Politics.
// ===========================================================================

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

    let mut tell_commit_tick = None;
    for _ in 0..40 {
        h.step_once();
        if agent_belief_about(&h.world, listener, office).is_some() {
            tell_commit_tick = Some(h.scheduler.current_tick());
            break;
        }
    }

    let tell_commit_tick = tell_commit_tick
        .expect("listener should receive the office belief through the tell action");
    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled for social-political emergence");
    assert!(
        action_sink.events_for(informant).iter().any(|event| {
            event.tick <= tell_commit_tick
                && event.action_name == "tell"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        }),
        "office belief should arrive only after the informant has committed a tell action"
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

    let told_belief = agent_belief_about(&h.world, listener, office)
        .expect("listener should receive the office belief through the tell action");
    assert!(
        matches!(
            told_belief.source,
            PerceptionSource::Report {
                from,
                chain_len: 1
            } if from == informant
        ),
        "office belief should arrive as a first-hand report from the informant"
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

    let mut tell_commit_tick = None;
    for _ in 0..40 {
        h.step_once();
        if agent_belief_about(&h.world, listener, office).is_some() {
            tell_commit_tick = Some(h.scheduler.current_tick());
            break;
        }
    }

    let tell_commit_tick = tell_commit_tick
        .expect("listener should receive the same-place office belief through Tell");
    assert!(
        h.driver
            .trace_sink()
            .expect("decision tracing should be enabled for same-place social-political emergence")
            .goal_history_for(
                speaker,
                &GoalKind::ShareBelief {
                    listener,
                    subject: office,
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
            }),
        "same-place office belief should arrive only after the speaker commits Tell"
    );

    let told_belief = agent_belief_about(&h.world, listener, office)
        .expect("listener should receive the office belief through Tell");
    assert!(
        matches!(
            told_belief.source,
            PerceptionSource::Report {
                from,
                chain_len: 1
            } if from == speaker
        ),
        "listener should learn the same-place office fact as a first-hand report from the speaker"
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
    }

    h.step_once();

    let trace_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled for political start-failure scenario");
    assert_eq!(
        trace_sink
            .trace_at(loser, Tick(0))
            .and_then(|trace| match &trace.outcome {
                DecisionOutcome::Planning(planning) => planning.selection.selected.as_ref(),
                _ => None,
            })
            .map(|goal| goal.kind),
        Some(GoalKind::TreatWounds { patient: loser }),
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
    h.driver = worldwake_ai::AgentTickDriver::new(worldwake_ai::PlanningBudget::default());
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
                    OfficeSuccessionOutcome::SupportInstalled { holder, .. } if holder == winner
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
            .any(|goal| goal.kind == GoalKind::ClaimOffice { office }),
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
        subject: recent_subject,
    };
    let share_office = GoalKind::ShareBelief {
        listener,
        subject: office,
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
                RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief,
            );
        saw_office_generated |= speaker_trace.goal_status(&share_office).is_generated();

        if agent_belief_about(&h.world, listener, office).is_some() {
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
                        subject: recent_subject,
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
                        subject: office,
                    })
        })
        .expect("speaker should commit a tell for the office fact");
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

    let told_office_belief = agent_belief_about(&h.world, listener, office)
        .expect("listener should receive the office belief through Tell");
    assert!(
        matches!(
            told_office_belief.source,
            PerceptionSource::Report {
                from,
                chain_len: 1
            } if from == speaker
        ),
        "listener should receive the office fact as a first-hand report from the speaker"
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
        office_tell_commit_order < (
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
fn golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact_replays_deterministically(
) {
    let first =
        run_already_told_recent_subject_does_not_crowd_out_untold_office_fact(Seed([46; 32]));
    let second =
        run_already_told_recent_subject_does_not_crowd_out_untold_office_fact(Seed([46; 32]));
    assert_eq!(
        first, second,
        "crowd-out prevention should replay deterministically"
    );
}
