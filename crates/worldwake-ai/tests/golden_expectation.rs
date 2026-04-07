//! Golden tests for expectation-driven search behavior and replay fidelity.

mod golden_harness;

use golden_harness::*;
use worldwake_ai::{DecisionOutcome, GoalKind, SelectedPlanSource};
use worldwake_core::{
    AgentData, BeliefConfidencePolicy, ControlSource, ExpectationBasis, ExpectationId,
    ExpectationOutcome, ExpectationRecord, ExpectationState, HomeostaticNeeds, LastSeenRecord,
    MetabolismProfile, PerceptionProfile, Seed, Tick, UtilityProfile, ViolationDispositionProfile,
    hash_event_log, hash_world,
};
use worldwake_sim::{ActionTraceDetail, ActionTraceKind};

fn default_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        entity_memory_capacity: 32,
        entity_claim_capacity: 32,
        memory_retention_ticks: 240,
        observation_fidelity: pm(1000),
        confidence_policy: BeliefConfidencePolicy::default(),
        institutional_memory_capacity: 20,
        consultation_speed_factor: pm(500),
        contradiction_tolerance: pm(300),
    }
}

fn violation_profile() -> ViolationDispositionProfile {
    ViolationDispositionProfile {
        investigation_duration_ticks: nz(3),
        violation_memory_retention_ticks: 12,
        investigation_motive_weight: pm(400),
        ownership_motive_bonus: pm(200),
    }
}

fn set_control_source(
    h: &mut GoldenHarness,
    agent: worldwake_core::EntityId,
    control_source: ControlSource,
    tick: u64,
) {
    let mut txn = new_txn(&mut h.world, tick);
    txn.set_component_agent_data(agent, AgentData { control_source })
        .expect("golden expectation scenario should keep control source writable");
    commit_txn(txn, &mut h.event_log);
}

fn set_violation_profile(
    h: &mut GoldenHarness,
    agent: worldwake_core::EntityId,
    profile: ViolationDispositionProfile,
    tick: u64,
) {
    let mut txn = new_txn(&mut h.world, tick);
    txn.set_component_violation_disposition_profile(agent, profile)
        .expect("golden expectation scenario should keep violation profiles writable");
    commit_txn(txn, &mut h.event_log);
}

fn seed_expectation(
    h: &mut GoldenHarness,
    actor: worldwake_core::EntityId,
    subject: worldwake_core::EntityId,
    expected_place: worldwake_core::EntityId,
    tick: u64,
) -> ExpectationId {
    let mut txn = new_txn(&mut h.world, tick);
    let mut store = txn
        .get_component_expectation_store(actor)
        .cloned()
        .unwrap_or_default();
    let id = ExpectationId(1);
    store.records.insert(
        id,
        ExpectationRecord {
            id,
            owner: actor,
            subject,
            expected_place,
            deadline_tick: Tick(0),
            grace_ticks: 0,
            basis: ExpectationBasis::RoutineReturn,
            state: ExpectationState::Active,
            created_tick: Tick(tick),
        },
    );
    txn.set_component_expectation_store(actor, store)
        .expect("golden expectation scenario should keep expectation stores writable");
    commit_txn(txn, &mut h.event_log);
    id
}

fn planning_trace_at(
    h: &GoldenHarness,
    agent: worldwake_core::EntityId,
    tick: Tick,
) -> Option<&worldwake_ai::PlanningPipelineTrace> {
    let trace = h.driver.trace_sink()?.trace_at(agent, tick)?;
    match &trace.outcome {
        DecisionOutcome::Planning(planning) => Some(planning),
        _ => None,
    }
}

fn last_seen_record(
    h: &GoldenHarness,
    actor: worldwake_core::EntityId,
    subject: worldwake_core::EntityId,
) -> Option<LastSeenRecord> {
    h.world
        .get_component_last_seen_memory(actor)
        .and_then(|memory| memory.records.get(&subject).copied())
}

fn expectation_state(
    h: &GoldenHarness,
    actor: worldwake_core::EntityId,
    expectation_id: ExpectationId,
) -> ExpectationState {
    h.world
        .get_component_expectation_store(actor)
        .and_then(|store| store.records.get(&expectation_id))
        .map_or_else(
            || panic!("missing expectation record {expectation_id}"),
            |record| record.state,
        )
}

struct SearchScenarioOutcome {
    world_hash: worldwake_core::StateHash,
    log_hash: worldwake_core::StateHash,
}

fn run_overdue_expectation_search(seed: Seed) -> SearchScenarioOutcome {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let searcher = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Searcher",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let subject = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Missing",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, subject, ControlSource::None, 0);
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        searcher,
        default_perception_profile(),
    );
    set_violation_profile(&mut h, searcher, violation_profile(), 0);
    let expectation_id = seed_expectation(&mut h, searcher, subject, ORCHARD_FARM, 0);

    let mut overdue_tick = None;
    let mut generated_tick = None;
    let mut selected_tick = None;

    for tick in 0..80_u64 {
        h.step_once();

        let state = expectation_state(&h, searcher, expectation_id);
        if state == ExpectationState::Overdue && overdue_tick.is_none() {
            overdue_tick = Some(Tick(tick));
        }

        if let Some(planning) = planning_trace_at(&h, searcher, Tick(tick)) {
            let generated_search = planning.candidates.generated.iter().any(|goal| {
                matches!(
                    goal.goal_key.kind,
                    GoalKind::SearchForMissing {
                        subject: candidate_subject,
                        ..
                    } if candidate_subject == subject
                )
            });
            if generated_search && generated_tick.is_none() {
                generated_tick = Some(Tick(tick));
            }

            let selected_search = planning.selection.selected_goal().is_some_and(|goal| {
                matches!(
                    goal.kind,
                    GoalKind::SearchForMissing {
                        subject: candidate_subject,
                        ..
                    } if candidate_subject == subject
                )
            });
            if selected_search && selected_tick.is_none() {
                selected_tick = Some(Tick(tick));
            }
        }

        if matches!(
            expectation_state(&h, searcher, expectation_id),
            ExpectationState::Resolved {
                outcome: ExpectationOutcome::FoundSafe {
                    at_place: ORCHARD_FARM
                }
            }
        ) && last_seen_record(&h, searcher, subject).is_some()
        {
            break;
        }
    }

    let overdue_tick = overdue_tick.expect("expectation should become overdue before search");
    let generated_tick =
        generated_tick.expect("decision trace should generate SearchForMissing after overdue");
    let selected_tick =
        selected_tick.expect("decision trace should select SearchForMissing in this scenario");
    assert!(
        overdue_tick <= generated_tick,
        "overdue expectation should exist before or at search-candidate generation: overdue={overdue_tick:?} generated={generated_tick:?}"
    );
    assert!(
        generated_tick <= selected_tick,
        "SearchForMissing should be selected no earlier than it is generated: generated={generated_tick:?} selected={selected_tick:?}"
    );

    let selected_planning = planning_trace_at(&h, searcher, selected_tick)
        .expect("selected SearchForMissing tick should have a planning trace");
    let selected_plan = selected_planning
        .selection
        .selected_plan
        .as_ref()
        .expect("selected SearchForMissing tick should retain a selected plan");
    let next_step = selected_plan
        .next_step
        .as_ref()
        .expect("selected SearchForMissing plan should expose a next step");
    assert_eq!(
        selected_planning.selection.selected_plan_source,
        Some(SelectedPlanSource::SearchSelection),
        "expectation-driven search should come from a fresh search result"
    );
    assert!(
        matches!(
            next_step.op_kind,
            worldwake_ai::PlannerOpKind::Travel | worldwake_ai::PlannerOpKind::SearchPlace
        ),
        "selected SearchForMissing plan should be at either the travel leg or the colocated search leg"
    );

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled for expectation scenario");
    let searcher_events = action_sink.events_for(searcher);
    let travel_commit = searcher_events
        .iter()
        .find_map(|event| {
            (event.action_name == "travel"
                && matches!(event.kind, ActionTraceKind::Committed { .. }))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .expect("expectation-driven search should commit travel before searching");
    let search_commit = searcher_events
        .iter()
        .find_map(|event| {
            (event.action_name == "search_place"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && matches!(
                    event.detail.as_ref(),
                    Some(ActionTraceDetail::SearchPlace { subject: traced_subject })
                        if *traced_subject == subject
                ))
            .then_some((event.tick, event.sequence_in_tick))
        })
        .expect("expectation-driven search should commit search_place for the missing subject");
    assert!(
        travel_commit < search_commit,
        "travel should commit before search_place for a remote expectation; travel={travel_commit:?} search={search_commit:?}"
    );

    assert_eq!(
        expectation_state(&h, searcher, expectation_id),
        ExpectationState::Resolved {
            outcome: ExpectationOutcome::FoundSafe {
                at_place: ORCHARD_FARM,
            },
        },
        "search should resolve the overdue expectation as FoundSafe at Orchard Farm"
    );
    let last_seen = last_seen_record(&h, searcher, subject)
        .expect("search should record a last-seen entry for the found subject");
    assert_eq!(last_seen.place, ORCHARD_FARM);
    assert_eq!(last_seen.source, searcher);
    assert_eq!(
        last_seen.provenance,
        worldwake_core::LastSeenProvenance::DirectObservation
    );

    SearchScenarioOutcome {
        world_hash: hash_world(&h.world).unwrap(),
        log_hash: hash_event_log(&h.event_log).unwrap(),
    }
}

// Scenario 120: Overdue Expectation Drives Search
// Systems: ExpectationCheck, AI, Travel, SearchPlace
// GoalKinds: SearchForMissing
// ActionDomains: Travel, Epistemic
// Places: VillageSquare, OrchardFarm
// Principles: 1, 3, 7, 8, 12, 17
// Setup: A searcher at VillageSquare holds one active RoutineReturn expectation
//   for a passive subject expected at OrchardFarm, with deadline_tick=0 and
//   grace_ticks=0 so ExpectationCheck makes it overdue after the opening tick.
//   The searcher has ViolationDispositionProfile and PerceptionProfile; the
//   subject is held at OrchardFarm with ControlSource::None so the scenario
//   isolates expectation-driven search rather than unrelated autonomous motion.
// Proves: ExpectationCheck transitions the record to Overdue, AI generates and
//   selects SearchForMissing, the selected plan includes remote travel and
//   search_place at OrchardFarm, and the final search resolves the expectation
//   to FoundSafe while updating LastSeenMemory locally.
// Chain: ExpectationStore Active -> ExpectationCheck Overdue transition ->
//   SearchForMissing candidate and plan selection -> travel commit ->
//   search_place commit -> ExpectationStore resolution and LastSeenMemory
//   update.
#[test]
fn golden_overdue_expectation_drives_search() {
    let _ = run_overdue_expectation_search(Seed([0x67; 32]));
}

#[test]
fn golden_overdue_expectation_drives_search_replays_deterministically() {
    let first = run_overdue_expectation_search(Seed([0x67; 32]));
    let second = run_overdue_expectation_search(Seed([0x67; 32]));

    assert_eq!(first.world_hash, second.world_hash);
    assert_eq!(first.log_hash, second.log_hash);
}
