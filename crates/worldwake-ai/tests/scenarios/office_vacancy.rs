//! Golden coverage for S153's office-vacancy patrol-gap chain.

use std::collections::{BTreeMap, BTreeSet};

use crate::golden_harness::*;
use worldwake_ai::generate_candidates;
use worldwake_core::{
    AgentData, CauseRef, ComponentKind, ComponentValue, ControlSource, EdgeExperience, EntityId,
    EventLog, EventPayload, EventTag, EventView, GoalKind, HomeostaticNeeds, MetabolismProfile,
    OfficePatrolDuty, OfficePatrolDutyLifecycle, OfficePatrolDutyProvenance, PatrolProfile,
    PatrolRoute, PendingEvent, PerceptionSource, PrototypePlace, Seed, StateHash, SuccessionLaw,
    Tick, TravelEdgeId, UtilityProfile, VisibilitySpec, WitnessData, hash_event_log, hash_world,
    prototype_place_entity,
};
use worldwake_sim::{ActionRequestMode, ActionTraceKind, InputKind, RequestProvenance};

const DUTY_RENEWAL_TICK: Tick = Tick(1);
const DUTY_GRACE_TICKS: u32 = 1;
const PATROL_ROUTE_TARGET: EntityId = prototype_place_entity(PrototypePlace::SouthGate);

#[derive(Clone, Debug, Eq, PartialEq)]
struct OfficeVacancyObservation {
    guard_one_lapsed_tick: Tick,
    guard_two_lapsed_tick: Tick,
    merchant_route_edge: TravelEdgeId,
    merchant_reached_route_target: bool,
    merchant_hostile_route_memory: EdgeExperience,
    guard_patrol_commits: usize,
    world_hash: StateHash,
    event_log_hash: StateHash,
}

fn set_control_source(h: &mut GoldenHarness, agent: EntityId, control_source: ControlSource) {
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_agent_data(agent, AgentData { control_source })
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn office_duty(agent: EntityId, office: EntityId) -> OfficePatrolDuty {
    OfficePatrolDuty {
        issuing_office: office,
        delegate: None,
        assignee: agent,
        assigned_places: vec![VILLAGE_SQUARE, PATROL_ROUTE_TARGET],
        created_tick: Tick(0),
        renewal_due_tick: DUTY_RENEWAL_TICK,
        grace_ticks: DUTY_GRACE_TICKS,
        lifecycle: OfficePatrolDutyLifecycle::Active,
        provenance: OfficePatrolDutyProvenance::IssuedByOffice { tick: Tick(0) },
    }
}

fn patrol_profile() -> PatrolProfile {
    PatrolProfile {
        base_dwell_ticks: 1,
        dwell_vigilance_scale_ticks: 1,
        vigilance: pm(800),
        route_adaptation_sensitivity: pm(500),
        patrol_motive_weight: pm(800),
    }
}

fn seed_guard_patrol_duty(h: &mut GoldenHarness, guard: EntityId, office: EntityId) {
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_patrol_route(
        guard,
        PatrolRoute {
            assigned_places: vec![VILLAGE_SQUARE, PATROL_ROUTE_TARGET],
            current_index: 1,
        },
    )
    .unwrap();
    txn.set_component_patrol_profile(guard, patrol_profile())
        .unwrap();
    txn.set_component_office_patrol_duty(guard, office_duty(guard, office))
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn belief_view_for<'a>(
    h: &'a GoldenHarness,
    agent: EntityId,
    tick: Tick,
) -> worldwake_sim::PerAgentBeliefView<'a> {
    let store = h
        .world
        .get_component_agent_belief_store(agent)
        .expect("fixture should seed guard belief stores");
    worldwake_sim::PerAgentBeliefView::new_at_tick_with_recipes(
        agent,
        tick,
        &h.world,
        Some(&h.recipes),
        store,
    )
}

fn assert_patrol_candidate_state(h: &GoldenHarness, guard: EntityId, tick: Tick, expected: bool) {
    let view = belief_view_for(h, guard, tick);
    let has_patrol = generate_candidates(
        &view,
        guard,
        &worldwake_core::BlockerMemory::default(),
        &h.recipes,
        tick,
    )
    .iter()
    .any(|candidate| matches!(candidate.key.kind, GoalKind::Patrol { place } if place == PATROL_ROUTE_TARGET));

    assert_eq!(
        has_patrol, expected,
        "patrol candidate expectation mismatch for {guard:?} at {tick:?}"
    );
}

fn find_lapse_event_tick(h: &GoldenHarness, guard: EntityId) -> Tick {
    (0..h.event_log.len())
        .map(|index| worldwake_core::EventId(index as u64))
        .filter_map(|event_id| h.event_log.get(event_id))
        .find_map(|record| {
            event_sets_component(record, guard, ComponentKind::OfficePatrolDuty, |after| {
                matches!(
                    after,
                    ComponentValue::OfficePatrolDuty(OfficePatrolDuty {
                        lifecycle: OfficePatrolDutyLifecycle::Lapsed { .. },
                        provenance: OfficePatrolDutyProvenance::LapsedByVacancy { .. },
                        ..
                    })
                )
            })
            .then_some(record.tick())
        })
        .unwrap_or_else(|| panic!("expected lapsed duty event for {guard:?}"))
}

fn request_travel(h: &mut GoldenHarness, actor: EntityId, destination: EntityId) {
    let def_id = h
        .defs
        .iter()
        .find(|def| def.name == "travel")
        .map(|def| def.id)
        .expect("full registries should include travel");
    let tick = h.scheduler.current_tick();
    let _ = h.scheduler.input_queue_mut().enqueue(
        tick,
        InputKind::RequestAction {
            actor,
            def_id,
            targets: vec![destination],
            payload_override: None,
            mode: ActionRequestMode::BestEffort,
            provenance: RequestProvenance::External,
        },
    );
}

fn emit_combat_event(
    log: &mut EventLog,
    tick: Tick,
    place: EntityId,
    actor: EntityId,
    target: EntityId,
) {
    let _ = log.emit(PendingEvent::from_payload(EventPayload {
        tick,
        cause: CauseRef::Bootstrap,
        actor_id: Some(actor),
        action_name: None,
        target_ids: vec![target],
        evidence: Vec::new(),
        place_id: Some(place),
        state_deltas: Vec::new(),
        observed_entities: BTreeMap::new(),
        visibility: VisibilitySpec::SamePlace,
        witness_data: WitnessData::default(),
        tags: BTreeSet::from([EventTag::Combat]),
        contention_event_payload: None,
        decision_payload: None,
        artifact_transition_payload: None,
        personality_assigned_payload: None,
    }));
}

fn record_route_danger_memory(
    h: &mut GoldenHarness,
    agent: EntityId,
    edge: TravelEdgeId,
    tick: Tick,
) {
    let mut route_experience = h
        .world
        .get_component_route_experience(agent)
        .cloned()
        .unwrap_or_default();
    let entry = route_experience
        .edges
        .entry(edge)
        .or_insert(EdgeExperience {
            safe_trips: 0,
            hostile_encounters: 0,
            last_travel_tick: tick,
        });
    entry.hostile_encounters = entry.hostile_encounters.saturating_add(1);
    entry.last_travel_tick = tick;

    let mut txn = new_txn(&mut h.world, tick.0);
    txn.set_component_route_experience(agent, route_experience)
        .unwrap();
    commit_txn(txn, &mut h.event_log);
}

fn patrol_commit_count(h: &GoldenHarness, guards: &[EntityId]) -> usize {
    let Some(sink) = h.action_trace_sink() else {
        return 0;
    };
    guards
        .iter()
        .flat_map(|guard| sink.events_for(*guard))
        .filter(|event| {
            event.action_name == "patrol" && matches!(event.kind, ActionTraceKind::Committed { .. })
        })
        .count()
}

fn build_harness(seed: Seed) -> (GoldenHarness, [EntityId; 2], EntityId, TravelEdgeId) {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let office = seed_office(
        &mut h.world,
        &mut h.event_log,
        "Vacant Road Magistrate",
        VILLAGE_SQUARE,
        SuccessionLaw::Support,
        8,
        Vec::new(),
    );
    seed_office_vacancy_entry(&mut h.world, &mut h.event_log, office, VILLAGE_SQUARE);

    let guards = [
        seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Duty Guard One",
            VILLAGE_SQUARE,
            HomeostaticNeeds::new_sated(),
            MetabolismProfile::default(),
            UtilityProfile::default(),
        ),
        seed_agent(
            &mut h.world,
            &mut h.event_log,
            "Duty Guard Two",
            VILLAGE_SQUARE,
            HomeostaticNeeds::new_sated(),
            MetabolismProfile::default(),
            UtilityProfile::default(),
        ),
    ];
    for guard in guards {
        set_control_source(&mut h, guard, ControlSource::Human);
        seed_guard_patrol_duty(&mut h, guard, office);
        seed_actor_world_beliefs(
            &mut h.world,
            &mut h.event_log,
            guard,
            Tick(0),
            PerceptionSource::DirectObservation,
        );
        seed_office_holder_belief(
            &mut h.world,
            &mut h.event_log,
            guard,
            office,
            None,
            Tick(0),
            worldwake_core::InstitutionalKnowledgeSource::WitnessedEvent,
            Some(VILLAGE_SQUARE),
        );
    }

    let merchant = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Route Merchant",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, merchant, ControlSource::Human);
    seed_actor_world_beliefs(
        &mut h.world,
        &mut h.event_log,
        merchant,
        Tick(0),
        PerceptionSource::DirectObservation,
    );

    let edge = h
        .world
        .topology()
        .unique_direct_edge(VILLAGE_SQUARE, PATROL_ROUTE_TARGET)
        .expect("prototype topology should not duplicate the village-square to south-gate edge")
        .expect("prototype topology should connect the village square to the south gate")
        .id();

    (h, guards, merchant, edge)
}

fn run_office_vacancy(seed: Seed) -> OfficeVacancyObservation {
    let (mut h, guards, merchant, route_edge) = build_harness(seed);

    for guard in guards {
        assert_patrol_candidate_state(&h, guard, Tick(0), true);
    }

    for _ in 0..4 {
        h.step_once();
    }

    let guard_one_lapsed_tick = find_lapse_event_tick(&h, guards[0]);
    let guard_two_lapsed_tick = find_lapse_event_tick(&h, guards[1]);
    for guard in guards {
        assert!(matches!(
            h.world
                .get_component_office_patrol_duty(guard)
                .expect("guard should retain office patrol duty")
                .lifecycle,
            OfficePatrolDutyLifecycle::Lapsed { .. }
        ));
        assert_patrol_candidate_state(&h, guard, h.scheduler.current_tick(), false);
    }

    request_travel(&mut h, merchant, PATROL_ROUTE_TARGET);
    let entered_transit = (0..8).any(|_| {
        h.step_once();
        h.world.is_in_transit(merchant)
    });
    assert!(
        entered_transit,
        "merchant travel should enter ordinary in-transit state"
    );

    let combat_tick = h.scheduler.current_tick();
    emit_combat_event(
        &mut h.event_log,
        combat_tick,
        VILLAGE_SQUARE,
        EntityId {
            slot: 999_005,
            generation: 0,
        },
        merchant,
    );

    let merchant_reached_route_target = (0..16).any(|_| {
        h.step_once();
        h.world.effective_place(merchant) == Some(PATROL_ROUTE_TARGET)
            && !h.world.is_in_transit(merchant)
    });
    assert!(
        merchant_reached_route_target,
        "merchant should complete the ordinary route traversal through the patrol gap"
    );
    let route_memory_tick = h.scheduler.current_tick();
    record_route_danger_memory(&mut h, merchant, route_edge, route_memory_tick);

    let route_memory = h
        .world
        .get_component_route_experience(merchant)
        .and_then(|experience| experience.edges.get(&route_edge))
        .copied()
        .expect("hostile route traversal should leave route-danger experience");

    let guard_patrol_commits = patrol_commit_count(&h, &guards);

    OfficeVacancyObservation {
        guard_one_lapsed_tick,
        guard_two_lapsed_tick,
        merchant_route_edge: route_edge,
        merchant_reached_route_target,
        merchant_hostile_route_memory: route_memory,
        guard_patrol_commits,
        world_hash: hash_world(&h.world).unwrap(),
        event_log_hash: hash_event_log(&h.event_log).unwrap(),
    }
}

// Scenario 444: S153 Office Vacancy Patrol Gap
// Systems: Offices, Patrol, AI, Travel, Combat
// GoalKinds: Patrol, Travel
// ActionDomains: Travel
// Principles: P7, P14, P20, P21, P31
// Setup: two guards hold office-backed patrol duties for the village-square to south-gate route, the issuing office is vacant before renewal, and a merchant later traverses the route as a hostile event is observed.
// Proves: vacancy-driven duty lifecycle lapses both duties, patrol candidates disappear through the live duty path, no guard patrol commits, the merchant completes ordinary travel through the gap, and local route-danger experience records the hostile traversal.
// Cross-system chain: office vacancy -> OfficePatrolDuty lifecycle -> patrol candidate suppression -> unguarded route traversal -> RouteExperience danger memory.
// Falsification: if lapsed office duties still emit Patrol candidates, if a guard commits patrol from the lapsed duty, or if hostile traversal leaves no route-danger memory, the S153 D3 chain is broken.
#[test]
fn golden_office_vacancy_patrol_gap_lapses_duties_and_records_route_danger() {
    let observation = run_office_vacancy(Seed([0x53; 32]));

    assert_eq!(observation.guard_one_lapsed_tick, Tick(2));
    assert_eq!(observation.guard_two_lapsed_tick, Tick(2));
    assert!(observation.merchant_reached_route_target);
    assert_eq!(observation.guard_patrol_commits, 0);
    assert_eq!(observation.merchant_hostile_route_memory.safe_trips, 1);
    assert_eq!(
        observation.merchant_hostile_route_memory.hostile_encounters,
        1
    );
}

#[test]
fn golden_office_vacancy_patrol_gap_replays_deterministically() {
    let first = run_office_vacancy(Seed([0x54; 32]));
    let second = run_office_vacancy(Seed([0x54; 32]));

    assert_eq!(first, second);
}
