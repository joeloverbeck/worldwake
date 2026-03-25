use crate::decision_trace::{
    BindingRejection, GoalSwitchSummary, PlanAttemptTrace, PlanSearchOutcome,
    PlanSearchTrace, PlannedStepSummary, RankedGoalSummary, SelectedPlanReplacementKind,
    SelectedPlanReplacementTrace, SelectedPlanSearchProvenance, SelectedPlanSource,
    SelectedPlanTrace, SelectionTrace,
};
use crate::search::PlanSearchResult;
use crate::{
    authoritative_target, build_planning_snapshot_with_blocked_facility_uses, revalidate_next_step,
    search_plan, select_best_plan, AgentDecisionRuntime, DirtySet, PlannedPlan,
    PlannedStep, PlannerOpSemantics, PlanningBudget, RankedGoal,
};
use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    ActionDefId, ActiveGoal, BlockedIntentMemory, IntentionFrame, Permille, Tick,
};
use worldwake_sim::{ActionHandlerRegistry, RecipeRegistry, Scheduler};

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
) -> Vec<(
    crate::GoalKey,
    PlanSearchResult,
    Vec<BindingRejection>,
    Vec<crate::decision_trace::SearchExpansionSummary>,
)> {
    let view = runtime_belief_view(agent, world, scheduler, action_defs);
    let candidates_to_plan: Vec<_> = ranked_candidates
        .iter()
        .take(usize::from(budget.max_candidates_to_plan))
        .collect();

    // Build a single merged snapshot with the union of all candidates' evidence
    // sets. This avoids N separate snapshot constructions (each with BFS +
    // Floyd-Warshall + entity queries) when N candidates share similar evidence.
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

    candidates_to_plan
        .into_iter()
        .map(|ranked| {
            let mut rejections = Vec::new();
            let mut expansions = Vec::new();
            let result = search_plan(
                &snapshot,
                &ranked.grounded,
                semantics_table,
                action_defs,
                action_handlers,
                budget,
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
            (ranked.grounded.key, result, rejections, expansions)
        })
        .collect()
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
    if !runtime.dirty.is_empty() {
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
                        return (Some(step), Some(true));
                    }
                }
            }
        }

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
        );
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
                .find(|candidate| Some(candidate.grounded.key) == active_goal.as_ref().map(|ag| ag.goal_key))
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

    if !runtime.dirty.is_empty() {
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
        );

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
            selection_trace.plan_replacement =
                summarize_plan_replacement(runtime, current_goal_before_selection, selected_goal, &selected_plan, action_defs);

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
                .find(|candidate| Some(candidate.grounded.key) == active_goal.as_ref().map(|ag| ag.goal_key))
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
