//! Golden tests for care-domain behavior.

mod golden_harness;

use golden_harness::*;
use worldwake_ai::{DecisionOutcome, GoalKind, PlannerOpKind};
use worldwake_core::{
    hash_event_log, hash_world, total_live_lot_quantity, BodyPart, CommodityKind, DeprivationKind,
    EntityId, HomeostaticNeeds, MetabolismProfile, PerceptionSource, Quantity, Seed, StateHash,
    Tick, UtilityProfile, Wound, WoundCause, WoundId, WoundList,
};
use worldwake_sim::{
    step_tick, ActionStartFailureReason, ActionTraceKind, AutonomousControllerRuntime,
    TickInputContext, TickInputError, TickInputProducer, TickStepServices,
};

struct ClearPatientWoundsAfterPlanning<'a> {
    inner: AutonomousControllerRuntime<'a>,
    patient: EntityId,
    cleared: bool,
}

impl TickInputProducer for ClearPatientWoundsAfterPlanning<'_> {
    fn produce_inputs(&mut self, ctx: TickInputContext<'_>) -> Result<(), TickInputError> {
        let TickInputContext {
            world,
            event_log,
            scheduler,
            rng,
            action_defs,
            action_handlers,
            recipe_registry,
            pending_replans,
            tick,
        } = ctx;
        self.inner.produce_inputs(TickInputContext {
            world,
            event_log,
            scheduler,
            rng,
            action_defs,
            action_handlers,
            recipe_registry,
            pending_replans,
            tick,
        })?;

        if !self.cleared {
            let mut txn = new_txn(world, tick.0);
            txn.set_component_wound_list(self.patient, WoundList::default())
                .unwrap();
            commit_txn(txn, event_log);
            self.cleared = true;
        }

        Ok(())
    }
}

fn seed_wounded_patient(h: &mut GoldenHarness) -> EntityId {
    seed_wounded_agent_at(h, "Patient", VILLAGE_SQUARE)
}

fn seed_wounded_agent_at(h: &mut GoldenHarness, name: &str, place: EntityId) -> EntityId {
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        name,
        place,
        HomeostaticNeeds::new(pm(100), pm(0), pm(100), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_wound_list(
        agent,
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

    agent
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

// ---------------------------------------------------------------------------
// Scenario 2c-self: Wounded agent self-treats with medicine
// ---------------------------------------------------------------------------

fn run_self_care_with_medicine(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    let agent = seed_wounded_agent_at(&mut h, "SelfHealer", VILLAGE_SQUARE);

    give_commodity(
        &mut h.world,
        &mut h.event_log,
        agent,
        VILLAGE_SQUARE,
        CommodityKind::Medicine,
        Quantity(1),
    );

    let initial_medicine = h.agent_commodity_qty(agent, CommodityKind::Medicine);
    let initial_wound_load = h.agent_wound_load(agent);
    let initial_total_medicine = total_live_lot_quantity(&h.world, CommodityKind::Medicine);

    let mut medicine_consumed = false;
    let mut wound_load_decreased = false;

    for _ in 0..80 {
        h.step_once();

        let agent_medicine = h.agent_commodity_qty(agent, CommodityKind::Medicine);
        let agent_wound_load = h.agent_wound_load(agent);
        let total_medicine = total_live_lot_quantity(&h.world, CommodityKind::Medicine);

        medicine_consumed |= agent_medicine < initial_medicine;
        wound_load_decreased |= agent_wound_load < initial_wound_load;

        assert!(
            total_medicine <= initial_total_medicine,
            "medicine lots should not increase: initial={initial_total_medicine}, now={total_medicine}"
        );
        assert!(!h.agent_is_dead(agent), "self-healer must stay alive");

        if medicine_consumed && wound_load_decreased {
            break;
        }
    }

    assert!(
        medicine_consumed,
        "wounded agent should consume own medicine for self-treatment"
    );
    assert!(
        wound_load_decreased,
        "wound load should decrease after self-treatment"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_self_care_with_medicine() {
    let _ = run_self_care_with_medicine(Seed([20; 32]));
}

#[test]
fn golden_self_care_with_medicine_replays_deterministically() {
    let first = run_self_care_with_medicine(Seed([21; 32]));
    let second = run_self_care_with_medicine(Seed([21; 32]));

    assert_eq!(
        first, second,
        "self-care scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 2c-self-acquire: Wounded agent acquires ground medicine, self-treats
// ---------------------------------------------------------------------------

fn run_self_care_acquires_ground_medicine(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);
    let agent = seed_wounded_agent_at(&mut h, "SelfHealer", VILLAGE_SQUARE);
    let _medicine =
        place_ground_commodity(&mut h, VILLAGE_SQUARE, CommodityKind::Medicine, Quantity(1));

    let initial_medicine = h.agent_commodity_qty(agent, CommodityKind::Medicine);
    let initial_wound_load = h.agent_wound_load(agent);
    let initial_total_medicine = total_live_lot_quantity(&h.world, CommodityKind::Medicine);

    let mut medicine_acquired = false;
    let mut medicine_consumed = false;
    let mut wound_load_decreased = false;

    for _ in 0..80 {
        h.step_once();

        let agent_medicine = h.agent_commodity_qty(agent, CommodityKind::Medicine);
        let agent_wound_load = h.agent_wound_load(agent);
        let total_medicine = total_live_lot_quantity(&h.world, CommodityKind::Medicine);

        medicine_acquired |= agent_medicine > initial_medicine;
        medicine_consumed |= medicine_acquired && agent_medicine == Quantity(0);
        wound_load_decreased |= agent_wound_load < initial_wound_load;

        assert!(
            total_medicine <= initial_total_medicine,
            "medicine lots should not increase: initial={initial_total_medicine}, now={total_medicine}"
        );
        assert!(!h.agent_is_dead(agent), "self-healer must stay alive");

        if medicine_acquired && medicine_consumed && wound_load_decreased {
            break;
        }
    }

    assert!(
        medicine_acquired,
        "wounded agent should pick up accessible ground medicine for self-treatment"
    );
    assert!(
        medicine_consumed,
        "wounded agent should consume acquired medicine for self-treatment"
    );
    assert!(
        wound_load_decreased,
        "wound load should decrease after self-treatment with acquired medicine"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_self_care_acquires_ground_medicine() {
    let _ = run_self_care_acquires_ground_medicine(Seed([22; 32]));
}

#[test]
fn golden_self_care_acquires_ground_medicine_replays_deterministically() {
    let first = run_self_care_acquires_ground_medicine(Seed([23; 32]));
    let second = run_self_care_acquires_ground_medicine(Seed([23; 32]));

    assert_eq!(
        first, second,
        "self-care ground medicine acquisition should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 2c-report: Indirect wound report does NOT trigger care goal
// ---------------------------------------------------------------------------

fn run_indirect_report_no_care(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);

    // Observer at Village Square — well-fed, carries medicine.
    let observer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Observer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(100), pm(0), pm(100), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        observer,
        VILLAGE_SQUARE,
        CommodityKind::Medicine,
        Quantity(1),
    );

    // Wounded patient at Orchard Farm — different place.
    let patient = seed_wounded_agent_at(&mut h, "RemotePatient", ORCHARD_FARM);

    // Seed observer's belief about patient via *Report* (not DirectObservation).
    // This should NOT trigger a TreatWounds goal per the direct-observation gate.
    let reporter = observer; // self-report for simplicity; source identity doesn't matter
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        observer,
        &[patient],
        Tick(0),
        PerceptionSource::Report {
            from: reporter,
            chain_len: 1,
        },
    );

    let initial_medicine = h.agent_commodity_qty(observer, CommodityKind::Medicine);

    // Run enough ticks for any care-driven travel+heal to complete if it were triggered.
    for _ in 0..60 {
        h.step_once();

        assert!(!h.agent_is_dead(observer), "observer must stay alive");
        // Patient may die from wounds — that's fine for this test.
    }

    let final_medicine = h.agent_commodity_qty(observer, CommodityKind::Medicine);
    assert_eq!(
        final_medicine, initial_medicine,
        "observer must NOT consume medicine based on a Report-sourced wound belief; \
         only DirectObservation should trigger care. initial={initial_medicine}, final={final_medicine}"
    );

    // Observer should remain at Village Square (no care-driven travel).
    let observer_place = h.world.effective_place(observer);
    assert_eq!(
        observer_place,
        Some(VILLAGE_SQUARE),
        "observer must stay at Village Square — indirect report should not trigger care travel"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_indirect_report_does_not_trigger_care() {
    let _ = run_indirect_report_no_care(Seed([24; 32]));
}

#[test]
fn golden_indirect_report_does_not_trigger_care_replays_deterministically() {
    let first = run_indirect_report_no_care(Seed([25; 32]));
    let second = run_indirect_report_no_care(Seed([25; 32]));

    assert_eq!(
        first, second,
        "indirect report no-care scenario should replay deterministically"
    );
}

// ---------------------------------------------------------------------------
// Scenario 2c-invalidation: Care goal invalidates when patient self-heals
// ---------------------------------------------------------------------------

fn run_care_goal_invalidation(seed: Seed) -> (StateHash, StateHash) {
    let mut h = GoldenHarness::new(seed);

    // Patient at Village Square — wounded, carries medicine, will self-treat.
    let patient = seed_wounded_agent_at(&mut h, "Patient", VILLAGE_SQUARE);
    give_commodity(
        &mut h.world,
        &mut h.event_log,
        patient,
        VILLAGE_SQUARE,
        CommodityKind::Medicine,
        Quantity(1),
    );

    // Healer at Village Square — no medicine, observes patient's wounds.
    // Cannot treat (no medicine), so their TreatWounds{patient} goal must be
    // satisfied by the patient's own self-healing.
    let healer = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Healer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(100), pm(0), pm(100), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );

    let initial_patient_wound_load = h.agent_wound_load(patient);
    assert!(
        initial_patient_wound_load > 0,
        "patient should start wounded"
    );

    let mut patient_wound_cleared = false;

    for _ in 0..80 {
        h.step_once();

        let patient_wound_load = h.agent_wound_load(patient);
        let healer_medicine = h.agent_commodity_qty(healer, CommodityKind::Medicine);

        // Healer must never acquire medicine (there is none to acquire).
        assert_eq!(
            healer_medicine,
            Quantity(0),
            "healer should never have medicine — patient must self-treat"
        );

        assert!(!h.agent_is_dead(patient), "patient must stay alive");
        assert!(!h.agent_is_dead(healer), "healer must stay alive");

        if patient_wound_load == 0 {
            patient_wound_cleared = true;
            break;
        }
    }

    assert!(
        patient_wound_cleared,
        "patient should self-treat and clear all wounds via own medicine"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_care_goal_invalidation_when_patient_heals() {
    let _ = run_care_goal_invalidation(Seed([26; 32]));
}

#[test]
fn golden_care_goal_invalidation_when_patient_heals_replays_deterministically() {
    let first = run_care_goal_invalidation(Seed([27; 32]));
    let second = run_care_goal_invalidation(Seed([27; 32]));

    assert_eq!(
        first, second,
        "care goal invalidation scenario should replay deterministically"
    );
}

#[allow(clippy::too_many_lines)]
fn run_care_pre_start_wound_disappearance_records_blocker(seed: Seed) -> (StateHash, StateHash) {
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
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        healer,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    h.driver.enable_tracing();
    h.enable_action_tracing();

    {
        let controllers = AutonomousControllerRuntime::new(vec![&mut h.driver]);
        let mut producer = ClearPatientWoundsAfterPlanning {
            inner: controllers,
            patient,
            cleared: false,
        };

        let result = step_tick(
            &mut h.world,
            &mut h.event_log,
            &mut h.scheduler,
            &mut h.controller,
            &mut h.rng,
            TickStepServices {
                action_defs: &h.defs,
                action_handlers: &h.handlers,
                recipe_registry: &h.recipes,
                systems: &worldwake_systems::dispatch_table(),
                input_producer: Some(&mut producer),
                action_trace: h.action_trace.as_mut(),
                request_resolution_trace: None,
                politics_trace: h.politics_trace.as_mut(),
            },
        )
        .unwrap();

        assert_eq!(result.tick, Tick(0));
    }

    let care_goal = GoalKind::TreatWounds { patient };
    let care_goal_key = worldwake_core::GoalKey::from(care_goal);
    let trace_tick_0 = h
        .driver
        .trace_sink()
        .expect("tracing should be enabled")
        .trace_at(healer, Tick(0))
        .expect("healer should have a tick 0 trace");
    let planning_tick_0 = match &trace_tick_0.outcome {
        DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected Planning outcome at tick 0, got {other:?}"),
    };

    assert!(
        planning_tick_0
            .candidates
            .generated
            .contains(&care_goal_key),
        "healer should generate TreatWounds before the authoritative start race"
    );
    let selected_plan = planning_tick_0
        .selection
        .selected_plan
        .as_ref()
        .expect("healer should select a care plan before the start failure");
    assert_eq!(
        planning_tick_0
            .selection
            .selected
            .expect("tick 0 should have a selected goal")
            .kind,
        care_goal
    );
    assert_eq!(
        selected_plan
            .next_step
            .as_ref()
            .expect("selected plan should expose its next step")
            .op_kind,
        PlannerOpKind::Heal
    );

    let tick_0_events = h
        .action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for_at(healer, Tick(0));
    assert_eq!(tick_0_events.len(), 1);
    assert!(matches!(
        tick_0_events[0].kind,
        ActionTraceKind::StartFailed { .. }
    ));

    assert_eq!(h.scheduler.action_start_failures().len(), 1);
    assert_eq!(
        h.scheduler.action_start_failures()[0].reason,
        ActionStartFailureReason::PreconditionFailed("TargetHasWounds(0)".to_string())
    );

    h.step_once();

    let trace_tick_1 = h
        .driver
        .trace_sink()
        .expect("tracing should stay enabled")
        .trace_at(healer, Tick(1))
        .expect("healer should have a tick 1 trace");
    let planning_tick_1 = match &trace_tick_1.outcome {
        DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected Planning outcome at tick 1, got {other:?}"),
    };
    assert_eq!(planning_tick_1.action_start_failures.len(), 1);
    assert_eq!(
        planning_tick_1.action_start_failures[0].reason,
        ActionStartFailureReason::PreconditionFailed("TargetHasWounds(0)".to_string())
    );

    let blocked = h
        .world
        .get_component_blocked_intent_memory(healer)
        .expect("healer should carry blocked intent memory after start failure");
    assert_eq!(blocked.intents.len(), 1);
    assert_eq!(blocked.intents[0].goal_key.kind, care_goal);
    assert_eq!(blocked.intents[0].related_entity, Some(patient));
    assert!(
        h.scheduler.action_start_failures().is_empty(),
        "tick 1 reconciliation should drain the structured start failure"
    );
    assert!(
        h.agent_active_action_name(healer) != Some("heal"),
        "failed care start should not leave the rejected heal step active"
    );

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn golden_care_pre_start_wound_disappearance_records_blocker() {
    let _ = run_care_pre_start_wound_disappearance_records_blocker(Seed([28; 32]));
}

#[test]
fn golden_care_pre_start_wound_disappearance_records_blocker_replays_deterministically() {
    let first = run_care_pre_start_wound_disappearance_records_blocker(Seed([29; 32]));
    let second = run_care_pre_start_wound_disappearance_records_blocker(Seed([29; 32]));

    assert_eq!(
        first, second,
        "care start-abort regression should replay deterministically"
    );
}
