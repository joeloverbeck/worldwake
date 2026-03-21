//! Golden tests for care-domain behavior.

mod golden_harness;

use golden_harness::*;
use worldwake_ai::{
    apply_hypothetical_transition, build_planning_snapshot, build_semantics_table,
    generate_candidates, search_plan, DecisionOutcome, GoalKind, PlanningBudget,
    PlanningEntityRef, PlanningState, PlannerOpKind,
};
use worldwake_core::{
    hash_event_log, hash_world, total_live_lot_quantity, BlockedIntent, BlockedIntentMemory,
    BlockingFact, BodyPart, CommodityKind, DeprivationKind, EntityId, GoalKey, HomeostaticNeeds,
    MetabolismProfile, PerceptionSource, Quantity, Seed, StateHash, Tick, UtilityProfile, Wound,
    WoundCause, WoundId, WoundList,
};
use worldwake_sim::{
    get_affordances, step_tick, ActionStartFailureReason, ActionTraceKind,
    AutonomousControllerRuntime, PerAgentBeliefView, TickInputContext, TickInputError,
    TickInputProducer, TickStepServices,
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
        UtilityProfile {
            social_weight: pm(0),
            care_weight: pm(800),
            ..UtilityProfile::default()
        },
    );
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.clear_component_tell_profile(healer).unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    assert_eq!(
        h.world.get_component_tell_profile(healer),
        None,
        "scenario should explicitly remove the healer tell profile to isolate care planning"
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

fn setup_remote_ground_medicine_care_scenario(
    seed: Seed,
) -> (GoldenHarness, EntityId, EntityId, EntityId) {
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
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_combat_profile(patient, no_recovery_combat_profile())
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }
    let medicine =
        place_ground_commodity(&mut h, ORCHARD_FARM, CommodityKind::Medicine, Quantity(1));

    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        healer,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    let _ = seed_belief_from_world(
        &mut h.world,
        &mut h.event_log,
        healer,
        medicine,
        Tick(0),
        PerceptionSource::Report {
            from: patient,
            chain_len: 1,
        },
    );
    {
        let blocked = BlockedIntentMemory {
            intents: vec![
                BlockedIntent {
                    goal_key: GoalKey::from(GoalKind::ShareBelief {
                        listener: patient,
                        subject: patient,
                    }),
                    blocking_fact: BlockingFact::Unknown,
                    related_entity: Some(patient),
                    related_place: Some(VILLAGE_SQUARE),
                    related_action: None,
                    observed_tick: Tick(0),
                    expires_tick: Tick(200),
                },
                BlockedIntent {
                    goal_key: GoalKey::from(GoalKind::ShareBelief {
                        listener: patient,
                        subject: medicine,
                    }),
                    blocking_fact: BlockingFact::Unknown,
                    related_entity: Some(patient),
                    related_place: Some(VILLAGE_SQUARE),
                    related_action: None,
                    observed_tick: Tick(0),
                    expires_tick: Tick(200),
                },
            ],
        };
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_blocked_intent_memory(healer, blocked)
            .unwrap();
        commit_txn(txn, &mut h.event_log);
    }

    (h, healer, patient, medicine)
}

fn assert_remote_care_tick_zero_plan(
    h: &GoldenHarness,
    healer: EntityId,
    patient: EntityId,
) -> worldwake_ai::AgentDecisionTrace {
    let tick_0_trace = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled for remote care")
        .trace_at(healer, Tick(0))
        .expect("healer should have a tick 0 trace")
        .clone();
    let tick_0_planning = match &tick_0_trace.outcome {
        DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected planning trace for remote care tick 0, got {other:?}"),
    };
    let tick_0_selected_plan = tick_0_planning
        .selection
        .selected_plan
        .as_ref()
        .expect("remote care should select a plan on tick 0");
    let tick_0_next_step = tick_0_selected_plan
        .next_step
        .as_ref()
        .expect("remote care selected plan should expose its next step");

    assert_eq!(
        tick_0_planning.selection.selected,
        Some(GoalKey::from(GoalKind::TreatWounds { patient })),
        "remote care should select TreatWounds on tick 0"
    );
    assert_eq!(
        tick_0_next_step.op_kind,
        PlannerOpKind::Travel,
        "remote care should start by traveling toward the remote medicine location"
    );
    assert_eq!(
        tick_0_selected_plan
            .steps
            .iter()
            .filter(|step| step.op_kind == PlannerOpKind::Travel)
            .map(|step| step.targets.clone())
            .find(|targets| *targets == vec![ORCHARD_FARM]),
        Some(vec![ORCHARD_FARM]),
        "remote care should select a travel path that includes Orchard Farm for the reported medicine lot"
    );
    assert!(
        tick_0_selected_plan
            .steps
            .iter()
            .any(|step| step.op_kind == PlannerOpKind::MoveCargo),
        "remote care selected plan should include a cargo move step for medicine pickup"
    );
    assert!(
        tick_0_selected_plan
            .steps
            .iter()
            .any(|step| step.op_kind == PlannerOpKind::Heal),
        "remote care selected plan should include a heal step after medicine pickup"
    );

    tick_0_trace
}

fn assert_remote_care_action_sequence(healer_events: &[&worldwake_sim::ActionTraceEvent]) {
    let pick_up_commit = healer_events
        .iter()
        .find_map(|event| {
            (event.action_name == "pick_up"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .expect("remote medicine care should have a committed pick_up event");
    let heal_commit = healer_events
        .iter()
        .find_map(|event| {
            (event.action_name == "heal"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .expect("remote medicine care should have a committed heal event");

    assert!(
        pick_up_commit < heal_commit,
        "remote medicine care should commit pick_up before heal; events={healer_events:?}"
    );
}

fn run_healer_acquires_remote_ground_medicine_for_patient(seed: Seed) -> (StateHash, StateHash) {
    let (mut h, healer, patient, _medicine) = setup_remote_ground_medicine_care_scenario(seed);

    h.driver.enable_tracing();
    h.enable_action_tracing();

    let initial_wound_load = h.agent_wound_load(patient);
    let initial_total_medicine = total_live_lot_quantity(&h.world, CommodityKind::Medicine);

    let mut visited_orchard = false;
    let mut wound_load_decreased = false;
    let mut heal_committed = false;

    for _ in 0..120 {
        h.step_once();

        visited_orchard |= h.world.effective_place(healer) == Some(ORCHARD_FARM);
        wound_load_decreased |= h.agent_wound_load(patient) < initial_wound_load;
        heal_committed |= h
            .action_trace_sink()
            .expect("action tracing should be enabled for remote care")
            .events_for(healer)
            .iter()
            .any(|event| {
                event.action_name == "heal"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            });

        assert!(
            total_live_lot_quantity(&h.world, CommodityKind::Medicine) <= initial_total_medicine,
            "medicine lots should not increase during remote care procurement"
        );

        if heal_committed {
            break;
        }
    }

    let healer_events = h
        .action_trace_sink()
        .expect("action tracing should be enabled for remote care")
        .events_for(healer)
        .clone();
    let tick_0_trace = assert_remote_care_tick_zero_plan(&h, healer, patient);
    assert!(
        visited_orchard,
        "healer should travel to Orchard Farm before remote treatment succeeds; trace={tick_0_trace:?}; events={healer_events:?}"
    );
    assert!(
        wound_load_decreased,
        "patient wound load should decrease after the healer procures remote medicine"
    );
    assert!(
        heal_committed,
        "remote medicine care should reach a committed heal step"
    );
    assert_eq!(
        h.agent_commodity_qty(healer, CommodityKind::Medicine),
        Quantity(0),
        "healer should not retain medicine after the successful heal"
    );

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled for remote care");
    let healer_events = action_sink.events_for(healer);
    let travel_count = healer_events
        .iter()
        .filter(|event| event.action_name == "travel")
        .count();
    assert!(
        travel_count >= 2,
        "remote medicine care should require outbound and return travel; saw events: {healer_events:?}"
    );
    assert!(
        healer_events.iter().any(
            |event| event.action_name == "pick_up"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        ),
        "remote medicine care should commit a pick_up step"
    );
    assert!(
        healer_events.iter().any(
            |event| event.action_name == "heal"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
        ),
        "remote medicine care should commit a heal step; saw events: {healer_events:?}"
    );
    assert_remote_care_action_sequence(&healer_events);

    (
        hash_world(&h.world).unwrap(),
        hash_event_log(&h.event_log).unwrap(),
    )
}

#[test]
fn remote_treat_wounds_snapshot_supports_pick_up_transition_at_orchard() {
    let (h, healer, patient, medicine) =
        setup_remote_ground_medicine_care_scenario(Seed([42; 32]));
    let view = PerAgentBeliefView::from_world(healer, &h.world);
    let grounded = generate_candidates(
        &view,
        healer,
        &BlockedIntentMemory::default(),
        &h.recipes,
        Tick(0),
    )
    .into_iter()
    .find(|candidate| candidate.key == GoalKey::from(GoalKind::TreatWounds { patient }))
    .expect("scenario should generate TreatWounds");
    let snapshot = build_planning_snapshot(
        &view,
        healer,
        &grounded.evidence_entities,
        &grounded.evidence_places,
        7,
    );
    let state = PlanningState::new(&snapshot).move_actor_to(ORCHARD_FARM);
    let affordances = get_affordances(&state, healer, &h.defs, &h.handlers);
    let pick_up = affordances
        .iter()
        .find(|affordance| {
            h.defs.get(affordance.def_id).is_some_and(|def| {
                def.name == "pick_up" && affordance.bound_targets == vec![medicine]
            })
        })
        .expect("remote care planning state should expose pick_up for the reported medicine lot");
    let semantics = build_semantics_table(&h.defs);
    let transition = apply_hypothetical_transition(
        &grounded,
        semantics
            .get(&pick_up.def_id)
            .expect("pick_up should be classified by planner semantics"),
        state,
        &[PlanningEntityRef::Authoritative(medicine)],
        pick_up.payload_override.as_ref(),
    );

    assert!(
        transition.is_some(),
        "pick_up transition should be hypothetically applicable for the reported medicine lot"
    );
}

#[test]
fn remote_treat_wounds_search_needs_eight_step_depth_budget_in_prototype_topology() {
    let (h, healer, patient, _medicine) = setup_remote_ground_medicine_care_scenario(Seed([43; 32]));
    let view = PerAgentBeliefView::from_world(healer, &h.world);
    let grounded = generate_candidates(
        &view,
        healer,
        &BlockedIntentMemory::default(),
        &h.recipes,
        Tick(0),
    )
    .into_iter()
    .find(|candidate| candidate.key == GoalKey::from(GoalKind::TreatWounds { patient }))
    .expect("scenario should generate TreatWounds");
    let snapshot = build_planning_snapshot(
        &view,
        healer,
        &grounded.evidence_entities,
        &grounded.evidence_places,
        7,
    );
    let semantics = build_semantics_table(&h.defs);

    let shallow = search_plan(
        &snapshot,
        &grounded,
        &semantics,
        &h.defs,
        &h.handlers,
        &PlanningBudget {
            max_plan_depth: 6,
            ..PlanningBudget::default()
        },
        &[VILLAGE_SQUARE],
        None,
        None,
    );
    let deep = search_plan(
        &snapshot,
        &grounded,
        &semantics,
        &h.defs,
        &h.handlers,
        &PlanningBudget::default(),
        &[VILLAGE_SQUARE],
        None,
        None,
    );

    assert!(
        matches!(
            shallow,
            worldwake_ai::PlanSearchResult::FrontierExhausted { .. }
        ),
        "the previous six-step depth budget should fail specifically by frontier exhaustion under the capped depth budget"
    );
    let deep_plan = match deep {
        worldwake_ai::PlanSearchResult::Found(plan) => plan,
        other => panic!(
            "the default depth budget should admit the lawful remote-care route through Orchard Farm, got {other:?}"
        ),
    };
    assert_eq!(
        deep_plan
            .steps
            .first()
            .expect("remote-care search result should contain a first step")
            .op_kind,
        PlannerOpKind::Travel,
        "the lawful remote-care route should begin with travel"
    );
    assert_eq!(
        deep_plan
            .steps
            .iter()
            .filter(|step| step.op_kind == PlannerOpKind::Travel)
            .map(|step| step.targets.clone())
            .find(|targets| *targets == vec![PlanningEntityRef::Authoritative(ORCHARD_FARM)]),
        Some(vec![PlanningEntityRef::Authoritative(ORCHARD_FARM)]),
        "the lawful remote-care route should include a travel leg to Orchard Farm"
    );
    assert!(
        deep_plan
            .steps
            .iter()
            .any(|step| step.op_kind == PlannerOpKind::MoveCargo),
        "the lawful remote-care route should include medicine pickup"
    );
    assert!(
        deep_plan.steps.iter().any(|step| step.op_kind == PlannerOpKind::Heal),
        "the lawful remote-care route should include healing after pickup"
    );
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

#[test]
fn golden_healer_acquires_remote_ground_medicine_for_patient() {
    let _ = run_healer_acquires_remote_ground_medicine_for_patient(Seed([18; 32]));
}

#[test]
fn golden_healer_acquires_remote_ground_medicine_for_patient_replays_deterministically() {
    let first = run_healer_acquires_remote_ground_medicine_for_patient(Seed([19; 32]));
    let second = run_healer_acquires_remote_ground_medicine_for_patient(Seed([19; 32]));

    assert_eq!(
        first, second,
        "remote care medicine acquisition scenario should replay deterministically"
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
