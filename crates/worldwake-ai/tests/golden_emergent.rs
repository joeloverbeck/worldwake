//! Golden tests for cross-system emergent behavior involving care.
//!
//! These tests prove that care (S07) interacts with other systems —
//! metabolism, combat, loot, travel, transport — to produce emergent
//! multi-system chains.  No single system orchestrates these outcomes;
//! they emerge from concrete state + utility-driven AI ranking.

mod golden_harness;

use golden_harness::*;
use worldwake_core::{
    hash_event_log, hash_world, total_live_lot_quantity, BeliefConfidencePolicy, BodyPart,
    CombatProfile, CommodityKind, ComponentKind, ComponentValue, DeadAt, EventTag, EventView,
    HomeostaticNeeds, KnownRecipes, MetabolismProfile, PerceptionProfile, PerceptionSource,
    Quantity, RelationValue, Seed, StateHash, SuccessionLaw, Tick, UtilityProfile, Wound,
    WoundCause, WoundId, WoundList,
};
use worldwake_sim::{ActionTraceKind, OfficeSuccessionOutcome};

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
            event_id == death_event_id || event_id == vacancy_event_id || event_id == install_event_id
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
