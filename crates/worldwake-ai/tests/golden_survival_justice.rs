//! Golden tests for the survival justice roadmap row.

mod golden_harness;

use std::collections::BTreeSet;
use std::path::PathBuf;

use golden_harness::*;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    CommodityKind, DriveThresholds, EntityId, InstitutionalClaim, PunishmentKind, RecordKind,
    SocialObservationDetail, Tick, ViolationKind, institutional::MissingPersonReportStatus,
};
use worldwake_sim::{ActionTraceKind, RequestResolutionOutcome, RequestResolutionRejectionReason};

const SURVIVAL_TICKS: u32 = 1440;

#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentSurvivalObservation {
    alive: bool,
    critical_thresholds: DriveThresholds,
    critical_need_runs: SurvivalNeedRunTracker,
    committed_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct JusticeObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    merchant: AgentSurvivalObservation,
    stuck_idle_windows: Vec<StuckIdleWindow>,
    stage_tick: Tick,
    theft_tick: Tick,
    office_holder_tick: Tick,
    investigate_tick: Tick,
    accuse_tick: Option<Tick>,
    fine_tick: Option<Tick>,
    accusation_recorded: bool,
    fine_verdict_recorded: bool,
    search_place_tick: Option<Tick>,
    report_found_tick: Option<Tick>,
    searcher_committed_actions: BTreeSet<String>,
    searcher_start_failed_actions: BTreeSet<String>,
    found_status_recorded: bool,
    searcher_expectation_found_safe: bool,
    searcher_exact_identity_rejections: u32,
    first_ranked_accuse_tick: Option<Tick>,
    first_selected_accuse_tick: Option<Tick>,
    merchant_id: EntityId,
    thief_id: EntityId,
    final_violation_suspicions: Vec<String>,
    final_social_suspicions: Vec<String>,
    first_violation_suspected_theft_tick: Option<Tick>,
    first_social_suspected_theft_tick: Option<Tick>,
    final_thief_apple_quantity: worldwake_core::Quantity,
    final_thief_place: Option<EntityId>,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-justice.ron")
}

fn load_survival_justice_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival justice scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival justice scenario should spawn");
    let mut harness = GoldenHarness::from_simulation_state(&spawned.state);
    let agents = harness
        .world
        .query_name_and_agent_data()
        .map(|(agent, _, _)| agent)
        .collect::<Vec<_>>();
    for agent in agents {
        let place = harness
            .world
            .effective_place(agent)
            .expect("scenario agents should have a starting place");
        let local_entities = harness
            .world
            .entities()
            .filter(|entity| harness.world.effective_place(*entity) == Some(place))
            .filter(|entity| *entity != agent)
            .collect::<Vec<_>>();
        seed_actor_beliefs(
            &mut harness.world,
            &mut harness.event_log,
            agent,
            &local_entities,
            Tick(0),
            worldwake_core::PerceptionSource::DirectObservation,
        );
    }
    harness.driver.enable_tracing();
    harness.enable_action_tracing();
    harness.enable_perception_tracing();
    harness.enable_request_resolution_tracing();
    (harness, def)
}

fn find_named_agent(h: &GoldenHarness, expected_name: &str) -> EntityId {
    h.world
        .query_name_and_agent_data()
        .find_map(|(entity, name, _)| (name.0 == expected_name).then_some(entity))
        .unwrap_or_else(|| panic!("scenario should include {expected_name}"))
}

fn find_named_entity(h: &GoldenHarness, expected_name: &str) -> EntityId {
    h.world
        .query_name()
        .find_map(|(entity, name)| (name.0 == expected_name).then_some(entity))
        .unwrap_or_else(|| panic!("scenario should include {expected_name}"))
}

fn contract_run_limit_overrides(
    limits: Option<&worldwake_cli::scenario::types::SurvivalCriticalRunLimitsDef>,
) -> SurvivalCriticalRunLimitOverrides {
    let Some(limits) = limits else {
        return SurvivalCriticalRunLimitOverrides::default();
    };

    SurvivalCriticalRunLimitOverrides {
        hunger: limits.hunger,
        thirst: limits.thirst,
        fatigue: limits.fatigue,
        bladder: limits.bladder,
        dirtiness: limits.dirtiness,
    }
}

fn observe_idle_window(
    idle_state: &mut (Option<u32>, u16, u32),
    had_action: bool,
    needs: &worldwake_core::HomeostaticNeeds,
    tick_num: u32,
    contract: &worldwake_cli::scenario::types::SurvivalHealthContractDef,
    windows: &mut Vec<StuckIdleWindow>,
    agent_name: &str,
) {
    if had_action {
        if let Some(start_tick) = idle_state.0.take()
            && idle_state.2 >= contract.max_idle_window_ticks_with_elevated_need
            && idle_state.1 > contract.elevated_need_floor.value()
        {
            windows.push(StuckIdleWindow {
                agent_name: agent_name.to_string(),
                start_tick,
                end_tick: tick_num.saturating_sub(1),
                max_need_at_start: idle_state.1,
            });
        }
        idle_state.2 = 0;
    } else {
        if idle_state.0.is_none() {
            idle_state.0 = Some(tick_num);
            idle_state.1 = max_need_value(needs);
        }
        idle_state.2 += 1;
    }
}

fn run_survival_justice() -> JusticeObservation {
    let (mut h, def) = load_survival_justice_harness();
    let contract =
        expect_survival_health_contract(def.survival_health_contract.as_ref(), "survival justice")
            .clone();
    let merchant = find_named_agent(&h, "Merchant Sera");
    let thief = find_named_agent(&h, "Thief Rana");
    let searcher = find_named_agent(&h, "Searcher Ivo");
    let missing = find_named_agent(&h, "Missing Pru");
    let office = find_named_entity(&h, "Market Warden");
    let merchant_thresholds = h
        .world
        .get_component_drive_thresholds(merchant)
        .copied()
        .expect("merchant should have drive thresholds");
    let mut merchant_need_runs = SurvivalNeedRunTracker::default();
    let mut merchant_idle_state: (Option<u32>, u16, u32) = (None, 0, 0);
    let mut stuck_idle_windows = Vec::new();

    let mut stage_tick = None;
    let mut theft_tick = None;
    let mut office_holder_tick = None;
    let mut investigate_tick = None;
    let mut accuse_tick = None;
    let mut fine_tick = None;
    let mut search_place_tick = None;
    let mut report_found_tick = None;
    let mut first_violation_suspected_theft_tick = None;
    let mut first_social_suspected_theft_tick = None;

    for tick_num in 0..SURVIVAL_TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");

        let merchant_needs = h
            .world
            .get_component_homeostatic_needs(merchant)
            .copied()
            .expect("merchant should always have needs");
        if first_violation_suspected_theft_tick.is_none()
            && h.world
                .get_component_violation_memory(merchant)
                .is_some_and(|memory| {
                    memory
                        .violations
                        .iter()
                        .any(|record| matches!(record.kind, ViolationKind::SuspectedTheft { .. }))
                })
        {
            first_violation_suspected_theft_tick = Some(tick);
        }
        if first_social_suspected_theft_tick.is_none()
            && h.world
                .get_component_agent_belief_store(merchant)
                .is_some_and(|store| {
                    store.iter_social_observations().any(|observation| {
                        matches!(
                            observation.detail,
                            SocialObservationDetail::SuspectedTheft { .. }
                        )
                    })
                })
        {
            first_social_suspected_theft_tick = Some(tick);
        }
        merchant_need_runs.observe(&merchant_needs, &merchant_thresholds);
        let merchant_had_action = action_sink
            .events_for_at(merchant, tick)
            .iter()
            .any(|event| !matches!(event.kind, ActionTraceKind::StartFailed { .. }));
        observe_idle_window(
            &mut merchant_idle_state,
            merchant_had_action,
            &merchant_needs,
            tick_num,
            &contract,
            &mut stuck_idle_windows,
            "Merchant Sera",
        );

        if office_holder_tick.is_none() && h.world.office_holder(office) == Some(merchant) {
            office_holder_tick = Some(tick);
        }

        for event in action_sink.events_for_at(merchant, tick) {
            if stage_tick.is_none()
                && event.action_name == "stage_stock_for_sale"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
            {
                stage_tick = Some(tick);
            }
            if investigate_tick.is_none()
                && event.action_name == "investigate"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
            {
                investigate_tick = Some(tick);
            }
            if accuse_tick.is_none()
                && event.action_name == "accuse"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
            {
                accuse_tick = Some(tick);
            }
            if fine_tick.is_none()
                && event.action_name == "fine"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
            {
                fine_tick = Some(tick);
            }
        }

        if theft_tick.is_none()
            && action_sink.events_for_at(thief, tick).iter().any(|event| {
                event.action_name == "steal"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            })
        {
            theft_tick = Some(tick);
        }
        for event in action_sink.events_for_at(searcher, tick) {
            if search_place_tick.is_none()
                && event.action_name == "search_place"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
            {
                search_place_tick = Some(tick);
            }
            if report_found_tick.is_none()
                && event.action_name == "report_found"
                && matches!(event.kind, ActionTraceKind::Committed { .. })
            {
                report_found_tick = Some(tick);
            }
        }
    }

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let merchant_actions = action_sink
        .events_for(merchant)
        .iter()
        .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
        .map(|event| event.action_name.clone())
        .collect::<BTreeSet<_>>();
    let searcher_committed_actions = action_sink
        .events_for(searcher)
        .iter()
        .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
        .map(|event| event.action_name.clone())
        .collect::<BTreeSet<_>>();
    let searcher_start_failed_actions = action_sink
        .events_for(searcher)
        .iter()
        .filter(|event| matches!(event.kind, ActionTraceKind::StartFailed { .. }))
        .map(|event| event.action_name.clone())
        .collect::<BTreeSet<_>>();
    let decision_trace_sink = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled");
    let first_ranked_accuse_tick = decision_trace_sink
        .traces_for(merchant)
        .into_iter()
        .find_map(|trace| match &trace.outcome {
            worldwake_ai::DecisionOutcome::Planning(planning) => planning
                .candidates
                .ranked
                .iter()
                .any(|summary| {
                    matches!(
                        summary.opportunity.goal_key.kind,
                        worldwake_core::GoalKind::Accuse { .. }
                    )
                })
                .then_some(trace.tick),
            _ => None,
        });
    let first_selected_accuse_tick = decision_trace_sink
        .traces_for(merchant)
        .into_iter()
        .find_map(|trace| match &trace.outcome {
            worldwake_ai::DecisionOutcome::Planning(planning) => matches!(
                planning.selection.selected_goal().map(|goal| goal.kind),
                Some(worldwake_core::GoalKind::Accuse { .. })
            )
            .then_some(trace.tick),
            _ => None,
        });
    let merchant_violation_memory = h.world.get_component_violation_memory(merchant).cloned();
    let merchant_social_observations = h
        .world
        .get_component_agent_belief_store(merchant)
        .map(|store| store.iter_social_observations().collect::<Vec<_>>())
        .unwrap_or_default();
    let final_violation_suspicions = merchant_violation_memory
        .as_ref()
        .map(|memory| {
            memory
                .violations
                .iter()
                .filter_map(|record| match record.kind {
                    ViolationKind::SuspectedTheft { theft, suspect } => {
                        Some(format!("{:?}->{suspect:?}", theft.commodity))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let final_social_suspicions = merchant_social_observations
        .iter()
        .filter_map(|observation| match observation.detail {
            SocialObservationDetail::SuspectedTheft { theft, suspect } => {
                Some(format!("{:?}->{suspect:?}", theft.commodity))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let final_thief_apple_quantity = h
        .world
        .controlled_commodity_quantity(thief, CommodityKind::Apple);
    let final_thief_place = h.world.effective_place(thief);
    let crime_register = h
        .world
        .query_record_data()
        .find_map(|(_, data)| (data.record_kind == RecordKind::CrimeRegister).then_some(data))
        .unwrap_or_else(|| panic!("survival justice should spawn a crime register"));
    let accusation_recorded = crime_register.entries.iter().any(|entry| {
        matches!(
            entry.claim,
            InstitutionalClaim::Accusation { accuser, .. } if accuser == merchant
        )
    });
    let fine_verdict_recorded = crime_register.entries.iter().any(|entry| {
        matches!(
            entry.claim,
            InstitutionalClaim::Verdict {
                accused: _,
                violation_id: _,
                punishment: PunishmentKind::Fine { .. },
                effective_tick: _,
            }
        )
    });
    let searcher_expectation_found_safe = h
        .world
        .get_component_expectation_store(searcher)
        .is_some_and(|store| {
            store.records.values().any(|record| {
                record.subject == missing
                    && matches!(
                        record.state,
                        worldwake_core::ExpectationState::Resolved {
                            outcome: worldwake_core::ExpectationOutcome::FoundSafe { .. }
                        }
                    )
            })
        });
    let found_status_recorded = h
        .world
        .query_record_data()
        .filter(|(_, data)| data.record_kind == RecordKind::OfficeRegister)
        .any(|(_, data)| {
            data.entries.iter().any(|entry| {
                matches!(
                    entry.claim,
                    InstitutionalClaim::MissingPersonStatus {
                        subject,
                        reporter,
                        status: MissingPersonReportStatus::FoundSafe { .. },
                        ..
                    } if subject == missing && reporter == searcher
                )
            })
        });
    let searcher_exact_identity_rejections = h
        .request_resolution_trace_sink()
        .map(|sink| {
            sink.events_for(searcher)
                .into_iter()
                .filter(|event| {
                    matches!(
                        event.outcome,
                        RequestResolutionOutcome::RejectedBeforeStart {
                            reason: RequestResolutionRejectionReason::ExactIdentityRequired
                        }
                    )
                })
                .count()
                .try_into()
                .expect("exact identity rejection count exceeds u32")
        })
        .unwrap_or_default();

    JusticeObservation {
        contract,
        merchant: AgentSurvivalObservation {
            alive: h.world.is_alive(merchant),
            critical_thresholds: merchant_thresholds,
            critical_need_runs: merchant_need_runs,
            committed_actions: merchant_actions.clone(),
        },
        stuck_idle_windows,
        stage_tick: stage_tick.unwrap_or_else(|| {
            panic!("merchant should commit stage_stock_for_sale in survival justice")
        }),
        theft_tick: theft_tick.unwrap_or_else(|| {
            panic!("thief should commit steal in survival justice")
        }),
        office_holder_tick: office_holder_tick.unwrap_or_else(|| {
            panic!("merchant should become Market Warden holder in survival justice")
        }),
        investigate_tick: investigate_tick.unwrap_or_else(|| {
            panic!(
                "merchant should commit investigate in survival justice; committed_actions={merchant_actions:?}; violation_memory={merchant_violation_memory:?}; social_observations={merchant_social_observations:?}"
            )
        }),
        accuse_tick,
        fine_tick,
        accusation_recorded,
        fine_verdict_recorded,
        search_place_tick,
        report_found_tick,
        searcher_committed_actions,
        searcher_start_failed_actions,
        found_status_recorded,
        searcher_expectation_found_safe,
        searcher_exact_identity_rejections,
        first_ranked_accuse_tick,
        first_selected_accuse_tick,
        merchant_id: merchant,
        thief_id: thief,
        final_violation_suspicions,
        final_social_suspicions,
        first_violation_suspected_theft_tick,
        first_social_suspected_theft_tick,
        final_thief_apple_quantity,
        final_thief_place,
    }
}

// ---------------------------------------------------------------------------
// Scenario 177: Survival Justice Proves Accusation Substrate
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Trade, Perception, Offices, Investigation
// GoalKinds: StealItem, InvestigateViolation, Accuse
// ActionDomains: Social, Trade, Needs
// Places: Market Square
// Principles: 4, 6, 7, 8, 12, 20, 21
//
// Setup: Run the authored survival justice scenario for 1440 ticks. `Merchant
//   Sera` begins as lawful `Market Warden` holder at `Market Square`, stages
//   owned apples for sale, responds to local stock disappearance with the live
//   investigation action, then retains the theft case long enough to commit
//   `accuse` under survival pressure.
//
// Proves: the tracked merchant satisfies the authored survival-health
//   contract; the merchant starts from a lawful office-holder substrate, stages
//   sale stock, and the live accusation chain remains active in the same
//   authored survival run where theft also occurs. The same accusation case
//   then reaches truthful fine punishment and records the verdict.
//   The scenario intentionally stops short of claiming that search/report_found
//   is part of this accusation-substrate seam.
//
// Chain: lawful office-holder substrate -> staged apples become stealable ->
//   the scenario reaches real `steal`, `investigate`, `accuse`, and `fine`
//   commits under the same survival envelope, and the crime register records
//   the accusation and fine verdict for the same justice run.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_justice_proves_accusation_substrate() {
    let observation = run_survival_justice();
    let run_limit_overrides =
        contract_run_limit_overrides(observation.contract.critical_run_limits.as_ref());

    assert!(
        observation.merchant.alive,
        "Merchant Sera should remain alive for the full {SURVIVAL_TICKS}-tick scenario; observation={observation:?}"
    );
    assert_authored_critical_runs_with_overrides(
        observation.contract.max_authored_critical_run_ticks,
        run_limit_overrides,
        "Merchant Sera",
        &observation.merchant.critical_thresholds,
        &observation.merchant.critical_need_runs,
    );
    assert_required_self_care_families(
        &observation.contract.required_self_care_families,
        "Merchant Sera",
        &observation.merchant.committed_actions,
        "survival-justice",
    );
    assert_no_stuck_idle_windows(
        observation
            .contract
            .max_idle_window_ticks_with_elevated_need,
        observation.contract.elevated_need_floor.value(),
        "survival-justice",
        &observation.stuck_idle_windows,
    );

    assert!(
        observation.office_holder_tick <= observation.stage_tick,
        "merchant should begin from a lawful office-holder substrate before staging stock; observation={observation:?}"
    );
    assert!(
        observation.stage_tick <= observation.theft_tick,
        "merchant should stage sale stock before the theft commit; observation={observation:?}"
    );
    assert!(
        observation.theft_tick <= observation.investigate_tick,
        "merchant should investigate after the theft commit; observation={observation:?}"
    );
    assert!(
        observation.accuse_tick.is_some(),
        "merchant should commit accuse in survival justice; observation={observation:?}"
    );
    assert!(
        observation.investigate_tick <= observation.accuse_tick.expect("checked above"),
        "merchant should accuse after the theft investigation commit; observation={observation:?}"
    );
    assert!(
        observation.accusation_recorded,
        "crime register should record the accusation case; observation={observation:?}"
    );
}

#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_justice_proves_fine_punishment_for_same_theft_case() {
    let observation = run_survival_justice();
    let accuse_tick = observation
        .accuse_tick
        .expect("merchant should still commit accuse before punishment");
    let Some(fine_tick) = observation.fine_tick else {
        panic!(
            "merchant should commit fine for the same accusation case; observation={observation:?}"
        );
    };

    assert!(
        accuse_tick <= fine_tick,
        "fine should commit after accuse in survival justice; observation={observation:?}"
    );
    assert!(
        observation.fine_verdict_recorded,
        "crime register should record a fine verdict for the accusation case; observation={observation:?}"
    );
}

// ---------------------------------------------------------------------------
// Scenario 178: Survival Justice Proves Search And Report Found
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Search, Reports, Perception, Offices
// GoalKinds: SearchForMissing, ReportFound
// ActionDomains: Social, Needs
// Places: Market Square
// Principles: 6, 7, 8, 14, 17, 18, 20
//
// Setup: Run the authored survival justice scenario for 1440 ticks. `Searcher
//   Ivo` begins with an overdue expectation for colocated `Missing Pru` at
//   `Market Square` and a matching local last-seen record.
//
// Proves: the searcher commits `search_place` for the overdue missing-person
//   expectation, resolves it as found safe, then commits `report_found` and
//   writes the found-person status to the local office register. The same run
//   also asserts that stale exact-bound `ask_about_person` requests no longer
//   recur for this local-search branch.
//
// Chain: overdue local expectation -> planner selects direct `search_place`
//   instead of stale `ask_about_person` -> expectation resolves found safe ->
//   `report_found` writes the missing-person status claim.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_justice_proves_search_and_report_found() {
    let observation = run_survival_justice();
    let Some(search_place_tick) = observation.search_place_tick else {
        panic!(
            "searcher should commit search_place for the overdue missing-person expectation; observation={observation:?}"
        );
    };
    let Some(report_found_tick) = observation.report_found_tick else {
        panic!(
            "searcher should report the found missing person after search_place; observation={observation:?}"
        );
    };

    assert!(
        search_place_tick <= report_found_tick,
        "report_found should follow search_place in survival justice; observation={observation:?}"
    );
    assert!(
        observation.searcher_expectation_found_safe,
        "search_place should resolve Searcher Ivo's expectation as found safe; observation={observation:?}"
    );
    assert!(
        observation.found_status_recorded,
        "office register should record Searcher Ivo's found-person status report; observation={observation:?}"
    );
    assert_eq!(
        observation.searcher_exact_identity_rejections, 0,
        "stale ask_about_person exact-identity rejections should not recur in survival justice; observation={observation:?}"
    );
}

#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_justice_replays_deterministically() {
    assert_eq!(run_survival_justice(), run_survival_justice());
}
