use crate::decision_trace::{
    BindingRejection, GoalSwitchSummary, PlanAttemptTrace, PlanSearchOutcome, PlanSearchTrace,
    PlannedStepSummary, RankedGoalSummary, SelectedPlanReplacementKind,
    SelectedPlanReplacementTrace, SelectedPlanSearchProvenance, SelectedPlanSource,
    SelectedPlanTrace, SelectionTrace,
};
use crate::exhaustion::{derive_invalidation_conditions, invalidate_exhausted_goals};
use crate::search::PlanSearchResult;
use crate::{
    authoritative_target, build_planning_snapshot_with_blocked_facility_uses, revalidate_next_step,
    search_plan, select_best_plan, AgentDecisionRuntime, DirtySet, ExhaustionEntry, PlannedPlan,
    PlannedStep, PlannerOpSemantics, PlanningBudget, RankedGoal,
};
use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    ActionDefId, ActiveGoal, BlockedIntentMemory, IntentionFrame, Permille, Tick,
};
use worldwake_sim::{
    ActionHandlerRegistry, GoalBeliefView, RecipeRegistry, RuntimeBeliefView, Scheduler,
};

use super::{current_step, runtime_belief_view, update_frame_for_adopted_plan};

/// Build a `PlannedStepSummary` from a `PlannedStep` for trace output.
pub(super) fn summarize_step(
    step: &PlannedStep,
    action_defs: &worldwake_sim::ActionDefRegistry,
) -> PlannedStepSummary {
    let action_name = action_defs
        .get(step.def_id)
        .map_or_else(|| "unknown".to_owned(), |def| def.name.clone());
    PlannedStepSummary {
        action_def_id: step.def_id,
        action_name,
        op_kind: step.op_kind,
        targets: step
            .targets
            .iter()
            .filter_map(|t| authoritative_target(*t))
            .collect(),
        estimated_ticks: step.estimated_ticks,
    }
}

pub(super) fn summarize_selected_plan(
    plan: &PlannedPlan,
    current_step_index: usize,
    action_defs: &worldwake_sim::ActionDefRegistry,
    search_provenance: Option<SelectedPlanSearchProvenance>,
) -> SelectedPlanTrace {
    SelectedPlanTrace {
        steps: plan
            .steps
            .iter()
            .map(|step| summarize_step(step, action_defs))
            .collect(),
        terminal_kind: plan.terminal_kind,
        next_step_index: (current_step_index < plan.steps.len()).then_some(current_step_index),
        next_step: plan
            .steps
            .get(current_step_index)
            .map(|step| summarize_step(step, action_defs)),
        search_provenance,
    }
}

pub(super) fn summarize_search_provenance(
    selected_goal: worldwake_core::GoalKey,
    plans: &[(
        crate::GoalKey,
        PlanSearchResult,
        Vec<BindingRejection>,
        Vec<crate::decision_trace::SearchExpansionSummary>,
    )],
) -> Option<SelectedPlanSearchProvenance> {
    let (_, result, _, expansions) = plans
        .iter()
        .find(|(goal, _, _, _)| *goal == selected_goal)?;
    if !matches!(result, PlanSearchResult::Found(_)) {
        return None;
    }
    let root = expansions.first();
    Some(SelectedPlanSearchProvenance {
        expansions_used: expansions.len() as u16,
        root_remaining_travel_ticks: root.map_or(0, |summary| summary.remaining_travel_ticks),
        root_travel_pruning: root.and_then(|summary| summary.travel_pruning.clone()),
    })
}

pub(super) fn summarize_plan_replacement(
    runtime: &AgentDecisionRuntime,
    active_goal: Option<worldwake_core::GoalKey>,
    selected_goal: worldwake_core::GoalKey,
    selected_plan: &PlannedPlan,
    action_defs: &worldwake_sim::ActionDefRegistry,
) -> Option<SelectedPlanReplacementTrace> {
    let previous_goal = active_goal?;
    let previous_next_step = current_step(runtime).map(|step| summarize_step(step, action_defs));
    let new_next_step = selected_plan
        .steps
        .first()
        .map(|step| summarize_step(step, action_defs));
    if previous_goal == selected_goal && previous_next_step == new_next_step {
        return None;
    }
    Some(SelectedPlanReplacementTrace {
        previous_goal,
        new_goal: selected_goal,
        previous_next_step,
        new_next_step,
        kind: if previous_goal == selected_goal {
            SelectedPlanReplacementKind::SameGoalBranchReplanned
        } else {
            SelectedPlanReplacementKind::GoalChanged
        },
    })
}

pub(super) fn summarize_ranked_goal(ranked: &RankedGoal) -> RankedGoalSummary {
    RankedGoalSummary {
        goal: ranked.grounded.key,
        priority_class: ranked.priority_class,
        motive_score: ranked.motive_score,
        provenance: ranked.provenance.clone(),
        feasibility: ranked.feasibility,
    }
}

pub(super) fn determine_selected_plan_source(
    selected_goal: worldwake_core::GoalKey,
    current_goal_before_selection: Option<worldwake_core::GoalKey>,
    plans: &[(worldwake_core::GoalKey, Option<PlannedPlan>)],
) -> SelectedPlanSource {
    if plans
        .iter()
        .any(|(goal, plan)| *goal == selected_goal && plan.is_some())
    {
        SelectedPlanSource::SearchSelection
    } else {
        debug_assert_eq!(current_goal_before_selection, Some(selected_goal));
        SelectedPlanSource::RetainedCurrentPlan
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_candidate_plans(
    world: &worldwake_core::World,
    scheduler: &Scheduler,
    agent: worldwake_core::EntityId,
    ranked_candidates: &[RankedGoal],
    blocked_memory: &BlockedIntentMemory,
    current_tick: Tick,
    budget: &PlanningBudget,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    action_defs: &worldwake_sim::ActionDefRegistry,
    action_handlers: &ActionHandlerRegistry,
    recipe_registry: &RecipeRegistry,
    collect_rejections: bool,
    collect_expansion_summaries: bool,
    exhaustion_cache: &std::collections::BTreeMap<crate::GoalKey, ExhaustionEntry>,
) -> Vec<(
    crate::GoalKey,
    PlanSearchResult,
    Vec<BindingRejection>,
    Vec<crate::decision_trace::SearchExpansionSummary>,
)> {
    let view = runtime_belief_view(agent, world, scheduler, action_defs);
    let mut seen_goals = BTreeSet::new();
    let candidates_to_plan: Vec<_> = ranked_candidates
        .iter()
        .filter(|c| seen_goals.insert(c.grounded.key))
        .filter(|c| {
            !exhaustion_cache
                .get(&c.grounded.key)
                .is_some_and(ExhaustionEntry::suppresses_planning)
        })
        .take(usize::from(budget.max_candidates_to_plan))
        .collect();

    // All candidates filtered by exhausted-goal skip set — no snapshot needed.
    if candidates_to_plan.is_empty() {
        return Vec::new();
    }

    // The current evidence model still relies on a merged view here for some
    // lawful prerequisite chains. Keep snapshot construction shared until the
    // per-opportunity evidence surface is strong enough to preserve those plans.
    let mut merged_evidence_entities = BTreeSet::new();
    let mut merged_evidence_places = BTreeSet::new();
    for ranked in &candidates_to_plan {
        merged_evidence_entities.extend(ranked.grounded.evidence_entities.iter().copied());
        merged_evidence_places.extend(ranked.grounded.evidence_places.iter().copied());
    }
    let snapshot = build_planning_snapshot_with_blocked_facility_uses(
        &view,
        agent,
        &merged_evidence_entities,
        &merged_evidence_places,
        budget.snapshot_travel_horizon,
        blocked_memory,
        current_tick,
    );

    let mut results = Vec::with_capacity(candidates_to_plan.len());
    for ranked in candidates_to_plan {
        let mut rejections = Vec::new();
        let mut expansions = Vec::new();
        // Exponential backoff on search budget for goals that previously
        // exhausted the budget. Each consecutive exhaustion halves the
        // retry budget (256→128→64→32 floor), cutting retry cost by
        // 50-87.5% while still allowing plan discovery.
        let effective_budget = match exhaustion_cache.get(&ranked.grounded.key) {
            Some(entry) if entry.is_budget_retry_pending() => {
                let mut reduced = budget.clone();
                reduced.max_node_expansions =
                    entry.effective_max_expansions(budget.max_node_expansions);
                reduced
            }
            _ => budget.clone(),
        };
        let result = search_plan(
            &snapshot,
            &ranked.grounded,
            semantics_table,
            action_defs,
            action_handlers,
            &effective_budget,
            recipe_registry,
            blocked_memory,
            current_tick,
            if collect_rejections {
                Some(&mut rejections)
            } else {
                None
            },
            if collect_expansion_summaries {
                Some(&mut expansions)
            } else {
                None
            },
        );
        let found = result.is_found();
        results.push((ranked.grounded.key, result, rejections, expansions));
        // Early termination: candidates are ranked by priority. If the
        // top-ranked candidate found a plan, lower-ranked candidates
        // cannot produce a better selection (compare_ranked_plans sorts
        // by priority_class first). Skip remaining searches.
        if found {
            break;
        }
    }
    results
}

/// Convert `PlanSearchResult` plans to `Option<PlannedPlan>` for APIs that
/// only care about found plans (selection, interrupt evaluation).
pub(super) fn plans_as_options(
    plans: &[(
        crate::GoalKey,
        PlanSearchResult,
        Vec<BindingRejection>,
        Vec<crate::decision_trace::SearchExpansionSummary>,
    )],
) -> Vec<(crate::GoalKey, Option<PlannedPlan>)> {
    plans
        .iter()
        .map(|(key, result, _, _)| (*key, result.clone().into_plan()))
        .collect()
}

fn try_continue_snapshot_plan(
    view: &impl RuntimeBeliefView,
    runtime: &mut AgentDecisionRuntime,
    ranked_candidates: &[RankedGoal],
    active_goal_key: Option<worldwake_core::GoalKey>,
    agent: worldwake_core::EntityId,
    action_defs: &worldwake_sim::ActionDefRegistry,
    action_handlers: &ActionHandlerRegistry,
) -> Option<PlannedStep> {
    if !runtime.dirty.is_snapshot_only() || runtime.current_plan.is_none() {
        return None;
    }

    let current_goal_still_top = ranked_candidates
        .first()
        .is_some_and(|top| Some(top.grounded.key) == active_goal_key);
    if !current_goal_still_top {
        return None;
    }

    let step = current_step(runtime).cloned()?;
    let valid = revalidate_next_step(
        view,
        agent,
        &step,
        &runtime.materialization_bindings,
        action_defs,
        action_handlers,
    );
    if !valid {
        return None;
    }

    runtime.dirty = DirtySet::default();
    Some(step)
}

fn record_exhausted_goals(
    runtime: &mut AgentDecisionRuntime,
    view: &dyn GoalBeliefView,
    agent: worldwake_core::EntityId,
    recipe_registry: &RecipeRegistry,
    plans: &[(
        crate::GoalKey,
        PlanSearchResult,
        Vec<BindingRejection>,
        Vec<crate::decision_trace::SearchExpansionSummary>,
    )],
    _tick: Tick,
) {
    for (key, result, _, _) in plans {
        match result {
            crate::PlanSearchResult::BudgetExhausted { .. }
            | crate::PlanSearchResult::FrontierExhausted { .. } => {
                let (invalidation_conditions, baseline) =
                    derive_invalidation_conditions(&key.kind, agent, view, recipe_registry);
                let prev_count = runtime
                    .exhaustion_cache
                    .get(key)
                    .map_or(0, |e| e.consecutive_budget_exhaustions);
                let entry = match result {
                    crate::PlanSearchResult::BudgetExhausted { .. } => {
                        let mut e =
                            ExhaustionEntry::budget_retry_pending(invalidation_conditions, baseline);
                        e.consecutive_budget_exhaustions = prev_count.saturating_add(1);
                        e
                    }
                    crate::PlanSearchResult::FrontierExhausted { .. } => {
                        ExhaustionEntry::frontier_exhausted(invalidation_conditions, baseline)
                    }
                    crate::PlanSearchResult::Found(_) | crate::PlanSearchResult::Unsupported => {
                        unreachable!("match guard excludes non-exhaustion results")
                    }
                };
                runtime.exhaustion_cache.insert(*key, entry);
            }
            crate::PlanSearchResult::Found(_) | crate::PlanSearchResult::Unsupported => {
                runtime.exhaustion_cache.remove(key);
            }
        }
    }
}

fn has_pending_budget_retry(runtime: &AgentDecisionRuntime) -> bool {
    runtime
        .exhaustion_cache
        .values()
        .any(ExhaustionEntry::is_budget_retry_pending)
}

fn adopt_selected_plan(
    runtime: &mut AgentDecisionRuntime,
    active_goal: &mut Option<ActiveGoal>,
    jc: &mut Option<IntentionFrame>,
    ranked_candidates: &[RankedGoal],
    selected_plan: PlannedPlan,
    tick: Tick,
) {
    runtime.materialization_bindings.clear();
    *active_goal = Some(ActiveGoal {
        goal_key: selected_plan.goal,
        adopted_at: tick,
    });
    *jc = update_frame_for_adopted_plan(jc.as_ref(), &selected_plan, tick, runtime);
    runtime.current_plan = Some(selected_plan);
    runtime.current_step_index = 0;
    runtime.step_in_flight = false;
    runtime.last_priority_class = ranked_candidates
        .iter()
        .find(|candidate| {
            Some(candidate.grounded.key) == active_goal.as_ref().map(|ag| ag.goal_key)
        })
        .map(|candidate| candidate.priority_class);
}

fn clear_current_plan(
    runtime: &mut AgentDecisionRuntime,
    active_goal: &mut Option<ActiveGoal>,
    jc: &mut Option<IntentionFrame>,
    ranked_candidates: &[RankedGoal],
) {
    if jc.is_some() {
        runtime.last_frame_clear_reason = Some(worldwake_core::FrameClearReason::LostPlan);
    }
    *jc = None;
    runtime.materialization_bindings.clear();
    *active_goal = None;
    runtime.current_plan = None;
    runtime.current_step_index = 0;
    runtime.step_in_flight = false;
    runtime.last_priority_class = ranked_candidates
        .first()
        .map(|candidate| candidate.priority_class);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn plan_and_validate_next_step(
    world: &worldwake_core::World,
    scheduler: &Scheduler,
    runtime: &mut AgentDecisionRuntime,
    active_goal: &mut Option<ActiveGoal>,
    jc: &mut Option<IntentionFrame>,
    agent: worldwake_core::EntityId,
    ranked_candidates: &[RankedGoal],
    blocked_memory: &BlockedIntentMemory,
    default_switch_margin: Permille,
    frame_switch_margin: Permille,
    tick: Tick,
    budget: &PlanningBudget,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    action_defs: &worldwake_sim::ActionDefRegistry,
    action_handlers: &ActionHandlerRegistry,
    recipe_registry: &RecipeRegistry,
) -> (Option<PlannedStep>, Option<bool>) {
    // A second read view covers plan selection and step validation after the active-action fork.
    let view = runtime_belief_view(agent, world, scheduler, action_defs);
    let active_goal_key = active_goal.as_ref().map(|ag| ag.goal_key);
    let should_plan = !runtime.dirty.is_empty() || has_pending_budget_retry(runtime);
    if should_plan {
        if let Some(step) = try_continue_snapshot_plan(
            &view,
            runtime,
            ranked_candidates,
            active_goal_key,
            agent,
            action_defs,
            action_handlers,
        ) {
            return (Some(step), Some(true));
        }

        invalidate_exhausted_goals(
            &mut runtime.exhaustion_cache,
            &view,
            agent,
            view.in_transit_state(agent).is_some(),
            runtime.dirty.contains(DirtySet::FACILITIES),
            runtime.dirty.contains(DirtySet::BLOCKER_CLEANUP),
        );

        let plans = build_candidate_plans(
            world,
            scheduler,
            agent,
            ranked_candidates,
            blocked_memory,
            tick,
            budget,
            semantics_table,
            action_defs,
            action_handlers,
            recipe_registry,
            false,
            false,
            &runtime.exhaustion_cache,
        );

        // Record newly exhausted goals for next tick.
        record_exhausted_goals(runtime, &view, agent, recipe_registry, &plans, tick);
        for (key, result, _, _) in &plans {
            if result.is_found() {
                runtime.exhaustion_cache.remove(key);
            }
        }
        let plans_options = plans_as_options(&plans);

        if let Some(selected_plan) = select_best_plan(
            ranked_candidates,
            &plans_options,
            active_goal_key,
            runtime,
            jc.as_ref(),
            default_switch_margin,
            frame_switch_margin,
        ) {
            adopt_selected_plan(
                runtime,
                active_goal,
                jc,
                ranked_candidates,
                selected_plan,
                tick,
            );
        } else {
            clear_current_plan(runtime, active_goal, jc, ranked_candidates);
        }
        runtime.dirty = DirtySet::default();
    }

    let next_step = current_step(runtime).cloned();
    let next_step_valid = next_step.as_ref().map(|step| {
        revalidate_next_step(
            &view,
            agent,
            step,
            &runtime.materialization_bindings,
            action_defs,
            action_handlers,
        )
    });
    (next_step, next_step_valid)
}

/// Wrapper around `plan_and_validate_next_step` that also captures trace data.
///
/// Returns `(next_step, valid, plan_continued, plan_search_trace, selection_trace)`.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn plan_and_validate_next_step_traced(
    world: &worldwake_core::World,
    scheduler: &Scheduler,
    runtime: &mut AgentDecisionRuntime,
    active_goal: &mut Option<ActiveGoal>,
    jc: &mut Option<IntentionFrame>,
    agent: worldwake_core::EntityId,
    ranked_candidates: &[RankedGoal],
    blocked_memory: &BlockedIntentMemory,
    default_switch_margin: Permille,
    frame_switch_margin: Permille,
    tick: Tick,
    budget: &PlanningBudget,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    action_defs: &worldwake_sim::ActionDefRegistry,
    action_handlers: &ActionHandlerRegistry,
    tracing: bool,
    previous_goal: Option<worldwake_core::GoalKey>,
    recipe_registry: &RecipeRegistry,
) -> (
    Option<PlannedStep>,
    Option<bool>,
    bool,
    Option<PlanSearchTrace>,
    Option<SelectionTrace>,
) {
    if !tracing {
        let (step, valid) = plan_and_validate_next_step(
            world,
            scheduler,
            runtime,
            active_goal,
            jc,
            agent,
            ranked_candidates,
            blocked_memory,
            default_switch_margin,
            frame_switch_margin,
            tick,
            budget,
            semantics_table,
            action_defs,
            action_handlers,
            recipe_registry,
        );
        return (step, valid, false, None, None);
    }

    // Traced path: inline the logic to capture intermediate results.
    let view = runtime_belief_view(agent, world, scheduler, action_defs);
    let active_goal_key = active_goal.as_ref().map(|ag| ag.goal_key);
    let mut plan_search_trace = PlanSearchTrace {
        attempts: Vec::new(),
    };
    let mut selection_trace = SelectionTrace {
        selected: None,
        selected_plan: None,
        selected_plan_source: None,
        goal_switch: None,
        previous_goal,
        plan_replacement: None,
    };
    let mut plan_continued = false;

    let should_plan = !runtime.dirty.is_empty() || has_pending_budget_retry(runtime);
    if should_plan {
        if runtime.dirty.is_snapshot_only() && runtime.current_plan.is_some() {
            let current_goal_still_top = ranked_candidates
                .first()
                .is_some_and(|top| Some(top.grounded.key) == active_goal_key);
            if current_goal_still_top {
                if let Some(step) = current_step(runtime).cloned() {
                    let valid = revalidate_next_step(
                        &view,
                        agent,
                        &step,
                        &runtime.materialization_bindings,
                        action_defs,
                        action_handlers,
                    );
                    if valid {
                        runtime.dirty = DirtySet::default();
                        plan_continued = true;
                        selection_trace.selected = active_goal_key;
                        selection_trace.selected_plan = runtime.current_plan.as_ref().map(|plan| {
                            summarize_selected_plan(
                                plan,
                                runtime.current_step_index,
                                action_defs,
                                None,
                            )
                        });
                        selection_trace.selected_plan_source =
                            Some(SelectedPlanSource::SnapshotContinuation);
                        return (
                            Some(step),
                            Some(true),
                            plan_continued,
                            Some(plan_search_trace),
                            Some(selection_trace),
                        );
                    }
                }
            }
        }

        invalidate_exhausted_goals(
            &mut runtime.exhaustion_cache,
            &view,
            agent,
            view.in_transit_state(agent).is_some(),
            runtime.dirty.contains(DirtySet::FACILITIES),
            runtime.dirty.contains(DirtySet::BLOCKER_CLEANUP),
        );

        let plans = build_candidate_plans(
            world,
            scheduler,
            agent,
            ranked_candidates,
            blocked_memory,
            tick,
            budget,
            semantics_table,
            action_defs,
            action_handlers,
            recipe_registry,
            true,
            true,
            &runtime.exhaustion_cache,
        );

        record_exhausted_goals(runtime, &view, agent, recipe_registry, &plans, tick);
        for (key, result, _, _) in &plans {
            if result.is_found() {
                runtime.exhaustion_cache.remove(key);
            }
        }

        for (goal_key, result, rejections, expansions) in &plans {
            plan_search_trace.attempts.push(plan_search_result_to_trace(
                *goal_key,
                result,
                action_defs,
                rejections.clone(),
                expansions.clone(),
            ));
        }

        let plans_options = plans_as_options(&plans);
        let current_goal_before_selection = active_goal.as_ref().map(|ag| ag.goal_key);

        if let Some(selected_plan) = select_best_plan(
            ranked_candidates,
            &plans_options,
            current_goal_before_selection,
            runtime,
            jc.as_ref(),
            default_switch_margin,
            frame_switch_margin,
        ) {
            let selected_goal = selected_plan.goal;
            let selected_plan_source = determine_selected_plan_source(
                selected_goal,
                current_goal_before_selection,
                &plans_options,
            );
            let search_provenance =
                matches!(selected_plan_source, SelectedPlanSource::SearchSelection)
                    .then(|| summarize_search_provenance(selected_goal, &plans))
                    .flatten();
            selection_trace.selected = Some(selected_goal);
            selection_trace.selected_plan = Some(summarize_selected_plan(
                &selected_plan,
                0,
                action_defs,
                search_provenance,
            ));
            selection_trace.selected_plan_source = Some(selected_plan_source);
            selection_trace.plan_replacement = summarize_plan_replacement(
                runtime,
                current_goal_before_selection,
                selected_goal,
                &selected_plan,
                action_defs,
            );

            if let Some(prev) = previous_goal {
                if prev != selected_goal {
                    let prev_rank = ranked_candidates.iter().find(|c| c.grounded.key == prev);
                    let new_rank = ranked_candidates
                        .iter()
                        .find(|c| c.grounded.key == selected_goal);
                    let kind = match (prev_rank, new_rank) {
                        (Some(p), Some(n)) if n.priority_class > p.priority_class => {
                            crate::GoalSwitchKind::HigherPriorityGoal
                        }
                        _ => crate::GoalSwitchKind::SameClassMargin,
                    };
                    selection_trace.goal_switch = Some(GoalSwitchSummary {
                        from: prev,
                        to: selected_goal,
                        kind,
                    });
                }
            }

            runtime.materialization_bindings.clear();
            *active_goal = Some(ActiveGoal {
                goal_key: selected_plan.goal,
                adopted_at: tick,
            });
            *jc = update_frame_for_adopted_plan(jc.as_ref(), &selected_plan, tick, runtime);
            runtime.current_plan = Some(selected_plan);
            runtime.current_step_index = 0;
            runtime.step_in_flight = false;
            runtime.last_priority_class = ranked_candidates
                .iter()
                .find(|candidate| {
                    Some(candidate.grounded.key) == active_goal.as_ref().map(|ag| ag.goal_key)
                })
                .map(|candidate| candidate.priority_class);
        } else {
            if jc.is_some() {
                runtime.last_frame_clear_reason = Some(worldwake_core::FrameClearReason::LostPlan);
            }
            *jc = None;
            runtime.materialization_bindings.clear();
            *active_goal = None;
            runtime.current_plan = None;
            runtime.current_step_index = 0;
            runtime.step_in_flight = false;
            runtime.last_priority_class = ranked_candidates
                .first()
                .map(|candidate| candidate.priority_class);
        }
        runtime.dirty = DirtySet::default();
    }

    let next_step = current_step(runtime).cloned();
    let next_step_valid = next_step.as_ref().map(|step| {
        revalidate_next_step(
            &view,
            agent,
            step,
            &runtime.materialization_bindings,
            action_defs,
            action_handlers,
        )
    });

    (
        next_step,
        next_step_valid,
        plan_continued,
        Some(plan_search_trace),
        Some(selection_trace),
    )
}

/// Convert a `PlanSearchResult` into a `PlanAttemptTrace` for the trace model.
pub(super) fn plan_search_result_to_trace(
    goal: worldwake_core::GoalKey,
    result: &PlanSearchResult,
    action_defs: &worldwake_sim::ActionDefRegistry,
    binding_rejections: Vec<BindingRejection>,
    expansion_summaries: Vec<crate::decision_trace::SearchExpansionSummary>,
) -> PlanAttemptTrace {
    let outcome = match result {
        PlanSearchResult::Found(plan) => PlanSearchOutcome::Found {
            steps: plan
                .steps
                .iter()
                .map(|s| summarize_step(s, action_defs))
                .collect(),
            terminal_kind: plan.terminal_kind,
        },
        PlanSearchResult::Unsupported => PlanSearchOutcome::Unsupported,
        PlanSearchResult::BudgetExhausted { expansions_used } => {
            PlanSearchOutcome::BudgetExhausted {
                expansions_used: *expansions_used,
            }
        }
        PlanSearchResult::FrontierExhausted { expansions_used } => {
            PlanSearchOutcome::FrontierExhausted {
                expansions_used: *expansions_used,
            }
        }
    };
    PlanAttemptTrace {
        goal,
        outcome,
        binding_rejections,
        expansion_summaries,
    }
}

#[cfg(test)]
mod tests {
    use super::{has_pending_budget_retry, record_exhausted_goals};
    use crate::{
        AgentDecisionRuntime, ExhaustionEntry, ExhaustionInvalidationCondition,
        ExhaustionRetryState, GoalKey, GoalKind, PlanSearchResult, PlanTerminalKind, PlannedPlan,
    };
    use worldwake_core::{
        build_prototype_world, CauseRef, CommodityKind, ControlSource, EventLog,
        HomeostaticNeeds, Quantity, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
    };
    use worldwake_sim::{PerAgentBeliefView, RecipeRegistry};

    fn consume_goal(commodity: CommodityKind) -> GoalKey {
        GoalKey::from(GoalKind::ConsumeOwnedCommodity { commodity })
    }

    fn found_plan(goal: GoalKey) -> PlannedPlan {
        PlannedPlan::new(goal, Vec::new(), PlanTerminalKind::GoalSatisfied)
    }

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        )
    }

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut event_log = EventLog::new();
        let _ = txn.commit(&mut event_log);
    }

    fn setup_agent_world() -> (World, worldwake_core::EntityId, worldwake_core::EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Planner", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_homeostatic_needs(agent, HomeostaticNeeds::new_sated())
                .unwrap();
            commit_txn(txn);
            agent
        };

        (world, agent, place)
    }

    #[test]
    fn record_exhausted_goals_replaces_frontier_suppression_with_budget_retry_state() {
        let goal = consume_goal(CommodityKind::Bread);
        let mut runtime = AgentDecisionRuntime::default();
        runtime.exhaustion_cache.insert(
            goal,
            ExhaustionEntry {
                retry_state: ExhaustionRetryState::FrontierExhausted,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                consecutive_budget_exhaustions: 0,
            },
        );

        let plans = vec![(
            goal,
            PlanSearchResult::BudgetExhausted {
                expansions_used: 12,
            },
            Vec::new(),
            Vec::new(),
        )];
        let (world, agent, _) = setup_agent_world();
        let view = PerAgentBeliefView::from_world(agent, &world);
        record_exhausted_goals(
            &mut runtime,
            &view,
            agent,
            &RecipeRegistry::new(),
            &plans,
            Tick(9),
        );

        let entry = runtime.exhaustion_cache.get(&goal).unwrap();
        assert_eq!(entry.retry_state, ExhaustionRetryState::BudgetRetryPending);
        assert_eq!(
            entry.invalidation_conditions,
            vec![ExhaustionInvalidationCondition::CommodityChanged(
                CommodityKind::Bread
            )]
        );
    }

    #[test]
    fn record_exhausted_goals_derives_goal_aware_conditions_and_baseline() {
        let goal = consume_goal(CommodityKind::Bread);
        let mut runtime = AgentDecisionRuntime::default();
        let (mut world, agent, place) = setup_agent_world();
        {
            let mut txn = new_txn(&mut world, 2);
            let bread = txn.create_item_lot(CommodityKind::Bread, Quantity(2)).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_possessor(bread, agent).unwrap();
            commit_txn(txn);
        }
        let view = PerAgentBeliefView::from_world(agent, &world);

        let plans = vec![(
            goal,
            PlanSearchResult::BudgetExhausted {
                expansions_used: 12,
            },
            Vec::new(),
            Vec::new(),
        )];
        record_exhausted_goals(
            &mut runtime,
            &view,
            agent,
            &RecipeRegistry::new(),
            &plans,
            Tick(9),
        );

        let entry = runtime.exhaustion_cache.get(&goal).unwrap();
        assert_eq!(entry.retry_state, ExhaustionRetryState::BudgetRetryPending);
        assert_eq!(
            entry.invalidation_conditions,
            vec![ExhaustionInvalidationCondition::CommodityChanged(
                CommodityKind::Bread
            )]
        );
        assert_eq!(
            entry.baseline.commodity_quantities,
            vec![(CommodityKind::Bread, Quantity(2))]
        );
        assert_eq!(entry.baseline.position, Some(place));
    }

    #[test]
    fn record_exhausted_goals_removes_only_successful_goal_entry() {
        let solved_goal = consume_goal(CommodityKind::Bread);
        let retained_goal = consume_goal(CommodityKind::Water);
        let mut runtime = AgentDecisionRuntime::default();
        runtime.exhaustion_cache.insert(
            solved_goal,
            ExhaustionEntry {
                retry_state: ExhaustionRetryState::FrontierExhausted,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                consecutive_budget_exhaustions: 0,
            },
        );
        runtime.exhaustion_cache.insert(
            retained_goal,
            ExhaustionEntry {
                retry_state: ExhaustionRetryState::BudgetRetryPending,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                consecutive_budget_exhaustions: 0,
            },
        );

        let plans = vec![(
            solved_goal,
            PlanSearchResult::Found(found_plan(solved_goal)),
            Vec::new(),
            Vec::new(),
        )];
        let (world, agent, _) = setup_agent_world();
        let view = PerAgentBeliefView::from_world(agent, &world);

        record_exhausted_goals(
            &mut runtime,
            &view,
            agent,
            &RecipeRegistry::new(),
            &plans,
            Tick(10),
        );

        assert!(!runtime.exhaustion_cache.contains_key(&solved_goal));
        assert_eq!(
            runtime.exhaustion_cache.get(&retained_goal),
            Some(&ExhaustionEntry {
                retry_state: ExhaustionRetryState::BudgetRetryPending,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                consecutive_budget_exhaustions: 0,
            })
        );
    }

    #[test]
    fn record_exhausted_goals_records_frontier_exhaustion_as_suppressing_retry_state() {
        let goal = consume_goal(CommodityKind::Bread);
        let mut runtime = AgentDecisionRuntime::default();
        let plans = vec![(
            goal,
            PlanSearchResult::FrontierExhausted { expansions_used: 12 },
            Vec::new(),
            Vec::new(),
        )];
        let (world, agent, _) = setup_agent_world();
        let view = PerAgentBeliefView::from_world(agent, &world);

        record_exhausted_goals(
            &mut runtime,
            &view,
            agent,
            &RecipeRegistry::new(),
            &plans,
            Tick(9),
        );

        let entry = runtime.exhaustion_cache.get(&goal).unwrap();
        assert_eq!(entry.retry_state, ExhaustionRetryState::FrontierExhausted);
        assert!(entry.suppresses_planning());
    }

    #[test]
    fn frontier_exhaustion_suppresses_planning_but_budget_retry_does_not() {
        let frontier_entry = ExhaustionEntry {
            retry_state: ExhaustionRetryState::FrontierExhausted,
            invalidation_conditions: Vec::new(),
            baseline: crate::ExhaustionBaseline::default(),
            consecutive_budget_exhaustions: 0,
        };
        let entry = ExhaustionEntry {
            retry_state: ExhaustionRetryState::BudgetRetryPending,
            invalidation_conditions: Vec::new(),
            baseline: crate::ExhaustionBaseline::default(),
            consecutive_budget_exhaustions: 0,
        };

        assert!(frontier_entry.suppresses_planning());
        assert!(!entry.suppresses_planning());
        assert!(entry.is_budget_retry_pending());
    }

    #[test]
    fn has_pending_budget_retry_detects_retryable_budget_entries() {
        let mut runtime = AgentDecisionRuntime::default();
        runtime.exhaustion_cache.insert(
            consume_goal(CommodityKind::Bread),
            ExhaustionEntry {
                retry_state: ExhaustionRetryState::BudgetRetryPending,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                consecutive_budget_exhaustions: 0,
            },
        );
        runtime.exhaustion_cache.insert(
            consume_goal(CommodityKind::Water),
            ExhaustionEntry {
                retry_state: ExhaustionRetryState::FrontierExhausted,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                consecutive_budget_exhaustions: 0,
            },
        );

        assert!(has_pending_budget_retry(&runtime));

        runtime.exhaustion_cache.insert(
            consume_goal(CommodityKind::Apple),
            ExhaustionEntry {
                retry_state: ExhaustionRetryState::FrontierExhausted,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                consecutive_budget_exhaustions: 0,
            },
        );
        runtime
            .exhaustion_cache
            .retain(|_, entry| !entry.is_budget_retry_pending());

        assert!(!has_pending_budget_retry(&runtime));
    }
}
