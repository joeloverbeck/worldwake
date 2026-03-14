//! Golden tests for care-domain behavior.

mod golden_harness;

use golden_harness::*;
use worldwake_core::{
    hash_event_log, hash_world, total_live_lot_quantity, BodyPart, CommodityKind, DeprivationKind,
    HomeostaticNeeds, MetabolismProfile, Quantity, Seed, StateHash, Tick, UtilityProfile, Wound,
    WoundCause, WoundId, WoundList,
};

fn seed_wounded_patient(h: &mut GoldenHarness) -> worldwake_core::EntityId {
    let patient = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Patient",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(100), pm(0), pm(100), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_wound_list(
        patient,
        WoundList {
            wounds: vec![Wound {
                id: WoundId(1),
                body_part: BodyPart::Torso,
                cause: WoundCause::Deprivation(DeprivationKind::Starvation),
                severity: pm(360),
                inflicted_at: Tick(0),
                bleed_rate_per_tick: pm(60),
            }],
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);

    patient
}

fn run_healing_scenario(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    let healer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Healer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(100), pm(0), pm(100), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let patient = seed_wounded_patient(&mut h);

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        healer,
        VILLAGE_SQUARE,
        CommodityKind::Medicine,
        Quantity(1),
    );

    let initial_medicine = h.agent_commodity_qty(healer, CommodityKind::Medicine);
    let initial_wound_load = h.agent_wound_load(patient);
    let initial_total_medicine = total_live_lot_quantity(&h.world, CommodityKind::Medicine);

    let mut medicine_consumed = false;
    let mut wound_load_decreased = false;

    for _ in 0..80 {
        h.step_once();

        let healer_medicine = h.agent_commodity_qty(healer, CommodityKind::Medicine);
        let patient_wound_load = h.agent_wound_load(patient);
        let total_medicine = total_live_lot_quantity(&h.world, CommodityKind::Medicine);

        medicine_consumed |= healer_medicine < initial_medicine;
        wound_load_decreased |= patient_wound_load < initial_wound_load;

        assert!(
            total_medicine <= initial_total_medicine,
            "medicine lots should not increase: initial={initial_total_medicine}, now={total_medicine}"
        );
        assert!(!h.agent_is_dead(healer), "healer must stay alive");
        assert!(!h.agent_is_dead(patient), "patient must stay alive");

        if medicine_consumed && wound_load_decreased {
            break;
        }
    }

    assert!(
        medicine_consumed,
        "healer should consume medicine while treating the wounded patient"
    );
    assert!(
        wound_load_decreased,
        "patient wound load should decrease after the heal action completes"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

fn place_ground_commodity(
    h: &mut GoldenHarness,
    place: worldwake_core::EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
) -> worldwake_core::EntityId {
    let mut txn = new_txn(&mut h.world, 0);
    let lot = txn.create_item_lot(commodity, quantity).unwrap();
    txn.set_ground_location(lot, place).unwrap();
    commit_txn(txn, &mut h.event_log);
    lot
}

fn run_healer_acquires_ground_medicine_for_patient(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    let healer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Healer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(100), pm(0), pm(100), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let patient = seed_wounded_patient(&mut h);
    let _medicine =
        place_ground_commodity(&mut h, VILLAGE_SQUARE, CommodityKind::Medicine, Quantity(1));

    let initial_medicine = h.agent_commodity_qty(healer, CommodityKind::Medicine);
    let initial_wound_load = h.agent_wound_load(patient);
    let initial_total_medicine = total_live_lot_quantity(&h.world, CommodityKind::Medicine);

    let mut medicine_acquired = false;
    let mut medicine_consumed = false;
    let mut wound_load_decreased = false;

    for _ in 0..80 {
        h.step_once();

        let healer_medicine = h.agent_commodity_qty(healer, CommodityKind::Medicine);
        let patient_wound_load = h.agent_wound_load(patient);
        let total_medicine = total_live_lot_quantity(&h.world, CommodityKind::Medicine);

        medicine_acquired |= healer_medicine > initial_medicine;
        medicine_consumed |= medicine_acquired && healer_medicine == Quantity(0);
        wound_load_decreased |= patient_wound_load < initial_wound_load;

        assert!(
            total_medicine <= initial_total_medicine,
            "medicine lots should not increase: initial={initial_total_medicine}, now={total_medicine}"
        );
        assert!(!h.agent_is_dead(healer), "healer must stay alive");
        assert!(!h.agent_is_dead(patient), "patient must stay alive");

        if medicine_acquired && medicine_consumed && wound_load_decreased {
            break;
        }
    }

    assert!(
        medicine_acquired,
        "healer should acquire accessible medicine before healing the patient"
    );
    assert!(
        medicine_consumed,
        "healer should consume medicine while treating the patient"
    );
    assert!(
        wound_load_decreased,
        "patient wound load should decrease after the healer acquires and uses medicine"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_healing_wounded_agent() {
    let _ = run_healing_scenario(Seed([14; 32]));
}

#[test]
fn golden_healing_wounded_agent_replays_deterministically() {
    let first = run_healing_scenario(Seed([15; 32]));
    let second = run_healing_scenario(Seed([15; 32]));

    assert_eq!(
        first, second,
        "healing scenario should replay deterministically"
    );
}

#[test]
fn golden_healer_acquires_ground_medicine_for_patient() {
    let _ = run_healer_acquires_ground_medicine_for_patient(Seed([16; 32]));
}

#[test]
fn golden_healer_acquires_ground_medicine_for_patient_replays_deterministically() {
    let first = run_healer_acquires_ground_medicine_for_patient(Seed([17; 32]));
    let second = run_healer_acquires_ground_medicine_for_patient(Seed([17; 32]));

    assert_eq!(
        first, second,
        "care medicine acquisition scenario should replay deterministically"
    );
}
