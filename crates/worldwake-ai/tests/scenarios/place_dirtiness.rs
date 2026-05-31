//! Golden coverage for S129 place dirtiness and facility wear.

use crate::golden_harness::*;
use std::num::{NonZeroU8, NonZeroU32};
use worldwake_ai::{DecisionOutcome, GoalKey, GoalKind, PlanSearchOutcome};
use worldwake_core::{
    AgentData, BelievedContentionState, CommodityKind, ControlSource, DecisionEventPayload,
    DeprivationExposure, EntityId, EventTag, EventView, HomeostaticNeedId, HomeostaticNeeds,
    LatrineFullness, MetabolismProfile, OpportunityAnchor, OpportunityKey, PerceptionSource,
    PlaceDirtiness, Quantity, ResourceSource, RestCapacity, Seed, SleepQualityProfile, Tick,
    UtilityProfile, WashBasinState, WorkstationTag, build_believed_entity_state,
};
use worldwake_sim::{ActionRequestMode, ActionTraceKind, InputKind, RequestProvenance};

fn set_control_source(h: &mut GoldenHarness, agent: EntityId, control_source: ControlSource) {
    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_component_agent_data(agent, AgentData { control_source })
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn set_needs(h: &mut GoldenHarness, agent: EntityId, needs: HomeostaticNeeds) {
    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_component_homeostatic_needs(agent, needs).unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn quiet_metabolism() -> MetabolismProfile {
    MetabolismProfile {
        hunger_rate: pm(0),
        thirst_rate: pm(0),
        fatigue_rate: pm(0),
        bladder_rate: pm(0),
        dirtiness_rate: pm(0),
        toilet_ticks: nz(1),
        wash_ticks: nz(1),
        min_sleep_ticks: nz(4),
        ..MetabolismProfile::default()
    }
}

fn hygiene_utility() -> UtilityProfile {
    UtilityProfile {
        hunger_weight: pm(50),
        thirst_weight: pm(50),
        fatigue_weight: pm(1000),
        bladder_weight: pm(1000),
        dirtiness_weight: pm(1000),
        ..UtilityProfile::default()
    }
}

fn request_action(h: &mut GoldenHarness, actor: EntityId, def_name: &str, targets: Vec<EntityId>) {
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
            payload_override: None,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
}

fn run_until_commits(
    h: &mut GoldenHarness,
    actor: EntityId,
    action_name: &str,
    target_count: usize,
    tick_budget: u32,
) {
    for _ in 0..tick_budget {
        h.step_once();
        let committed = h
            .action_trace_sink()
            .expect("action tracing should be enabled")
            .events_for(actor)
            .into_iter()
            .filter(|event| {
                event.action_name == action_name
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            })
            .count();
        if committed >= target_count {
            return;
        }
    }
    panic!("{action_name} should commit {target_count} times within {tick_budget} ticks");
}

fn place_wash_basin(h: &mut GoldenHarness, place: EntityId, state: WashBasinState) -> EntityId {
    let basin = place_workstation(
        &mut h.world,
        &mut h.event_log,
        place,
        WorkstationTag::WashBasin,
        ProductionOutputOwner::Actor,
    );
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_wash_basin_state(basin, state).unwrap();
    commit_txn(txn, &mut h.event_log);
    basin
}

fn set_place_dirtiness(h: &mut GoldenHarness, place: EntityId, dirtiness: PlaceDirtiness) {
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_place_dirtiness(place, dirtiness).unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn waste_created_payloads(h: &GoldenHarness) -> Vec<worldwake_core::WasteCreatedPayload> {
    h.event_log
        .events_by_tag(EventTag::WasteCreated)
        .iter()
        .filter_map(|id| h.event_log.get(*id))
        .filter_map(|record| match record.decision_payload()? {
            DecisionEventPayload::WasteCreated(payload) => Some(payload.clone()),
            _ => None,
        })
        .collect()
}

fn wash_payloads(h: &GoldenHarness) -> Vec<worldwake_core::WashFacilityUsedPayload> {
    h.event_log
        .events_by_tag(EventTag::WashFacilityUsed)
        .iter()
        .filter_map(|id| h.event_log.get(*id))
        .filter_map(|record| match record.decision_payload()? {
            DecisionEventPayload::WashFacilityUsed(payload) => Some(payload.clone()),
            _ => None,
        })
        .collect()
}

fn assert_waste_lot(h: &GoldenHarness, lot: EntityId) {
    assert_eq!(
        h.world
            .get_component_item_lot(lot)
            .map(|item| (item.commodity, item.quantity)),
        Some((CommodityKind::Waste, Quantity(1))),
        "WasteCreated payload should reference a concrete Waste lot"
    );
}

fn planning_trace_at(
    h: &GoldenHarness,
    agent: EntityId,
    tick: Tick,
) -> &worldwake_ai::PlanningPipelineTrace {
    let trace = h
        .driver
        .trace_sink()
        .and_then(|sink| sink.trace_at(agent, tick))
        .expect("decision trace should exist");
    let DecisionOutcome::Planning(planning) = &trace.outcome else {
        panic!("agent should run planning at {tick:?}");
    };
    planning
}

// Scenario 362: Place Dirtiness Accumulates From Wilderness Relief
// Setup: Three human-controlled agents at an outdoor farm each relieve twice.
// Proves: Wilderness relief creates WasteCreated payloads backed by Waste lots and monotonically raises PlaceDirtiness despite decay.
// Chain: relieve_wilderness -> Waste lot + WasteCreated -> PlaceDirtiness -> event-log conservation proof.
#[test]
fn place_dirtiness_accumulates_from_repeated_wilderness_relief() {
    let mut h = GoldenHarness::new(Seed([0x87; 32]));
    h.enable_action_tracing();
    let agents = ["Aster", "Bryn", "Cala"].map(|name| {
        let agent = seed_agent(
            &mut h.world,
            &mut h.event_log,
            name,
            ORCHARD_FARM,
            HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(900), pm(0)),
            quiet_metabolism(),
            hygiene_utility(),
        );
        set_control_source(&mut h, agent, ControlSource::Human);
        agent
    });

    let mut observed_values = Vec::new();
    for _ in 0..2 {
        for agent in agents {
            set_needs(
                &mut h,
                agent,
                HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(900), pm(0)),
            );
            request_action(&mut h, agent, "relieve_wilderness", vec![]);
            run_until_commits(&mut h, agent, "relieve_wilderness", 1, 8);
            observed_values.push(
                h.world
                    .get_component_place_dirtiness(ORCHARD_FARM)
                    .unwrap()
                    .value,
            );
        }
    }

    assert_eq!(waste_created_payloads(&h).len(), 6);
    for payload in waste_created_payloads(&h) {
        assert_eq!(payload.place, ORCHARD_FARM);
        assert_eq!(
            payload.source,
            worldwake_core::WasteSource::WildernessRelief
        );
        assert_eq!(payload.place_dirtiness_delta, pm(80));
        assert_waste_lot(&h, payload.waste_lot);
    }
    assert!(
        observed_values.windows(2).all(|pair| pair[1] >= pair[0]),
        "PlaceDirtiness should not decrease during the relief phase: {observed_values:?}"
    );
}

// Scenario 363: Sleep Ranking Prefers Clean Place
// Setup: A fatigued AI agent knows an equally comfortable dirty current camp and clean reachable farm.
// Proves: PlaceDirtiness is read through the sleep ranking path and the cleaner place is selected.
// Chain: PlaceDirtiness belief -> per-place Sleep candidate -> ranking multiplier -> selected OpportunityKey.
#[test]
fn sleep_ranking_prefers_clean_place_over_dirty_place() {
    let mut h = GoldenHarness::new(Seed([0x88; 32]));
    h.driver.enable_tracing();
    set_place_dirtiness(
        &mut h,
        VILLAGE_SQUARE,
        PlaceDirtiness {
            value: pm(800),
            ..PlaceDirtiness::default()
        },
    );
    set_place_dirtiness(
        &mut h,
        ORCHARD_FARM,
        PlaceDirtiness {
            value: pm(50),
            ..PlaceDirtiness::default()
        },
    );
    let mut txn = new_txn(&mut h.world, 0);
    for place in [VILLAGE_SQUARE, ORCHARD_FARM] {
        txn.set_component_sleep_quality_profile(place, SleepQualityProfile::default())
            .unwrap();
        txn.set_component_rest_capacity(place, RestCapacity(NonZeroU32::new(1).unwrap()))
            .unwrap();
    }
    commit_txn(txn, &mut h.event_log);
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "TiredChooser",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(900), pm(0), pm(0)),
        quiet_metabolism(),
        hygiene_utility(),
    );
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        PerceptionSource::Inference,
    );
    seed_empty_rest_contention_belief(&mut h, agent, ORCHARD_FARM);

    h.step_once();
    let planning = planning_trace_at(&h, agent, Tick(0));
    let sleep_goal = GoalKey::from(GoalKind::Sleep);
    let dirty = OpportunityKey {
        goal_key: sleep_goal,
        anchor: OpportunityAnchor::Place(VILLAGE_SQUARE),
    };
    let clean = OpportunityKey {
        goal_key: sleep_goal,
        anchor: OpportunityAnchor::Place(ORCHARD_FARM),
    };
    assert!(planning.candidates.generated_contains_opportunity(dirty));
    assert!(planning.candidates.generated_contains_opportunity(clean));
    let ranked = planning.candidates.ranked_summaries_for_goal(sleep_goal);
    assert_eq!(ranked[0].opportunity, clean);
    assert_eq!(planning.selection.selected_opportunity, Some(clean));
}

fn seed_empty_rest_contention_belief(h: &mut GoldenHarness, actor: EntityId, place: EntityId) {
    let mut store = h
        .world
        .get_component_agent_belief_store(actor)
        .cloned()
        .expect("seeded actor should have a belief store");
    let mut state =
        build_believed_entity_state(&h.world, place, Tick(0), PerceptionSource::Inference)
            .expect("rest place should be observable for belief seeding");
    state.believed_contention = Some(BelievedContentionState {
        grant_holder: None,
        queue_length: 0,
        observed_tick: Tick(0),
    });
    store.update_entity(place, state);
    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_component_agent_belief_store(actor, store)
        .expect("golden harness should keep belief stores writable");
    commit_txn(txn, &mut h.event_log);
}

// Scenario 364: Wash Partial Success Uses Basin State
// Setup: A dirty human-controlled agent washes at a one-unit basin requiring two units for full success.
// Proves: WashFacilityUsed records partial success and authoritative needs/basin state change proportionally.
// Chain: WashBasinState -> TargetHasWashBasinClean -> wash commit -> WashFacilityUsed + HomeostaticNeeds.
#[test]
fn wash_partial_success_proportional_dirtiness_reduction() {
    let mut h = GoldenHarness::new(Seed([0x89; 32]));
    h.enable_action_tracing();
    let basin = place_wash_basin(
        &mut h,
        VILLAGE_SQUARE,
        WashBasinState {
            clean_water_units: 1,
            max_clean_water: 10,
            refill_per_tick: 0,
            units_per_full_wash: 2,
            dirtiness_level: pm(0),
            dirtiness_per_use: pm(50),
            max_effective_dirtiness: pm(1000),
        },
    );
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Washer",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(1000)),
        quiet_metabolism(),
        hygiene_utility(),
    );
    set_control_source(&mut h, agent, ControlSource::Human);
    request_action(&mut h, agent, "wash", vec![basin]);
    run_until_commits(&mut h, agent, "wash", 1, 8);

    let payload = wash_payloads(&h).pop().expect("wash should emit payload");
    assert_eq!(payload.basin, basin);
    assert_eq!(payload.water_consumed, 1);
    assert_eq!(payload.agent_dirtiness_delta, pm(500));
    assert_eq!(payload.basin_dirtiness_delta, pm(25));
    assert!(payload.partial);
    assert_eq!(h.agent_dirtiness(agent), pm(500));
    assert_eq!(
        h.world.get_component_wash_basin_state(basin).unwrap(),
        &WashBasinState {
            clean_water_units: 0,
            max_clean_water: 10,
            refill_per_tick: 0,
            units_per_full_wash: 2,
            dirtiness_level: pm(25),
            dirtiness_per_use: pm(50),
            max_effective_dirtiness: pm(1000),
        }
    );
}

// Scenario 365: Latrine Overflow Creates Waste And Place Dirtiness
// Setup: A human-controlled agent uses an already-critical latrine.
// Proves: Overcapacity toilet use emits WasteCreated and increments PlaceDirtiness without reducing LatrineFullness.
// Chain: LatrineFullness -> toilet commit -> WasteCreated(OvercapacityLatrine) -> PlaceDirtiness.
#[test]
fn latrine_overflow_creates_waste_at_place_and_increments_place_dirtiness() {
    let mut h = GoldenHarness::new(Seed([0x8a; 32]));
    h.enable_action_tracing();
    let mut txn = new_txn(&mut h.world, 0);
    // S176 D3: the latrine starts just below its critical threshold so the
    // single toilet use is lawful and crosses the threshold, exercising the
    // retained overflow path (a latrine already at/above the threshold is now
    // blocked, not relieved).
    txn.set_component_latrine_fullness(
        PUBLIC_LATRINE,
        LatrineFullness {
            fill: pm(750),
            fill_per_use: pm(80),
            critical_threshold: pm(800),
        },
    )
    .unwrap();
    txn.set_component_place_dirtiness(
        PUBLIC_LATRINE,
        PlaceDirtiness {
            value: pm(100),
            decay_per_tick: pm(0),
            dirtiness_per_use: pm(80),
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "LatrineUser",
        PUBLIC_LATRINE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(900), pm(0)),
        quiet_metabolism(),
        hygiene_utility(),
    );
    set_control_source(&mut h, agent, ControlSource::Human);
    request_action(&mut h, agent, "toilet", vec![PUBLIC_LATRINE]);
    run_until_commits(&mut h, agent, "toilet", 1, 8);

    let payload = waste_created_payloads(&h)
        .pop()
        .expect("overflow should emit WasteCreated");
    assert_eq!(
        payload.source,
        worldwake_core::WasteSource::OvercapacityLatrine
    );
    assert_eq!(payload.place, PUBLIC_LATRINE);
    assert_eq!(payload.place_dirtiness_delta, pm(80));
    assert_waste_lot(&h, payload.waste_lot);
    assert_eq!(
        h.world
            .get_component_latrine_fullness(PUBLIC_LATRINE)
            .unwrap()
            .fill,
        pm(830)
    );
    assert_eq!(
        h.world
            .get_component_place_dirtiness(PUBLIC_LATRINE)
            .unwrap()
            .value,
        pm(180)
    );
}

// Scenario 366: Basin Natural Refill Draws From Co-Located Water
// Setup: An empty basin shares a place with a finite water ResourceSource and no agent actions.
// Proves: The maintenance pass refills only up to max_clean_water and consumes the source quantity.
// Chain: item_decay_system -> ResourceSource transfer -> WashBasinState refill.
#[test]
fn basin_natural_refill_from_colocated_water_source() {
    let mut h = GoldenHarness::new(Seed([0x8b; 32]));
    let basin = place_wash_basin(
        &mut h,
        VILLAGE_SQUARE,
        WashBasinState {
            clean_water_units: 0,
            max_clean_water: 5,
            refill_per_tick: 1,
            units_per_full_wash: 2,
            dirtiness_level: pm(0),
            dirtiness_per_use: pm(50),
            max_effective_dirtiness: pm(1000),
        },
    );
    let source = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        VILLAGE_SQUARE,
        WorkstationTag::Well,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(100),
            max_quantity: Quantity(100),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: NonZeroU32::new(1).unwrap(),
            quality: None,
        },
        ProductionOutputOwner::Actor,
    );

    for _ in 0..6 {
        h.step_once();
    }

    assert_eq!(
        h.world
            .get_component_wash_basin_state(basin)
            .unwrap()
            .clean_water_units,
        5
    );
    assert_eq!(
        h.world
            .get_component_resource_source(source)
            .unwrap()
            .available_quantity,
        Quantity(95)
    );
}

// Scenario 367: AI Selects Non-Empty Basin After Empty Basin Is Unusable
// Setup: A dirty AI agent knows two local basins; one is empty and the other has water.
// Proves: Candidate generation/ranking exposes the clean basin and does not select the empty basin.
// Chain: WashBasinState -> wash candidate emission -> ranking modifier -> selected basin anchor.
#[test]
fn wash_ai_selects_non_empty_basin_when_other_basin_is_empty() {
    let mut h = GoldenHarness::new(Seed([0x8c; 32]));
    h.driver.enable_tracing();
    h.enable_action_tracing();
    let empty = place_wash_basin(
        &mut h,
        VILLAGE_SQUARE,
        WashBasinState {
            clean_water_units: 0,
            max_clean_water: 10,
            refill_per_tick: 0,
            units_per_full_wash: 2,
            dirtiness_level: pm(0),
            dirtiness_per_use: pm(50),
            max_effective_dirtiness: pm(1000),
        },
    );
    let usable = place_wash_basin(
        &mut h,
        VILLAGE_SQUARE,
        WashBasinState {
            clean_water_units: 1,
            max_clean_water: 10,
            refill_per_tick: 0,
            units_per_full_wash: 2,
            dirtiness_level: pm(0),
            dirtiness_per_use: pm(50),
            max_effective_dirtiness: pm(1000),
        },
    );
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Replanner",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(900)),
        quiet_metabolism(),
        hygiene_utility(),
    );
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    request_action(&mut h, agent, "wash", vec![empty]);
    h.step_once();
    let planning = planning_trace_at(&h, agent, Tick(0));
    let wash_goal = GoalKey::from(GoalKind::Wash);
    let empty_key = OpportunityKey {
        goal_key: wash_goal,
        anchor: OpportunityAnchor::Entity(empty),
    };
    let usable_key = OpportunityKey {
        goal_key: wash_goal,
        anchor: OpportunityAnchor::Entity(usable),
    };
    assert!(
        !planning
            .candidates
            .generated_contains_opportunity(empty_key)
    );
    assert!(
        planning
            .candidates
            .generated_contains_opportunity(usable_key)
    );
    let ranked = planning.candidates.ranked_summaries_for_goal(wash_goal);
    assert_eq!(ranked[0].opportunity, usable_key);
    assert_eq!(planning.selection.selected_opportunity, Some(usable_key));
    let wash_events = h
        .action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(agent)
        .into_iter()
        .filter(|event| event.action_name == "wash")
        .collect::<Vec<_>>();
    assert!(
        wash_events
            .iter()
            .any(|event| matches!(event.kind, ActionTraceKind::StartFailed { .. })),
        "stale empty-basin request should fail before the replanned usable basin is selected: {wash_events:?}"
    );
}

// Scenario 367A: Wash Re-emerges After First Cycle Relief
// Setup: A dirty AI agent washes at a stocked local basin, then its dirtiness
// is driven back above critical with an active exposure counter.
// Proves: The second-cycle Wash candidate is generated and planner search
// finds a Wash plan rather than losing the branch after relief reset.
// Chain: first wash commit -> dirtiness re-critical -> wash candidate emission
// -> planner search found for the same local basin.
#[test]
fn wash_re_emerges_after_first_cycle_drops_dirtiness_below_critical() {
    let mut h = GoldenHarness::new(Seed([
        0x8c, 0x51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0,
    ]));
    h.driver.enable_tracing();
    h.enable_action_tracing();
    let basin = place_wash_basin(&mut h, VILLAGE_SQUARE, WashBasinState::default());
    let _well = place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        VILLAGE_SQUARE,
        WorkstationTag::Well,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(20),
            max_quantity: Quantity(20),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: NonZeroU32::new(1).unwrap(),
            quality: None,
        },
        ProductionOutputOwner::Actor,
    );
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "SecondCycleWasher",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(950)),
        quiet_metabolism(),
        hygiene_utility(),
    );
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    run_until_commits(&mut h, agent, "wash", 1, 8);
    assert!(
        h.agent_dirtiness(agent)
            < h.world
                .get_component_drive_thresholds(agent)
                .unwrap()
                .dirtiness
                .critical(),
        "first wash should drop dirtiness below critical"
    );

    let params = h
        .world
        .get_component_drive_escalation_profile(agent)
        .unwrap()
        .params_for(HomeostaticNeedId::Dirtiness);
    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_component_homeostatic_needs(
        agent,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(950)),
    )
    .unwrap();
    txn.set_component_deprivation_exposure(
        agent,
        DeprivationExposure {
            dirtiness_critical_ticks: params.start_after_ticks + 1,
            ..DeprivationExposure::default()
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);

    let decision_tick = h.scheduler.current_tick();
    h.step_once();
    let planning = planning_trace_at(&h, agent, decision_tick);
    let wash_key = OpportunityKey {
        goal_key: GoalKey::from(GoalKind::Wash),
        anchor: OpportunityAnchor::Entity(basin),
    };
    assert!(planning.candidates.generated_contains_opportunity(wash_key));
    assert!(
        planning.planning.attempts.iter().any(|attempt| {
            attempt.goal.kind == GoalKind::Wash
                && matches!(attempt.outcome, PlanSearchOutcome::Found { .. })
        }),
        "second-cycle wash should have a found plan: {:?}",
        planning.planning.attempts
    );
}

// Scenario 368: Place Dirtiness Saturates With Zero Decay
// Setup: Repeated wilderness relief occurs at a zero-decay outdoor place.
// Proves: PlaceDirtiness saturates at 1000 and does not overflow or decrease.
// Chain: repeated relief -> saturating Permille addition -> durable place state.
#[test]
fn place_dirtiness_saturates_with_zero_decay() {
    let mut h = GoldenHarness::new(Seed([0x8d; 32]));
    h.enable_action_tracing();
    set_place_dirtiness(
        &mut h,
        ORCHARD_FARM,
        PlaceDirtiness {
            value: pm(960),
            decay_per_tick: pm(0),
            dirtiness_per_use: pm(80),
        },
    );
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Saturator",
        ORCHARD_FARM,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(900), pm(0)),
        quiet_metabolism(),
        hygiene_utility(),
    );
    set_control_source(&mut h, agent, ControlSource::Human);
    for expected_commits in 1..=2 {
        set_needs(
            &mut h,
            agent,
            HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(900), pm(0)),
        );
        request_action(&mut h, agent, "relieve_wilderness", vec![]);
        run_until_commits(&mut h, agent, "relieve_wilderness", expected_commits, 8);
    }
    assert_eq!(
        h.world
            .get_component_place_dirtiness(ORCHARD_FARM)
            .unwrap()
            .value,
        pm(1000)
    );
}

// Scenario 369: Wash Basin Plateaus At Zero With Zero Refill
// Setup: One partial wash drains a zero-refill basin, then maintenance advances.
// Proves: WashBasinState stays at zero without hidden refill or negative water.
// Chain: wash commit -> clean_water_units reaches zero -> item_decay_system zero-refill plateau.
#[test]
fn wash_basin_plateaus_at_zero_with_zero_refill() {
    let mut h = GoldenHarness::new(Seed([0x8e; 32]));
    h.enable_action_tracing();
    let basin = place_wash_basin(
        &mut h,
        VILLAGE_SQUARE,
        WashBasinState {
            clean_water_units: 1,
            max_clean_water: 5,
            refill_per_tick: 0,
            units_per_full_wash: 2,
            dirtiness_level: pm(0),
            dirtiness_per_use: pm(50),
            max_effective_dirtiness: pm(1000),
        },
    );
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "PlateauWasher",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(900)),
        quiet_metabolism(),
        hygiene_utility(),
    );
    set_control_source(&mut h, agent, ControlSource::Human);
    request_action(&mut h, agent, "wash", vec![basin]);
    run_until_commits(&mut h, agent, "wash", 1, 8);
    for _ in 0..4 {
        h.step_once();
    }
    assert_eq!(
        h.world
            .get_component_wash_basin_state(basin)
            .unwrap()
            .clean_water_units,
        0
    );
}
