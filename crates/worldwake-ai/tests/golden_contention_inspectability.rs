//! Golden coverage for S142 contention event inspectability.
//!
//! These tests assert the event-log surface because S142's contract is the
//! append-only contention-resolution record itself: `ContentionResolved`
//! payloads, deterministic claimant ordering, and blocker backreferences.

mod golden_harness;

use std::collections::{BTreeMap, BTreeSet};
use std::num::{NonZeroU8, NonZeroU32};
use std::path::PathBuf;

use golden_harness::*;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario};
use worldwake_core::{
    AcquisitionQuantity, AffordanceKey, AgentData, Blocker, BlockerClearingCondition, BlockerKey,
    BlockerMemory, BlockingFact, ClaimantOutcome, CommodityKind, CommodityPurpose,
    ContentionIntents, ContentionQueue, ContentionResolutionRule, ControlSource, EntityId, EventId,
    EventTag, EventView, GoalKey, GoalKind, HomeostaticNeeds, KnownRecipes, MetabolismProfile,
    PerceptionSource, Quantity, QueuedContentionIntent, ResourceSource, Seed, Tick, UtilityProfile,
    WorkstationTag,
};
use worldwake_sim::{ActionRequestMode, InputKind, RequestProvenance};

const APPLE_RECIPE_ID: worldwake_core::RecipeId = worldwake_core::RecipeId(0);
const WATER_RECIPE_ID: worldwake_core::RecipeId = worldwake_core::RecipeId(2);
const SURVIVAL_TICKS: u32 = 1440;

fn build_contention_harness(seed: Seed) -> GoldenHarness {
    let mut h = GoldenHarness::with_recipes(seed, build_multi_recipe_registry());
    h.driver.enable_tracing();
    h.enable_action_tracing();
    h
}

fn set_control_source(h: &mut GoldenHarness, agent: EntityId, control_source: ControlSource) {
    let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
    txn.set_component_agent_data(agent, AgentData { control_source })
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn action_id(h: &GoldenHarness, name: &str) -> worldwake_core::ActionDefId {
    h.defs.iter().find(|def| def.name == name).map_or_else(
        || panic!("full registries should include {name}"),
        |def| def.id,
    )
}

fn request_action(h: &mut GoldenHarness, actor: EntityId, def_name: &str, targets: Vec<EntityId>) {
    let def_id = action_id(h, def_name);
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

fn place_orchard(h: &mut GoldenHarness, place: EntityId, extraction_slots: u8) -> EntityId {
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        place,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(20),
            max_quantity: Quantity(20),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: NonZeroU8::new(extraction_slots).unwrap(),
            extraction_duration_ticks: NonZeroU32::new(3).unwrap(),
        },
        ProductionOutputOwner::Actor,
    )
}

fn place_well(h: &mut GoldenHarness, place: EntityId, extraction_slots: u8) -> EntityId {
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        place,
        WorkstationTag::Well,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(20),
            max_quantity: Quantity(20),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: NonZeroU8::new(extraction_slots).unwrap(),
            extraction_duration_ticks: NonZeroU32::new(3).unwrap(),
        },
        ProductionOutputOwner::Actor,
    )
}

fn place_contention_managed_well(h: &mut GoldenHarness, place: EntityId) -> EntityId {
    place_exclusive_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        place,
        WorkstationTag::Well,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: Quantity(20),
            max_quantity: Quantity(20),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: NonZeroU8::new(1).unwrap(),
            extraction_duration_ticks: NonZeroU32::new(3).unwrap(),
        },
        NonZeroU32::new(3).unwrap(),
        ProductionOutputOwner::Actor,
    )
}

fn seed_hungry_agent(h: &mut GoldenHarness, name: &str, place: EntityId) -> EntityId {
    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        name,
        place,
        HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::with([APPLE_RECIPE_ID]),
    );
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    agent
}

fn seed_thirsty_agent(h: &mut GoldenHarness, name: &str, place: EntityId) -> EntityId {
    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        name,
        place,
        HomeostaticNeeds::new(pm(0), pm(850), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::with([WATER_RECIPE_ID]),
    );
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    agent
}

fn set_extraction_queue(
    h: &mut GoldenHarness,
    workstation: EntityId,
    waiters: &[(EntityId, Tick)],
    action_name: &str,
) {
    let action = action_id(h, action_name);
    let mut queue = ContentionQueue::default();
    for (actor, queued_at) in waiters {
        queue.enqueue(*actor, action, *queued_at, None).unwrap();
    }
    let mut queues = h
        .world
        .get_component_resource_extraction_queues(workstation)
        .cloned()
        .expect("workstation should have extraction queues");
    queues.queues[0] = queue;
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_resource_extraction_queues(workstation, queues)
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn set_facility_queue(
    h: &mut GoldenHarness,
    facility: EntityId,
    waiters: &[(EntityId, Tick)],
    action_name: &str,
) {
    let action = action_id(h, action_name);
    let mut queue = ContentionQueue::default();
    for (actor, queued_at) in waiters {
        queue.enqueue(*actor, action, *queued_at, None).unwrap();
    }
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_contention_queue(facility, queue).unwrap();
    for (actor, _) in waiters {
        txn.set_component_contention_intents(
            *actor,
            ContentionIntents {
                intents: BTreeMap::from([(
                    facility,
                    QueuedContentionIntent {
                        goal_key: GoalKey::from(GoalKind::AcquireCommodity {
                            commodity: CommodityKind::Water,
                            purpose: CommodityPurpose::SelfConsume,
                            quantity: AcquisitionQuantity::single(),
                        }),
                        intended_action: action,
                    },
                )]),
            },
        )
        .unwrap();
    }
    commit_txn(txn, &mut h.event_log);
}

fn set_reservation_conflict_blocker(
    h: &mut GoldenHarness,
    actor: EntityId,
    affordance: AffordanceKey,
    tick: Tick,
    contention_event: Option<EventId>,
) {
    let blocker_key = BlockerKey {
        goal_key: GoalKey::from(GoalKind::Sleep),
        place: Some(ORCHARD_FARM),
        target: Some(affordance.facility),
        action_def: Some(affordance.action),
    };
    let mut memory = BlockerMemory::default();
    memory.record(Blocker {
        blocker_key,
        blocking_fact: BlockingFact::ReservationConflict {
            affordance,
            contention_event,
        },
        diagnostic_context: None,
        observed_tick: tick,
        expires_tick: Tick(tick.0 + 20),
        clearing_condition: BlockerClearingCondition::ContentionChanged {
            facility: affordance.facility,
        },
        baseline_snapshot: None,
    });
    let mut txn = new_txn(&mut h.world, tick.0);
    txn.set_component_blocker_memory(actor, memory).unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn contention_payloads(
    h: &GoldenHarness,
) -> Vec<(EventId, worldwake_core::ContentionEventPayload)> {
    h.event_log
        .events_by_tag(EventTag::ContentionResolved)
        .iter()
        .filter_map(|event_id| {
            h.event_log
                .get(*event_id)
                .and_then(EventView::contention_event_payload)
                .cloned()
                .map(|payload| (*event_id, payload))
        })
        .collect()
}

fn payload_for_facility(
    h: &GoldenHarness,
    facility: EntityId,
) -> (EventId, worldwake_core::ContentionEventPayload) {
    contention_payloads(h)
        .into_iter()
        .find(|(_, payload)| payload.contested_affordance.facility == facility)
        .unwrap_or_else(|| panic!("expected contention payload for {facility:?}"))
}

fn survival_contested_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-contested.ron")
}

fn load_survival_contested_harness() -> GoldenHarness {
    let def = load_scenario_file(&survival_contested_path())
        .expect("survival-contested scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival-contested scenario should spawn");
    let mut harness = GoldenHarness::from_simulation_state(&spawned.state);
    harness.driver.enable_tracing();
    harness.enable_action_tracing();
    harness
}

fn run_survival_contested_contention_payloads(
    ticks: u32,
) -> Vec<worldwake_core::ContentionEventPayload> {
    let mut h = load_survival_contested_harness();
    for _ in 0..ticks {
        h.step_once();
    }
    contention_payloads(&h)
        .into_iter()
        .map(|(_, payload)| payload)
        .collect()
}

fn blocker_memory(agent: EntityId, h: &GoldenHarness) -> BlockerMemory {
    h.world
        .get_component_blocker_memory(agent)
        .cloned()
        .unwrap_or_default()
}

fn reservation_conflict_event_refs(memory: &BlockerMemory) -> Vec<EventId> {
    memory
        .intents
        .values()
        .filter_map(|blocker| {
            let BlockingFact::ReservationConflict {
                contention_event, ..
            } = blocker.blocking_fact
            else {
                return None;
            };
            contention_event
        })
        .collect()
}

// Scenario 393: Single-Slot Orchard Emits Arrival-Time Contention Payload
//
// Setup: Three human-controlled agents are co-located at a single-slot orchard with a pre-seeded waiting queue before the head actor makes a real harvest request.
// The head actor makes a real `harvest:Harvest Apples` request, isolating the
// S142 snapshot-before-mutate contract from unrelated AI arrival timing.
//
// Proves: Resource-extraction grant emits ContentionResolved with all queued claimants in ordinal order, first Granted, later QueuedBehind, and ArrivalTime rule.
// The first actor is marked Granted, later actors are marked QueuedBehind, and
// `ArrivalTime` is the explicit rule.
//
// Chain: ResourceExtractionQueues.waiting -> harvest start -> grant_or_signal_full -> set_contention_event_payload -> EventLog::events_by_tag(ContentionResolved).
// The event is then queryable through EventLog::events_by_tag(ContentionResolved).
#[test]
fn golden_three_agents_single_slot_orchard_emit_per_grant() {
    let mut h = build_contention_harness(Seed([0x42; 32]));
    let orchard = place_orchard(&mut h, ORCHARD_FARM, 1);
    let agent_a = seed_hungry_agent(&mut h, "Aria", ORCHARD_FARM);
    let agent_b = seed_hungry_agent(&mut h, "Bram", ORCHARD_FARM);
    let agent_c = seed_hungry_agent(&mut h, "Cael", ORCHARD_FARM);
    for agent in [agent_a, agent_b, agent_c] {
        set_control_source(&mut h, agent, ControlSource::Human);
    }
    set_extraction_queue(
        &mut h,
        orchard,
        &[(agent_a, Tick(4)), (agent_b, Tick(5)), (agent_c, Tick(6))],
        "harvest:Harvest Apples",
    );

    request_action(&mut h, agent_a, "harvest:Harvest Apples", vec![orchard]);
    h.step_once();

    let (_event_id, payload) = payload_for_facility(&h, orchard);
    assert_eq!(payload.contested_affordance.facility, orchard);
    assert_eq!(
        payload.contested_affordance.action,
        action_id(&h, "harvest:Harvest Apples")
    );
    assert_eq!(payload.place, ORCHARD_FARM);
    assert_eq!(
        payload.resolution_rule,
        ContentionResolutionRule::ArrivalTime
    );
    assert_eq!(payload.total_claimants, 3);
    assert_eq!(payload.winner, Some(agent_a));
    assert_eq!(payload.at_tick, Tick(0));
    assert_eq!(
        payload
            .claimants
            .iter()
            .map(|claimant| (
                claimant.agent,
                claimant.arrived_tick,
                claimant.queue_position,
                claimant.outcome
            ))
            .collect::<Vec<_>>(),
        vec![
            (agent_a, Tick(4), 1, ClaimantOutcome::Granted),
            (agent_b, Tick(5), 2, ClaimantOutcome::QueuedBehind),
            (agent_c, Tick(6), 3, ClaimantOutcome::QueuedBehind),
        ]
    );
}

// Scenario 394: Survival Contested Emits Resource And Facility Contention
//
// Setup: Run authored survival-contested.ron long enough for resource-extraction and facility-queue contention substrates to emit.
//
// Proves: Scenario-backed path emits ContentionResolved from both substrate families with concrete (facility, action) keys and deterministic claimant ordering when claimants are present.
// Every emitted payload carries a concrete `(facility, action)` key plus
// deterministic claimant order when claimants are present.
//
// Chain: authored scenario -> AI self-care/acquisition -> queue/grant substrates -> typed contention events.
#[test]
#[ignore = "manual-only: scenario-backed contention run; use explicit ignored command"]
fn golden_survival_contested_multi_substrate_emission() {
    let payloads = run_survival_contested_contention_payloads(400);
    assert!(
        !payloads.is_empty(),
        "survival-contested should emit at least one contention event"
    );

    let mut saw_resource_extraction = false;
    let mut saw_facility_queue = false;
    for payload in &payloads {
        assert_eq!(
            payload.resolution_rule,
            ContentionResolutionRule::ArrivalTime
        );
        if let Some(first_claimant) = payload.claimants.first() {
            assert_eq!(payload.winner, Some(first_claimant.agent));
        }
        let mut positions = BTreeSet::new();
        for claimant in &payload.claimants {
            assert!(
                positions.insert(claimant.queue_position),
                "queue positions should be unique within a payload: {payload:?}"
            );
        }
        match payload.claimants.len() {
            0 => saw_resource_extraction = true,
            _ => saw_facility_queue = true,
        }
    }
    assert!(
        saw_resource_extraction,
        "survival-contested should include direct resource-extraction grants; payloads={payloads:?}"
    );
    assert!(
        saw_facility_queue,
        "survival-contested should include facility queue promotions; payloads={payloads:?}"
    );
}

// Scenario 395: Well Facility Queue Admission Emits Contention Payload
//
// Setup: Two thirsty human-controlled agents queue for one auto-promoting contention-managed well with a pre-seeded facility queue.
// The queue is pre-seeded so the facility system owns the resolution moment
// directly.
//
// Proves: Facility-queue path emits ContentionResolved on the QueueGrantPromoted event with the head Granted and the following claimant behind.
// The head waiter is granted and the following claimant is retained behind
// them.
//
// Chain: ContentionQueue.waiting -> contention_system::promote_ready_head -> commit_queue_update -> typed contention event plus QueueGrantPromoted tag.
// The same event also carries the QueueGrantPromoted tag.
#[test]
fn golden_well_facility_queue_admission() {
    let mut h = build_contention_harness(Seed([0x43; 32]));
    let well = place_contention_managed_well(&mut h, VILLAGE_SQUARE);
    let agent_a = seed_thirsty_agent(&mut h, "Well User A", VILLAGE_SQUARE);
    let agent_b = seed_thirsty_agent(&mut h, "Well User B", VILLAGE_SQUARE);
    for agent in [agent_a, agent_b] {
        set_control_source(&mut h, agent, ControlSource::Human);
    }
    set_facility_queue(
        &mut h,
        well,
        &[(agent_a, Tick(10)), (agent_b, Tick(11))],
        "harvest:Harvest Water",
    );

    h.step_once();

    let (event_id, payload) = payload_for_facility(&h, well);
    assert!(
        h.event_log
            .events_by_tag(EventTag::QueueGrantPromoted)
            .contains(&event_id),
        "facility contention event should share the QueueGrantPromoted event"
    );
    assert_eq!(payload.contested_affordance.facility, well);
    assert_eq!(
        payload.contested_affordance.action,
        action_id(&h, "harvest:Harvest Water")
    );
    assert_eq!(payload.place, VILLAGE_SQUARE);
    assert_eq!(
        payload.resolution_rule,
        ContentionResolutionRule::ArrivalTime
    );
    assert_eq!(payload.total_claimants, 2);
    assert_eq!(payload.winner, Some(agent_a));
    assert_eq!(
        payload
            .claimants
            .iter()
            .map(|claimant| (
                claimant.agent,
                claimant.arrived_tick,
                claimant.queue_position,
                claimant.outcome
            ))
            .collect::<Vec<_>>(),
        vec![
            (agent_a, Tick(10), 1, ClaimantOutcome::Granted),
            (agent_b, Tick(11), 2, ClaimantOutcome::QueuedBehind),
        ]
    );
}

// Scenario 396: Reservation Conflict Backreference Resolves To Event Payload
//
// Setup: A real harvest grant emits ContentionResolved, then BlockerMemory stores that event id in ReservationConflict.contention_event.
// The stored field is the same `BlockingFact::ReservationConflict.contention_event`
// carrier that the AI lookup helper writes.
//
// Proves: Stored blocker backreference resolves to the corresponding ContentionResolved event payload; private AI lookup helper remains unit-covered.
// The private AI lookup helper is covered by focused `agent_tick::execution`
// unit tests; this golden owns the persisted cross-carrier event-log shape.
//
// Chain: harvest grant -> typed contention event -> BlockerMemory ReservationConflict.contention_event -> EventLog payload lookup.
#[test]
fn golden_blocker_memory_attribution_payload_resolves() {
    let mut h = build_contention_harness(Seed([0x44; 32]));
    let well = place_well(&mut h, ORCHARD_FARM, 1);
    let agent_a = seed_thirsty_agent(&mut h, "Aria", ORCHARD_FARM);
    let agent_b = seed_thirsty_agent(&mut h, "Bram", ORCHARD_FARM);
    set_control_source(&mut h, agent_a, ControlSource::Human);
    set_control_source(&mut h, agent_b, ControlSource::Human);
    let affordance = AffordanceKey {
        facility: well,
        action: action_id(&h, "harvest:Harvest Water"),
    };

    request_action(&mut h, agent_a, "harvest:Harvest Water", vec![well]);
    h.step_once();

    let (event_id, payload) = payload_for_facility(&h, well);
    assert_eq!(payload.contested_affordance.facility, well);
    assert_eq!(payload.contested_affordance.action, affordance.action);
    assert_eq!(payload.at_tick, Tick(0));
    assert_eq!(payload.winner, Some(agent_a));

    set_reservation_conflict_blocker(&mut h, agent_b, affordance, Tick(0), Some(event_id));
    let refs = reservation_conflict_event_refs(&blocker_memory(agent_b, &h));
    assert!(
        refs.contains(&event_id),
        "blocker memory should record the same contention event id; refs={refs:?}, event_id={event_id:?}"
    );
    let resolved_payload = h
        .event_log
        .get(event_id)
        .and_then(EventView::contention_event_payload)
        .expect("recorded blocker event id should resolve to contention payload");
    assert_eq!(resolved_payload.contested_affordance, affordance);
}

// Scenario 397: Survival Contested Contention Events Replay Deterministically
//
// Setup: Run survival-contested.ron twice from its authored seed for the full 1440-tick window and capture ContentionResolved payloads.
// Each run captures only `ContentionResolved` payloads.
//
// Proves: S142 event emission is deterministic at event-log payload surface across same-seed independent runs.
// Same seed, same authored scenario, same contention sequence.
//
// Chain: scenario spawn -> 1440 ticks -> EventLog::events_by_tag slice -> payload equality across independent runs.
#[test]
#[ignore = "manual-only: 1440-tick replay parity; use explicit ignored command"]
fn golden_survival_contested_replay_parity() {
    let first = run_survival_contested_contention_payloads(SURVIVAL_TICKS);
    let second = run_survival_contested_contention_payloads(SURVIVAL_TICKS);
    assert_eq!(first, second);
    assert!(
        !first.is_empty(),
        "replay parity should cover a non-empty contention sequence"
    );
}
