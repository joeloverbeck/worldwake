use crate::GoalKindPlannerExt;
use crate::candidate_generation::relieved_needs_for_commodity;
use crate::decision_trace::{
    BindingRejection, GoalSwitchSummary, PlanAttemptTrace, PlanSearchOutcome, PlanSearchTrace,
    PlannedStepSummary, RankedGoalSummary, SameGoalPlanningStopReason, SameGoalPlanningTrace,
    SelectedPlanReplacementKind, SelectedPlanReplacementTrace, SelectedPlanSearchProvenance,
    SelectedPlanSource, SelectedPlanTrace, SelectionTrace, SideBenefitTrace,
    SnapshotContinuationOutcome, SnapshotContinuationTrace, StrategicStepTrace,
    TargetBeliefPresence,
};
use crate::exhaustion::{derive_invalidation_conditions, invalidate_exhausted_goals};
use crate::perf_telemetry::record_planning_phase_duration;
use crate::plan_selection::SelectionCandidatePlan;
use crate::search::{PlanSearchResult, SearchTraceMetadata, search_plan_with_trace_metadata};
use crate::{
    AgentDecisionRuntime, DirtySet, ExhaustionEntry, ExhaustionRetryState, OpportunityKey,
    PlanValue, PlannedPlan, PlannedStep, PlannerOpSemantics, RankedGoal, authoritative_target,
    build_planning_snapshot_with_blocked_facility_uses, revalidate_next_step, select_best_plan,
};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;
use worldwake_core::{
    ActionDefId, ActiveGoal, BlockerMemory, CognitiveProfile, EntityId, ExecutionBudget, GoalKind,
    IntentionFrame, Permille, Tick,
};
use worldwake_sim::{
    ActionHandlerRegistry, GoalBeliefView, RecipeRegistry, RuntimeBeliefView, Scheduler,
    SpatialBeliefView,
};

use super::{current_step, runtime_belief_view, update_frame_for_adopted_plan};

#[derive(Clone, Debug)]
pub(crate) struct CandidatePlanSearch {
    pub opportunity: OpportunityKey,
    pub result: PlanSearchResult,
    pub perceived_cost: Option<u32>,
    pub trace_metadata: SearchTraceMetadata,
    pub binding_rejections: Vec<BindingRejection>,
    pub expansion_summaries: Vec<crate::decision_trace::SearchExpansionSummary>,
}

impl CandidatePlanSearch {
    fn selected_plan(&self) -> Option<&PlannedPlan> {
        match &self.result {
            PlanSearchResult::Found(plan) => Some(plan.as_ref()),
            PlanSearchResult::Unsupported
            | PlanSearchResult::BudgetExhausted { .. }
            | PlanSearchResult::FrontierExhausted { .. } => None,
        }
    }

    fn selection_candidate(&self) -> SelectionCandidatePlan {
        SelectionCandidatePlan {
            searched_opportunity: self.opportunity,
            found_plan: self.selected_plan().cloned(),
            perceived_cost: self.perceived_cost,
        }
    }
}

type PlanningStepTraceResult = (
    Option<PlannedStep>,
    Option<bool>,
    bool,
    Option<PlanSearchTrace>,
    Option<SelectionTrace>,
    BTreeSet<worldwake_core::HomeostaticNeedId>,
);

fn found_plan_blocks_later_goals(plan: &PlannedPlan) -> bool {
    match plan.terminal_kind {
        crate::PlanTerminalKind::GoalSatisfied | crate::PlanTerminalKind::CombatCommitment => true,
        crate::PlanTerminalKind::ProgressBarrier => {
            !matches!(plan.goal.kind, GoalKind::InvestigateViolation { .. })
        }
    }
}

fn perceived_selection_cost(snapshot: &crate::PlanningSnapshot, plan: &PlannedPlan) -> Option<u32> {
    let state = crate::PlanningState::new(snapshot);
    let mut current_place = SpatialBeliefView::effective_place(&state, snapshot.actor())?;
    let mut total = 0u32;

    for step in &plan.steps {
        let step_cost = if step.op_kind == crate::PlannerOpKind::Travel {
            let destination = step
                .targets
                .first()
                .copied()
                .and_then(crate::authoritative_target)?;
            let cost = snapshot
                .direct_perceived_travel_cost(current_place, destination)
                .unwrap_or(step.estimated_ticks);
            current_place = destination;
            cost
        } else {
            step.estimated_ticks
        };
        total = total.checked_add(step_cost)?;
    }

    Some(total)
}

/// Build a `PlannedStepSummary` from a `PlannedStep` for trace output.
pub(super) fn summarize_step(
    step: &PlannedStep,
    action_defs: &worldwake_sim::ActionDefRegistry,
) -> PlannedStepSummary {
    let action_def = action_defs.get(step.def_id);
    let action_name = action_def.map_or_else(|| "unknown".to_owned(), |def| def.name.clone());
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
        binding_strictness: action_def.map(|def| def.binding_strictness),
    }
}

pub(super) fn summarize_selected_plan(
    plan: &PlannedPlan,
    current_step_index: usize,
    action_defs: &worldwake_sim::ActionDefRegistry,
    search_provenance: Option<SelectedPlanSearchProvenance>,
    plan_value: &PlanValue,
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
        primary_motive: plan_value.primary_motive,
        total_value: plan_value.total_value,
        side_benefits: plan_value
            .side_benefits
            .iter()
            .map(SideBenefitTrace::from)
            .collect(),
    }
}

fn selected_plan_value(
    ranked_candidates: &[RankedGoal],
    selected_plan: &PlannedPlan,
    side_benefit_weight: Permille,
) -> Option<PlanValue> {
    let ranked = ranked_candidates.iter().find(|candidate| {
        candidate.grounded.key == selected_plan.opportunity.goal_key
            && candidate.grounded.anchor == selected_plan.opportunity.anchor
    })?;
    Some(crate::build_plan_value(
        selected_plan.clone(),
        ranked.priority_class,
        ranked.motive_score,
        ranked_candidates,
        side_benefit_weight,
    ))
}

pub(super) fn summarize_search_provenance(
    plans: &[CandidatePlanSearch],
    selected_opportunity: OpportunityKey,
) -> Option<SelectedPlanSearchProvenance> {
    let selected_plan = plans.iter().find(|plan| {
        plan.opportunity == selected_opportunity
            && matches!(plan.result, PlanSearchResult::Found(_))
    })?;
    let expansions = &selected_plan.expansion_summaries;
    let root = expansions.first();
    Some(SelectedPlanSearchProvenance {
        expansions_used: expansions.len() as u16,
        root_remaining_travel_ticks: root.map_or(0, |summary| summary.remaining_travel_ticks),
        root_travel_pruning: root.and_then(|summary| summary.travel_pruning.clone()),
        selected_root_travel_destination: selected_plan
            .selected_plan()
            .and_then(|plan| plan.steps.first())
            .filter(|step| step.op_kind == crate::PlannerOpKind::Travel)
            .and_then(|step| step.targets.first().copied())
            .and_then(crate::authoritative_target),
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
    let previous_opportunity = runtime.current_plan.as_ref().map(|plan| plan.opportunity);
    let previous_next_step = current_step(runtime).map(|step| summarize_step(step, action_defs));
    let new_next_step = selected_plan
        .steps
        .first()
        .map(|step| summarize_step(step, action_defs));
    Some(SelectedPlanReplacementTrace {
        previous_goal,
        new_goal: selected_goal,
        previous_next_step,
        new_next_step,
        kind: if previous_goal == selected_goal {
            if previous_opportunity == Some(selected_plan.opportunity) {
                SelectedPlanReplacementKind::SameGoalBranchRefreshed
            } else {
                SelectedPlanReplacementKind::SameGoalSiblingReplaced
            }
        } else {
            SelectedPlanReplacementKind::GoalChanged
        },
    })
}

pub(super) fn summarize_ranked_goal(ranked: &RankedGoal) -> RankedGoalSummary {
    RankedGoalSummary {
        opportunity: OpportunityKey {
            goal_key: ranked.grounded.key,
            anchor: ranked.grounded.anchor,
        },
        priority_class: ranked.priority_class,
        motive_score: ranked.motive_score,
        provenance: ranked.provenance.clone(),
        source_reliability_discount: ranked.source_reliability_discount.clone(),
        competition_discount: ranked.competition_discount.clone(),
        feasibility: ranked.feasibility,
    }
}

pub(super) fn determine_selected_plan_source(
    selected_opportunity: OpportunityKey,
    current_goal_before_selection: Option<worldwake_core::GoalKey>,
    plans: &[SelectionCandidatePlan],
) -> SelectedPlanSource {
    if plans
        .iter()
        .any(|plan| plan.searched_opportunity == selected_opportunity && plan.found_plan.is_some())
    {
        SelectedPlanSource::SearchSelection
    } else {
        debug_assert_eq!(
            current_goal_before_selection,
            Some(selected_opportunity.goal_key)
        );
        SelectedPlanSource::RetainedCurrentPlan
    }
}

#[allow(clippy::too_many_arguments, clippy::trivially_copy_pass_by_ref)]
pub(super) fn build_candidate_plans(
    world: &worldwake_core::World,
    scheduler: &Scheduler,
    agent: worldwake_core::EntityId,
    ranked_candidates: &[RankedGoal],
    committed_opportunity: Option<OpportunityKey>,
    blocked_memory: &BlockerMemory,
    current_tick: Tick,
    cognitive: &CognitiveProfile,
    execution_budget: &ExecutionBudget,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    action_defs: &worldwake_sim::ActionDefRegistry,
    action_handlers: &ActionHandlerRegistry,
    recipe_registry: &RecipeRegistry,
    collect_rejections: bool,
    collect_expansion_summaries: bool,
    exhaustion_cache: &std::collections::BTreeMap<OpportunityKey, ExhaustionEntry>,
) -> Vec<CandidatePlanSearch> {
    let view = runtime_belief_view(agent, world, scheduler, action_defs, recipe_registry);
    let admitted_candidates: Vec<_> = ranked_candidates
        .iter()
        .filter(|c| {
            opportunity_admitted_by_exhaustion(
                exhaustion_cache,
                OpportunityKey {
                    goal_key: c.grounded.key,
                    anchor: c.grounded.anchor,
                },
                current_tick,
            )
        })
        .collect();
    let admitted_candidates =
        prioritize_same_goal_replan_candidates(admitted_candidates, committed_opportunity);
    let candidates_to_plan: Vec<_> = admitted_candidates
        .into_iter()
        .take(usize::from(cognitive.max_candidates_to_plan))
        .collect();

    // All candidates filtered by exhausted-goal skip set — no snapshot needed.
    if candidates_to_plan.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::with_capacity(candidates_to_plan.len());
    let mut continue_same_goal_after_found = None;
    for ranked in candidates_to_plan {
        if let Some(found_goal) = continue_same_goal_after_found
            && ranked.grounded.key != found_goal
        {
            break;
        }
        let mut rejections = Vec::new();
        let mut expansions = Vec::new();
        let mut trace_metadata = SearchTraceMetadata::default();
        let snapshot = build_planning_snapshot_with_blocked_facility_uses(
            &view,
            agent,
            &ranked.grounded.evidence_entities,
            &ranked.grounded.evidence_places,
            cognitive.snapshot_travel_horizon,
            blocked_memory,
            current_tick,
            ranked.grounded.key.kind.relevant_op_kinds(),
            cognitive.max_snapshot_entities_per_place,
        );
        let opportunity = OpportunityKey {
            goal_key: ranked.grounded.key,
            anchor: ranked.grounded.anchor,
        };
        // Apply search budget backoff for goals with 3+ consecutive exhaustion
        // failures. Each failure beyond the 2nd halves the budget (floor 16).
        // The first two retries use full budget to give goals a fair chance
        // after conditions may have changed; subsequent retries reduce budget
        // for chronically unsolvable goals.
        let effective_cognitive = match exhaustion_cache.get(&opportunity) {
            Some(entry) if entry.consecutive_failures >= 3 => {
                let shift = u32::from(entry.consecutive_failures.saturating_sub(2).min(4));
                let reduced = cognitive.max_node_expansions >> shift;
                let mut c = *cognitive;
                c.max_node_expansions = reduced.max(16);
                c
            }
            _ => *cognitive,
        };
        let result = search_plan_with_trace_metadata(
            &snapshot,
            &ranked.grounded,
            semantics_table,
            action_defs,
            action_handlers,
            &effective_cognitive,
            execution_budget,
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
            Some(&mut trace_metadata),
        );
        let found_blocks_later_goals = match &result {
            PlanSearchResult::Found(plan) => found_plan_blocks_later_goals(plan),
            PlanSearchResult::Unsupported
            | PlanSearchResult::BudgetExhausted { .. }
            | PlanSearchResult::FrontierExhausted { .. } => false,
        };
        let perceived_cost = match &result {
            PlanSearchResult::Found(plan) => perceived_selection_cost(&snapshot, plan),
            PlanSearchResult::Unsupported
            | PlanSearchResult::BudgetExhausted { .. }
            | PlanSearchResult::FrontierExhausted { .. } => None,
        };
        results.push(CandidatePlanSearch {
            opportunity,
            result,
            perceived_cost,
            trace_metadata,
            binding_rejections: rejections,
            expansion_summaries: expansions,
        });
        if found_blocks_later_goals {
            continue_same_goal_after_found = Some(opportunity.goal_key);
        }
    }
    results
}

fn prioritize_same_goal_replan_candidates(
    candidates: Vec<&RankedGoal>,
    committed_opportunity: Option<OpportunityKey>,
) -> Vec<&RankedGoal> {
    let Some(committed_opportunity) = committed_opportunity else {
        return candidates;
    };
    if !candidates.iter().any(|candidate| {
        candidate.grounded.key == committed_opportunity.goal_key
            && candidate.grounded.anchor == committed_opportunity.anchor
    }) {
        return candidates;
    }

    let mut preferred = Vec::new();
    let mut same_goal = Vec::new();
    let mut rest = Vec::new();

    for candidate in candidates {
        let opportunity = OpportunityKey {
            goal_key: candidate.grounded.key,
            anchor: candidate.grounded.anchor,
        };
        if opportunity == committed_opportunity {
            preferred.push(candidate);
        } else if candidate.grounded.key == committed_opportunity.goal_key {
            same_goal.push(candidate);
        } else {
            rest.push(candidate);
        }
    }

    preferred.into_iter().chain(same_goal).chain(rest).collect()
}

fn opportunity_admitted_by_exhaustion(
    exhaustion_cache: &std::collections::BTreeMap<OpportunityKey, ExhaustionEntry>,
    opportunity: OpportunityKey,
    current_tick: Tick,
) -> bool {
    match exhaustion_cache.get(&opportunity) {
        Some(entry) if entry.suppresses_planning() => false,
        Some(entry) if !entry.is_retry_eligible(current_tick) => false,
        _ => true,
    }
}

pub(super) fn summarize_same_goal_planning_trace(
    ranked_candidates: &[RankedGoal],
    cognitive: &CognitiveProfile,
    current_tick: Tick,
    exhaustion_cache: &std::collections::BTreeMap<OpportunityKey, ExhaustionEntry>,
    plans: &[CandidatePlanSearch],
) -> Option<SameGoalPlanningTrace> {
    if plans.is_empty() {
        return None;
    }

    let admitted_candidates: Vec<_> = ranked_candidates
        .iter()
        .filter(|candidate| {
            opportunity_admitted_by_exhaustion(
                exhaustion_cache,
                OpportunityKey {
                    goal_key: candidate.grounded.key,
                    anchor: candidate.grounded.anchor,
                },
                current_tick,
            )
        })
        .collect();
    let admitted_cap = usize::from(cognitive.max_candidates_to_plan);
    let candidate_cap_hit = admitted_candidates.len() > admitted_cap;
    let continuation_trigger = plans
        .iter()
        .find(|plan| plan.result.is_found())
        .map(|plan| plan.opportunity);
    let stop_reason =
        if let Some(found_goal) = continuation_trigger.map(|opportunity| opportunity.goal_key) {
            if let Some(next_candidate) = admitted_candidates
                .into_iter()
                .take(admitted_cap)
                .skip(plans.len())
                .find(|candidate| candidate.grounded.key != found_goal)
            {
                SameGoalPlanningStopReason::EncounteredDifferentGoal {
                    next_goal: next_candidate.grounded.key,
                }
            } else if candidate_cap_hit {
                SameGoalPlanningStopReason::ReachedCandidatePlanCap
            } else {
                SameGoalPlanningStopReason::ExhaustedAdmittedOpportunities
            }
        } else if candidate_cap_hit {
            SameGoalPlanningStopReason::ReachedCandidatePlanCap
        } else {
            SameGoalPlanningStopReason::ExhaustedAdmittedOpportunities
        };

    Some(SameGoalPlanningTrace {
        continuation_trigger,
        stop_reason,
    })
}

pub(super) fn selection_candidates(plans: &[CandidatePlanSearch]) -> Vec<SelectionCandidatePlan> {
    plans
        .iter()
        .map(CandidatePlanSearch::selection_candidate)
        .collect()
}

fn ranked_goal_for_opportunity(
    ranked_candidates: &[RankedGoal],
    opportunity: OpportunityKey,
) -> Option<&RankedGoal> {
    ranked_candidates.iter().find(|candidate| {
        candidate.grounded.key == opportunity.goal_key
            && candidate.grounded.anchor == opportunity.anchor
    })
}

fn summarize_snapshot_continuation(
    current_opportunity: OpportunityKey,
    ranked_candidates: &[RankedGoal],
    planning_switch_margin: Permille,
) -> SnapshotContinuationTrace {
    let top = ranked_candidates.first();
    let current = ranked_goal_for_opportunity(ranked_candidates, current_opportunity);
    let top_opportunity = top.map(|ranked| OpportunityKey {
        goal_key: ranked.grounded.key,
        anchor: ranked.grounded.anchor,
    });
    let motive_delta = top
        .zip(current)
        .map(|(top, current)| top.motive_score.saturating_sub(current.motive_score));

    let outcome = match (top, current) {
        (_, None) | (None, Some(_)) => {
            SnapshotContinuationOutcome::ReplannedCurrentOpportunityMissing
        }
        (Some(top), Some(_)) if top_opportunity == Some(current_opportunity) => {
            SnapshotContinuationOutcome::ContinuedAsTopRanked
        }
        (Some(top), Some(current)) if top.priority_class > current.priority_class => {
            SnapshotContinuationOutcome::ReplannedHigherPriorityClass
        }
        (Some(top), Some(current))
            if top.motive_score
                >= current.motive_score + u32::from(planning_switch_margin.value()) =>
        {
            SnapshotContinuationOutcome::ReplannedMarginExceeded
        }
        (Some(_), Some(_)) => SnapshotContinuationOutcome::ContinuedWithinMargin,
    };

    SnapshotContinuationTrace {
        current_opportunity,
        current_priority_class: current.map(|ranked| ranked.priority_class),
        current_motive_score: current.map(|ranked| ranked.motive_score),
        top_opportunity,
        top_priority_class: top.map(|ranked| ranked.priority_class),
        top_motive_score: top.map(|ranked| ranked.motive_score),
        planning_switch_margin,
        motive_delta,
        outcome,
    }
}

fn try_continue_snapshot_plan(
    view: &impl RuntimeBeliefView,
    runtime: &mut AgentDecisionRuntime,
    ranked_candidates: &[RankedGoal],
    planning_switch_margin: Permille,
    agent: worldwake_core::EntityId,
    action_defs: &worldwake_sim::ActionDefRegistry,
    action_handlers: &ActionHandlerRegistry,
) -> Option<PlannedStep> {
    if !runtime.dirty.is_snapshot_only() || runtime.current_plan.is_none() {
        return None;
    }

    let plan = runtime.current_plan.as_ref()?;
    let continuation = summarize_snapshot_continuation(
        plan.opportunity,
        ranked_candidates,
        planning_switch_margin,
    );
    if !continuation.continues_plan() {
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
    plans: &[CandidatePlanSearch],
    tick: Tick,
    cognitive: &CognitiveProfile,
) -> BTreeSet<worldwake_core::HomeostaticNeedId> {
    let mut pending_tracker_increments = BTreeSet::new();
    for plan in plans {
        match &plan.result {
            crate::PlanSearchResult::BudgetExhausted { .. }
            | crate::PlanSearchResult::FrontierExhausted { .. } => {
                if matches!(plan.result, crate::PlanSearchResult::BudgetExhausted { .. })
                    && let Some(commodity) = plan
                        .opportunity
                        .goal_key
                        .kind
                        .target_commodity(recipe_registry)
                {
                    pending_tracker_increments.extend(relieved_needs_for_commodity(commodity));
                }
                let (invalidation_conditions, baseline) = derive_invalidation_conditions(
                    &plan.opportunity.goal_key.kind,
                    agent,
                    view,
                    recipe_registry,
                );
                let entry = match &plan.result {
                    crate::PlanSearchResult::BudgetExhausted { .. } => {
                        match runtime.exhaustion_cache.get(&plan.opportunity) {
                            Some(existing)
                                if existing.retry_state
                                    == ExhaustionRetryState::BudgetRetryPending =>
                            {
                                let mut entry = existing.clone();
                                entry.invalidation_conditions = invalidation_conditions;
                                entry.baseline = baseline;
                                entry.record_budget_exhaustion(tick, cognitive);
                                entry
                            }
                            _ => ExhaustionEntry::budget_retry_pending(
                                invalidation_conditions,
                                baseline,
                                tick,
                                cognitive,
                            ),
                        }
                    }
                    crate::PlanSearchResult::FrontierExhausted { .. } => frontier_exhaustion_entry(
                        &plan.opportunity.goal_key.kind,
                        invalidation_conditions,
                        baseline,
                        tick,
                        cognitive,
                    ),
                    crate::PlanSearchResult::Found(_) | crate::PlanSearchResult::Unsupported => {
                        unreachable!("match guard excludes non-exhaustion results")
                    }
                };
                runtime.exhaustion_cache.insert(plan.opportunity, entry);
            }
            crate::PlanSearchResult::Found(_) | crate::PlanSearchResult::Unsupported => {
                runtime.exhaustion_cache.remove(&plan.opportunity);
            }
        }
    }
    pending_tracker_increments
}

fn frontier_exhaustion_entry(
    goal_kind: &GoalKind,
    invalidation_conditions: Vec<crate::ExhaustionInvalidationCondition>,
    baseline: crate::ExhaustionBaseline,
    tick: Tick,
    cognitive: &CognitiveProfile,
) -> ExhaustionEntry {
    match goal_kind {
        // Sleep is a direct local self-care action. If a single search pass
        // exhausts its frontier, suppressing it until a band/position change
        // can strand the agent inside one authored critical band.
        GoalKind::Sleep => ExhaustionEntry::budget_retry_pending(
            invalidation_conditions,
            baseline,
            tick,
            cognitive,
        ),
        _ => ExhaustionEntry::frontier_exhausted(invalidation_conditions, baseline),
    }
}

fn has_pending_budget_retry(runtime: &AgentDecisionRuntime, current_tick: Tick) -> bool {
    runtime
        .exhaustion_cache
        .values()
        .any(|entry| entry.is_retry_eligible(current_tick))
}

fn adopt_selected_plan(
    runtime: &mut AgentDecisionRuntime,
    active_goal: &mut Option<ActiveGoal>,
    jc: &mut Option<IntentionFrame>,
    facility_intents: &mut worldwake_core::ContentionIntents,
    ranked_candidates: &[RankedGoal],
    selected_plan: PlannedPlan,
    tick: Tick,
) {
    runtime.materialization_bindings.clear();
    facility_intents.intents.clear();
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
    facility_intents: &mut worldwake_core::ContentionIntents,
    ranked_candidates: &[RankedGoal],
) {
    if jc.is_some() {
        runtime.last_frame_clear_reason = Some(worldwake_core::FrameClearReason::LostPlan);
    }
    *jc = None;
    runtime.materialization_bindings.clear();
    facility_intents.intents.clear();
    *active_goal = None;
    runtime.current_plan = None;
    runtime.current_step_index = 0;
    runtime.step_in_flight = false;
    runtime.last_priority_class = ranked_candidates
        .first()
        .map(|candidate| candidate.priority_class);
}

#[allow(clippy::too_many_arguments, clippy::trivially_copy_pass_by_ref)]
pub(super) fn plan_and_validate_next_step(
    world: &worldwake_core::World,
    scheduler: &Scheduler,
    runtime: &mut AgentDecisionRuntime,
    active_goal: &mut Option<ActiveGoal>,
    jc: &mut Option<IntentionFrame>,
    facility_intents: &mut worldwake_core::ContentionIntents,
    agent: worldwake_core::EntityId,
    ranked_candidates: &[RankedGoal],
    blocked_memory: &BlockerMemory,
    default_switch_margin: Permille,
    frame_switch_margin: Permille,
    side_benefit_weight: Permille,
    tick: Tick,
    cognitive: &CognitiveProfile,
    execution_budget: &ExecutionBudget,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    action_defs: &worldwake_sim::ActionDefRegistry,
    action_handlers: &ActionHandlerRegistry,
    recipe_registry: &RecipeRegistry,
) -> (Option<PlannedStep>, Option<bool>) {
    let planning_start = Instant::now();
    let result = (|| {
        // A second read view covers plan selection and step validation after the active-action fork.
        let view = runtime_belief_view(agent, world, scheduler, action_defs, recipe_registry);
        let active_goal_key = active_goal.as_ref().map(|ag| ag.goal_key);
        let committed_opportunity = runtime
            .current_plan
            .as_ref()
            .map(|plan| plan.opportunity)
            .filter(|opportunity| Some(opportunity.goal_key) == active_goal_key);
        let should_plan = !runtime.dirty.is_empty() || has_pending_budget_retry(runtime, tick);
        if should_plan {
            if let Some(step) = try_continue_snapshot_plan(
                &view,
                runtime,
                ranked_candidates,
                cognitive.planning_switch_margin,
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
                committed_opportunity,
                blocked_memory,
                tick,
                cognitive,
                execution_budget,
                semantics_table,
                action_defs,
                action_handlers,
                recipe_registry,
                false,
                false,
                &runtime.exhaustion_cache,
            );

            // Record newly exhausted goals for next tick.
            let _ = record_exhausted_goals(
                runtime,
                &view,
                agent,
                recipe_registry,
                &plans,
                tick,
                cognitive,
            );
            for plan in &plans {
                if plan.result.is_found() {
                    runtime.exhaustion_cache.remove(&plan.opportunity);
                }
            }
            let selection_plans = selection_candidates(&plans);

            if let Some(selected_plan) = select_best_plan(
                ranked_candidates,
                &selection_plans,
                active_goal_key,
                runtime,
                jc.as_ref(),
                crate::SelectionPolicy {
                    side_benefit_weight,
                    default_switch_margin,
                    frame_switch_margin,
                },
            ) {
                adopt_selected_plan(
                    runtime,
                    active_goal,
                    jc,
                    facility_intents,
                    ranked_candidates,
                    selected_plan,
                    tick,
                );
            } else {
                clear_current_plan(
                    runtime,
                    active_goal,
                    jc,
                    facility_intents,
                    ranked_candidates,
                );
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
    })();
    record_planning_phase_duration(tick, planning_start.elapsed());
    result
}

/// Wrapper around `plan_and_validate_next_step` that also captures trace data.
///
/// Returns `(next_step, valid, plan_continued, plan_search_trace, selection_trace, pending_tracker_increments)`.
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref
)]
pub(super) fn plan_and_validate_next_step_traced(
    world: &worldwake_core::World,
    scheduler: &Scheduler,
    runtime: &mut AgentDecisionRuntime,
    active_goal: &mut Option<ActiveGoal>,
    jc: &mut Option<IntentionFrame>,
    facility_intents: &mut worldwake_core::ContentionIntents,
    agent: worldwake_core::EntityId,
    ranked_candidates: &[RankedGoal],
    blocked_memory: &BlockerMemory,
    default_switch_margin: Permille,
    frame_switch_margin: Permille,
    side_benefit_weight: Permille,
    tick: Tick,
    cognitive: &CognitiveProfile,
    execution_budget: &ExecutionBudget,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    action_defs: &worldwake_sim::ActionDefRegistry,
    action_handlers: &ActionHandlerRegistry,
    tracing: bool,
    previous_goal: Option<worldwake_core::GoalKey>,
    recipe_registry: &RecipeRegistry,
) -> PlanningStepTraceResult {
    if !tracing {
        let (step, valid) = plan_and_validate_next_step(
            world,
            scheduler,
            runtime,
            active_goal,
            jc,
            facility_intents,
            agent,
            ranked_candidates,
            blocked_memory,
            default_switch_margin,
            frame_switch_margin,
            side_benefit_weight,
            tick,
            cognitive,
            execution_budget,
            semantics_table,
            action_defs,
            action_handlers,
            recipe_registry,
        );
        return (step, valid, false, None, None, BTreeSet::new());
    }

    // Traced path: inline the logic to capture intermediate results.
    let view = runtime_belief_view(agent, world, scheduler, action_defs, recipe_registry);
    let mut plan_search_trace = PlanSearchTrace {
        attempts: Vec::new(),
        same_goal_trace: None,
    };
    let mut selection_trace = SelectionTrace {
        selected_opportunity: None,
        selected_plan: None,
        selected_plan_source: None,
        goal_switch: None,
        previous_goal,
        plan_replacement: None,
        snapshot_continuation: None,
    };
    let mut plan_continued = false;
    let mut pending_tracker_increments = BTreeSet::new();

    let should_plan = !runtime.dirty.is_empty() || has_pending_budget_retry(runtime, tick);
    if should_plan {
        if runtime.dirty.is_snapshot_only()
            && let Some(plan_for_trace) = runtime.current_plan.as_ref()
        {
            let continuation = summarize_snapshot_continuation(
                plan_for_trace.opportunity,
                ranked_candidates,
                cognitive.planning_switch_margin,
            );
            selection_trace.snapshot_continuation = Some(continuation.clone());
            if continuation.continues_plan()
                && let Some(step) = current_step(runtime).cloned()
            {
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
                    selection_trace.selected_opportunity =
                        runtime.current_plan.as_ref().map(|plan| plan.opportunity);
                    selection_trace.selected_plan =
                        runtime.current_plan.as_ref().and_then(|plan| {
                            selected_plan_value(ranked_candidates, plan, side_benefit_weight).map(
                                |plan_value| {
                                    summarize_selected_plan(
                                        plan,
                                        runtime.current_step_index,
                                        action_defs,
                                        None,
                                        &plan_value,
                                    )
                                },
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
                        BTreeSet::new(),
                    );
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
            runtime
                .current_plan
                .as_ref()
                .map(|plan| plan.opportunity)
                .filter(|opportunity| {
                    Some(opportunity.goal_key)
                        == active_goal.as_ref().map(|active_goal| active_goal.goal_key)
                }),
            blocked_memory,
            tick,
            cognitive,
            execution_budget,
            semantics_table,
            action_defs,
            action_handlers,
            recipe_registry,
            true,
            true,
            &runtime.exhaustion_cache,
        );

        pending_tracker_increments = record_exhausted_goals(
            runtime,
            &view,
            agent,
            recipe_registry,
            &plans,
            tick,
            cognitive,
        );
        for plan in &plans {
            if plan.result.is_found() {
                runtime.exhaustion_cache.remove(&plan.opportunity);
            }
        }

        let known_entities: BTreeSet<EntityId> = view
            .known_entity_beliefs(agent)
            .into_iter()
            .map(|(entity, _)| entity)
            .collect();
        for plan in &plans {
            plan_search_trace.attempts.push(plan_search_result_to_trace(
                plan.opportunity.goal_key,
                plan.opportunity.anchor,
                &plan.result,
                action_defs,
                planning_time_target_belief_presence(
                    plan.opportunity.goal_key.kind,
                    &known_entities,
                ),
                &plan.trace_metadata,
                plan.binding_rejections.clone(),
                plan.expansion_summaries.clone(),
            ));
        }
        plan_search_trace.same_goal_trace = summarize_same_goal_planning_trace(
            ranked_candidates,
            cognitive,
            tick,
            &runtime.exhaustion_cache,
            &plans,
        );

        let selection_plans = selection_candidates(&plans);
        let current_goal_before_selection = active_goal.as_ref().map(|ag| ag.goal_key);

        if let Some(selected_plan) = select_best_plan(
            ranked_candidates,
            &selection_plans,
            current_goal_before_selection,
            runtime,
            jc.as_ref(),
            crate::SelectionPolicy {
                side_benefit_weight,
                default_switch_margin,
                frame_switch_margin,
            },
        ) {
            let selected_goal = selected_plan.goal;
            let selected_opportunity = selected_plan.opportunity;
            let selected_plan_source = determine_selected_plan_source(
                selected_opportunity,
                current_goal_before_selection,
                &selection_plans,
            );
            let search_provenance =
                matches!(selected_plan_source, SelectedPlanSource::SearchSelection)
                    .then(|| summarize_search_provenance(&plans, selected_opportunity))
                    .flatten();
            let plan_value =
                selected_plan_value(ranked_candidates, &selected_plan, side_benefit_weight)
                    .expect("selected plan must map back to a ranked opportunity");
            selection_trace.selected_opportunity = Some(selected_plan.opportunity);
            selection_trace.selected_plan = Some(summarize_selected_plan(
                &selected_plan,
                0,
                action_defs,
                search_provenance,
                &plan_value,
            ));
            selection_trace.selected_plan_source = Some(selected_plan_source);
            selection_trace.plan_replacement = summarize_plan_replacement(
                runtime,
                current_goal_before_selection,
                selected_goal,
                &selected_plan,
                action_defs,
            );

            if let Some(prev) = previous_goal
                && prev != selected_goal
            {
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

            runtime.materialization_bindings.clear();
            facility_intents.intents.clear();
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
            facility_intents.intents.clear();
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
        pending_tracker_increments,
    )
}

/// Convert a `PlanSearchResult` into a `PlanAttemptTrace` for the trace model.
#[allow(clippy::too_many_arguments)]
pub(super) fn plan_search_result_to_trace(
    goal: worldwake_core::GoalKey,
    opportunity_anchor: worldwake_core::OpportunityAnchor,
    result: &PlanSearchResult,
    action_defs: &worldwake_sim::ActionDefRegistry,
    target_belief_presence: TargetBeliefPresence,
    trace_metadata: &SearchTraceMetadata,
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
        opportunity_anchor,
        outcome,
        strategic_plan: trace_metadata.strategic_plan.as_ref().map(|plan| {
            plan.steps
                .iter()
                .map(|step| StrategicStepTrace {
                    destination: step.destination,
                    sub_goal: format!("{:?}", step.sub_goal),
                    estimated_travel_ticks: step.estimated_travel_ticks,
                })
                .collect()
        }),
        tactical_goal: trace_metadata.tactical_goal.clone(),
        landmarks_extracted: trace_metadata.landmarks_extracted,
        landmark_orderings: trace_metadata.landmark_orderings,
        target_belief_presence,
        binding_rejections,
        expansion_summaries,
    }
}

fn planning_time_target_belief_presence(
    goal: GoalKind,
    known_entities: &BTreeSet<EntityId>,
) -> TargetBeliefPresence {
    match goal_target_entity(goal) {
        Some(target) if known_entities.contains(&target) => TargetBeliefPresence::Present,
        Some(_) => TargetBeliefPresence::Absent,
        None => TargetBeliefPresence::NotApplicable,
    }
}

fn goal_target_entity(goal: GoalKind) -> Option<EntityId> {
    match goal {
        GoalKind::SupportCandidateForOffice { candidate, .. } => Some(candidate),
        _ => worldwake_core::GoalKey::from(goal).entity,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CandidatePlanSearch, found_plan_blocks_later_goals, has_pending_budget_retry,
        plan_search_result_to_trace, planning_time_target_belief_presence, record_exhausted_goals,
        selected_plan_value, summarize_ranked_goal, summarize_selected_plan,
        summarize_snapshot_continuation,
    };
    use crate::{
        AgentDecisionRuntime, DirtySet, ExhaustionEntry, ExhaustionInvalidationCondition,
        ExhaustionRetryState, GoalKey, GoalKind, GoalPriorityClass, GroundedGoal,
        OpportunityAnchor, OpportunityKey, PlanSearchResult, PlanTerminalKind, PlannedPlan,
        PlannedStep, ProfileFixture, RankedGoal, build_semantics_table,
        decision_trace::{
            CompetitionDiscount, SnapshotContinuationOutcome, SourceReliabilityDiscount,
            TargetBeliefPresence,
        },
        feasibility::FeasibilityHint,
        search::SearchTraceMetadata,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        ActionDefId, ActionDomain, BodyCostPerTick, CauseRef, CognitiveProfile, CommodityKind,
        CommodityPurpose, ControlSource, EventLog, ExecutionBudget, HomeostaticNeeds,
        MerchandiseProfile, PerceptionSource, Permille, Place, Quantity, Tick, Topology,
        TravelEdge, TravelEdgeId, VisibilitySpec, WitnessData, WorkstationTag, World, WorldTxn,
        build_believed_entity_state, build_prototype_world,
    };
    use worldwake_sim::{
        ActionDef, ActionDefRegistry, ActionHandlerId, ActionHandlerRegistry, ActionPayload,
        BindingStrictness, DurationExpr, Interruptibility, PerAgentBeliefView, RecipeDefinition,
        RecipeRegistry, Scheduler, SystemManifest,
    };
    use worldwake_systems::build_full_action_registries;

    fn consume_goal(commodity: CommodityKind) -> GoalKey {
        GoalKey::from(GoalKind::ConsumeOwnedCommodity { commodity })
    }

    fn consume_opportunity(commodity: CommodityKind, anchor: OpportunityAnchor) -> OpportunityKey {
        OpportunityKey {
            goal_key: consume_goal(commodity),
            anchor,
        }
    }

    fn opportunity(goal: GoalKey) -> OpportunityKey {
        OpportunityKey {
            goal_key: goal,
            anchor: OpportunityAnchor::None,
        }
    }

    fn place_entity(slot: u32) -> worldwake_core::EntityId {
        worldwake_core::EntityId {
            slot,
            generation: 0,
        }
    }

    fn found_plan(goal: GoalKey) -> PlannedPlan {
        PlannedPlan::new(
            opportunity(goal),
            goal,
            Vec::new(),
            PlanTerminalKind::GoalSatisfied,
        )
    }

    fn cognitive(reasoning: &ProfileFixture) -> CognitiveProfile {
        CognitiveProfile {
            max_candidates_to_plan: reasoning.max_candidates_to_plan,
            max_candidates_per_expansion: CognitiveProfile::default().max_candidates_per_expansion,
            max_plan_depth: reasoning.max_plan_depth,
            max_travel_candidates_per_expansion: CognitiveProfile::default()
                .max_travel_candidates_per_expansion,
            snapshot_travel_horizon: reasoning.snapshot_travel_horizon,
            max_node_expansions: reasoning.max_node_expansions,
            switch_margin: reasoning.switch_margin,
            planning_switch_margin: CognitiveProfile::default().planning_switch_margin,
            transient_block_ticks: reasoning.transient_block_ticks,
            unknown_block_ticks: reasoning.unknown_block_ticks,
            structural_block_ticks: reasoning.structural_block_ticks,
            initial_cooldown_ticks: reasoning.initial_cooldown_ticks,
            max_cooldown_ticks: reasoning.max_cooldown_ticks,
            max_snapshot_entities_per_place: CognitiveProfile::default()
                .max_snapshot_entities_per_place,
            landmark_extraction_depth: CognitiveProfile::default().landmark_extraction_depth,
            use_ff_heuristic: CognitiveProfile::default().use_ff_heuristic,
        }
    }

    fn execution_budget(reasoning: &ProfileFixture) -> ExecutionBudget {
        ExecutionBudget::new(
            reasoning.beam_width,
            reasoning.max_prerequisite_locations,
            ExecutionBudget::default().preferred_operator_boost(),
        )
    }

    fn acquire_goal(
        commodity: CommodityKind,
        anchor: OpportunityAnchor,
        evidence_entities: BTreeSet<worldwake_core::EntityId>,
        evidence_places: BTreeSet<worldwake_core::EntityId>,
    ) -> GroundedGoal {
        GroundedGoal {
            key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity,
                purpose: CommodityPurpose::SelfConsume,
            }),
            anchor,
            evidence_entities,
            evidence_places,
        }
    }

    fn searched_plan(opportunity: OpportunityKey, result: PlanSearchResult) -> CandidatePlanSearch {
        CandidatePlanSearch {
            opportunity,
            result,
            perceived_cost: None,
            trace_metadata: SearchTraceMetadata::default(),
            binding_rejections: Vec::new(),
            expansion_summaries: Vec::new(),
        }
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

    /// Seed an agent's belief store with `BelievedEntityState` snapshots for the
    /// given entities, mirroring what `perception_system` does for agents with a
    /// `PerceptionProfile`. Needed for bare planner-unit tests where no
    /// perception pipeline runs but the belief view must reflect the world.
    fn seed_beliefs(
        world: &mut World,
        agent: worldwake_core::EntityId,
        entities: &[worldwake_core::EntityId],
        observed_tick: Tick,
    ) {
        let mut store = world
            .get_component_agent_belief_store(agent)
            .cloned()
            .unwrap_or_default();
        for entity in entities {
            if let Some(state) = build_believed_entity_state(
                world,
                *entity,
                observed_tick,
                PerceptionSource::DirectObservation,
            ) {
                store.update_entity(*entity, state);
            }
        }
        let mut txn = new_txn(world, observed_tick.0);
        txn.set_component_agent_belief_store(agent, store)
            .expect("agent belief store must be writable");
        commit_txn(txn);
    }

    fn cargo_topology(
        origin: worldwake_core::EntityId,
        destination: worldwake_core::EntityId,
    ) -> Topology {
        let mut topology = Topology::new();
        topology
            .add_place(
                origin,
                Place {
                    name: "Origin".to_string(),
                    capacity: None,
                    tags: BTreeSet::default(),
                },
            )
            .unwrap();
        topology
            .add_place(
                destination,
                Place {
                    name: "Destination".to_string(),
                    capacity: None,
                    tags: BTreeSet::default(),
                },
            )
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(1), origin, destination, 2, None).unwrap())
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(2), destination, origin, 2, None).unwrap())
            .unwrap();
        topology
    }

    fn three_place_topology(
        origin: worldwake_core::EntityId,
        destination: worldwake_core::EntityId,
        extra: worldwake_core::EntityId,
    ) -> Topology {
        let mut topology = cargo_topology(origin, destination);
        topology
            .add_place(
                extra,
                Place {
                    name: "Extra".to_string(),
                    capacity: None,
                    tags: BTreeSet::default(),
                },
            )
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(3), origin, extra, 2, None).unwrap())
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(4), extra, origin, 2, None).unwrap())
            .unwrap();
        topology
    }

    fn four_place_topology(
        origin: worldwake_core::EntityId,
        destination: worldwake_core::EntityId,
        extra_a: worldwake_core::EntityId,
        extra_b: worldwake_core::EntityId,
    ) -> Topology {
        let mut topology = three_place_topology(origin, destination, extra_a);
        topology
            .add_place(
                extra_b,
                Place {
                    name: "Extra B".to_string(),
                    capacity: None,
                    tags: BTreeSet::default(),
                },
            )
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(5), origin, extra_b, 2, None).unwrap())
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(6), extra_b, origin, 2, None).unwrap())
            .unwrap();
        topology
    }

    fn entity(slot: u32) -> worldwake_core::EntityId {
        worldwake_core::EntityId {
            slot,
            generation: 0,
        }
    }

    fn build_full_registries() -> (ActionDefRegistry, ActionHandlerRegistry, RecipeRegistry) {
        let recipes = RecipeRegistry::new();
        let registries = build_full_action_registries(&recipes).unwrap();
        (registries.defs, registries.handlers, recipes)
    }

    fn ranked_goal(goal: GroundedGoal) -> RankedGoal {
        RankedGoal {
            grounded: goal,
            priority_class: GoalPriorityClass::High,
            motive_score: 100,
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            feasibility: FeasibilityHint::Likely,
        }
    }

    fn bread_recipe_registry() -> (RecipeRegistry, worldwake_core::RecipeId) {
        let mut recipes = RecipeRegistry::new();
        let recipe_id = recipes.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::new(
                Permille::new(1).unwrap(),
                Permille::new(1).unwrap(),
                Permille::new(1).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(1).unwrap(),
            ),
        });
        (recipes, recipe_id)
    }

    #[test]
    fn planning_time_target_belief_presence_marks_present_absent_and_na() {
        let patient = entity(8);
        let mut known_entities = BTreeSet::new();
        known_entities.insert(patient);

        assert_eq!(
            planning_time_target_belief_presence(
                GoalKind::TreatWounds { patient },
                &known_entities,
            ),
            TargetBeliefPresence::Present
        );
        assert_eq!(
            planning_time_target_belief_presence(
                GoalKind::RaidTarget { target: entity(11) },
                &known_entities,
            ),
            TargetBeliefPresence::Absent
        );
        assert_eq!(
            planning_time_target_belief_presence(GoalKind::Sleep, &known_entities),
            TargetBeliefPresence::NotApplicable
        );
    }

    #[test]
    fn support_candidate_uses_candidate_for_target_belief_presence() {
        let office = entity(4);
        let candidate = entity(9);
        let mut known_entities = BTreeSet::new();
        known_entities.insert(candidate);

        assert_eq!(
            planning_time_target_belief_presence(
                GoalKind::SupportCandidateForOffice { office, candidate },
                &known_entities,
            ),
            TargetBeliefPresence::Present
        );
    }

    #[test]
    fn plan_search_trace_persists_target_belief_presence() {
        let (action_defs, _handlers, _recipes) = build_full_registries();
        let trace = plan_search_result_to_trace(
            GoalKey::from(GoalKind::TreatWounds {
                patient: entity(12),
            }),
            OpportunityAnchor::None,
            &PlanSearchResult::FrontierExhausted { expansions_used: 2 },
            &action_defs,
            TargetBeliefPresence::Absent,
            &SearchTraceMetadata::default(),
            Vec::new(),
            Vec::new(),
        );

        assert_eq!(trace.target_belief_presence, TargetBeliefPresence::Absent);
    }

    #[test]
    fn plan_search_trace_converts_two_phase_trace_metadata() {
        let (action_defs, _handlers, _recipes) = build_full_registries();
        let trace = plan_search_result_to_trace(
            GoalKey::from(GoalKind::ProduceCommodity {
                recipe_id: worldwake_core::RecipeId(7),
            }),
            OpportunityAnchor::None,
            &PlanSearchResult::FrontierExhausted { expansions_used: 2 },
            &action_defs,
            TargetBeliefPresence::NotApplicable,
            &SearchTraceMetadata {
                strategic_plan: Some(crate::search::strategic::StrategicPlan {
                    steps: vec![crate::search::strategic::StrategicStep {
                        destination: entity(55),
                        sub_goal: crate::search::strategic::TacticalSubGoal::AcquirePrerequisite(
                            worldwake_core::CommodityKind::Firewood,
                        ),
                        estimated_travel_ticks: 4,
                    }],
                }),
                tactical_goal: Some(
                    "AcquirePrerequisite { commodity: Firewood, destination: EntityId(55) }"
                        .to_string(),
                ),
                landmarks_extracted: 3,
                landmark_orderings: 2,
            },
            Vec::new(),
            Vec::new(),
        );

        assert_eq!(trace.landmarks_extracted, 3);
        assert_eq!(trace.landmark_orderings, 2);
        assert_eq!(
            trace.tactical_goal.as_deref(),
            Some("AcquirePrerequisite { commodity: Firewood, destination: EntityId(55) }")
        );
        let steps = trace
            .strategic_plan
            .as_ref()
            .expect("trace should preserve strategic plan metadata");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].destination, entity(55));
        assert_eq!(steps[0].sub_goal, "AcquirePrerequisite(Firewood)");
    }

    fn ranked_goal_with_score(
        opportunity: OpportunityKey,
        priority_class: GoalPriorityClass,
        motive_score: u32,
    ) -> RankedGoal {
        RankedGoal {
            grounded: GroundedGoal {
                key: opportunity.goal_key,
                anchor: opportunity.anchor,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
            },
            priority_class,
            motive_score,
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            feasibility: FeasibilityHint::Likely,
        }
    }

    #[test]
    fn summarize_ranked_goal_preserves_competition_discount() {
        let goal = acquire_goal(
            CommodityKind::Bread,
            OpportunityAnchor::Place(place_entity(40)),
            BTreeSet::new(),
            BTreeSet::new(),
        );
        let mut ranked = ranked_goal(goal);
        ranked.competition_discount = Some(CompetitionDiscount {
            observed_competitors: vec![entity(7), entity(8)],
            domain: ActionDomain::Production,
            effective_discount: Permille::new(400).unwrap(),
            pre_discount_motive: 100,
            post_discount_motive: 60,
        });

        let summary = summarize_ranked_goal(&ranked);

        assert_eq!(summary.competition_discount, ranked.competition_discount);
    }

    #[test]
    fn summarize_ranked_goal_preserves_source_reliability_discount() {
        let goal = acquire_goal(
            CommodityKind::Bread,
            OpportunityAnchor::Place(place_entity(40)),
            BTreeSet::from([entity(9)]),
            BTreeSet::new(),
        );
        let mut ranked = ranked_goal(goal);
        ranked.source_reliability_discount = Some(SourceReliabilityDiscount {
            source_entity: entity(9),
            commodity: CommodityKind::Bread,
            failure_ratio_permille: 500,
            pre_discount_motive: 100,
            post_discount_motive: 50,
        });

        let summary = summarize_ranked_goal(&ranked);

        assert_eq!(
            summary.source_reliability_discount,
            ranked.source_reliability_discount
        );
    }

    #[test]
    fn summarize_selected_plan_preserves_side_benefit_trace_fields() {
        let market = place_entity(40);
        let orchard = place_entity(41);
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let plan = PlannedPlan::new(
            OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::None,
            },
            goal,
            vec![
                PlannedStep {
                    def_id: ActionDefId(1),
                    targets: vec![crate::PlanningEntityRef::Authoritative(market)],
                    payload_override: None,
                    op_kind: crate::PlannerOpKind::Travel,
                    estimated_ticks: 3,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                },
                PlannedStep {
                    def_id: ActionDefId(2),
                    targets: vec![crate::PlanningEntityRef::Authoritative(orchard)],
                    payload_override: None,
                    op_kind: crate::PlannerOpKind::Travel,
                    estimated_ticks: 2,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                },
            ],
            PlanTerminalKind::GoalSatisfied,
        );
        let ranked_candidates = vec![
            RankedGoal {
                grounded: GroundedGoal {
                    key: goal,
                    anchor: OpportunityAnchor::None,
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                },
                priority_class: GoalPriorityClass::High,
                motive_score: 800,
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                feasibility: FeasibilityHint::Likely,
            },
            RankedGoal {
                grounded: GroundedGoal {
                    key: GoalKey::from(GoalKind::SellCommodity {
                        commodity: CommodityKind::Apple,
                    }),
                    anchor: OpportunityAnchor::Place(market),
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                },
                priority_class: GoalPriorityClass::Low,
                motive_score: 300,
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                feasibility: FeasibilityHint::Likely,
            },
        ];
        let plan_value =
            selected_plan_value(&ranked_candidates, &plan, Permille::new(100).unwrap())
                .expect("selected plan should resolve to ranked primary motive");

        let summary =
            summarize_selected_plan(&plan, 0, &ActionDefRegistry::new(), None, &plan_value);

        assert_eq!(summary.primary_motive, 800);
        assert_eq!(summary.total_value, 830);
        assert_eq!(summary.side_benefits.len(), 1);
        assert_eq!(summary.side_benefits[0].at_place, market);
        assert_eq!(summary.side_benefits[0].estimated_value, 30);
    }

    #[test]
    fn summarize_step_carries_binding_strictness_snapshot_from_action_def() {
        let mut action_defs = ActionDefRegistry::new();
        action_defs.register(ActionDef {
            id: ActionDefId(0),
            name: "heal".to_string(),
            domain: ActionDomain::Combat,
            actor_constraints: vec![],
            targets: vec![],
            preconditions: vec![],
            reservation_requirements: vec![],
            duration: DurationExpr::Fixed(NonZeroU32::new(3).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: BindingStrictness::ExactIdentity,
        });
        action_defs.register(ActionDef {
            id: ActionDefId(1),
            name: "eat".to_string(),
            domain: ActionDomain::Needs,
            actor_constraints: vec![],
            targets: vec![],
            preconditions: vec![],
            reservation_requirements: vec![],
            duration: DurationExpr::Fixed(NonZeroU32::new(2).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: BindingStrictness::FungibleEquivalentCommodity,
        });

        let exact = super::summarize_step(
            &PlannedStep {
                def_id: ActionDefId(0),
                targets: vec![],
                payload_override: None,
                op_kind: crate::PlannerOpKind::Heal,
                estimated_ticks: 3,
                is_materialization_barrier: false,
                expected_materializations: vec![],
            },
            &action_defs,
        );
        let fungible = super::summarize_step(
            &PlannedStep {
                def_id: ActionDefId(1),
                targets: vec![],
                payload_override: None,
                op_kind: crate::PlannerOpKind::Consume,
                estimated_ticks: 2,
                is_materialization_barrier: false,
                expected_materializations: vec![],
            },
            &action_defs,
        );

        assert_eq!(
            exact.binding_strictness,
            Some(BindingStrictness::ExactIdentity)
        );
        assert_eq!(
            fungible.binding_strictness,
            Some(BindingStrictness::FungibleEquivalentCommodity)
        );
    }

    #[test]
    fn snapshot_continuation_continues_when_same_class_delta_is_below_margin() {
        let current = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::Sleep),
            anchor: OpportunityAnchor::Place(entity(41)),
        };
        let top = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            }),
            anchor: OpportunityAnchor::Place(entity(42)),
        };
        let trace = summarize_snapshot_continuation(
            current,
            &[
                ranked_goal_with_score(top, GoalPriorityClass::High, 900),
                ranked_goal_with_score(current, GoalPriorityClass::High, 800),
            ],
            Permille::new(150).unwrap(),
        );

        assert_eq!(
            trace.outcome,
            SnapshotContinuationOutcome::ContinuedWithinMargin
        );
        assert_eq!(trace.motive_delta, Some(100));
        assert!(trace.continues_plan());
    }

    #[test]
    fn snapshot_continuation_replans_when_same_class_delta_meets_margin() {
        let current = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::Sleep),
            anchor: OpportunityAnchor::Place(entity(43)),
        };
        let top = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            }),
            anchor: OpportunityAnchor::Place(entity(44)),
        };
        let trace = summarize_snapshot_continuation(
            current,
            &[
                ranked_goal_with_score(top, GoalPriorityClass::High, 950),
                ranked_goal_with_score(current, GoalPriorityClass::High, 800),
            ],
            Permille::new(150).unwrap(),
        );

        assert_eq!(
            trace.outcome,
            SnapshotContinuationOutcome::ReplannedMarginExceeded
        );
        assert_eq!(trace.motive_delta, Some(150));
        assert!(!trace.continues_plan());
    }

    #[test]
    fn snapshot_continuation_replans_when_top_goal_has_higher_priority_class() {
        let current = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::Sleep),
            anchor: OpportunityAnchor::Place(entity(45)),
        };
        let top = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            }),
            anchor: OpportunityAnchor::Place(entity(46)),
        };
        let trace = summarize_snapshot_continuation(
            current,
            &[
                ranked_goal_with_score(top, GoalPriorityClass::Critical, 820),
                ranked_goal_with_score(current, GoalPriorityClass::High, 1000),
            ],
            Permille::new(300).unwrap(),
        );

        assert_eq!(
            trace.outcome,
            SnapshotContinuationOutcome::ReplannedHigherPriorityClass
        );
        assert_eq!(trace.motive_delta, Some(0));
        assert!(!trace.continues_plan());
    }

    #[test]
    fn snapshot_continuation_replans_when_current_opportunity_is_missing() {
        let current = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::Sleep),
            anchor: OpportunityAnchor::Place(entity(47)),
        };
        let top = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            }),
            anchor: OpportunityAnchor::Place(entity(48)),
        };
        let trace = summarize_snapshot_continuation(
            current,
            &[ranked_goal_with_score(top, GoalPriorityClass::High, 900)],
            Permille::new(150).unwrap(),
        );

        assert_eq!(
            trace.outcome,
            SnapshotContinuationOutcome::ReplannedCurrentOpportunityMissing
        );
        assert_eq!(trace.current_priority_class, None);
        assert_eq!(trace.current_motive_score, None);
        assert_eq!(trace.motive_delta, None);
        assert!(!trace.continues_plan());
    }

    #[test]
    fn snapshot_continuation_margin_zero_replans_on_any_same_class_shift() {
        let current = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::Sleep),
            anchor: OpportunityAnchor::Place(entity(49)),
        };
        let top = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            }),
            anchor: OpportunityAnchor::Place(entity(50)),
        };
        let trace = summarize_snapshot_continuation(
            current,
            &[
                ranked_goal_with_score(top, GoalPriorityClass::High, 801),
                ranked_goal_with_score(current, GoalPriorityClass::High, 800),
            ],
            Permille::ZERO,
        );

        assert_eq!(
            trace.outcome,
            SnapshotContinuationOutcome::ReplannedMarginExceeded
        );
        assert_eq!(trace.motive_delta, Some(1));
        assert!(!trace.continues_plan());
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
    fn candidate_search_does_not_use_other_admitted_candidate_evidence() {
        let origin = entity(11);
        let market = entity(12);
        let mut world = World::new(cargo_topology(origin, market)).unwrap();
        let (agent, seller) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Hungry", ControlSource::Ai).unwrap();
            let seller = txn.create_agent("Seller", ControlSource::Ai).unwrap();
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(3))
                .unwrap();
            txn.set_ground_location(agent, origin).unwrap();
            txn.set_ground_location(seller, market).unwrap();
            txn.set_ground_location(bread, market).unwrap();
            txn.set_possessor(bread, seller).unwrap();
            txn.set_component_homeostatic_needs(
                agent,
                HomeostaticNeeds::new(
                    worldwake_core::Permille::new(800).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                ),
            )
            .unwrap();
            txn.set_component_merchandise_profile(
                seller,
                MerchandiseProfile {
                    sale_kinds: [CommodityKind::Bread].into_iter().collect(),
                    home_facility: Some(market),
                },
            )
            .unwrap();
            commit_txn(txn);
            (agent, seller)
        };
        let (defs, handlers, recipes) = build_full_registries();
        let semantics = build_semantics_table(&defs);
        let scheduler = Scheduler::new(SystemManifest::canonical());
        let ranked_candidates = vec![
            ranked_goal(GroundedGoal {
                key: GoalKey::from(GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: CommodityPurpose::SelfConsume,
                }),
                anchor: worldwake_core::OpportunityAnchor::None,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
            }),
            ranked_goal(GroundedGoal {
                key: GoalKey::from(GoalKind::Sleep),
                anchor: worldwake_core::OpportunityAnchor::Place(market),
                evidence_entities: BTreeSet::from([seller]),
                evidence_places: BTreeSet::from([market]),
            }),
        ];
        let budget = ProfileFixture {
            snapshot_travel_horizon: 0,
            max_candidates_to_plan: 2,
            ..ProfileFixture::default()
        };

        let plans = super::build_candidate_plans(
            &world,
            &scheduler,
            agent,
            &ranked_candidates,
            None,
            &worldwake_core::BlockerMemory::default(),
            Tick(1),
            &cognitive(&budget),
            &execution_budget(&budget),
            &semantics,
            &defs,
            &handlers,
            &recipes,
            false,
            false,
            &BTreeMap::new(),
        );

        let first = plans
            .first()
            .expect("primary admitted candidate should be searched");
        assert_eq!(
            first.opportunity.goal_key,
            GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            })
        );
        assert!(
            !first.result.is_found(),
            "AcquireCommodity(Bread) search should not be able to use the remote seller evidence attached only to a different admitted candidate"
        );
    }

    #[test]
    fn same_goal_ranked_opportunities_are_attempted_in_order() {
        let origin = entity(21);
        let market = entity(22);
        let mut world = World::new(cargo_topology(origin, market)).unwrap();
        let (agent, bread) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Hungry", ControlSource::Ai).unwrap();
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(3))
                .unwrap();
            txn.set_ground_location(agent, origin).unwrap();
            txn.set_ground_location(bread, origin).unwrap();
            txn.set_component_homeostatic_needs(
                agent,
                HomeostaticNeeds::new(
                    worldwake_core::Permille::new(800).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                ),
            )
            .unwrap();
            commit_txn(txn);
            (agent, bread)
        };
        let (defs, handlers, recipes) = build_full_registries();
        let semantics = build_semantics_table(&defs);
        let scheduler = Scheduler::new(SystemManifest::canonical());
        let ranked_candidates = vec![
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(market),
                BTreeSet::new(),
                BTreeSet::from([market]),
            )),
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(origin),
                BTreeSet::from([bread]),
                BTreeSet::from([origin]),
            )),
        ];
        let budget = ProfileFixture {
            snapshot_travel_horizon: 4,
            max_candidates_to_plan: 2,
            ..ProfileFixture::default()
        };

        let plans = super::build_candidate_plans(
            &world,
            &scheduler,
            agent,
            &ranked_candidates,
            None,
            &worldwake_core::BlockerMemory::default(),
            Tick(1),
            &cognitive(&budget),
            &execution_budget(&budget),
            &semantics,
            &defs,
            &handlers,
            &recipes,
            false,
            false,
            &BTreeMap::new(),
        );

        assert_eq!(
            plans.len(),
            2,
            "same-goal sibling opportunities should both be admitted in ranked order"
        );
        assert_eq!(
            plans[0].opportunity.anchor,
            OpportunityAnchor::Place(market)
        );
        assert!(
            !matches!(plans[0].result, PlanSearchResult::Unsupported),
            "the first sibling opportunity should still be admitted to search even when it does not find a plan"
        );
        assert_eq!(
            plans[1].opportunity.anchor,
            OpportunityAnchor::Place(origin)
        );
        assert!(
            !matches!(plans[1].result, PlanSearchResult::Unsupported),
            "the later sibling opportunity should still be searched rather than suppressed before search"
        );
    }

    #[test]
    fn committed_opportunity_clusters_same_goal_siblings_ahead_of_interleaved_goals() {
        let market = entity(23);
        let origin = entity(24);
        let inn = entity(25);
        let bread_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let ranked_candidates = [
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(inn),
                BTreeSet::new(),
                BTreeSet::from([inn]),
            )),
            ranked_goal(GroundedGoal {
                key: GoalKey::from(GoalKind::SellCommodity {
                    commodity: CommodityKind::Firewood,
                }),
                anchor: OpportunityAnchor::Place(market),
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::from([market]),
            }),
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(market),
                BTreeSet::new(),
                BTreeSet::from([market]),
            )),
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(origin),
                BTreeSet::new(),
                BTreeSet::from([origin]),
            )),
        ];

        let prioritized = super::prioritize_same_goal_replan_candidates(
            ranked_candidates.iter().collect(),
            Some(OpportunityKey {
                goal_key: bread_goal,
                anchor: OpportunityAnchor::Place(market),
            }),
        );

        assert_eq!(
            prioritized
                .iter()
                .map(|candidate| OpportunityKey {
                    goal_key: candidate.grounded.key,
                    anchor: candidate.grounded.anchor,
                })
                .collect::<Vec<_>>(),
            vec![
                OpportunityKey {
                    goal_key: bread_goal,
                    anchor: OpportunityAnchor::Place(market),
                },
                OpportunityKey {
                    goal_key: bread_goal,
                    anchor: OpportunityAnchor::Place(inn),
                },
                OpportunityKey {
                    goal_key: bread_goal,
                    anchor: OpportunityAnchor::Place(origin),
                },
                OpportunityKey {
                    goal_key: GoalKey::from(GoalKind::SellCommodity {
                        commodity: CommodityKind::Firewood,
                    }),
                    anchor: OpportunityAnchor::Place(market),
                },
            ]
        );
    }

    #[test]
    fn exhausted_same_goal_opportunity_does_not_block_later_sibling() {
        let origin = entity(31);
        let market = entity(32);
        let mut world = World::new(cargo_topology(origin, market)).unwrap();
        let (agent, bread) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Hungry", ControlSource::Ai).unwrap();
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(3))
                .unwrap();
            txn.set_ground_location(agent, origin).unwrap();
            txn.set_ground_location(bread, origin).unwrap();
            txn.set_component_homeostatic_needs(
                agent,
                HomeostaticNeeds::new(
                    worldwake_core::Permille::new(800).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                ),
            )
            .unwrap();
            commit_txn(txn);
            (agent, bread)
        };
        let (defs, handlers, recipes) = build_full_registries();
        let semantics = build_semantics_table(&defs);
        let scheduler = Scheduler::new(SystemManifest::canonical());
        let ranked_candidates = vec![
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(market),
                BTreeSet::new(),
                BTreeSet::from([market]),
            )),
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(origin),
                BTreeSet::from([bread]),
                BTreeSet::from([origin]),
            )),
        ];
        let budget = ProfileFixture {
            snapshot_travel_horizon: 4,
            max_candidates_to_plan: 2,
            ..ProfileFixture::default()
        };
        let exhausted = OpportunityKey {
            goal_key: ranked_candidates[0].grounded.key,
            anchor: ranked_candidates[0].grounded.anchor,
        };
        let exhaustion_cache = BTreeMap::from([(
            exhausted,
            ExhaustionEntry::frontier_exhausted(Vec::new(), crate::ExhaustionBaseline::default()),
        )]);

        let plans = super::build_candidate_plans(
            &world,
            &scheduler,
            agent,
            &ranked_candidates,
            None,
            &worldwake_core::BlockerMemory::default(),
            Tick(1),
            &cognitive(&budget),
            &execution_budget(&budget),
            &semantics,
            &defs,
            &handlers,
            &recipes,
            false,
            false,
            &exhaustion_cache,
        );

        assert_eq!(
            plans.len(),
            1,
            "an exhausted sibling should be skipped without suppressing later same-goal opportunities"
        );
        assert_eq!(
            plans[0].opportunity.anchor,
            OpportunityAnchor::Place(origin)
        );
        assert!(
            !matches!(plans[0].result, PlanSearchResult::Unsupported),
            "the non-exhausted sibling should still reach search"
        );
    }

    #[test]
    fn summarize_search_provenance_uses_selected_opportunity() {
        let goal = GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        });
        let local = OpportunityKey {
            goal_key: goal,
            anchor: OpportunityAnchor::Place(place_entity(51)),
        };
        let remote = OpportunityKey {
            goal_key: goal,
            anchor: OpportunityAnchor::Place(place_entity(52)),
        };
        let remote_destination = place_entity(77);
        let mut local_plan = searched_plan(
            local,
            PlanSearchResult::Found(Box::new(PlannedPlan::new(
                local,
                goal,
                Vec::new(),
                PlanTerminalKind::GoalSatisfied,
            ))),
        );
        local_plan
            .expansion_summaries
            .push(crate::decision_trace::SearchExpansionSummary {
                depth: 0,
                remaining_travel_ticks: 2,
                combined_places_count: 1,
                prerequisite_places_count: 0,
                candidates_generated: 1,
                candidates_skipped: 0,
                preferred_candidates: 0,
                terminal_successors: 0,
                non_terminal_before_beam: 1,
                non_terminal_after_beam: 1,
                found_goal_satisfied: true,
                landmark_heuristic: 0,
                ff_heuristic: None,
                helpful_action_count: 0,
                travel_pruning: None,
                prerequisite_guidance: None,
                expansion_candidates: Vec::new(),
                root_candidates: Vec::new(),
                root_omissions: Vec::new(),
            });
        let mut remote_plan = searched_plan(
            remote,
            PlanSearchResult::Found(Box::new(PlannedPlan::new(
                remote,
                goal,
                vec![PlannedStep {
                    def_id: ActionDefId(9),
                    targets: vec![crate::PlanningEntityRef::Authoritative(remote_destination)],
                    payload_override: None,
                    op_kind: crate::PlannerOpKind::Travel,
                    estimated_ticks: 2,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                }],
                PlanTerminalKind::GoalSatisfied,
            ))),
        );
        remote_plan
            .expansion_summaries
            .push(crate::decision_trace::SearchExpansionSummary {
                depth: 0,
                remaining_travel_ticks: 7,
                combined_places_count: 1,
                prerequisite_places_count: 0,
                candidates_generated: 1,
                candidates_skipped: 0,
                preferred_candidates: 0,
                terminal_successors: 0,
                non_terminal_before_beam: 1,
                non_terminal_after_beam: 1,
                found_goal_satisfied: true,
                landmark_heuristic: 0,
                ff_heuristic: None,
                helpful_action_count: 0,
                travel_pruning: None,
                prerequisite_guidance: None,
                expansion_candidates: Vec::new(),
                root_candidates: Vec::new(),
                root_omissions: Vec::new(),
            });

        let provenance =
            super::summarize_search_provenance(&[local_plan, remote_plan], remote).unwrap();

        assert_eq!(provenance.root_remaining_travel_ticks, 7);
        assert_eq!(
            provenance.selected_root_travel_destination,
            Some(remote_destination)
        );
    }

    #[test]
    fn investigate_progress_barrier_found_plan_does_not_block_later_goals() {
        let goal = GoalKey::from(GoalKind::InvestigateViolation {
            violation_id: worldwake_core::ViolationId(9),
            place: entity(10),
        });
        let barrier_plan = PlannedPlan::new(
            opportunity(goal),
            goal,
            Vec::new(),
            PlanTerminalKind::ProgressBarrier,
        );

        assert!(!found_plan_blocks_later_goals(&barrier_plan));
    }

    #[test]
    fn produce_progress_barrier_found_plan_blocks_later_goals() {
        let goal = GoalKey::from(GoalKind::ProduceCommodity {
            recipe_id: worldwake_core::RecipeId(3),
        });
        let barrier_plan = PlannedPlan::new(
            opportunity(goal),
            goal,
            Vec::new(),
            PlanTerminalKind::ProgressBarrier,
        );

        assert!(found_plan_blocks_later_goals(&barrier_plan));
    }

    #[test]
    fn satisfied_and_combat_found_plans_block_later_goals() {
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let satisfied_plan = PlannedPlan::new(
            opportunity(goal),
            goal,
            Vec::new(),
            PlanTerminalKind::GoalSatisfied,
        );
        let combat_plan = PlannedPlan::new(
            opportunity(goal),
            goal,
            Vec::new(),
            PlanTerminalKind::CombatCommitment,
        );

        assert!(found_plan_blocks_later_goals(&satisfied_plan));
        assert!(found_plan_blocks_later_goals(&combat_plan));
    }

    #[test]
    fn traced_planning_records_same_goal_opportunity_attempt_order() {
        let origin = entity(41);
        let market = entity(42);
        let mut world = World::new(cargo_topology(origin, market)).unwrap();
        let (agent, bread) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Hungry", ControlSource::Ai).unwrap();
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(3))
                .unwrap();
            txn.set_ground_location(agent, origin).unwrap();
            txn.set_ground_location(bread, origin).unwrap();
            txn.set_component_homeostatic_needs(
                agent,
                HomeostaticNeeds::new(
                    worldwake_core::Permille::new(800).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                ),
            )
            .unwrap();
            commit_txn(txn);
            (agent, bread)
        };
        seed_beliefs(&mut world, agent, &[bread, origin, market], Tick(1));
        let (defs, handlers, recipes) = build_full_registries();
        let semantics = build_semantics_table(&defs);
        let scheduler = Scheduler::new(SystemManifest::canonical());
        let ranked_candidates = vec![
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(market),
                BTreeSet::new(),
                BTreeSet::from([market]),
            )),
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(origin),
                BTreeSet::from([bread]),
                BTreeSet::from([origin]),
            )),
        ];
        let budget = ProfileFixture {
            snapshot_travel_horizon: 4,
            max_candidates_to_plan: 2,
            ..ProfileFixture::default()
        };
        let mut runtime = AgentDecisionRuntime {
            dirty: DirtySet::NO_PLAN,
            ..AgentDecisionRuntime::default()
        };
        let mut active_goal = None;
        let mut frame = None;
        let mut facility_intents = worldwake_core::ContentionIntents::default();

        let (_, _, _, plan_search_trace, _, _) = super::plan_and_validate_next_step_traced(
            &world,
            &scheduler,
            &mut runtime,
            &mut active_goal,
            &mut frame,
            &mut facility_intents,
            agent,
            &ranked_candidates,
            &worldwake_core::BlockerMemory::default(),
            worldwake_core::Permille::new(0).unwrap(),
            worldwake_core::Permille::new(0).unwrap(),
            worldwake_core::Permille::new(100).unwrap(),
            Tick(1),
            &cognitive(&budget),
            &execution_budget(&budget),
            &semantics,
            &defs,
            &handlers,
            true,
            None,
            &recipes,
        );

        let plan_search_trace =
            plan_search_trace.expect("traced planning should record attempt order");
        let attempts = &plan_search_trace.attempts;
        assert_eq!(attempts.len(), 2);
        assert_eq!(
            attempts[0].opportunity_anchor,
            OpportunityAnchor::Place(market)
        );
        assert_eq!(
            attempts[1].opportunity_anchor,
            OpportunityAnchor::Place(origin)
        );
        assert!(
            !matches!(
                attempts[0].outcome,
                crate::decision_trace::PlanSearchOutcome::Unsupported
            ),
            "the first same-goal opportunity should remain a real admitted search attempt even when it finds no plan"
        );
        assert!(
            !matches!(
                attempts[1].outcome,
                crate::decision_trace::PlanSearchOutcome::Unsupported
            ),
            "the later same-goal opportunity should appear as a real search attempt in the trace"
        );
        assert_eq!(
            plan_search_trace.same_goal_trace,
            Some(crate::SameGoalPlanningTrace {
                continuation_trigger: None,
                stop_reason: crate::SameGoalPlanningStopReason::ExhaustedAdmittedOpportunities,
            })
        );
    }

    #[test]
    fn same_goal_planning_trace_records_different_goal_stop_after_found_sibling() {
        let goal = GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        });
        let market = OpportunityKey {
            goal_key: goal,
            anchor: OpportunityAnchor::Place(entity(61)),
        };
        let orchard = OpportunityKey {
            goal_key: goal,
            anchor: OpportunityAnchor::Place(entity(62)),
        };
        let sleep_goal = GroundedGoal {
            key: GoalKey::from(GoalKind::Sleep),
            anchor: OpportunityAnchor::Place(entity(63)),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
        };
        let ranked_candidates = vec![
            ranked_goal(GroundedGoal {
                key: goal,
                anchor: market.anchor,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
            }),
            ranked_goal(GroundedGoal {
                key: goal,
                anchor: orchard.anchor,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
            }),
            ranked_goal(sleep_goal.clone()),
        ];
        let plans = vec![
            searched_plan(
                market,
                PlanSearchResult::FrontierExhausted { expansions_used: 1 },
            ),
            searched_plan(
                orchard,
                PlanSearchResult::Found(Box::new(PlannedPlan::new(
                    orchard,
                    goal,
                    Vec::new(),
                    PlanTerminalKind::GoalSatisfied,
                ))),
            ),
        ];

        assert_eq!(
            super::summarize_same_goal_planning_trace(
                &ranked_candidates,
                &cognitive(&ProfileFixture {
                    max_candidates_to_plan: 3,
                    ..ProfileFixture::default()
                }),
                Tick(1),
                &BTreeMap::new(),
                &plans,
            ),
            Some(crate::SameGoalPlanningTrace {
                continuation_trigger: Some(orchard),
                stop_reason: crate::SameGoalPlanningStopReason::EncounteredDifferentGoal {
                    next_goal: sleep_goal.key,
                },
            })
        );
    }

    #[test]
    fn same_goal_planning_trace_records_candidate_cap_stop_reason() {
        let origin = entity(51);
        let market = entity(52);
        let camp = entity(53);
        let mut world = World::new(three_place_topology(origin, market, camp)).unwrap();
        let (agent, bread) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Hungry", ControlSource::Ai).unwrap();
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(3))
                .unwrap();
            txn.set_ground_location(agent, origin).unwrap();
            txn.set_ground_location(bread, origin).unwrap();
            txn.set_component_homeostatic_needs(
                agent,
                HomeostaticNeeds::new(
                    worldwake_core::Permille::new(800).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                ),
            )
            .unwrap();
            commit_txn(txn);
            (agent, bread)
        };
        seed_beliefs(&mut world, agent, &[bread, origin, market, camp], Tick(1));
        let (defs, handlers, recipes) = build_full_registries();
        let semantics = build_semantics_table(&defs);
        let scheduler = Scheduler::new(SystemManifest::canonical());
        let ranked_candidates = vec![
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(market),
                BTreeSet::new(),
                BTreeSet::from([market]),
            )),
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(origin),
                BTreeSet::from([bread]),
                BTreeSet::from([origin]),
            )),
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(camp),
                BTreeSet::new(),
                BTreeSet::from([camp]),
            )),
        ];
        let plans = super::build_candidate_plans(
            &world,
            &scheduler,
            agent,
            &ranked_candidates,
            None,
            &worldwake_core::BlockerMemory::default(),
            Tick(1),
            &cognitive(&ProfileFixture {
                snapshot_travel_horizon: 4,
                max_candidates_to_plan: 2,
                ..ProfileFixture::default()
            }),
            &execution_budget(&ProfileFixture {
                snapshot_travel_horizon: 4,
                max_candidates_to_plan: 2,
                ..ProfileFixture::default()
            }),
            &semantics,
            &defs,
            &handlers,
            &recipes,
            false,
            false,
            &BTreeMap::new(),
        );

        assert_eq!(plans.len(), 2);
        assert_eq!(
            super::summarize_same_goal_planning_trace(
                &ranked_candidates,
                &cognitive(&ProfileFixture {
                    snapshot_travel_horizon: 4,
                    max_candidates_to_plan: 2,
                    ..ProfileFixture::default()
                }),
                Tick(1),
                &BTreeMap::new(),
                &plans,
            ),
            Some(crate::SameGoalPlanningTrace {
                continuation_trigger: None,
                stop_reason: crate::SameGoalPlanningStopReason::ReachedCandidatePlanCap,
            })
        );
    }

    #[test]
    fn summarize_same_goal_planning_trace_uses_same_exhaustion_admission_as_candidate_plans() {
        let origin = entity(101);
        let frontier = entity(102);
        let cooling_down = entity(103);
        let retry_ready = entity(104);
        let fresh = origin;
        let mut world = World::new(four_place_topology(
            origin,
            frontier,
            cooling_down,
            retry_ready,
        ))
        .unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Hungry", ControlSource::Ai).unwrap();
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(3))
                .unwrap();
            txn.set_ground_location(agent, origin).unwrap();
            txn.set_ground_location(bread, fresh).unwrap();
            txn.set_component_homeostatic_needs(
                agent,
                HomeostaticNeeds::new(
                    worldwake_core::Permille::new(800).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                ),
            )
            .unwrap();
            commit_txn(txn);
            agent
        };
        let (defs, handlers, recipes) = build_full_registries();
        let semantics = build_semantics_table(&defs);
        let scheduler = Scheduler::new(SystemManifest::canonical());
        let ranked_candidates = vec![
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(frontier),
                BTreeSet::new(),
                BTreeSet::from([frontier]),
            )),
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(cooling_down),
                BTreeSet::new(),
                BTreeSet::from([cooling_down]),
            )),
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(retry_ready),
                BTreeSet::new(),
                BTreeSet::from([retry_ready]),
            )),
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(fresh),
                BTreeSet::new(),
                BTreeSet::from([fresh]),
            )),
        ];
        let exhaustion_cache = BTreeMap::from([
            (
                OpportunityKey {
                    goal_key: ranked_candidates[0].grounded.key,
                    anchor: ranked_candidates[0].grounded.anchor,
                },
                ExhaustionEntry {
                    retry_state: ExhaustionRetryState::FrontierExhausted,
                    invalidation_conditions: Vec::new(),
                    baseline: crate::ExhaustionBaseline::default(),
                    next_retry_tick: None,
                    consecutive_failures: 0,
                },
            ),
            (
                OpportunityKey {
                    goal_key: ranked_candidates[1].grounded.key,
                    anchor: ranked_candidates[1].grounded.anchor,
                },
                ExhaustionEntry {
                    retry_state: ExhaustionRetryState::BudgetRetryPending,
                    invalidation_conditions: Vec::new(),
                    baseline: crate::ExhaustionBaseline::default(),
                    next_retry_tick: Some(Tick(20)),
                    consecutive_failures: 2,
                },
            ),
            (
                OpportunityKey {
                    goal_key: ranked_candidates[2].grounded.key,
                    anchor: ranked_candidates[2].grounded.anchor,
                },
                ExhaustionEntry {
                    retry_state: ExhaustionRetryState::BudgetRetryPending,
                    invalidation_conditions: Vec::new(),
                    baseline: crate::ExhaustionBaseline::default(),
                    next_retry_tick: Some(Tick(10)),
                    consecutive_failures: 2,
                },
            ),
        ]);
        let budget = ProfileFixture {
            snapshot_travel_horizon: 4,
            max_candidates_to_plan: 2,
            max_node_expansions: 0,
            ..ProfileFixture::default()
        };

        let plans = super::build_candidate_plans(
            &world,
            &scheduler,
            agent,
            &ranked_candidates,
            None,
            &worldwake_core::BlockerMemory::default(),
            Tick(10),
            &cognitive(&budget),
            &execution_budget(&budget),
            &semantics,
            &defs,
            &handlers,
            &recipes,
            false,
            false,
            &exhaustion_cache,
        );

        assert_eq!(
            plans
                .iter()
                .map(|plan| plan.opportunity)
                .collect::<Vec<_>>(),
            vec![
                OpportunityKey {
                    goal_key: ranked_candidates[2].grounded.key,
                    anchor: ranked_candidates[2].grounded.anchor,
                },
                OpportunityKey {
                    goal_key: ranked_candidates[3].grounded.key,
                    anchor: ranked_candidates[3].grounded.anchor,
                },
            ]
        );
        assert_eq!(
            super::summarize_same_goal_planning_trace(
                &ranked_candidates,
                &cognitive(&budget),
                Tick(10),
                &exhaustion_cache,
                &plans,
            ),
            Some(crate::SameGoalPlanningTrace {
                continuation_trigger: None,
                stop_reason: crate::SameGoalPlanningStopReason::ExhaustedAdmittedOpportunities,
            }),
            "the trace summary should see the same two admitted opportunities as real planning; if it over-admits the filtered entries it will report hitting the candidate cap instead"
        );
    }

    #[test]
    fn record_exhausted_goals_replaces_frontier_suppression_with_budget_retry_state() {
        let goal = consume_opportunity(CommodityKind::Bread, OpportunityAnchor::None);
        let mut runtime = AgentDecisionRuntime::default();
        runtime.exhaustion_cache.insert(
            goal,
            ExhaustionEntry {
                retry_state: ExhaustionRetryState::FrontierExhausted,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                next_retry_tick: None,
                consecutive_failures: 0,
            },
        );

        let plans = vec![searched_plan(
            goal,
            PlanSearchResult::BudgetExhausted {
                expansions_used: 12,
            },
        )];
        let (world, agent, _) = setup_agent_world();
        let view = PerAgentBeliefView::from_world(agent, &world);
        let tracker_increments = record_exhausted_goals(
            &mut runtime,
            &view,
            agent,
            &RecipeRegistry::new(),
            &plans,
            Tick(9),
            &cognitive(&ProfileFixture::default()),
        );
        assert_eq!(
            tracker_increments,
            BTreeSet::from([worldwake_core::HomeostaticNeedId::Hunger])
        );

        let entry = runtime.exhaustion_cache.get(&goal).unwrap();
        assert_eq!(entry.retry_state, ExhaustionRetryState::BudgetRetryPending);
        assert_eq!(entry.consecutive_failures, 1);
        assert_eq!(
            entry.next_retry_tick,
            Some(Tick(
                9 + u64::from(ProfileFixture::default().initial_cooldown_ticks)
            ))
        );
        assert_eq!(
            entry.invalidation_conditions,
            vec![ExhaustionInvalidationCondition::CommodityChanged(
                CommodityKind::Bread
            )]
        );
    }

    #[test]
    fn record_exhausted_goals_derives_goal_aware_conditions_and_baseline() {
        let goal = consume_opportunity(CommodityKind::Bread, OpportunityAnchor::None);
        let mut runtime = AgentDecisionRuntime::default();
        let (mut world, agent, place) = setup_agent_world();
        {
            let mut txn = new_txn(&mut world, 2);
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(2))
                .unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_possessor(bread, agent).unwrap();
            commit_txn(txn);
        }
        let view = PerAgentBeliefView::from_world(agent, &world);

        let plans = vec![searched_plan(
            goal,
            PlanSearchResult::BudgetExhausted {
                expansions_used: 12,
            },
        )];
        let _ = record_exhausted_goals(
            &mut runtime,
            &view,
            agent,
            &RecipeRegistry::new(),
            &plans,
            Tick(9),
            &cognitive(&ProfileFixture::default()),
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
    fn record_exhausted_goals_emits_hunger_increment_for_budget_exhausted_produce_goal() {
        let (recipes, recipe_id) = bread_recipe_registry();
        let goal = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::ProduceCommodity { recipe_id }),
            anchor: OpportunityAnchor::None,
        };
        let mut runtime = AgentDecisionRuntime::default();
        let plans = vec![searched_plan(
            goal,
            PlanSearchResult::BudgetExhausted {
                expansions_used: 12,
            },
        )];
        let (world, agent, _) = setup_agent_world();
        let view = PerAgentBeliefView::from_world(agent, &world);

        let tracker_increments = record_exhausted_goals(
            &mut runtime,
            &view,
            agent,
            &recipes,
            &plans,
            Tick(9),
            &cognitive(&ProfileFixture::default()),
        );

        assert_eq!(
            tracker_increments,
            BTreeSet::from([worldwake_core::HomeostaticNeedId::Hunger])
        );
    }

    #[test]
    fn record_exhausted_goals_doubles_cooldown_for_repeated_budget_retry_entries() {
        let goal = consume_opportunity(CommodityKind::Bread, OpportunityAnchor::None);
        let budget = ProfileFixture {
            initial_cooldown_ticks: 4,
            max_cooldown_ticks: 64,
            ..ProfileFixture::default()
        };
        let mut runtime = AgentDecisionRuntime::default();
        runtime.exhaustion_cache.insert(
            goal,
            ExhaustionEntry {
                retry_state: ExhaustionRetryState::BudgetRetryPending,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                next_retry_tick: Some(Tick(5)),
                consecutive_failures: 1,
            },
        );
        let plans = vec![searched_plan(
            goal,
            PlanSearchResult::BudgetExhausted {
                expansions_used: 12,
            },
        )];
        let (world, agent, _) = setup_agent_world();
        let view = PerAgentBeliefView::from_world(agent, &world);

        let _ = record_exhausted_goals(
            &mut runtime,
            &view,
            agent,
            &RecipeRegistry::new(),
            &plans,
            Tick(9),
            &cognitive(&budget),
        );

        let entry = runtime.exhaustion_cache.get(&goal).unwrap();
        assert_eq!(entry.consecutive_failures, 2);
        assert_eq!(entry.next_retry_tick, Some(Tick(17)));
    }

    #[test]
    fn record_exhausted_goals_removes_only_successful_opportunity_entry() {
        let solved_goal = consume_opportunity(
            CommodityKind::Bread,
            OpportunityAnchor::Place(place_entity(1)),
        );
        let retained_goal = consume_opportunity(
            CommodityKind::Bread,
            OpportunityAnchor::Place(place_entity(2)),
        );
        let mut runtime = AgentDecisionRuntime::default();
        runtime.exhaustion_cache.insert(
            solved_goal,
            ExhaustionEntry {
                retry_state: ExhaustionRetryState::FrontierExhausted,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                next_retry_tick: None,
                consecutive_failures: 0,
            },
        );
        runtime.exhaustion_cache.insert(
            retained_goal,
            ExhaustionEntry {
                retry_state: ExhaustionRetryState::BudgetRetryPending,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                next_retry_tick: Some(Tick(14)),
                consecutive_failures: 2,
            },
        );

        let plans = vec![searched_plan(
            solved_goal,
            PlanSearchResult::Found(Box::new(found_plan(solved_goal.goal_key))),
        )];
        let (world, agent, _) = setup_agent_world();
        let view = PerAgentBeliefView::from_world(agent, &world);

        let _ = record_exhausted_goals(
            &mut runtime,
            &view,
            agent,
            &RecipeRegistry::new(),
            &plans,
            Tick(10),
            &cognitive(&ProfileFixture::default()),
        );

        assert!(!runtime.exhaustion_cache.contains_key(&solved_goal));
        assert_eq!(
            runtime.exhaustion_cache.get(&retained_goal),
            Some(&ExhaustionEntry {
                retry_state: ExhaustionRetryState::BudgetRetryPending,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                next_retry_tick: Some(Tick(14)),
                consecutive_failures: 2,
            })
        );
    }

    #[test]
    fn record_exhausted_goals_records_frontier_exhaustion_as_suppressing_retry_state() {
        let goal = consume_opportunity(CommodityKind::Bread, OpportunityAnchor::None);
        let mut runtime = AgentDecisionRuntime::default();
        let plans = vec![searched_plan(
            goal,
            PlanSearchResult::FrontierExhausted {
                expansions_used: 12,
            },
        )];
        let (world, agent, _) = setup_agent_world();
        let view = PerAgentBeliefView::from_world(agent, &world);

        let tracker_increments = record_exhausted_goals(
            &mut runtime,
            &view,
            agent,
            &RecipeRegistry::new(),
            &plans,
            Tick(9),
            &cognitive(&ProfileFixture::default()),
        );
        assert!(tracker_increments.is_empty());

        let entry = runtime.exhaustion_cache.get(&goal).unwrap();
        assert_eq!(entry.retry_state, ExhaustionRetryState::FrontierExhausted);
        assert!(entry.suppresses_planning());
    }

    #[test]
    fn record_exhausted_goals_records_sleep_frontier_exhaustion_as_budget_retry() {
        let goal = opportunity(GoalKey::from(GoalKind::Sleep));
        let mut runtime = AgentDecisionRuntime::default();
        let plans = vec![searched_plan(
            goal,
            PlanSearchResult::FrontierExhausted {
                expansions_used: 12,
            },
        )];
        let (world, agent, _) = setup_agent_world();
        let view = PerAgentBeliefView::from_world(agent, &world);
        let cognitive = cognitive(&ProfileFixture::default());

        let tracker_increments = record_exhausted_goals(
            &mut runtime,
            &view,
            agent,
            &RecipeRegistry::new(),
            &plans,
            Tick(9),
            &cognitive,
        );
        assert!(tracker_increments.is_empty());

        let entry = runtime.exhaustion_cache.get(&goal).unwrap();
        assert_eq!(entry.retry_state, ExhaustionRetryState::BudgetRetryPending);
        assert!(!entry.suppresses_planning());
        assert!(
            entry.next_retry_tick.is_some(),
            "sleep frontier exhaustion should retry on cooldown instead of suppressing indefinitely"
        );
    }

    #[test]
    fn frontier_exhaustion_suppresses_planning_but_budget_retry_does_not() {
        let frontier_entry = ExhaustionEntry {
            retry_state: ExhaustionRetryState::FrontierExhausted,
            invalidation_conditions: Vec::new(),
            baseline: crate::ExhaustionBaseline::default(),
            next_retry_tick: None,
            consecutive_failures: 0,
        };
        let entry = ExhaustionEntry {
            retry_state: ExhaustionRetryState::BudgetRetryPending,
            invalidation_conditions: Vec::new(),
            baseline: crate::ExhaustionBaseline::default(),
            next_retry_tick: Some(Tick(12)),
            consecutive_failures: 1,
        };

        assert!(frontier_entry.suppresses_planning());
        assert!(!entry.suppresses_planning());
        assert!(!entry.is_retry_eligible(Tick(11)));
        assert!(entry.is_retry_eligible(Tick(12)));
    }

    #[test]
    fn has_pending_budget_retry_detects_retryable_budget_entries() {
        let mut runtime = AgentDecisionRuntime::default();
        runtime.exhaustion_cache.insert(
            consume_opportunity(
                CommodityKind::Bread,
                OpportunityAnchor::Place(place_entity(1)),
            ),
            ExhaustionEntry {
                retry_state: ExhaustionRetryState::BudgetRetryPending,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                next_retry_tick: Some(Tick(12)),
                consecutive_failures: 1,
            },
        );
        runtime.exhaustion_cache.insert(
            consume_opportunity(
                CommodityKind::Water,
                OpportunityAnchor::Place(place_entity(2)),
            ),
            ExhaustionEntry {
                retry_state: ExhaustionRetryState::FrontierExhausted,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                next_retry_tick: None,
                consecutive_failures: 0,
            },
        );

        assert!(!has_pending_budget_retry(&runtime, Tick(11)));
        assert!(has_pending_budget_retry(&runtime, Tick(12)));

        runtime.exhaustion_cache.insert(
            consume_opportunity(
                CommodityKind::Apple,
                OpportunityAnchor::Place(place_entity(3)),
            ),
            ExhaustionEntry {
                retry_state: ExhaustionRetryState::FrontierExhausted,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                next_retry_tick: None,
                consecutive_failures: 0,
            },
        );
        runtime
            .exhaustion_cache
            .retain(|_, entry| entry.retry_state != ExhaustionRetryState::BudgetRetryPending);

        assert!(!has_pending_budget_retry(&runtime, Tick(99)));
    }

    #[test]
    fn build_candidate_plans_applies_budget_backoff_for_retry_eligible_exhaustion_entry() {
        let origin = entity(81);
        let market = entity(82);
        let mut world = World::new(cargo_topology(origin, market)).unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Hungry", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, origin).unwrap();
            txn.set_component_homeostatic_needs(
                agent,
                HomeostaticNeeds::new(
                    worldwake_core::Permille::new(800).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                ),
            )
            .unwrap();
            commit_txn(txn);
            agent
        };
        let (defs, handlers, recipes) = build_full_registries();
        let semantics = build_semantics_table(&defs);
        let scheduler = Scheduler::new(SystemManifest::canonical());
        let ranked_candidates = vec![ranked_goal(acquire_goal(
            CommodityKind::Bread,
            OpportunityAnchor::Place(market),
            BTreeSet::new(),
            BTreeSet::from([market]),
        ))];
        let opportunity = OpportunityKey {
            goal_key: ranked_candidates[0].grounded.key,
            anchor: ranked_candidates[0].grounded.anchor,
        };
        // 4 consecutive failures → backoff kicks in at 3+: shift = 4-2 = 2,
        // effective budget = 128 >> 2 = 32
        let exhaustion_cache = BTreeMap::from([(
            opportunity,
            ExhaustionEntry {
                retry_state: ExhaustionRetryState::BudgetRetryPending,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                next_retry_tick: Some(Tick(10)),
                consecutive_failures: 4,
            },
        )]);

        let plans = super::build_candidate_plans(
            &world,
            &scheduler,
            agent,
            &ranked_candidates,
            None,
            &worldwake_core::BlockerMemory::default(),
            Tick(10),
            &cognitive(&ProfileFixture {
                snapshot_travel_horizon: 4,
                max_candidates_to_plan: 1,
                max_node_expansions: 128,
                ..ProfileFixture::default()
            }),
            &execution_budget(&ProfileFixture {
                snapshot_travel_horizon: 4,
                max_candidates_to_plan: 1,
                max_node_expansions: 128,
                ..ProfileFixture::default()
            }),
            &semantics,
            &defs,
            &handlers,
            &recipes,
            false,
            false,
            &exhaustion_cache,
        );

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].opportunity, opportunity);
        // With 2 consecutive failures and base budget 128, effective budget is
        // 128 >> 2 = 32. The search should exhaust at 32 expansions, not 128.
        // With 4 consecutive failures and threshold 3, shift = 4-2 = 2,
        // effective budget = 128 >> 2 = 32. The search may exhaust the frontier
        // or budget at ≤32 expansions.
        let expansions_used = match &plans[0].result {
            PlanSearchResult::BudgetExhausted { expansions_used }
            | PlanSearchResult::FrontierExhausted { expansions_used } => *expansions_used,
            PlanSearchResult::Found(_) => 0,
            PlanSearchResult::Unsupported => {
                panic!("expected BudgetExhausted, FrontierExhausted, or Found, got Unsupported")
            }
        };
        assert!(
            expansions_used <= 32,
            "budget backoff should limit expansions to ≤32, got {expansions_used}"
        );
    }

    #[test]
    fn cooldown_ineligible_entry_does_not_block_later_same_goal_sibling() {
        let origin = entity(71);
        let market = entity(72);
        let mut world = World::new(cargo_topology(origin, market)).unwrap();
        let (agent, bread) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Hungry", ControlSource::Ai).unwrap();
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(3))
                .unwrap();
            txn.set_ground_location(agent, origin).unwrap();
            txn.set_ground_location(bread, origin).unwrap();
            txn.set_component_homeostatic_needs(
                agent,
                HomeostaticNeeds::new(
                    worldwake_core::Permille::new(800).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                ),
            )
            .unwrap();
            commit_txn(txn);
            (agent, bread)
        };
        let (defs, handlers, recipes) = build_full_registries();
        let semantics = build_semantics_table(&defs);
        let scheduler = Scheduler::new(SystemManifest::canonical());
        let ranked_candidates = vec![
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(market),
                BTreeSet::new(),
                BTreeSet::from([market]),
            )),
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(origin),
                BTreeSet::from([bread]),
                BTreeSet::from([origin]),
            )),
        ];
        let exhaustion_cache = BTreeMap::from([(
            OpportunityKey {
                goal_key: ranked_candidates[0].grounded.key,
                anchor: ranked_candidates[0].grounded.anchor,
            },
            ExhaustionEntry {
                retry_state: ExhaustionRetryState::BudgetRetryPending,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                next_retry_tick: Some(Tick(20)),
                consecutive_failures: 2,
            },
        )]);

        let plans = super::build_candidate_plans(
            &world,
            &scheduler,
            agent,
            &ranked_candidates,
            None,
            &worldwake_core::BlockerMemory::default(),
            Tick(10),
            &cognitive(&ProfileFixture {
                snapshot_travel_horizon: 4,
                max_candidates_to_plan: 2,
                ..ProfileFixture::default()
            }),
            &execution_budget(&ProfileFixture {
                snapshot_travel_horizon: 4,
                max_candidates_to_plan: 2,
                ..ProfileFixture::default()
            }),
            &semantics,
            &defs,
            &handlers,
            &recipes,
            false,
            false,
            &exhaustion_cache,
        );

        assert_eq!(plans.len(), 1);
        assert_eq!(
            plans[0].opportunity.anchor,
            OpportunityAnchor::Place(origin)
        );
    }

    #[test]
    fn cooldown_ineligible_entry_is_filtered_out_of_candidate_plans() {
        let origin = entity(91);
        let market = entity(92);
        let mut world = World::new(cargo_topology(origin, market)).unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Hungry", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, origin).unwrap();
            txn.set_component_homeostatic_needs(
                agent,
                HomeostaticNeeds::new(
                    worldwake_core::Permille::new(800).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                ),
            )
            .unwrap();
            commit_txn(txn);
            agent
        };
        let (defs, handlers, recipes) = build_full_registries();
        let semantics = build_semantics_table(&defs);
        let scheduler = Scheduler::new(SystemManifest::canonical());
        let ranked_candidates = vec![ranked_goal(acquire_goal(
            CommodityKind::Bread,
            OpportunityAnchor::Place(market),
            BTreeSet::new(),
            BTreeSet::from([market]),
        ))];
        let opportunity = OpportunityKey {
            goal_key: ranked_candidates[0].grounded.key,
            anchor: ranked_candidates[0].grounded.anchor,
        };
        let exhaustion_cache = BTreeMap::from([(
            opportunity,
            ExhaustionEntry {
                retry_state: ExhaustionRetryState::BudgetRetryPending,
                invalidation_conditions: Vec::new(),
                baseline: crate::ExhaustionBaseline::default(),
                next_retry_tick: Some(Tick(20)),
                consecutive_failures: 2,
            },
        )]);

        let plans = super::build_candidate_plans(
            &world,
            &scheduler,
            agent,
            &ranked_candidates,
            None,
            &worldwake_core::BlockerMemory::default(),
            Tick(10),
            &cognitive(&ProfileFixture {
                snapshot_travel_horizon: 4,
                max_candidates_to_plan: 1,
                ..ProfileFixture::default()
            }),
            &execution_budget(&ProfileFixture {
                snapshot_travel_horizon: 4,
                max_candidates_to_plan: 1,
                ..ProfileFixture::default()
            }),
            &semantics,
            &defs,
            &handlers,
            &recipes,
            false,
            false,
            &exhaustion_cache,
        );

        assert!(
            plans.is_empty(),
            "cooldown-ineligible retry entries should be filtered before search when no sibling opportunity remains"
        );
    }
}
