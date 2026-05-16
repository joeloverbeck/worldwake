use super::GoldenHarness;
use std::fmt::Write;
use worldwake_ai::{
    ActionStartFailureSummary, ActionTraceSnapshot, AgentDecisionTrace, CandidateTrace,
    CriticalWindowReport, DecisionOutcome, ExecutionTrace, ExhaustionSummary, GoalPriorityClass,
    GoalSwitchSummary, PatrolRouteSnapshotTrace, PlanAttemptTrace, PlanSearchOutcome,
    PlanSearchTrace, PlanningPipelineTrace, RankedGoalSummary, SelectedPlanSource,
    SelectedPlanTrace, SelectionTrace, SurvivalForensicExtractor,
};
use worldwake_core::{
    CommodityKind, DriveThresholds, EntityId, GoalKey, GoalKind, HomeostaticNeeds,
    OpportunityAnchor, OpportunityKey, Permille, Tick,
};

pub fn observe_critical_windows(
    extractor: &mut SurvivalForensicExtractor,
    harness: &GoldenHarness,
    agent: EntityId,
    tick: Tick,
    needs: &HomeostaticNeeds,
    thresholds: &DriveThresholds,
) {
    let decision_trace = harness
        .driver
        .trace_sink()
        .and_then(|sink| sink.trace_at(agent, tick));
    let action_snapshot =
        harness
            .action_trace_sink()
            .map_or_else(ActionTraceSnapshot::empty, |sink| {
                let active_action = harness
                    .scheduler
                    .active_actions()
                    .values()
                    .find(|instance| instance.actor == agent);
                let active_action_name = active_action
                    .and_then(|instance| harness.defs.get(instance.def_id))
                    .map(|def| def.name.as_str());
                ActionTraceSnapshot::from_sink(agent, tick, sink, active_action, active_action_name)
            });
    let local_state = worldwake_ai::LocalSurvivalStateSummary::capture(&harness.world, agent);
    extractor.observe(
        tick,
        needs,
        thresholds,
        decision_trace,
        &action_snapshot,
        &local_state,
    );
}

pub fn expect_sleep_progress_barrier_window(reports: &[CriticalWindowReport]) {
    let matched = reports.iter().any(|report| {
        let has_frontier_exhaustion = report.frames.iter().any(|frame| {
            matches!(
                frame.exhaustion_state,
                Some(ExhaustionSummary::FrontierExhausted { .. })
            )
        });
        let has_sleep_goal = report
            .frames
            .iter()
            .any(|frame| frame.selected_goal == Some(GoalKey::from(GoalKind::Sleep)));
        has_frontier_exhaustion && has_sleep_goal
    });
    assert!(
        matched,
        "expected a critical window with both Sleep selection and FrontierExhausted frames\n{}",
        dump_reports_for_debug(reports)
    );
}

pub fn expect_wash_vs_water_competition_window(reports: &[CriticalWindowReport]) {
    let matched = reports.iter().any(|report| {
        report.frames.iter().any(|frame| {
            let selected_is_competing_family = frame
                .selected_goal
                .is_some_and(|goal| is_wash_goal(&goal.kind) || is_water_acquire_goal(&goal.kind));
            let has_wash_family = frame
                .top_competitors
                .iter()
                .any(|competitor| is_wash_goal(&competitor.goal.kind))
                || frame
                    .selected_goal
                    .is_some_and(|goal| is_wash_goal(&goal.kind));
            let has_water_family = frame
                .top_competitors
                .iter()
                .any(|competitor| is_water_acquire_goal(&competitor.goal.kind))
                || frame
                    .selected_goal
                    .is_some_and(|goal| is_water_acquire_goal(&goal.kind));
            selected_is_competing_family && has_wash_family && has_water_family
        })
    });
    assert!(
        matched,
        "expected a critical window exposing both wash and water-acquire competition families\n{}",
        dump_reports_for_debug(reports)
    );
}

pub fn expect_deterministic_reports(a: &[CriticalWindowReport], b: &[CriticalWindowReport]) {
    assert_eq!(
        a,
        b,
        "expected identical critical-window reports across repeated synthetic runs\nleft:\n{}\nright:\n{}",
        dump_reports_for_debug(a),
        dump_reports_for_debug(b)
    );
}

pub fn dump_reports_for_debug(reports: &[CriticalWindowReport]) -> String {
    if reports.is_empty() {
        return "no critical-window reports captured".to_string();
    }

    let mut out = String::new();
    for report in reports {
        let _ = writeln!(
            out,
            "agent={:?} need={:?} window={}..{} threshold={} peak={}",
            report.agent,
            report.need,
            report.start_tick.0,
            report.end_tick.0,
            report.threshold.value(),
            report.peak_value.value()
        );
        for frame in &report.frames {
            let competitors = frame
                .top_competitors
                .iter()
                .map(|goal| format!("{:?}", goal.goal.kind))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(
                out,
                "  tick={} need={} selected={:?} plan_source={:?} competitors=[{}] active={:?} exhaustion={:?} blocker={:?} local={:?}",
                frame.tick.0,
                frame.need_value.value(),
                frame.selected_goal.map(|goal| goal.kind),
                frame.selected_plan_source,
                competitors,
                frame
                    .active_action
                    .as_ref()
                    .map(|action| action.action_name.as_str()),
                frame.exhaustion_state,
                frame.blocker_summary,
                frame.local_authoritative_summary
            );
        }
    }
    out
}

pub fn sample_local_survival_state_summary() -> worldwake_ai::LocalSurvivalStateSummary {
    worldwake_ai::LocalSurvivalStateSummary {
        place: Some(EntityId {
            slot: 50,
            generation: 0,
        }),
        water_source_present: true,
        wash_basin_present: false,
        sleep_affordance_present: true,
        food_source_present: true,
    }
}

pub fn synthetic_ranked_goal_summary(
    goal: GoalKey,
    priority_class: GoalPriorityClass,
    motive_score: u32,
) -> RankedGoalSummary {
    RankedGoalSummary {
        opportunity: OpportunityKey {
            goal_key: goal,
            anchor: OpportunityAnchor::None,
        },
        priority_class,
        motive_score,
        motive_source_contributions: Vec::new(),
        provenance: None,
        source_reliability_discount: None,
        competition_discount: None,
        source_composite: None,
        feasibility: worldwake_ai::FeasibilityHint::Likely,
        acquisition_quantity: None,
        artifact_axes: None,
    }
}

pub fn synthetic_planning_trace(
    agent: EntityId,
    tick: Tick,
    selected_goal: GoalKey,
    ranked: Vec<RankedGoalSummary>,
    selected_plan_source: Option<SelectedPlanSource>,
    attempt_outcome: PlanSearchOutcome,
) -> AgentDecisionTrace {
    AgentDecisionTrace {
        agent,
        tick,
        compiled_opportunities: Vec::new(),
        opportunity_compiler_load: None,
        snapshot_cache_counters: None,
        repair_attempts: Vec::new(),
        causal_link_cap_hits: Vec::new(),
        outcome: DecisionOutcome::Planning(Box::new(PlanningPipelineTrace {
            affordances: None,
            dirty: worldwake_ai::DirtySet::default(),
            plan_continued: false,
            candidates: CandidateTrace {
                generated: vec![OpportunityKey {
                    goal_key: selected_goal,
                    anchor: OpportunityAnchor::None,
                }],
                evidence: Vec::new(),
                fully_blocked_desires: Vec::new(),
                places_reachable: 0,
                places_after_belief_filter: 0,
                ranked,
                top_ranked_comparison: None,
                suppressed: Vec::new(),
                damped: Vec::new(),
                zero_motive: Vec::new(),
                omitted_political: Vec::new(),
                omitted_bandit: Vec::new(),
                omitted_social: Vec::new(),
                omitted_violation_detection: Vec::new(),
            },
            planning: PlanSearchTrace {
                attempts: vec![PlanAttemptTrace {
                    goal: selected_goal,
                    opportunity_anchor: OpportunityAnchor::None,
                    outcome: attempt_outcome,
                    strategic_budget: None,
                    strategic_plan: None,
                    tactical_goal: None,
                    landmarks_extracted: 0,
                    landmark_orderings: 0,
                    target_belief_presence:
                        worldwake_ai::decision_trace::TargetBeliefPresence::NotApplicable,
                    binding_rejections: Vec::new(),
                    expansion_summaries: Vec::new(),
                }],
                same_goal_trace: None,
            },
            selection: SelectionTrace {
                selected_opportunity: Some(OpportunityKey {
                    goal_key: selected_goal,
                    anchor: OpportunityAnchor::None,
                }),
                selected_plan: Some(SelectedPlanTrace {
                    steps: Vec::new(),
                    terminal_kind: worldwake_ai::PlanTerminalKind::GoalSatisfied,
                    next_step_index: None,
                    next_step: None,
                    search_provenance: None,
                    primary_motive: 900,
                    total_value: 900,
                    side_benefits: Vec::new(),
                }),
                selected_plan_source,
                goal_switch: Some(GoalSwitchSummary {
                    from: GoalKey::from(GoalKind::Sleep),
                    to: selected_goal,
                    kind: worldwake_ai::GoalSwitchKind::HigherPriorityGoal,
                }),
                previous_goal: None,
                plan_replacement: None,
                snapshot_continuation: None,
            },
            portfolio: None,
            execution: ExecutionTrace {
                enqueued_step: None,
                revalidation_passed: None,
                failure: None,
            },
            action_start_failures: Vec::<ActionStartFailureSummary>::new(),
            discrepancy_trace: Vec::new(),
            exhaustion_snapshot: Vec::new(),
            frame_transition: None,
            patrol_route: PatrolRouteSnapshotTrace::default(),
            selected_patrol_anchor: None,
            pursuit_invalidation: None,
        })),
    }
}

fn is_wash_goal(goal: &GoalKind) -> bool {
    matches!(goal, GoalKind::Wash)
}

fn is_water_acquire_goal(goal: &GoalKind) -> bool {
    matches!(
        goal,
        GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: worldwake_core::CommodityPurpose::SelfConsume,
            quantity: _,
        }
    )
}

#[allow(dead_code)]
fn pm(value: u16) -> Permille {
    Permille::new(value).unwrap()
}
