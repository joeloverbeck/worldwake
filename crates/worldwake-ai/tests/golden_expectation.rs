//! Golden tests for expectation-driven search behavior and replay fidelity.

mod golden_harness;

use golden_harness::*;
use worldwake_ai::{DecisionOutcome, GoalKind, GoalKey, SelectedPlanSource};
use worldwake_core::{
    AgentData, BeliefConfidencePolicy, ControlSource, ExpectationBasis, ExpectationId,
    ExpectationOutcome, ExpectationRecord, ExpectationState, HomeostaticNeeds,
    InstitutionalClaim, LastSeenProvenance, LastSeenRecord, MetabolismProfile,
    PerceptionProfile, Seed, Tick, UtilityProfile, ViolationDispositionProfile,
    ViolationKind, hash_event_log, hash_world, institutional::MissingPersonReportStatus,
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

fn report_first_utility_profile() -> UtilityProfile {
    UtilityProfile {
        social_weight: pm(900),
        care_weight: pm(1),
        ..UtilityProfile::default()
    }
}

fn search_only_utility_profile() -> UtilityProfile {
    UtilityProfile {
        social_weight: pm(0),
        care_weight: pm(200),
        ..UtilityProfile::default()
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

fn seed_last_seen(
    h: &mut GoldenHarness,
    actor: worldwake_core::EntityId,
    subject: worldwake_core::EntityId,
    place: worldwake_core::EntityId,
    observed_tick: u64,
    tick: u64,
) {
    let mut txn = new_txn(&mut h.world, tick);
    let mut memory = txn
        .get_component_last_seen_memory(actor)
        .cloned()
        .unwrap_or_default();
    memory.records.insert(
        subject,
        LastSeenRecord {
            subject,
            place,
            observed_tick: Tick(observed_tick),
            source: actor,
            provenance: LastSeenProvenance::DirectObservation,
        },
    );
    txn.set_component_last_seen_memory(actor, memory)
        .expect("golden expectation scenario should keep last-seen memory writable");
    commit_txn(txn, &mut h.event_log);
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

fn missing_person_status_claim(
    h: &GoldenHarness,
    office_register: worldwake_core::EntityId,
    subject: worldwake_core::EntityId,
) -> Option<InstitutionalClaim> {
    h.world
        .get_component_record_data(office_register)
        .and_then(|record_data| {
            record_data.active_entries().into_iter().find_map(|entry| match entry.claim {
                claim @ InstitutionalClaim::MissingPersonStatus {
                    subject: claim_subject,
                    ..
                } if claim_subject == subject => Some(claim),
                _ => None,
            })
        })
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
        search_only_utility_profile(),
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

struct ReportMissingScenarioOutcome {
    world_hash: worldwake_core::StateHash,
    log_hash: worldwake_core::StateHash,
}

fn run_report_missing_chain(seed: Seed) -> ReportMissingScenarioOutcome {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    let reporter = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Reporter",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        report_first_utility_profile(),
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
    let office_register = seed_office_register(&mut h.world, &mut h.event_log, VILLAGE_SQUARE);
    set_control_source(&mut h, subject, ControlSource::None, 0);
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        reporter,
        default_perception_profile(),
    );
    set_violation_profile(&mut h, reporter, violation_profile(), 0);
    let expectation_id = seed_expectation(&mut h, reporter, subject, VILLAGE_SQUARE, 0);
    seed_last_seen(&mut h, reporter, subject, ORCHARD_FARM, 0, 0);

    let report_goal = GoalKey::from(GoalKind::ReportMissing {
        subject,
        to_office: None,
        expectation_id: Some(expectation_id),
    });
    let search_goal = GoalKey::from(GoalKind::SearchForMissing {
        subject,
        last_seen: Some(ORCHARD_FARM),
    });
    let missing_violation = ViolationKind::EntityMissing {
        entity: subject,
        expected_place: VILLAGE_SQUARE,
    };

    let mut overdue_tick = None;
    let mut dual_generated_tick = None;
    let mut report_selected_tick = None;
    let mut post_report_search_only_tick = None;
    let mut search_selected_tick = None;
    let mut report_commit = None;
    let mut search_commit = None;
    let mut opening_selected_goal = None;
    let mut opening_ranked_goals = None;
    let mut opening_report_attempt = None;

    for tick in 0..120_u64 {
        h.step_once();

        let state = expectation_state(&h, reporter, expectation_id);
        if state == ExpectationState::Overdue && overdue_tick.is_none() {
            overdue_tick = Some(Tick(tick));
        }

        if let Some(planning) = planning_trace_at(&h, reporter, Tick(tick)) {
            let generated_report = planning.candidates.generated_contains_goal(report_goal);
            let generated_search = planning.candidates.generated_contains_goal(search_goal);
            if generated_report && generated_search && dual_generated_tick.is_none() {
                dual_generated_tick = Some(Tick(tick));
                opening_selected_goal = planning.selection.selected_goal();
                opening_ranked_goals = Some(
                    planning
                        .candidates
                        .ranked
                        .iter()
                        .map(|summary| summary.opportunity.goal_key)
                        .collect::<Vec<_>>(),
                );
                opening_report_attempt = planning
                    .planning
                    .attempts
                    .iter()
                    .find(|attempt| attempt.goal == report_goal)
                    .map(|attempt| format!("{attempt:?}"));
            }
            if planning.selection.selected_goal() == Some(report_goal) && report_selected_tick.is_none()
            {
                report_selected_tick = Some(Tick(tick));
            }
            if let Some((report_tick, _)) = report_commit {
                if Tick(tick) > report_tick
                    && generated_search
                    && !generated_report
                    && post_report_search_only_tick.is_none()
                {
                    post_report_search_only_tick = Some(Tick(tick));
                }
                if Tick(tick) > report_tick
                    && planning.selection.selected_goal() == Some(search_goal)
                    && search_selected_tick.is_none()
                {
                    search_selected_tick = Some(Tick(tick));
                }
            }
        }

        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled for expectation scenario");
        let reporter_events = action_sink.events_for(reporter);
        if report_commit.is_none() {
            report_commit = reporter_events.iter().find_map(|event| {
                (event.action_name == "report_missing"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
                    && matches!(
                        event.detail.as_ref(),
                        Some(ActionTraceDetail::ReportMissing {
                            expectation_id: traced_id,
                        }) if *traced_id == expectation_id
                    ))
                .then_some((event.tick, event.sequence_in_tick))
            });
            if report_commit.is_some() {
                let violation_memory = h
                    .world
                    .get_component_violation_memory(reporter)
                    .expect("report_missing should create ViolationMemory for the reporter");
                assert!(
                    violation_memory
                        .violations
                        .iter()
                        .any(|record| record.kind == missing_violation),
                    "report_missing should record EntityMissing for the overdue expectation"
                );
                assert_eq!(
                    missing_person_status_claim(&h, office_register, subject),
                    Some(InstitutionalClaim::MissingPersonStatus {
                        subject,
                        reporter,
                        status: MissingPersonReportStatus::Missing {
                            expected_place: VILLAGE_SQUARE,
                        },
                        effective_tick: Tick(3),
                    }),
                    "report_missing should append MissingPersonStatus to the OfficeRegister at the expected place"
                );
            }
        }
        if search_commit.is_none() {
            search_commit = reporter_events.iter().find_map(|event| {
                (event.action_name == "search_place"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
                    && matches!(
                        event.detail.as_ref(),
                        Some(ActionTraceDetail::SearchPlace {
                            subject: traced_subject,
                        }) if *traced_subject == subject
                    ))
                .then_some((event.tick, event.sequence_in_tick))
            });
        }

        if report_commit.is_some()
            && post_report_search_only_tick.is_some()
            && search_selected_tick.is_some()
            && search_commit.is_some()
        {
            break;
        }
    }

    let overdue_tick = overdue_tick.expect("expectation should become overdue before reporting");
    let dual_generated_tick =
        dual_generated_tick.expect("opening planning should generate both report and search");
    let report_selected_tick = report_selected_tick.unwrap_or_else(|| {
        panic!(
            "opening planning should select ReportMissing first; selected={opening_selected_goal:?} ranked={opening_ranked_goals:?} report_attempt={opening_report_attempt:?}"
        )
    });
    let report_commit =
        report_commit.expect("report_missing should commit before the scenario shifts to search");
    let post_report_search_only_tick = post_report_search_only_tick
        .expect("post-report planning should suppress ReportMissing but keep SearchForMissing");
    let search_selected_tick =
        search_selected_tick.expect("post-report planning should select SearchForMissing");
    let search_commit = search_commit.expect("search_place should commit for the missing subject");

    assert!(
        overdue_tick <= dual_generated_tick,
        "expectation should be overdue before dual candidate generation: overdue={overdue_tick:?} generated={dual_generated_tick:?}"
    );
    assert!(
        dual_generated_tick <= report_selected_tick,
        "ReportMissing should not be selected before both candidates are generated: generated={dual_generated_tick:?} selected={report_selected_tick:?}"
    );
    assert!(
        report_selected_tick <= report_commit.0,
        "report_missing should be selected before it commits: selected={report_selected_tick:?} commit={report_commit:?}"
    );
    assert!(
        report_commit < search_commit,
        "search_place should commit only after report_missing commits: report={report_commit:?} search={search_commit:?}"
    );
    assert!(
        report_commit.0 < post_report_search_only_tick,
        "suppression should appear only after report_missing commits: report={report_commit:?} post_report={post_report_search_only_tick:?}"
    );
    assert!(
        post_report_search_only_tick <= search_selected_tick,
        "SearchForMissing should be selected no earlier than the post-report suppression tick: post_report={post_report_search_only_tick:?} search_selected={search_selected_tick:?}"
    );

    ReportMissingScenarioOutcome {
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

// Scenario 121: Report Missing Creates Institutional Record Then Searches
// Systems: ExpectationCheck, AI, ReportMissing, ViolationMemory, OfficeRegister, Travel, SearchPlace
// GoalKinds: ReportMissing, SearchForMissing
// ActionDomains: Social, Travel, Epistemic
// Places: VillageSquare, OrchardFarm
// Principles: 1, 3, 7, 8, 12, 17
// Setup: A reporter at VillageSquare holds one active RoutineReturn expectation
//   for a passive subject expected back at VillageSquare, while the reporter's
//   LastSeenMemory still points to OrchardFarm. A unique OfficeRegister exists
//   at VillageSquare, and the reporter uses a utility profile that makes the
//   opening overdue response prefer reporting over immediate searching.
// Proves: Once ExpectationCheck makes the record overdue, AI generates both
//   ReportMissing and SearchForMissing, selects report_missing first, records
//   EntityMissing plus MissingPersonStatus at the expected-place OfficeRegister,
//   then suppresses duplicate ReportMissing while continuing into the search
//   path and eventually finding the subject at OrchardFarm.
// Chain: ExpectationStore Active -> ExpectationCheck Overdue transition ->
//   dual ReportMissing/SearchForMissing generation -> report_missing commit ->
//   ViolationMemory + OfficeRegister MissingPersonStatus update ->
//   ReportMissing suppression with SearchForMissing retained ->
//   travel/search_place -> ExpectationStore resolution and LastSeenMemory update.
#[test]
fn golden_report_missing_creates_violation_and_institutional_record() {
    let _ = run_report_missing_chain(Seed([0x68; 32]));
}

#[test]
fn golden_report_missing_creates_violation_and_institutional_record_replays_deterministically() {
    let first = run_report_missing_chain(Seed([0x68; 32]));
    let second = run_report_missing_chain(Seed([0x68; 32]));

    assert_eq!(first.world_hash, second.world_hash);
    assert_eq!(first.log_hash, second.log_hash);
}
