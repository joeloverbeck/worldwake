//! Golden tests for combat, death, and opportunistic looting.

mod golden_harness;

use std::collections::BTreeSet;

use golden_harness::*;
use worldwake_ai::DecisionOutcome;
use worldwake_core::{
    hash_event_log, hash_world, total_live_lot_quantity, AgentData, CombatProfile, CombatStance,
    CommodityKind, ControlSource, DeadAt, DeprivationExposure, GoalKind, HomeostaticNeeds, KnownRecipes,
    MetabolismProfile, PrototypePlace, Quantity, ResourceSource, Seed, StateHash, Tick,
    UtilityProfile, WorkstationTag, Wound, WoundCause, WoundId, WoundList,
};
use worldwake_sim::{
    ActionDuration, ActionInstance, ActionPayload, ActionStatus, ActionTraceKind,
    CombatActionPayload,
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
            nz(10),
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
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );

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

fn build_loot_suppressed_under_self_care_scenario(
    seed: Seed,
) -> (
    GoldenHarness,
    worldwake_core::EntityId,
    u64,
    worldwake_core::Permille,
) {
    let mut h = GoldenHarness::new(seed);
    let scavenger = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Scavenger",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        scavenger,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
    );

    let corpse = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Corpse",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        corpse,
        VILLAGE_SQUARE,
        CommodityKind::Coin,
        Quantity(5),
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_dead_at(corpse, DeadAt(Tick(0))).unwrap();
        commit_txn(txn, &mut h.event_log);
        seed_actor_local_beliefs(
            &mut h.world,
            &mut h.event_log,
            scavenger,
            Tick(0),
            worldwake_core::PerceptionSource::DirectObservation,
        );
    }

    let initial_coin_total = total_live_lot_quantity(&h.world, CommodityKind::Coin);
    let hunger_high = h
        .world
        .get_component_drive_thresholds(scavenger)
        .expect("scavenger should have drive thresholds")
        .hunger
        .high();

    (h, scavenger, initial_coin_total, hunger_high)
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
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        patient,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );

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

fn run_loot_suppressed_under_self_care_scenario(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, scavenger, initial_coin_total, hunger_high) =
        build_loot_suppressed_under_self_care_scenario(seed);
    let mut first_eat_tick = None;
    let mut first_hunger_below_high_tick = None;
    let mut first_loot_tick = None;

    for tick in 0..40 {
        h.step_once();

        if first_eat_tick.is_none() && h.agent_active_action_name(scavenger) == Some("eat") {
            first_eat_tick = Some(tick);
        }

        let hunger = h.agent_hunger(scavenger);
        if first_hunger_below_high_tick.is_none() && hunger < hunger_high {
            first_hunger_below_high_tick = Some(tick);
        }

        let scavenger_coin = h.agent_commodity_qty(scavenger, CommodityKind::Coin);
        if first_loot_tick.is_none() && scavenger_coin > Quantity(0) {
            first_loot_tick = Some(tick);
        }

        let coin_total = total_live_lot_quantity(&h.world, CommodityKind::Coin);
        assert_eq!(
            coin_total, initial_coin_total,
            "Coin lot conservation violated: expected {initial_coin_total}, got {coin_total}"
        );

        if hunger >= hunger_high {
            assert_eq!(
                scavenger_coin,
                Quantity(0),
                "Scavenger should not gain corpse coins while hunger remains high-or-above"
            );
        }

        if first_eat_tick.is_some()
            && first_hunger_below_high_tick.is_some()
            && first_loot_tick.is_some()
        {
            break;
        }
    }

    let eat_tick = first_eat_tick.expect("Scavenger should begin eating before looting");
    let hunger_relief_tick = first_hunger_below_high_tick
        .expect("Scavenger hunger should fall below the high threshold after eating");
    let loot_tick =
        first_loot_tick.expect("Scavenger should loot the corpse after self-care pressure lifts");

    assert!(
        eat_tick < loot_tick,
        "Scavenger should start eating before corpse loot resolves"
    );
    assert!(
        hunger_relief_tick < loot_tick,
        "Corpse loot should only resolve after hunger falls below the high threshold"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
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

    assert!(
        saw_bleed_phase,
        "wound severity should rise during the bleed phase"
    );
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
    assert!(
        wound_pruned,
        "recovered wound should be pruned from WoundList"
    );

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
    let burier = seed_agent(
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
        ProductionOutputOwner::Actor,
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_dead_at(corpse, DeadAt(Tick(0))).unwrap();
        commit_txn(txn, &mut h.event_log);
        seed_actor_local_beliefs(
            &mut h.world,
            &mut h.event_log,
            burier,
            Tick(0),
            worldwake_core::PerceptionSource::DirectObservation,
        );
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
    assert_eq!(
        h.world.get_component_dead_at(corpse),
        Some(&DeadAt(Tick(0)))
    );
    assert!(
        h.world.is_alive(corpse),
        "burial should not delete the corpse entity"
    );
    assert_eq!(
        h.world
            .get_component_workstation_marker(grave_plot)
            .unwrap()
            .0,
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
            nz(10),
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
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        traveler,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        traveler,
        bandit_camp,
        CommodityKind::Coin,
        Quantity(5),
    );

    let _orchard_source = place_workstation_with_source(
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
        ProductionOutputOwner::Actor,
    );
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        traveler,
        Tick(0),
        worldwake_core::PerceptionSource::Inference,
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
        nz(10),
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
        nz(10),
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
    let combat_arena = worldwake_core::prototype_place_entity(PrototypePlace::BanditCamp);
    let attacker = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Attacker",
        combat_arena,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::new(),
    );
    let defender = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        "Defender",
        combat_arena,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::new(),
    );

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_agent_data(
        attacker,
        AgentData {
            control_source: ControlSource::Human,
        },
    )
    .unwrap();
    txn.set_component_combat_profile(attacker, living_combat_attacker_profile())
        .unwrap();
    txn.set_component_combat_profile(defender, living_combat_defender_profile())
        .unwrap();
    commit_txn(txn, &mut h.event_log);
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        attacker,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        defender,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        attacker,
        combat_arena,
        CommodityKind::Coin,
        Quantity(3),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        defender,
        combat_arena,
        CommodityKind::Coin,
        Quantity(2),
    );
    add_hostility(&mut h.world, &mut h.event_log, attacker, defender);
    let attack_def = h
        .defs
        .iter()
        .find(|def| def.name == "attack")
        .expect("combat registries should include attack");
    let attack_instance_id = h.scheduler.allocate_instance_id();
    h.scheduler.insert_action(ActionInstance {
        instance_id: attack_instance_id,
        def_id: attack_def.id,
        payload: ActionPayload::Combat(CombatActionPayload {
            target: defender,
            weapon: worldwake_core::CombatWeaponRef::Unarmed,
        }),
        actor: attacker,
        targets: vec![defender],
        start_tick: Tick(0),
        remaining_duration: ActionDuration::Finite(3),
        status: ActionStatus::Active,
        reservation_ids: Vec::new(),
        local_state: None,
    });

    let initial_coin_total = total_live_lot_quantity(&h.world, CommodityKind::Coin);
    (h, attacker, defender, initial_coin_total)
}

fn run_living_combat_observation(
    h: &mut GoldenHarness,
    attacker: worldwake_core::EntityId,
    _defender: worldwake_core::EntityId,
    initial_coin_total: u64,
) -> bool {
    let mut saw_attack_action = false;

    for _ in 0..40 {
        h.step_once();

        saw_attack_action |= h.scheduler.active_actions().values().any(|instance| {
            instance.actor == attacker
                && h.defs
                    .get(instance.def_id)
                    .is_some_and(|def| def.name == "attack")
        });

        let coin_total = total_live_lot_quantity(&h.world, CommodityKind::Coin);
        assert_eq!(
            coin_total, initial_coin_total,
            "Coin lot conservation violated: expected {initial_coin_total}, got {coin_total}"
        );

        if saw_attack_action {
            break;
        }
    }

    saw_attack_action
}

fn run_living_combat_scenario(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, attacker, defender, initial_coin_total) = build_living_combat_scenario(seed);
    let saw_attack_action =
        run_living_combat_observation(&mut h, attacker, defender, initial_coin_total);

    assert!(
        saw_attack_action,
        "attacker should commit to an attack action"
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
    let (mut h, attacker, defender, initial_coin_total) =
        build_living_combat_scenario(Seed([23; 32]));
    h.driver.enable_tracing();
    let origin = h
        .world
        .effective_place(defender)
        .expect("defender should start at a concrete place");

    let mut saw_attacker_attack = false;
    let mut saw_reduce_danger_selection = false;
    let mut saw_defender_defend_action = false;
    let mut saw_defender_defending_stance = false;
    let mut saw_defender_relocate = false;
    let mut defender_actions = BTreeSet::new();

    for _ in 0..40 {
        h.step_once();
        let current_tick = Tick(h.scheduler.current_tick().0.saturating_sub(1));
        saw_reduce_danger_selection |= h
            .driver
            .trace_sink()
            .expect("decision tracing should be enabled")
            .trace_at(defender, current_tick)
            .is_some_and(|trace| match &trace.outcome {
                DecisionOutcome::Planning(planning) => planning
                    .selection
                    .selected
                    .as_ref()
                    .is_some_and(|goal| goal.kind == GoalKind::ReduceDanger),
                _ => false,
            });

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
            && saw_reduce_danger_selection
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
        saw_reduce_danger_selection,
        "defender should select the ReduceDanger goal under active attack"
    );
    assert!(
        saw_defender_defend_action || saw_defender_defending_stance || saw_defender_relocate,
        "defender should autonomously enter a concrete mitigation path such as defend or relocation; observed defender actions: {defender_actions:?}"
    );
}

#[test]
fn golden_defend_replans_after_finite_stance_expires() {
    let (mut h, _attacker, defender, _initial_coin_total) =
        build_living_combat_scenario(Seed([33; 32]));
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_combat_profile(
        defender,
        CombatProfile::new(
            pm(1000),
            pm(700),
            pm(50),
            pm(900),
            pm(500),
            pm(25),
            pm(18),
            pm(100),
            pm(20),
            nz(6),
            nz(3),
        ),
    )
    .unwrap();
    txn.set_component_wound_list(
        defender,
        WoundList {
            wounds: vec![Wound {
                id: WoundId(1),
                body_part: worldwake_core::BodyPart::Torso,
                cause: WoundCause::Deprivation(worldwake_core::DeprivationKind::Starvation),
                severity: pm(120),
                inflicted_at: Tick(0),
                bleed_rate_per_tick: pm(0),
            }],
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);

    let defend_def = h
        .defs
        .iter()
        .find(|def| def.name == "defend")
        .expect("combat registries should include defend");
    let defend_instance = ActionInstance {
        instance_id: h.scheduler.allocate_instance_id(),
        def_id: defend_def.id,
        payload: ActionPayload::None,
        actor: defender,
        targets: Vec::new(),
        start_tick: Tick(0),
        remaining_duration: ActionDuration::Finite(3),
        status: ActionStatus::Active,
        reservation_ids: Vec::new(),
        local_state: None,
    };
    h.scheduler.insert_action(defend_instance);
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_combat_stance(defender, CombatStance::Defending)
        .unwrap();
    commit_txn(txn, &mut h.event_log);
    assert_eq!(h.agent_active_action_name(defender), Some("defend"));

    for _ in 0..30 {
        h.step_once();
    }

    let trace_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled");
    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let defender_events = action_sink.events_for(defender);
    let seeded_defend_commit_tick = defender_events.iter().find_map(|event| {
        (event.action_name == "defend"
            && matches!(event.kind, ActionTraceKind::Committed { .. }))
        .then_some(event.tick)
    });
    let replans_after_seeded_defend = (1u64..=30).any(|tick| {
        trace_sink
            .trace_at(defender, Tick(tick))
            .is_some_and(|trace| matches!(trace.outcome, DecisionOutcome::Planning(_)))
    });
    let resumed_action_after_commit = seeded_defend_commit_tick.is_some_and(|commit_tick| {
        defender_events.iter().any(|event| {
            event.tick > commit_tick
                && matches!(
                    event.kind,
                    ActionTraceKind::Started { .. } | ActionTraceKind::Committed { .. }
                )
        })
    });

    assert!(
        replans_after_seeded_defend,
        "the defender should re-enter the decision pipeline after the seeded defend commitment"
    );
    assert!(
        seeded_defend_commit_tick.is_some(),
        "the seeded finite defend should commit; events: {defender_events:?}"
    );
    assert!(
        resumed_action_after_commit,
        "the defender should start or commit another action after the seeded finite defend resolves; events: {defender_events:?}"
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
fn golden_loot_suppressed_under_self_care_pressure() {
    let _ = run_loot_suppressed_under_self_care_scenario(Seed([29; 32]));
}

#[test]
fn golden_loot_suppressed_under_self_care_pressure_replays_deterministically() {
    let seed = Seed([29; 32]);

    let first = run_loot_suppressed_under_self_care_scenario(seed);
    let second = run_loot_suppressed_under_self_care_scenario(seed);

    assert_eq!(
        first, second,
        "loot suppression under self-care pressure should replay deterministically"
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

// ---------------------------------------------------------------------------
// Scenario S03a: Multi-Corpse Loot Binding (S03 — matches_binding)
// ---------------------------------------------------------------------------

fn build_multi_corpse_loot_binding_scenario(
    seed: Seed,
) -> (
    GoldenHarness,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
    u64,
    u64,
) {
    let mut h = GoldenHarness::new(seed);

    let corpse_a = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "CorpseA",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        corpse_a,
        VILLAGE_SQUARE,
        CommodityKind::Coin,
        Quantity(5),
    );

    let corpse_b = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "CorpseB",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        corpse_b,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(3),
    );

    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_dead_at(corpse_a, DeadAt(Tick(0)))
            .unwrap();
        txn.set_component_dead_at(corpse_b, DeadAt(Tick(0)))
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    let looter = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Looter",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        looter,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );

    let initial_coin_total = total_live_lot_quantity(&h.world, CommodityKind::Coin);
    let initial_bread_total = total_live_lot_quantity(&h.world, CommodityKind::Bread);

    (
        h,
        corpse_a,
        corpse_b,
        looter,
        initial_coin_total,
        initial_bread_total,
    )
}

fn run_multi_corpse_loot_binding_scenario(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, corpse_a, corpse_b, looter, initial_coin_total, initial_bread_total) =
        build_multi_corpse_loot_binding_scenario(seed);

    // Loot completes within a single tick, so we observe sequential acquisition
    // between ticks rather than active loot targets mid-tick.
    let mut first_coin_tick = None;
    let mut first_bread_tick = None;
    let mut sequential_looting_verified = true;

    for tick in 0..30 {
        h.step_once();

        // Conservation every tick.
        let coin_total = total_live_lot_quantity(&h.world, CommodityKind::Coin);
        assert_eq!(
            coin_total, initial_coin_total,
            "Coin lot conservation violated: expected {initial_coin_total}, got {coin_total}"
        );
        let bread_total = total_live_lot_quantity(&h.world, CommodityKind::Bread);
        assert_eq!(
            bread_total, initial_bread_total,
            "Bread lot conservation violated: expected {initial_bread_total}, got {bread_total}"
        );

        let looter_coin = h.agent_commodity_qty(looter, CommodityKind::Coin);
        let looter_bread = h.agent_commodity_qty(looter, CommodityKind::Bread);

        if first_coin_tick.is_none() && looter_coin > Quantity(0) {
            first_coin_tick = Some(tick);
        }
        if first_bread_tick.is_none() && looter_bread > Quantity(0) {
            first_bread_tick = Some(tick);
        }

        // While only one corpse has been looted, the other must retain its items.
        if looter_coin > Quantity(0) && looter_bread == Quantity(0) {
            // Coin gained but not bread — corpse_b must still have its bread.
            let corpse_b_bread = h.agent_commodity_qty(corpse_b, CommodityKind::Bread);
            if corpse_b_bread == Quantity(0) {
                sequential_looting_verified = false;
            }
        }
        if looter_bread > Quantity(0) && looter_coin == Quantity(0) {
            // Bread gained but not coin — corpse_a must still have its coin.
            let corpse_a_coin = h.agent_commodity_qty(corpse_a, CommodityKind::Coin);
            if corpse_a_coin == Quantity(0) {
                sequential_looting_verified = false;
            }
        }

        if looter_coin > Quantity(0) && looter_bread > Quantity(0) {
            break;
        }
    }

    assert!(
        first_coin_tick.is_some() || first_bread_tick.is_some(),
        "Looter should have looted at least one corpse"
    );
    assert!(
        first_coin_tick.is_some() && first_bread_tick.is_some(),
        "Looter should eventually loot both corpses (gaining both Coin and Bread)"
    );
    assert!(
        sequential_looting_verified,
        "While looting the first corpse, the other corpse's inventory should remain untouched (binding targets one corpse at a time)"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_multi_corpse_loot_binding() {
    let _ = run_multi_corpse_loot_binding_scenario(Seed([30; 32]));
}

#[test]
fn golden_multi_corpse_loot_binding_replays_deterministically() {
    let first = run_multi_corpse_loot_binding_scenario(Seed([30; 32]));
    let second = run_multi_corpse_loot_binding_scenario(Seed([30; 32]));

    assert_eq!(
        first, second,
        "multi-corpse loot binding scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario S03b: Bury Suppressed Under Stress (S02 — evaluate_suppression)
// ---------------------------------------------------------------------------

fn build_bury_suppressed_under_stress_scenario(
    seed: Seed,
) -> (
    GoldenHarness,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
    worldwake_core::Permille,
) {
    let mut h = GoldenHarness::new(seed);

    // Corpse with NO loot — prevents LootCorpse goals from interfering.
    let corpse = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Corpse",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    let _grave_plot = place_workstation(
        &mut h.world,
        &mut h.event_log,
        VILLAGE_SQUARE,
        WorkstationTag::GravePlot,
        ProductionOutputOwner::Actor,
    );

    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_dead_at(corpse, DeadAt(Tick(0))).unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Burier with hunger above High threshold, plus bread to eat.
    let burier = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Burier",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        burier,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
    );

    let hunger_high = h
        .world
        .get_component_drive_thresholds(burier)
        .expect("burier should have drive thresholds")
        .hunger
        .high();

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        burier,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );

    (h, corpse, burier, hunger_high)
}

fn run_bury_suppressed_under_stress_scenario(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, corpse, burier, hunger_high) = build_bury_suppressed_under_stress_scenario(seed);

    let mut first_eat_tick = None;
    let mut first_hunger_below_high_tick = None;
    let mut first_bury_tick = None;

    for tick in 0..50 {
        h.step_once();

        if first_eat_tick.is_none() && h.agent_active_action_name(burier) == Some("eat") {
            first_eat_tick = Some(tick);
        }

        let hunger = h.agent_hunger(burier);
        if first_hunger_below_high_tick.is_none() && hunger < hunger_high {
            first_hunger_below_high_tick = Some(tick);
        }

        if first_bury_tick.is_none() && h.world.direct_container(corpse).is_some() {
            first_bury_tick = Some(tick);
        }

        // While hunger remains high-or-above, corpse must NOT be buried.
        if hunger >= hunger_high {
            assert!(
                h.world.direct_container(corpse).is_none(),
                "Corpse should not be buried while burier hunger remains at or above the high threshold (burial suppressed)"
            );
        }

        if first_eat_tick.is_some()
            && first_hunger_below_high_tick.is_some()
            && first_bury_tick.is_some()
        {
            break;
        }
    }

    let eat_tick = first_eat_tick.expect("Burier should eat bread before burying");
    let hunger_relief_tick = first_hunger_below_high_tick
        .expect("Burier hunger should fall below the high threshold after eating");
    let bury_tick =
        first_bury_tick.expect("Burier should bury the corpse after self-care pressure lifts");

    assert!(
        eat_tick < bury_tick,
        "Burier should start eating before corpse burial completes"
    );
    assert!(
        hunger_relief_tick < bury_tick,
        "Corpse burial should only complete after hunger falls below the high threshold"
    );

    // Final state: corpse is in a grave container at VILLAGE_SQUARE.
    let grave = h
        .world
        .direct_container(corpse)
        .expect("corpse should be buried into a grave container");
    assert_eq!(h.world.effective_place(grave), Some(VILLAGE_SQUARE));
    assert_eq!(h.world.effective_place(corpse), Some(VILLAGE_SQUARE));

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_bury_suppressed_under_stress() {
    let _ = run_bury_suppressed_under_stress_scenario(Seed([31; 32]));
}

#[test]
fn golden_bury_suppressed_under_stress_replays_deterministically() {
    let first = run_bury_suppressed_under_stress_scenario(Seed([31; 32]));
    let second = run_bury_suppressed_under_stress_scenario(Seed([31; 32]));

    assert_eq!(
        first, second,
        "bury suppressed under stress scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario S03c: Suppression Then Binding Combined (S02 + S03)
// ---------------------------------------------------------------------------

fn build_suppression_then_binding_scenario(
    seed: Seed,
) -> (
    GoldenHarness,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
    u64,
    worldwake_core::Permille,
) {
    let mut h = GoldenHarness::new(seed);

    let corpse_a = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "CorpseA",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        corpse_a,
        VILLAGE_SQUARE,
        CommodityKind::Coin,
        Quantity(5),
    );

    let corpse_b = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "CorpseB",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        corpse_b,
        VILLAGE_SQUARE,
        CommodityKind::Coin,
        Quantity(3),
    );

    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_dead_at(corpse_a, DeadAt(Tick(0)))
            .unwrap();
        txn.set_component_dead_at(corpse_b, DeadAt(Tick(0)))
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    // Scavenger with hunger above High threshold, plus bread.
    let scavenger = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Scavenger",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        scavenger,
        VILLAGE_SQUARE,
        CommodityKind::Bread,
        Quantity(1),
    );

    let hunger_high = h
        .world
        .get_component_drive_thresholds(scavenger)
        .expect("scavenger should have drive thresholds")
        .hunger
        .high();

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        scavenger,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );

    let initial_coin_total = total_live_lot_quantity(&h.world, CommodityKind::Coin);

    (
        h,
        corpse_a,
        corpse_b,
        scavenger,
        initial_coin_total,
        hunger_high,
    )
}

fn run_suppression_then_binding_scenario(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, corpse_a, corpse_b, scavenger, initial_coin_total, hunger_high) =
        build_suppression_then_binding_scenario(seed);

    let mut first_eat_tick = None;
    let mut first_hunger_below_high_tick = None;
    let mut first_loot_tick = None;
    let mut sequential_looting_verified = true;
    let mut both_looted = false;

    // Track which corpse lost coins first to verify sequential binding.
    let mut first_looted_corpse: Option<worldwake_core::EntityId> = None;

    for tick in 0..50 {
        h.step_once();

        // Conservation every tick.
        let coin_total = total_live_lot_quantity(&h.world, CommodityKind::Coin);
        assert_eq!(
            coin_total, initial_coin_total,
            "Coin lot conservation violated: expected {initial_coin_total}, got {coin_total}"
        );

        if first_eat_tick.is_none() && h.agent_active_action_name(scavenger) == Some("eat") {
            first_eat_tick = Some(tick);
        }

        let hunger = h.agent_hunger(scavenger);
        if first_hunger_below_high_tick.is_none() && hunger < hunger_high {
            first_hunger_below_high_tick = Some(tick);
        }

        let scavenger_coin = h.agent_commodity_qty(scavenger, CommodityKind::Coin);

        // While hunger >= high, scavenger must not gain any coins (suppression).
        if hunger >= hunger_high {
            assert_eq!(
                scavenger_coin,
                Quantity(0),
                "Scavenger should not gain corpse coins while hunger remains high-or-above (suppression active)"
            );
        }

        if first_loot_tick.is_none() && scavenger_coin > Quantity(0) {
            first_loot_tick = Some(tick);
        }

        // Track binding correctness via item observation between ticks.
        let first_corpse_remaining = h.agent_commodity_qty(corpse_a, CommodityKind::Coin);
        let second_corpse_remaining = h.agent_commodity_qty(corpse_b, CommodityKind::Coin);

        if first_looted_corpse.is_none() {
            if first_corpse_remaining < Quantity(5) {
                first_looted_corpse = Some(corpse_a);
            } else if second_corpse_remaining < Quantity(3) {
                first_looted_corpse = Some(corpse_b);
            }
        }

        // While the first corpse is being looted, the other must retain its full coin count.
        if let Some(first) = first_looted_corpse {
            if first == corpse_a && first_corpse_remaining > Quantity(0) {
                // corpse_a partially looted — corpse_b must still be full.
                if second_corpse_remaining < Quantity(3) {
                    sequential_looting_verified = false;
                }
            } else if first == corpse_b && second_corpse_remaining > Quantity(0) {
                // corpse_b partially looted — corpse_a must still be full.
                if first_corpse_remaining < Quantity(5) {
                    sequential_looting_verified = false;
                }
            }
        }

        // Check if both corpses have been fully looted (total of 8 coins transferred).
        if scavenger_coin == Quantity(8) {
            both_looted = true;
            break;
        }
    }

    let eat_tick = first_eat_tick.expect("Scavenger should eat bread before looting");
    let hunger_relief_tick = first_hunger_below_high_tick
        .expect("Scavenger hunger should fall below the high threshold after eating");
    let loot_tick =
        first_loot_tick.expect("Scavenger should loot corpse coins after suppression lifts");

    assert!(
        eat_tick < loot_tick,
        "Scavenger should start eating before gaining any corpse loot (suppression)"
    );
    assert!(
        hunger_relief_tick < loot_tick,
        "Corpse loot should only resolve after hunger falls below the high threshold"
    );
    assert!(
        first_looted_corpse.is_some(),
        "At least one corpse should have been looted"
    );
    assert!(
        sequential_looting_verified,
        "While looting one corpse, the other corpse's coins should remain intact (binding targets one corpse at a time)"
    );
    assert!(
        both_looted,
        "Scavenger should eventually loot all 8 coins from both corpses"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_suppression_then_binding_combined() {
    let _ = run_suppression_then_binding_scenario(Seed([32; 32]));
}

#[test]
fn golden_suppression_then_binding_combined_replays_deterministically() {
    let first = run_suppression_then_binding_scenario(Seed([32; 32]));
    let second = run_suppression_then_binding_scenario(Seed([32; 32]));

    assert_eq!(
        first, second,
        "suppression-then-binding combined scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Action trace integration
// ---------------------------------------------------------------------------

#[test]
fn golden_action_trace_records_loot_lifecycle() {
    let (mut h, _corpse_a, _corpse_b, looter, _, _) =
        build_multi_corpse_loot_binding_scenario(Seed([30; 32]));
    h.enable_action_tracing();

    for _ in 0..10 {
        h.step_once();
    }

    let sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let looter_events = sink.events_for(looter);

    // The looter should have at least 2 Started + 2 Committed events (one per corpse loot).
    let started_count = looter_events
        .iter()
        .filter(|e| matches!(e.kind, ActionTraceKind::Started { .. }))
        .count();
    let committed_count = looter_events
        .iter()
        .filter(|e| matches!(e.kind, ActionTraceKind::Committed { .. }))
        .count();

    assert!(
        started_count >= 2,
        "Looter should have at least 2 Started trace events (one per corpse); got {started_count}"
    );
    assert!(
        committed_count >= 2,
        "Looter should have at least 2 Committed trace events; got {committed_count}"
    );

    // Every Started event should have a matching Committed event at the same or later tick.
    for event in &looter_events {
        if let ActionTraceKind::Started { .. } = &event.kind {
            let has_commit = looter_events.iter().any(|e| {
                matches!(e.kind, ActionTraceKind::Committed { .. })
                    && e.action_name == event.action_name
                    && e.tick >= event.tick
            });
            assert!(
                has_commit,
                "Started '{}' at tick {} should have a matching Committed event",
                event.action_name, event.tick.0
            );
        }
    }

    // Verify loot actions specifically complete in the same tick they start.
    let loot_starts: Vec<_> = looter_events
        .iter()
        .filter(|e| e.action_name == "loot" && matches!(e.kind, ActionTraceKind::Started { .. }))
        .collect();
    let loot_commits: Vec<_> = looter_events
        .iter()
        .filter(|e| e.action_name == "loot" && matches!(e.kind, ActionTraceKind::Committed { .. }))
        .collect();

    assert_eq!(
        loot_starts.len(),
        loot_commits.len(),
        "Every loot start should have a corresponding commit"
    );

    for start in &loot_starts {
        let same_tick_commit = loot_commits.iter().any(|c| c.tick == start.tick);
        assert!(
            same_tick_commit,
            "Loot action started at tick {} should commit in the same tick (1-tick action)",
            start.tick.0
        );
    }
}
