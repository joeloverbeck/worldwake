//! Golden tests for expectation-driven search behavior and replay fidelity.

mod golden_harness;

use golden_harness::*;
use std::collections::BTreeMap;
use worldwake_ai::{DecisionOutcome, GoalKey, GoalKind, SelectedPlanSource};
use worldwake_core::{
    AgentData, BeliefConfidencePolicy, BelievedEntityState, BodyPart, ControlSource,
    DeprivationKind, ExpectationBasis, ExpectationId, ExpectationOutcome, ExpectationRecord,
    ExpectationState, HomeostaticNeeds, InstitutionalClaim, LastSeenProvenance, LastSeenRecord,
    MetabolismProfile, PerceptionProfile, PerceptionSource, Seed, Tick, UtilityProfile,
    ViolationDispositionProfile, ViolationKind, Wound, WoundCause, WoundId, WoundList,
    hash_event_log, hash_world, institutional::MissingPersonReportStatus,
};
use worldwake_sim::{ActionTraceDetail, ActionTraceKind};

fn default_perception_profile() -> PerceptionProfile {
    PerceptionProfile {
        entity_memory_capacity: 32,
        entity_claim_capacity: 32,
        memory_retention_ticks: 240,
        infrastructure_retention_ticks: 2400,
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
            record_data
                .active_entries()
                .into_iter()
                .find_map(|entry| match entry.claim {
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
            if planning.selection.selected_goal() == Some(report_goal)
                && report_selected_tick.is_none()
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

fn escort_utility_profile() -> UtilityProfile {
    UtilityProfile {
        care_weight: pm(900),
        social_weight: pm(0),
        ..UtilityProfile::default()
    }
}

struct EscortScenarioOutcome {
    world_hash: worldwake_core::StateHash,
    log_hash: worldwake_core::StateHash,
}

fn run_escort_to_safety(seed: Seed) -> EscortScenarioOutcome {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    // Escorter at Village Square with high care weight but NO PerceptionProfile.
    // Instead, wound knowledge is seeded via Report source so that TreatWounds
    // (which requires DirectObservation) is not emitted while EscortToSafety
    // (which only needs has_wounds from beliefs) fires.
    let escorter = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Escorter",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        escort_utility_profile(),
    );

    // Wounded entity at Village Square (co-located with escorter).
    let wounded = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Wounded",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, wounded, ControlSource::None, 0);

    // Give the wounded entity a stable wound (authoritative state).
    let wound = Wound {
        id: WoundId(1),
        body_part: BodyPart::Torso,
        cause: WoundCause::Deprivation(DeprivationKind::Starvation),
        severity: pm(400),
        inflicted_at: Tick(0),
        bleed_rate_per_tick: pm(0),
    };
    {
        let mut txn = new_txn(&mut h.world, 0);
        txn.set_component_wound_list(
            wounded,
            WoundList {
                wounds: vec![wound.clone()],
            },
        )
        .expect("golden escort scenario should keep wound list writable");
        commit_txn(txn, &mut h.event_log);
    }

    // Seed the escorter's belief about the wounded entity via Report source.
    // Report-based beliefs carry wound data but are NOT DirectObservation,
    // so TreatWounds candidate generation skips this entity while
    // EscortToSafety candidate generation (which checks has_wounds) succeeds.
    seed_belief(
        &mut h.world,
        &mut h.event_log,
        escorter,
        wounded,
        BelievedEntityState {
            believed_kind: None,
            last_known_place: Some(VILLAGE_SQUARE),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: vec![wound],
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            observed_tick: Tick(0),
            source: PerceptionSource::Report {
                from: wounded,
                chain_len: 1,
            },
        },
    );

    let mut escort_generated_tick = None;
    let mut escort_selected_tick = None;

    for tick in 0..80_u64 {
        h.step_once();

        if let Some(planning) = planning_trace_at(&h, escorter, Tick(tick)) {
            let generated_escort = planning.candidates.generated.iter().any(|goal| {
                matches!(
                    goal.goal_key.kind,
                    GoalKind::EscortToSafety {
                        subject: candidate_subject,
                        ..
                    } if candidate_subject == wounded
                )
            });
            if generated_escort && escort_generated_tick.is_none() {
                escort_generated_tick = Some(Tick(tick));
            }

            let selected_escort = planning.selection.selected_goal().is_some_and(|goal| {
                matches!(
                    goal.kind,
                    GoalKind::EscortToSafety {
                        subject: candidate_subject,
                        ..
                    } if candidate_subject == wounded
                )
            });
            if selected_escort && escort_selected_tick.is_none() {
                escort_selected_tick = Some(Tick(tick));
            }
        }

        // Break once the escort action commits (wounded entity moved).
        let action_sink = h.action_trace_sink();
        if let Some(sink) = &action_sink {
            let has_escort_commit = sink.events_for(escorter).iter().any(|event| {
                event.action_name == "escort_to_safety"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            });
            if has_escort_commit {
                break;
            }
        }
    }

    let escort_generated_tick = escort_generated_tick
        .expect("decision trace should generate EscortToSafety for wounded co-located entity");
    let escort_selected_tick =
        escort_selected_tick.expect("decision trace should select EscortToSafety in this scenario");
    assert!(
        escort_generated_tick <= escort_selected_tick,
        "EscortToSafety should be selected no earlier than generated: generated={escort_generated_tick:?} selected={escort_selected_tick:?}"
    );

    // Verify action traces: escort_to_safety should commit.
    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled for escort scenario");
    let escorter_events = action_sink.events_for(escorter);
    let escort_commit = escorter_events
        .iter()
        .find(|event| {
            event.action_name == "escort_to_safety"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
                && matches!(
                    event.detail.as_ref(),
                    Some(ActionTraceDetail::EscortToSafety {
                        subject: traced_subject,
                        ..
                    }) if *traced_subject == wounded
                )
        })
        .expect("escort_to_safety should commit with the wounded entity as subject");

    // Verify the wounded entity has moved to the destination.
    let escort_destination = match escort_commit.detail.as_ref().unwrap() {
        ActionTraceDetail::EscortToSafety { destination, .. } => *destination,
        _ => panic!("expected EscortToSafety detail"),
    };
    let wounded_place = h
        .world
        .effective_place(wounded)
        .expect("wounded entity should have a location after escort");
    assert_eq!(
        wounded_place, escort_destination,
        "wounded entity should be at the escort destination after commit"
    );

    // Verify escorter is also at the destination (co-location binding).
    let escorter_place = h
        .world
        .effective_place(escorter)
        .expect("escorter should have a location after escort");
    assert_eq!(
        escorter_place, escort_destination,
        "escorter should also be at the destination after commit (co-location binding)"
    );

    EscortScenarioOutcome {
        world_hash: hash_world(&h.world).unwrap(),
        log_hash: hash_event_log(&h.event_log).unwrap(),
    }
}

// Scenario 122: Escort Wounded Entity to Safety
// Systems: AI, Travel, EscortToSafety
// GoalKinds: EscortToSafety
// ActionDomains: Care, Travel
// Places: VillageSquare + adjacent destination
// Principles: 1, 3, 8, 10, 20
// Setup: An escorter at VillageSquare with high care_weight knows about a
//   wounded co-located entity via Report-sourced beliefs (not DirectObservation,
//   so TreatWounds candidate is suppressed and EscortToSafety is isolated).
//   The topology provides at least one adjacent reachable destination. The
//   wounded entity has ControlSource::None to isolate escort behavior.
// Proves: AI candidate generation emits EscortToSafety when the agent believes
//   a co-located entity is wounded, the planner selects it and produces a valid
//   plan, the escort_to_safety action commits, and both actor and charge arrive
//   at the destination (co-location binding).
// Chain: Wounded entity observation -> EscortToSafety candidate generation ->
//   plan selection -> escort_to_safety action -> co-located travel -> commit ->
//   both entities at destination.
#[test]
fn golden_escort_to_safety_after_finding_wounded() {
    let _ = run_escort_to_safety(Seed([0x69; 32]));
}

#[test]
fn golden_escort_to_safety_after_finding_wounded_replays_deterministically() {
    let first = run_escort_to_safety(Seed([0x69; 32]));
    let second = run_escort_to_safety(Seed([0x69; 32]));

    assert_eq!(first.world_hash, second.world_hash);
    assert_eq!(first.log_hash, second.log_hash);
}

// ---------------------------------------------------------------------------
// Helper: AskAboutPerson utility profile — care-weight only, no social
// ---------------------------------------------------------------------------

fn ask_search_utility_profile() -> UtilityProfile {
    UtilityProfile {
        care_weight: pm(200),
        social_weight: pm(0),
        ..UtilityProfile::default()
    }
}

struct AskAboutPersonScenarioOutcome {
    world_hash: worldwake_core::StateHash,
    log_hash: worldwake_core::StateHash,
}

fn run_ask_about_person_during_search(seed: Seed) -> AskAboutPersonScenarioOutcome {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    // Searcher at VillageSquare — has overdue expectation, no initial last-seen.
    let searcher = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Searcher",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        ask_search_utility_profile(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        searcher,
        default_perception_profile(),
    );
    set_violation_profile(&mut h, searcher, violation_profile(), 0);

    // Witness at VillageSquare — has LastSeenMemory for subject at OrchardFarm.
    // ControlSource::None so it doesn't act autonomously.
    let witness = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Witness",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, witness, ControlSource::None, 0);

    // Subject at OrchardFarm — passive, doesn't move.
    let subject = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Subject",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, subject, ControlSource::None, 0);

    // Seed the witness's LastSeenMemory: subject was at OrchardFarm.
    seed_last_seen(&mut h, witness, subject, ORCHARD_FARM, 0, 0);

    // Seed overdue expectation on the searcher: subject expected at OrchardFarm.
    // Using OrchardFarm (not VillageSquare) ensures SearchPlace is only valid
    // at OrchardFarm — the searcher cannot search locally first. This isolates
    // the AskAboutPerson progress-barrier path.
    let expectation_id = seed_expectation(&mut h, searcher, subject, ORCHARD_FARM, 0);

    let mut ask_commit = None;
    let mut search_commit = None;

    for _tick in 0..120_u64 {
        h.step_once();

        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled for ask-about-person scenario");
        let searcher_events = action_sink.events_for(searcher);

        // Detect ask_about_person commit.
        if ask_commit.is_none() {
            ask_commit = searcher_events.iter().find_map(|event| {
                (event.action_name == "ask_about_person"
                    && matches!(event.kind, ActionTraceKind::Committed { .. }))
                .then_some((event.tick, event.sequence_in_tick))
            });

            if ask_commit.is_some() {
                // Immediately after ask commit, verify last-seen transfer.
                let last_seen = last_seen_record(&h, searcher, subject);
                assert!(
                    last_seen.is_some(),
                    "ask_about_person commit should transfer LastSeenRecord to the searcher"
                );
                let record = last_seen.unwrap();
                assert_eq!(
                    record.place, ORCHARD_FARM,
                    "transferred last-seen should point to OrchardFarm"
                );
                assert_eq!(
                    record.provenance,
                    worldwake_core::LastSeenProvenance::Hearsay {
                        original_observer: witness,
                        chain_depth: 1,
                    },
                    "transferred last-seen should have hearsay provenance from the witness"
                );
            }
        }

        // Detect search_place commit for the subject.
        if search_commit.is_none() {
            search_commit = searcher_events.iter().find_map(|event| {
                (event.action_name == "search_place"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
                    && matches!(
                        event.detail.as_ref(),
                        Some(ActionTraceDetail::SearchPlace { subject: traced_subject })
                            if *traced_subject == subject
                    ))
                .then_some((event.tick, event.sequence_in_tick))
            });
        }

        // Break once both the ask and the final search have committed and
        // the expectation is resolved.
        if ask_commit.is_some()
            && search_commit.is_some()
            && matches!(
                expectation_state(&h, searcher, expectation_id),
                ExpectationState::Resolved { .. }
            )
        {
            break;
        }
    }

    let ask_commit =
        ask_commit.expect("searcher should commit ask_about_person for the missing subject");
    let search_commit = search_commit
        .expect("searcher should commit search_place for the missing subject after asking");
    assert!(
        ask_commit < search_commit,
        "ask_about_person should commit before the successful search_place: ask={ask_commit:?} search={search_commit:?}"
    );

    assert_eq!(
        expectation_state(&h, searcher, expectation_id),
        ExpectationState::Resolved {
            outcome: ExpectationOutcome::FoundSafe {
                at_place: ORCHARD_FARM,
            },
        },
        "search at the witness-revealed location should resolve the overdue expectation as FoundSafe"
    );

    AskAboutPersonScenarioOutcome {
        world_hash: hash_world(&h.world).unwrap(),
        log_hash: hash_event_log(&h.event_log).unwrap(),
    }
}

// ---------------------------------------------------------------------------
// Scenario 124: AskAboutPerson as Progress Barrier Within SearchForMissing
// ---------------------------------------------------------------------------
//
// Systems: ExpectationCheck, AI, AskAboutPerson, Travel, SearchPlace
// GoalKinds: SearchForMissing
// ActionDomains: Epistemic, Travel
// Places: VillageSquare, OrchardFarm
// Principles: 1, 7, 14, 15, 17, 20
//
// Setup: A searcher at VillageSquare holds one active RoutineReturn expectation
//   for a passive subject expected at OrchardFarm, with deadline_tick=0 and
//   grace_ticks=0 so ExpectationCheck makes it overdue quickly. The searcher has
//   no initial LastSeenMemory for the subject, so SearchPlace at OrchardFarm is
//   unreachable without learning the location first. A witness at VillageSquare
//   (ControlSource::None) has LastSeenMemory recording the subject at OrchardFarm.
//   The subject is at OrchardFarm with ControlSource::None.
//
// Proves: AskAboutPerson functions as a progress barrier for SearchForMissing.
//   The planner selects AskAboutPerson as a terminal step when the searcher is
//   co-located with a witness who has relevant last-seen info. After the ask
//   commits, the searcher's LastSeenMemory is updated with hearsay provenance
//   (P15: knowledge travels physically through testimony). Replanning with
//   updated beliefs directs search to OrchardFarm (P14: planner uses accessible
//   belief state, not witness knowledge). The full chain demonstrates emergent
//   multi-cycle information gathering (P1) driven by violated expectation (P17).
//
// Chain: ExpectationStore Active -> ExpectationCheck Overdue transition ->
//   SearchForMissing candidate -> AskAboutPerson progress barrier terminal ->
//   ask_about_person commit -> LastSeenMemory hearsay transfer ->
//   SearchForMissing replan with updated last_seen -> travel to OrchardFarm ->
//   search_place commit -> ExpectationStore resolution FoundSafe.
#[test]
fn golden_ask_about_person_during_search() {
    let _ = run_ask_about_person_during_search(Seed([0x6A; 32]));
}

#[test]
fn golden_ask_about_person_during_search_replays_deterministically() {
    let first = run_ask_about_person_during_search(Seed([0x6A; 32]));
    let second = run_ask_about_person_during_search(Seed([0x6A; 32]));

    assert_eq!(first.world_hash, second.world_hash);
    assert_eq!(first.log_hash, second.log_hash);
}

// ---------------------------------------------------------------------------
// Helper: ReportFound utility profile — social weight for reporting
// ---------------------------------------------------------------------------

struct ReportFoundScenarioOutcome {
    world_hash: worldwake_core::StateHash,
    log_hash: worldwake_core::StateHash,
}

fn run_report_found_after_search(seed: Seed) -> ReportFoundScenarioOutcome {
    let mut h = GoldenHarness::new(seed);
    h.driver.enable_tracing();
    h.enable_action_tracing();

    // Searcher at VillageSquare — has overdue expectation for subject at
    // OrchardFarm. Social weight enables ReportFound candidacy.
    let searcher = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Searcher",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        search_only_utility_profile(),
    );
    set_agent_perception_profile(
        &mut h.world,
        &mut h.event_log,
        searcher,
        default_perception_profile(),
    );
    set_violation_profile(&mut h, searcher, violation_profile(), 0);

    // OfficeRegister at OrchardFarm. ReportFound now surfaces planner-visible
    // OfficeRegister targets only; direct agent notification remains a lower-
    // level action path because another agent's overdue expectation is not
    // available on the searcher's lawful belief surface.
    let office_register = seed_office_register(&mut h.world, &mut h.event_log, ORCHARD_FARM);

    // Subject at OrchardFarm — passive, doesn't move.
    let subject = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Subject",
        ORCHARD_FARM,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    set_control_source(&mut h, subject, ControlSource::None, 0);

    // Seed the searcher's belief about the OfficeRegister at OrchardFarm.
    // Record entities are not projected through standard perception, so the
    // planner needs an explicit belief to discover the report target.
    seed_belief(
        &mut h.world,
        &mut h.event_log,
        searcher,
        office_register,
        BelievedEntityState {
            believed_kind: None,
            last_known_place: Some(ORCHARD_FARM),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            observed_tick: Tick(0),
            source: PerceptionSource::DirectObservation,
        },
    );

    // Seed overdue expectation: subject expected at OrchardFarm.
    let expectation_id = seed_expectation(&mut h, searcher, subject, ORCHARD_FARM, 0);

    let mut search_commit = None;
    let mut report_commit = None;
    let mut social_enabled = false;

    for _tick in 0..120_u64 {
        h.step_once();

        // Scope the immutable borrow on h so we can mutate h.world afterward.
        let found_search = {
            let action_sink = h
                .action_trace_sink()
                .expect("action tracing should be enabled for report-found scenario");
            let searcher_events = action_sink.events_for(searcher);

            if search_commit.is_none() {
                search_commit = searcher_events.iter().find_map(|event| {
                    (event.action_name == "search_place"
                        && matches!(event.kind, ActionTraceKind::Committed { .. })
                        && matches!(
                            event.detail.as_ref(),
                            Some(ActionTraceDetail::SearchPlace { subject: traced_subject })
                                if *traced_subject == subject
                        ))
                    .then_some((event.tick, event.sequence_in_tick))
                });
            }

            if report_commit.is_none() {
                report_commit = searcher_events.iter().find_map(|event| {
                    (event.action_name == "report_found"
                        && matches!(event.kind, ActionTraceKind::Committed { .. }))
                    .then_some((event.tick, event.sequence_in_tick))
                });
            }

            search_commit.is_some() && !social_enabled
        };

        // After search commits, enable social_weight so ReportFound
        // candidates can be generated.
        if found_search {
            social_enabled = true;
            let tick_val = search_commit.unwrap().0.0;
            let mut txn = new_txn(&mut h.world, tick_val);
            txn.set_component_utility_profile(
                searcher,
                UtilityProfile {
                    care_weight: pm(200),
                    social_weight: pm(900),
                    ..UtilityProfile::default()
                },
            )
            .expect("golden report-found scenario should keep utility profiles writable");
            commit_txn(txn, &mut h.event_log);
        }

        if report_commit.is_some() {
            let status = missing_person_status_claim(&h, office_register, subject);
            assert!(
                matches!(
                    status,
                    Some(InstitutionalClaim::MissingPersonStatus {
                        subject: claim_subject,
                        reporter: claim_reporter,
                        status: MissingPersonReportStatus::FoundSafe {
                            at_place: ORCHARD_FARM,
                        },
                        ..
                    }) if claim_subject == subject && claim_reporter == searcher
                ),
                "report_found should write FoundSafe status to the OfficeRegister; got {status:?}"
            );
            break;
        }
    }

    let search_commit =
        search_commit.expect("searcher should commit search_place for the missing subject");
    let report_commit =
        report_commit.expect("searcher should commit report_found after finding the subject");
    assert!(
        search_commit < report_commit,
        "search_place should commit before report_found: search={search_commit:?} report={report_commit:?}"
    );

    assert_eq!(
        expectation_state(&h, searcher, expectation_id),
        ExpectationState::Resolved {
            outcome: ExpectationOutcome::FoundSafe {
                at_place: ORCHARD_FARM,
            },
        },
        "expectation should be resolved as FoundSafe at OrchardFarm"
    );

    ReportFoundScenarioOutcome {
        world_hash: hash_world(&h.world).unwrap(),
        log_hash: hash_event_log(&h.event_log).unwrap(),
    }
}

// ---------------------------------------------------------------------------
// Scenario 125: ReportFound After Search Resolution
// ---------------------------------------------------------------------------
//
// Systems: ExpectationCheck, AI, Travel, SearchPlace, ReportFound, OfficeRegister
// GoalKinds: SearchForMissing, ReportFound
// ActionDomains: Travel, Epistemic, Social
// Places: VillageSquare, OrchardFarm
// Principles: 1, 3, 7, 10, 17, 20
//
// Setup: A searcher at VillageSquare holds one active RoutineReturn expectation
//   for a passive subject expected at OrchardFarm, with deadline_tick=0 and
//   grace_ticks=0. The searcher has LastSeenMemory pointing to OrchardFarm.
//   An OfficeRegister exists at OrchardFarm. The subject is at OrchardFarm
//   with ControlSource::None.
//
// Proves: After SearchForMissing resolves the expectation via search_place at
//   OrchardFarm (FoundSafe), the resolved state drives a new GoalKind::ReportFound
//   candidate (P10: aftermath creates future hooks). The planner selects
//   ReportFound as a progress-barrier terminal at OrchardFarm and commits
//   report_found, writing MissingPersonStatus::FoundSafe to the local
//   OfficeRegister (P17: violated expectation drives institutional response).
//   The full chain demonstrates emergent multi-goal sequencing through state
//   transitions (P1), not scripted chains.
//
// Chain: ExpectationStore Active -> ExpectationCheck Overdue -> SearchForMissing
//   candidate -> travel to OrchardFarm -> search_place commit -> ExpectationStore
//   Resolved(FoundSafe) -> ReportFound candidate -> report_found commit ->
//   OfficeRegister MissingPersonStatus::FoundSafe.
#[test]
fn golden_report_found_after_search() {
    let _ = run_report_found_after_search(Seed([0x6B; 32]));
}

#[test]
fn golden_report_found_after_search_replays_deterministically() {
    let first = run_report_found_after_search(Seed([0x6B; 32]));
    let second = run_report_found_after_search(Seed([0x6B; 32]));

    assert_eq!(first.world_hash, second.world_hash);
    assert_eq!(first.log_hash, second.log_hash);
}
