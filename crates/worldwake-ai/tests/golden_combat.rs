//! Golden tests for combat, death, and opportunistic looting.

mod golden_harness;

use std::collections::BTreeSet;

use golden_harness::*;
use worldwake_ai::derive_danger_pressure;
use worldwake_core::{
    hash_event_log, hash_world, total_live_lot_quantity, CombatProfile, CombatStance,
    CommodityKind, DeadAt, DeprivationExposure, EventTag, HomeostaticNeeds, KnownRecipes,
    MetabolismProfile, PrototypePlace, Quantity, ResourceSource, Seed, StateHash, Tick,
    UtilityProfile, WorkstationTag, WoundList,
};
use worldwake_sim::{OmniscientBeliefRuntime, OmniscientBeliefView};

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

fn seed_bleeding_recovery_patient(h: &mut GoldenHarness) -> worldwake_core::EntityId {
    let patient = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Recovery Patient",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_wound_list(
        patient,
        WoundList {
            wounds: vec![worldwake_core::Wound {
                id: worldwake_core::WoundId(1),
                body_part: worldwake_core::BodyPart::Torso,
                cause: worldwake_core::WoundCause::Deprivation(
                    worldwake_core::DeprivationKind::Starvation,
                ),
                severity: pm(50),
                inflicted_at: Tick(0),
                bleed_rate_per_tick: pm(100),
            }],
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);

    patient
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

fn run_wound_bleed_clotting_natural_recovery_scenario(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    let patient = seed_bleeding_recovery_patient(&mut h);
    let mut previous_severity = pm(50);
    let mut previous_bleed_rate = pm(100);
    let mut saw_bleed_phase = false;
    let mut saw_clotting = false;
    let mut saw_zero_bleed = false;
    let mut saw_recovery_phase = false;
    let mut wound_pruned = false;

    for _ in 0..32 {
        h.step_once();

        assert!(
            !h.agent_is_dead(patient),
            "recovery patient must stay alive throughout the wound lifecycle"
        );
        assert!(
            !h.agent_has_active_action(patient),
            "recovery patient should remain idle; this scenario should exercise passive wound progression rather than unrelated actions"
        );

        let needs = h
            .world
            .get_component_homeostatic_needs(patient)
            .expect("recovery patient should retain homeostatic needs");
        let thresholds = h
            .world
            .get_component_drive_thresholds(patient)
            .expect("recovery patient should retain drive thresholds");
        assert!(
            needs.hunger < thresholds.hunger.high()
                && needs.thirst < thresholds.thirst.high()
                && needs.fatigue < thresholds.fatigue.high(),
            "recovery gate should remain open on physiology throughout the scenario"
        );

        let wound_list = h
            .world
            .get_component_wound_list(patient)
            .expect("recovery patient should retain wound state component");

        if wound_list.wounds.is_empty() {
            wound_pruned = true;
            break;
        }

        let wound = &wound_list.wounds[0];
        saw_bleed_phase |= wound.severity > previous_severity;
        saw_clotting |= wound.bleed_rate_per_tick < previous_bleed_rate;
        saw_zero_bleed |= wound.bleed_rate_per_tick == pm(0);

        if previous_bleed_rate.value() > 0 {
            assert!(
                wound.severity >= previous_severity,
                "severity must not recover while the wound is still bleeding"
            );
        } else {
            saw_recovery_phase |= wound.severity < previous_severity;
        }

        previous_severity = wound.severity;
        previous_bleed_rate = wound.bleed_rate_per_tick;
    }

    assert!(saw_bleed_phase, "wound severity should rise during the bleed phase");
    assert!(
        saw_clotting,
        "bleed rate should fall under natural clot resistance"
    );
    assert!(
        saw_zero_bleed,
        "bleed rate should eventually reach zero before recovery begins"
    );
    assert!(
        saw_recovery_phase,
        "wound severity should fall after clotting completes"
    );
    assert!(wound_pruned, "recovered wound should be pruned from WoundList");

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
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

#[test]
fn golden_bury_corpse() {
    let mut h = GoldenHarness::new(Seed([14; 32]));
    let _burier = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Burier",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let corpse = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Corpse",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let grave_plot = place_workstation(
        &mut h.world,
        &mut h.event_log,
        VILLAGE_SQUARE,
        WorkstationTag::GravePlot,
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_dead_at(corpse, DeadAt(Tick(0))).unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    for _ in 0..50 {
        h.step_once();
        if h.world.direct_container(corpse).is_some() {
            break;
        }
    }

    let grave = h
        .world
        .direct_container(corpse)
        .expect("corpse should be buried into a grave container");
    assert_eq!(h.world.effective_place(grave), Some(VILLAGE_SQUARE));
    assert_eq!(h.world.effective_place(corpse), Some(VILLAGE_SQUARE));
    assert_eq!(h.world.get_component_dead_at(corpse), Some(&DeadAt(Tick(0))));
    assert!(h.world.is_alive(corpse), "burial should not delete the corpse entity");
    assert_eq!(
        h.world.get_component_workstation_marker(grave_plot).unwrap().0,
        WorkstationTag::GravePlot
    );
}

fn build_death_while_traveling_scenario(
    seed: Seed,
) -> (
    GoldenHarness,
    worldwake_core::EntityId,
    u64,
    worldwake_core::EntityId,
) {
    let mut h = GoldenHarness::new(seed);
    let bandit_camp = worldwake_core::prototype_place_entity(PrototypePlace::BanditCamp);

    let traveler = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Traveler",
        bandit_camp,
        HomeostaticNeeds::new(pm(850), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::new(
            pm(25),
            pm(3),
            pm(2),
            pm(4),
            pm(1),
            pm(20),
            nz(5),
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
        traveler,
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
    txn.set_component_deprivation_exposure(
        traveler,
        DeprivationExposure {
            hunger_critical_ticks: 0,
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
        traveler,
        bandit_camp,
        CommodityKind::Coin,
        Quantity(5),
    );

    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        ORCHARD_FARM,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(10),
            max_quantity: Quantity(10),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
        },
    );

    let initial_coin_total = total_live_lot_quantity(&h.world, CommodityKind::Coin);
    (h, traveler, initial_coin_total, bandit_camp)
}

fn run_death_while_traveling_scenario(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, traveler, initial_coin_total, origin) = build_death_while_traveling_scenario(seed);
    let forest_path = worldwake_core::prototype_place_entity(PrototypePlace::ForestPath);
    let mut left_origin = false;
    let mut saw_in_transit = false;
    let mut saw_active_travel = false;
    let mut reached_orchard = false;

    for _ in 0..100 {
        let was_in_transit = h.world.is_in_transit(traveler);

        h.step_once();
        if h.world.is_in_transit(traveler) {
            saw_in_transit = true;
        }
        if h.agent_has_active_action(traveler) {
            saw_active_travel = true;
        }
        if was_in_transit || h.world.effective_place(traveler) != Some(origin) {
            left_origin = true;
        }
        if h.world.effective_place(traveler) == Some(ORCHARD_FARM) {
            reached_orchard = true;
        }

        let coin_total = total_live_lot_quantity(&h.world, CommodityKind::Coin);
        assert_eq!(
            coin_total, initial_coin_total,
            "Coin lot conservation violated: expected {initial_coin_total}, got {coin_total}"
        );

        if h.agent_is_dead(traveler) {
            assert!(
                !h.agent_has_active_action(traveler),
                "Dead traveler should not retain an active action after death resolution"
            );
            assert!(
                !h.world.is_in_transit(traveler),
                "Dead traveler should not remain in transit after death resolution"
            );
            assert_eq!(
                h.world.effective_place(traveler),
                Some(forest_path),
                "Traveler should die at the intermediate route place reached before Orchard Farm"
            );
            break;
        }
    }

    assert!(
        left_origin,
        "Traveler should leave the origin to pursue distant food"
    );
    assert!(
        saw_in_transit,
        "Traveler should enter real in-transit state before death"
    );
    assert!(
        saw_active_travel,
        "Traveler should have an active travel action before death"
    );
    assert!(
        !reached_orchard,
        "Traveler should die before reaching Orchard Farm"
    );
    assert!(
        h.agent_is_dead(traveler),
        "Traveler should die from deprivation during the travel scenario"
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

    assert!(
        saw_attack_action,
        "attacker should commit to an attack action"
    );
    assert!(
        saw_combat_event,
        "living-combat scenario should emit at least one combat-tagged event"
    );
    assert!(
        defender_wounded,
        "defender should sustain at least one wound from living combat"
    );
    assert!(
        !h.agent_is_dead(attacker),
        "attacker should survive the scenario"
    );
    assert!(
        !h.agent_is_dead(defender),
        "defender should survive the scenario"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_reduce_danger_defensive_mitigation() {
    let (mut h, attacker, defender, initial_coin_total) = build_living_combat_scenario(Seed([23; 32]));
    let danger_high_threshold = h
        .world
        .get_component_drive_thresholds(defender)
        .expect("defender should have drive thresholds")
        .danger
        .high();
    let origin = h
        .world
        .effective_place(defender)
        .expect("defender should start at a concrete place");

    let mut saw_attacker_attack = false;
    let mut saw_defender_high_danger = false;
    let mut saw_defender_defend_action = false;
    let mut saw_defender_defending_stance = false;
    let mut saw_defender_relocate = false;
    let mut defender_actions = BTreeSet::new();

    for _ in 0..40 {
        h.step_once();

        let view = OmniscientBeliefView::with_runtime(
            &h.world,
            OmniscientBeliefRuntime::new(h.scheduler.active_actions(), &h.defs),
        );
        let defender_danger = derive_danger_pressure(&view, defender);
        saw_defender_high_danger |= defender_danger >= danger_high_threshold;

        saw_attacker_attack |= h.agent_active_action_name(attacker) == Some("attack");
        if let Some(action_name) = h.agent_active_action_name(defender) {
            defender_actions.insert(action_name.to_string());
        }
        saw_defender_defend_action |= h.agent_active_action_name(defender) == Some("defend");
        saw_defender_defending_stance |=
            h.agent_combat_stance(defender) == Some(CombatStance::Defending);
        saw_defender_relocate |= h.world.effective_place(defender) != Some(origin);

        let coin_total = total_live_lot_quantity(&h.world, CommodityKind::Coin);
        assert_eq!(
            coin_total, initial_coin_total,
            "Coin lot conservation violated: expected {initial_coin_total}, got {coin_total}"
        );

        if saw_attacker_attack
            && saw_defender_high_danger
            && (saw_defender_defend_action
                || saw_defender_defending_stance
                || saw_defender_relocate)
        {
            break;
        }
    }

    assert!(
        saw_attacker_attack,
        "attacker should initiate combat through the real attack action"
    );
    assert!(
        saw_defender_high_danger,
        "defender should reach high-or-above danger pressure under active attack"
    );
    assert!(
        saw_defender_defend_action || saw_defender_defending_stance || saw_defender_relocate,
        "defender should autonomously enter a concrete mitigation path such as defend or relocation; observed defender actions: {defender_actions:?}"
    );
}

#[test]
fn golden_wound_bleed_clotting_natural_recovery() {
    let _ = run_wound_bleed_clotting_natural_recovery_scenario(Seed([27; 32]));
}

#[test]
fn golden_wound_bleed_clotting_natural_recovery_replays_deterministically() {
    let first = run_wound_bleed_clotting_natural_recovery_scenario(Seed([28; 32]));
    let second = run_wound_bleed_clotting_natural_recovery_scenario(Seed([28; 32]));

    assert_eq!(
        first, second,
        "wound bleed/clot/recovery scenario should replay deterministically"
    );
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
fn golden_death_while_traveling() {
    let _ = run_death_while_traveling_scenario(Seed([12; 32]));
}

#[test]
fn golden_death_while_traveling_replays_deterministically() {
    let first = run_death_while_traveling_scenario(Seed([12; 32]));
    let second = run_death_while_traveling_scenario(Seed([12; 32]));

    assert_eq!(
        first, second,
        "death-while-traveling scenario should replay deterministically"
    );
}

#[test]
fn golden_combat_between_living_agents() {
    let _ = run_living_combat_scenario(Seed([21; 32]));
}

#[test]
fn golden_seed_sensitivity_living_combat_different_outcomes() {
    let seeds = [
        Seed([21; 32]),
        Seed([22; 32]),
        Seed([23; 32]),
        Seed([24; 32]),
        Seed([25; 32]),
        Seed([26; 32]),
    ];
    let mut outcomes = BTreeSet::new();

    for seed in seeds {
        outcomes.insert(run_living_combat_scenario(seed));
    }

    assert!(
        outcomes.len() > 1,
        "living-combat scenario should produce more than one valid outcome across a fixed set of distinct seeds"
    );
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
