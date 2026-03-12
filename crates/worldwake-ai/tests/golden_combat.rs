//! Golden tests for combat, death, and opportunistic looting.

mod golden_harness;

use golden_harness::*;
use worldwake_core::{
    hash_event_log, hash_world, total_live_lot_quantity, CombatProfile, CommodityKind,
    DeprivationExposure, EventTag, HomeostaticNeeds, KnownRecipes, MetabolismProfile, Quantity,
    Seed, StateHash, Tick, UtilityProfile, WoundList,
};

// ---------------------------------------------------------------------------
// Combat-specific helpers (only used by tests in this file)
// ---------------------------------------------------------------------------

fn seed_fragile_deprivation_victim(h: &mut GoldenHarness) -> worldwake_core::EntityId {
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Victim",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(950), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::new(
            pm(50),
            pm(3),
            pm(2),
            pm(4),
            pm(1),
            pm(20),
            nz(3),
            nz(240),
            nz(120),
            nz(40),
            nz(8),
            nz(12),
        ),
        UtilityProfile::default(),
    );

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_combat_profile(
        agent,
        CombatProfile::new(
            pm(200),
            pm(150),
            pm(500),
            pm(500),
            pm(80),
            pm(25),
            pm(18),
            pm(120),
            pm(35),
            nz(6),
        ),
    )
    .unwrap();
    txn.set_component_wound_list(
        agent,
        WoundList {
            wounds: vec![worldwake_core::Wound {
                id: worldwake_core::WoundId(1),
                body_part: worldwake_core::BodyPart::Torso,
                cause: worldwake_core::WoundCause::Deprivation(
                    worldwake_core::DeprivationKind::Starvation,
                ),
                severity: pm(150),
                inflicted_at: Tick(0),
                bleed_rate_per_tick: pm(0),
            }],
        },
    )
    .unwrap();
    txn.set_component_deprivation_exposure(
        agent,
        DeprivationExposure {
            hunger_critical_ticks: 2,
            thirst_critical_ticks: 0,
            fatigue_critical_ticks: 0,
            bladder_critical_ticks: 0,
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent,
        VILLAGE_SQUARE,
        CommodityKind::Coin,
        Quantity(5),
    );
    agent
}

fn build_death_and_loot_scenario(
    seed: Seed,
) -> (
    GoldenHarness,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
    u64,
) {
    let mut h = GoldenHarness::new(seed);
    let victim = seed_fragile_deprivation_victim(&mut h);
    let looter = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Looter",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(100), pm(0), pm(100), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let initial_coin_total = total_live_lot_quantity(&h.world, CommodityKind::Coin);

    (h, victim, looter, initial_coin_total)
}

fn run_death_and_loot_observation(
    h: &mut GoldenHarness,
    victim: worldwake_core::EntityId,
    looter: worldwake_core::EntityId,
    initial_coin_total: u64,
) -> (bool, bool) {
    let mut victim_died = false;
    let mut looter_gained_coin = false;

    for _ in 0..100 {
        h.step_once();

        if h.agent_is_dead(victim) {
            victim_died = true;
        }
        if h.agent_commodity_qty(looter, CommodityKind::Coin) > Quantity(0) {
            looter_gained_coin = true;
        }

        let coin_total = total_live_lot_quantity(&h.world, CommodityKind::Coin);
        assert_eq!(
            coin_total, initial_coin_total,
            "Coin lot conservation violated: expected {initial_coin_total}, got {coin_total}"
        );

        if victim_died && looter_gained_coin {
            break;
        }
    }

    (victim_died, looter_gained_coin)
}

fn run_death_and_loot_scenario(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, victim, looter, initial_coin_total) = build_death_and_loot_scenario(seed);
    let (victim_died, looter_gained_coin) =
        run_death_and_loot_observation(&mut h, victim, looter, initial_coin_total);

    assert!(
        victim_died,
        "Victim should die from deprivation wounds in the death-and-loot scenario"
    );
    assert!(
        looter_gained_coin,
        "Looter should gain coin within 100 ticks after the victim dies"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

fn living_combat_attacker_profile() -> CombatProfile {
    CombatProfile::new(
        pm(1000),
        pm(700),
        pm(900),
        pm(250),
        pm(40),
        pm(25),
        pm(18),
        pm(160),
        pm(35),
        nz(3),
    )
}

fn living_combat_defender_profile() -> CombatProfile {
    CombatProfile::new(
        pm(1000),
        pm(700),
        pm(350),
        pm(650),
        pm(120),
        pm(25),
        pm(18),
        pm(100),
        pm(20),
        nz(6),
    )
}

fn build_living_combat_scenario(
    seed: Seed,
) -> (
    GoldenHarness,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
    u64,
) {
    let mut h = GoldenHarness::new(seed);
    let attacker = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Attacker",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::new(),
    );
    let defender = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Defender",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::new(),
    );

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_combat_profile(attacker, living_combat_attacker_profile())
        .unwrap();
    txn.set_component_combat_profile(defender, living_combat_defender_profile())
        .unwrap();
    commit_txn(txn, &mut h.event_log);

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        attacker,
        VILLAGE_SQUARE,
        CommodityKind::Coin,
        Quantity(3),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        defender,
        VILLAGE_SQUARE,
        CommodityKind::Coin,
        Quantity(2),
    );
    add_hostility(&mut h.world, &mut h.event_log, attacker, defender);

    let initial_coin_total = total_live_lot_quantity(&h.world, CommodityKind::Coin);
    (h, attacker, defender, initial_coin_total)
}

fn run_living_combat_observation(
    h: &mut GoldenHarness,
    attacker: worldwake_core::EntityId,
    defender: worldwake_core::EntityId,
    initial_coin_total: u64,
) -> (bool, bool, bool) {
    let mut saw_attack_action = false;
    let mut saw_combat_event = false;
    let mut defender_wounded = false;

    for _ in 0..40 {
        h.step_once();

        saw_attack_action |= h.scheduler.active_actions().values().any(|instance| {
            instance.actor == attacker
                && h.defs
                    .get(instance.def_id)
                    .is_some_and(|def| def.name == "attack")
        });
        saw_combat_event |= !h.event_log.events_by_tag(EventTag::Combat).is_empty();
        defender_wounded |= h.agent_wound_load(defender) > 0;

        let coin_total = total_live_lot_quantity(&h.world, CommodityKind::Coin);
        assert_eq!(
            coin_total, initial_coin_total,
            "Coin lot conservation violated: expected {initial_coin_total}, got {coin_total}"
        );

        if saw_attack_action && saw_combat_event && defender_wounded {
            break;
        }
    }

    (saw_attack_action, saw_combat_event, defender_wounded)
}

fn run_living_combat_scenario(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, attacker, defender, initial_coin_total) = build_living_combat_scenario(seed);
    let (saw_attack_action, saw_combat_event, defender_wounded) =
        run_living_combat_observation(&mut h, attacker, defender, initial_coin_total);

    assert!(saw_attack_action, "attacker should commit to an attack action");
    assert!(
        saw_combat_event,
        "living-combat scenario should emit at least one combat-tagged event"
    );
    assert!(
        defender_wounded,
        "defender should sustain at least one wound from living combat"
    );
    assert!(!h.agent_is_dead(attacker), "attacker should survive the scenario");
    assert!(!h.agent_is_dead(defender), "defender should survive the scenario");

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Scenario 8: Death Cascade and Opportunistic Loot
// ---------------------------------------------------------------------------

#[test]
fn golden_death_cascade_and_opportunistic_loot() {
    let (mut h, agent_a, agent_b, initial_coin_total) =
        build_death_and_loot_scenario(Seed([8; 32]));
    let (a_died, b_looted) =
        run_death_and_loot_observation(&mut h, agent_a, agent_b, initial_coin_total);

    assert!(
        a_died,
        "Agent A should have died from deprivation wounds exceeding wound_capacity"
    );
    assert!(
        b_looted,
        "Agent B should have looted Agent A within 100 ticks after the deprivation death"
    );
}

#[test]
fn golden_death_cascade_and_opportunistic_loot_replays_deterministically() {
    let seed = Seed([8; 32]);

    let (world_hash_1, log_hash_1) = run_death_and_loot_scenario(seed);
    let (world_hash_2, log_hash_2) = run_death_and_loot_scenario(seed);

    assert_eq!(
        world_hash_1, world_hash_2,
        "Death-and-loot scenario must replay deterministically"
    );
    assert_eq!(
        log_hash_1, log_hash_2,
        "Death-and-loot event log must replay deterministically"
    );
}

#[test]
fn golden_combat_between_living_agents() {
    let _ = run_living_combat_scenario(Seed([21; 32]));
}

#[test]
fn golden_combat_between_living_agents_replays_deterministically() {
    let first = run_living_combat_scenario(Seed([22; 32]));
    let second = run_living_combat_scenario(Seed([22; 32]));

    assert_eq!(
        first, second,
        "living-combat scenario should replay deterministically"
    );
}
