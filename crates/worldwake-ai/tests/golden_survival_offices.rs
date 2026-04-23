//! Golden tests for the survival offices roadmap row.

mod golden_harness;

use std::collections::BTreeSet;
use std::path::PathBuf;

use golden_harness::*;
use worldwake_ai::DecisionOutcome;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{DriveThresholds, GoalKind, NoticeTopic, Tick};
use worldwake_sim::ActionTraceKind;

const SURVIVAL_TICKS: u32 = 1440;

#[derive(Clone, Debug, Eq, PartialEq)]
struct AgentSurvivalObservation {
    alive: bool,
    critical_thresholds: DriveThresholds,
    critical_need_runs: SurvivalNeedRunTracker,
    committed_actions: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OfficesObservation {
    contract: worldwake_cli::scenario::types::SurvivalHealthContractDef,
    agent: AgentSurvivalObservation,
    stuck_idle_windows: Vec<StuckIdleWindow>,
    first_claim_selection_tick: Tick,
    first_post_notice_selection_tick: Tick,
    first_post_notice_commit_tick: Tick,
    first_press_force_claim_tick: Tick,
    first_control_tick: Tick,
    first_holder_tick: Tick,
    posted_threat_warning_exists: bool,
}

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-offices.ron")
}

fn load_survival_offices_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival offices scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival offices scenario should spawn");
    let mut harness = GoldenHarness::from_simulation_state(&spawned.state);
    harness.driver.enable_tracing();
    harness.enable_action_tracing();
    (harness, def)
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

fn run_survival_offices() -> OfficesObservation {
    let (mut h, def) = load_survival_offices_harness();
    let contract =
        expect_survival_health_contract(def.survival_health_contract.as_ref(), "survival offices")
            .clone();
    let agent = h
        .world
        .query_name_and_agent_data()
        .find_map(|(entity, name, _)| (name.0 == "Claimant Rhea").then_some(entity))
        .expect("scenario should include Claimant Rhea");
    let office = h
        .world
        .query_name()
        .find_map(|(entity, name)| (name.0 == "Marsh Warden").then_some(entity))
        .expect("scenario should include Marsh Warden");
    let thresholds = h
        .world
        .get_component_drive_thresholds(agent)
        .copied()
        .expect("survival offices agent should have drive thresholds");
    let mut critical_need_runs = SurvivalNeedRunTracker::default();
    let mut stuck_idle_windows = Vec::new();
    let mut idle_state: (Option<u32>, u16, u32) = (None, 0, 0);

    let mut first_claim_selection_tick = None;
    let mut first_post_notice_selection_tick = None;
    let mut first_post_notice_commit_tick = None;
    let mut first_press_force_claim_tick = None;
    let mut first_control_tick = None;
    let mut first_holder_tick = None;

    for tick_num in 0..SURVIVAL_TICKS {
        h.step_once();
        let tick = Tick(u64::from(tick_num));
        let action_sink = h
            .action_trace_sink()
            .expect("action tracing should be enabled");

        let needs = h
            .world
            .get_component_homeostatic_needs(agent)
            .expect("survival offices agent should always have needs");
        critical_need_runs.observe(needs, &thresholds);

        let had_action = action_sink
            .events_for_at(agent, tick)
            .iter()
            .any(|event| !matches!(event.kind, ActionTraceKind::StartFailed { .. }));
        let (start, max_need, count) = &mut idle_state;
        if had_action {
            if let Some(s) = start.take()
                && *count >= contract.max_idle_window_ticks_with_elevated_need
                && *max_need > contract.elevated_need_floor.value()
            {
                stuck_idle_windows.push(StuckIdleWindow {
                    agent_name: "Claimant Rhea".to_string(),
                    start_tick: s,
                    end_tick: tick_num.saturating_sub(1),
                    max_need_at_start: *max_need,
                });
            }
            *count = 0;
        } else {
            if start.is_none() {
                *start = Some(tick_num);
                *max_need = max_need_value(needs);
            }
            *count += 1;
        }

        if first_claim_selection_tick.is_none() {
            let maybe_tick = h
                .driver
                .trace_sink()
                .expect("decision tracing should be enabled")
                .trace_at(agent, tick)
                .and_then(|trace| match &trace.outcome {
                    DecisionOutcome::Planning(planning)
                        if planning.selection.selected_goal().is_some_and(|goal| {
                            matches!(goal.kind, GoalKind::ClaimOffice { office: goal_office } if goal_office == office)
                        }) =>
                    {
                        Some(trace.tick)
                    }
                    _ => None,
                });
            first_claim_selection_tick = first_claim_selection_tick.or(maybe_tick);
        }

        if first_post_notice_selection_tick.is_none() {
            let maybe_tick = h
                .driver
                .trace_sink()
                .expect("decision tracing should be enabled")
                .trace_at(agent, tick)
                .and_then(|trace| match &trace.outcome {
                    DecisionOutcome::Planning(planning)
                        if planning.selection.selected_goal().is_some_and(|goal| {
                            matches!(
                                goal.kind,
                                GoalKind::PostNotice {
                                    topic: NoticeTopic::ThreatWarning { .. },
                                    ..
                                }
                            )
                        }) =>
                    {
                        Some(trace.tick)
                    }
                    _ => None,
                });
            first_post_notice_selection_tick = first_post_notice_selection_tick.or(maybe_tick);
        }

        if first_post_notice_commit_tick.is_none()
            && action_sink.events_for_at(agent, tick).iter().any(|event| {
                event.action_name == "post_notice"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            })
        {
            first_post_notice_commit_tick = Some(tick);
        }

        if first_press_force_claim_tick.is_none()
            && action_sink.events_for_at(agent, tick).iter().any(|event| {
                event.action_name == "press_force_claim"
                    && matches!(event.kind, ActionTraceKind::Committed { .. })
            })
        {
            first_press_force_claim_tick = Some(tick);
        }

        if first_control_tick.is_none() && h.world.office_controller(office) == Some(agent) {
            first_control_tick = Some(tick);
        }

        if first_holder_tick.is_none() && h.world.office_holder(office) == Some(agent) {
            first_holder_tick = Some(tick);
        }
    }

    let action_sink = h
        .action_trace_sink()
        .expect("action tracing should be enabled");
    let committed_actions = action_sink
        .events_for(agent)
        .iter()
        .filter(|event| matches!(event.kind, ActionTraceKind::Committed { .. }))
        .map(|event| event.action_name.clone())
        .collect::<BTreeSet<_>>();
    let trace_summaries = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .traces_for(agent)
        .into_iter()
        .map(|trace| format!("{:?}: {}", trace.tick, trace.outcome.summary()))
        .collect::<Vec<_>>();
    let post_notice_attempt_summaries = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .traces_for(agent)
        .into_iter()
        .filter_map(|trace| match &trace.outcome {
            DecisionOutcome::Planning(planning) => Some((trace.tick, planning)),
            _ => None,
        })
        .flat_map(|(tick, planning)| {
            planning
                .planning
                .attempts
                .iter()
                .filter(|attempt| {
                    matches!(
                        attempt.goal.kind,
                        GoalKind::PostNotice {
                            topic: NoticeTopic::ThreatWarning { .. },
                            ..
                        }
                    )
                })
                .map(move |attempt| {
                    let root = attempt
                        .expansion_summaries
                        .iter()
                        .find(|summary| summary.depth == 0);
                    format!(
                        "{tick:?}: outcome={:?}, roots={:?}, omissions={:?}, bindings={:?}",
                        attempt.outcome,
                        root.map(|summary| &summary.root_candidates),
                        root.map(|summary| &summary.root_omissions),
                        attempt.binding_rejections,
                    )
                })
        })
        .collect::<Vec<_>>();
    let posted_threat_warning_exists = h.world.query_artifact_header().any(|(artifact, header)| {
        header.created_at > Tick(0)
            && h.world
                .get_component_notice_content(artifact)
                .is_some_and(|content| matches!(content.topic, NoticeTopic::ThreatWarning { .. }))
    });

    OfficesObservation {
        contract,
        agent: AgentSurvivalObservation {
            alive: !h.agent_is_dead(agent),
            critical_thresholds: thresholds,
            critical_need_runs,
            committed_actions: committed_actions.clone(),
        },
        stuck_idle_windows,
        first_claim_selection_tick: first_claim_selection_tick.unwrap_or_else(|| {
            panic!(
                "scenario should select ClaimOffice under survival pressure; committed_actions={committed_actions:?}; traces={trace_summaries:?}; post_notice_attempts={post_notice_attempt_summaries:?}"
            )
        }),
        first_post_notice_selection_tick: first_post_notice_selection_tick.unwrap_or_else(|| {
            panic!(
                "scenario should select PostNotice from authored warning substrate; committed_actions={committed_actions:?}; traces={trace_summaries:?}; post_notice_attempts={post_notice_attempt_summaries:?}"
            )
        }),
        first_post_notice_commit_tick: first_post_notice_commit_tick.unwrap_or_else(|| {
            panic!(
                "scenario should commit post_notice from authored warning substrate; committed_actions={committed_actions:?}; traces={trace_summaries:?}; post_notice_attempts={post_notice_attempt_summaries:?}"
            )
        }),
        first_press_force_claim_tick: first_press_force_claim_tick.unwrap_or_else(|| {
            panic!(
                "scenario should commit press_force_claim after consulting the register; committed_actions={committed_actions:?}; traces={trace_summaries:?}; post_notice_attempts={post_notice_attempt_summaries:?}"
            )
        }),
        first_control_tick: first_control_tick.unwrap_or_else(|| {
            panic!(
                "scenario should establish office control after press_force_claim; committed_actions={committed_actions:?}; traces={trace_summaries:?}; post_notice_attempts={post_notice_attempt_summaries:?}"
            )
        }),
        first_holder_tick: first_holder_tick.unwrap_or_else(|| {
            panic!(
                "scenario should install the holder after force-control delay; committed_actions={committed_actions:?}; traces={trace_summaries:?}; post_notice_attempts={post_notice_attempt_summaries:?}"
            )
        }),
        posted_threat_warning_exists,
    }
}

// ---------------------------------------------------------------------------
// Scenario 175: Survival Offices Proves Force-Law Uptake Under Survival
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Offices
// GoalKinds: ClaimOffice, AcquireCommodity(SelfConsume), ConsumeOwnedCommodity,
//   Drink, Wash, Sleep, Relieve
// ActionDomains: Social, Needs
// Places: Council Green
// Principles: 6, 7, 14, 20
//
// Setup: Run the authored survival offices scenario for 1440 ticks. `Claimant
//   Rhea` starts at `Council Green` with one vacant force-law office, authored
//   remembered local conflict memory, and the co-located survival substrate
//   needed to keep eating, drinking, relieving, washing, and sleeping in the
//   same place. That makes the row about force-law office pressure and
//   autonomous notice posting competing with ongoing self-care rather than
//   travel or consult-record gating.
//
// Proves: the tracked agent satisfies the authored survival-health contract,
//   selects `ClaimOffice`, selects and commits `PostNotice` from the authored
//   warning substrate, commits `press_force_claim`, becomes force controller,
//   and only later installs as office holder after the force-law hold delay.
//
// Chain: authored remembered local conflict -> PostNotice selected and
//   committed -> new threat-warning notice artifact exists; in the same
//   authored survival run a vacant force-law office under survival pressure -> ClaimOffice
//   selected -> press_force_claim commits -> office controller mutates ->
//   delayed office holder installation.
#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_offices_proves_force_law_uptake() {
    let observation = run_survival_offices();
    let run_limit_overrides =
        contract_run_limit_overrides(observation.contract.critical_run_limits.as_ref());

    assert!(
        observation.agent.alive,
        "Claimant Rhea should remain alive for the full {SURVIVAL_TICKS}-tick scenario; observation={observation:?}"
    );
    assert_authored_critical_runs_with_overrides(
        observation.contract.max_authored_critical_run_ticks,
        run_limit_overrides,
        "Claimant Rhea",
        &observation.agent.critical_thresholds,
        &observation.agent.critical_need_runs,
    );
    assert_required_self_care_families(
        &observation.contract.required_self_care_families,
        "Claimant Rhea",
        &observation.agent.committed_actions,
        "survival-offices",
    );
    assert_no_stuck_idle_windows(
        observation
            .contract
            .max_idle_window_ticks_with_elevated_need,
        observation.contract.elevated_need_floor.value(),
        "survival-offices",
        &observation.stuck_idle_windows,
    );
    assert!(
        observation.first_claim_selection_tick <= observation.first_press_force_claim_tick,
        "press_force_claim must follow ClaimOffice selection; observation={observation:?}"
    );
    assert!(
        observation.first_post_notice_selection_tick <= observation.first_post_notice_commit_tick,
        "post_notice must follow PostNotice selection; observation={observation:?}"
    );
    assert!(
        observation.first_press_force_claim_tick <= observation.first_control_tick,
        "force control must not appear before the press_force_claim commit; observation={observation:?}"
    );
    assert!(
        observation.first_control_tick <= observation.first_holder_tick,
        "holder installation must follow force control and hold delay; observation={observation:?}"
    );
    assert!(
        observation.posted_threat_warning_exists,
        "post_notice should leave a threat-warning notice artifact created during the run; observation={observation:?}"
    );
}

#[test]
#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]
fn survival_offices_replays_deterministically() {
    assert_eq!(run_survival_offices(), run_survival_offices());
}
