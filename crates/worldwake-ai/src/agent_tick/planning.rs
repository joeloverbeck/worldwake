use crate::GoalDispatchKey;
use crate::GoalKindPlannerExt;
use crate::agenda_manager::{RejectionLifecycle, classify_rejection};
use crate::agent_tick::portfolio::{
    FeasibilityVerdict, Portfolio, SlotKind, assemble_portfolio, derive_operating_mode,
};
use crate::candidate_generation::relieved_needs_for_commodity;
use crate::decision_trace::{
    BindingRejection, CandidateSource, GoalSwitchSummary, PlanAttemptTrace, PlanSearchOutcome,
    PlanSearchTrace, PlannedStepSummary, PortfolioSlotTrace, PortfolioTrace, RankedGoalSummary,
    SameGoalPlanningStopReason, SameGoalPlanningTrace, SelectedPlanReplacementKind,
    SelectedPlanReplacementTrace, SelectedPlanSearchProvenance, SelectedPlanSource,
    SelectedPlanTrace, SelectionTrace, SideBenefitTrace, SnapshotAdmissionTrace,
    SnapshotCacheCounters, SnapshotContinuationOutcome, SnapshotContinuationTrace,
    StrategicStepTrace, TargetBeliefPresence, snapshot_admission_trace_entries,
};
use crate::exhaustion::{derive_invalidation_conditions, invalidate_exhausted_goals};
use crate::feasibility_probe;
use crate::goal_schema::{FrontierExhaustionStrategy, GoalDispatchKeySchemaExt};
use crate::opportunity_compiler::PerceivedOpportunityIndex;
use crate::perf_telemetry::record_planning_phase_duration;
use crate::plan_selection::SelectionCandidatePlan;
use crate::plan_step_expectations::{
    expire_plan_step_expectations, persist_expectation_store_update, write_plan_step_expectations,
};
use crate::search::{
    PartialPlanSkeletonSource, PlanSearchResult, SearchTraceMetadata,
    search_plan_with_trace_metadata_and_source,
};
use crate::{
    AcceptedRepairProvenance, AgendaEntry, AgendaPhase, AgendaState, AgentDecisionRuntime,
    DirtySet, ExhaustionEntry, ExhaustionRetryState, ExpectationFailureCause,
    ExpectationFailurePhase, KillCondition, OpportunityExpectationFailureIncident, OpportunityKey,
    PendingRepairContext, PlanTerminalKind, PlanValue, PlannedPlan, PlannedStep,
    PlannerOpSemantics, PlanningStateCacheCounters, RevivalTrigger, authoritative_target,
    budget_exhausted_partial_plan_segment,
    build_planning_snapshot_with_blocked_facility_uses_and_route_preference,
    information_barrier_partial_plan_segment, planner_ops::committed_source_for_offer,
    planner_ops::expectation_kind_for_offer, ranking::OrderedRanked, revalidate_next_step,
    select_best_plan,
};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;
use worldwake_core::{
    ActionDefId, BlockerMemory, CognitiveProfile, DecisionEventPayload, DiscrepancyMemory,
    EntityId, EventLog, EventTag, ExecutionBudget, GoalKind, IntentionFrame, OpportunityAnchor,
    Permille, PlanAdoptedPayload, RepairKind, RoutePreference, RoutePreferenceSummary,
    RouteSegment, TestimonyReliabilityKey, TestimonyTrustSummary, Tick,
    belief_topic_to_topic_scope,
};
use worldwake_sim::{
    ActionHandlerRegistry, ActionPayload, GoalBeliefView, ProfileBeliefView, RecipeRegistry,
    RuntimeBeliefView, Scheduler, SpatialBeliefView, get_affordances_for_defs,
};

use super::frame::plan_completion_tick_for_adoption;
use super::{
    assumptions_to_refs, current_step, emit_decision_event, populate_assumptions,
    runtime_belief_view, update_frame_for_adopted_plan,
};

#[derive(Clone, Debug)]
pub(crate) struct CandidatePlanSearch {
    pub opportunity: OpportunityKey,
    pub result: PlanSearchResult,
    pub perceived_cost: Option<u32>,
    pub skeleton_source: Option<PartialPlanSkeletonSource>,
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
    Option<PortfolioTrace>,
    Option<Vec<SnapshotAdmissionTrace>>,
    Option<SnapshotCacheCounters>,
    Option<PlanningStateCacheCounters>,
    BTreeSet<worldwake_core::HomeostaticNeedId>,
);

trait DecisionHistoryContextView: ProfileBeliefView + SpatialBeliefView {}

impl<T: ProfileBeliefView + SpatialBeliefView + ?Sized> DecisionHistoryContextView for T {}

#[derive(Clone, Debug)]
pub(super) struct CandidatePlanningPass {
    portfolio: Portfolio,
    plausible_slots: Vec<SlotKind>,
    search_order: Vec<OpportunityKey>,
    plans: Vec<CandidatePlanSearch>,
    snapshot_admissions: Option<Vec<SnapshotAdmissionTrace>>,
    snapshot_cache_counters: Option<SnapshotCacheCounters>,
    planning_state_cache_counters: Option<PlanningStateCacheCounters>,
}

impl CandidatePlanningPass {
    pub(super) fn selection_plans(&self) -> Vec<SelectionCandidatePlan> {
        selection_candidates(&self.plans)
    }

    fn plausible_opportunities(&self) -> Vec<OpportunityKey> {
        self.plausible_slots
            .iter()
            .filter_map(|kind| {
                self.portfolio.slots.get(kind).map(|slot| OpportunityKey {
                    goal_key: slot.ranked.offer.key,
                    anchor: slot.ranked.offer.anchor,
                })
            })
            .collect()
    }

    fn search_opportunities(&self) -> &[OpportunityKey] {
        &self.search_order
    }

    fn portfolio_trace(&self) -> PortfolioTrace {
        PortfolioTrace {
            slots: self
                .portfolio
                .slots
                .iter()
                .map(|(kind, slot)| {
                    (
                        *kind,
                        PortfolioSlotTrace {
                            goal_key: slot.ranked.offer.key,
                            motive_score: slot.ranked.motive_score,
                            feasibility: slot.feasibility.clone(),
                        },
                    )
                })
                .collect(),
            slots_attempted: self
                .plans
                .iter()
                .filter(|plan| self.plausible_opportunities().contains(&plan.opportunity))
                .count()
                .try_into()
                .expect("portfolio slot attempts exceed u8"),
        }
    }
}

impl std::ops::Deref for CandidatePlanningPass {
    type Target = [CandidatePlanSearch];

    fn deref(&self) -> &Self::Target {
        &self.plans
    }
}

fn found_plan_blocks_later_goals(plan: &PlannedPlan) -> bool {
    match plan.terminal_kind {
        crate::PlanTerminalKind::GoalSatisfied | crate::PlanTerminalKind::CombatCommitment => true,
        crate::PlanTerminalKind::InformationBarrier { .. }
        | crate::PlanTerminalKind::CoordinationBarrier { .. }
        | crate::PlanTerminalKind::ResourceBarrier { .. }
        | crate::PlanTerminalKind::JurisdictionBarrier { .. }
        | crate::PlanTerminalKind::SearchBudgetExhausted { .. } => {
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
    ranked_candidates: &OrderedRanked<'_>,
    selected_plan: &PlannedPlan,
    side_benefit_weight: Permille,
) -> Option<PlanValue> {
    let ranked = ranked_candidates.iter().find(|candidate| {
        candidate.offer.key == selected_plan.opportunity.goal_key
            && candidate.offer.anchor == selected_plan.opportunity.anchor
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

pub(super) fn summarize_ranked_goal(ranked: &AgendaEntry) -> RankedGoalSummary {
    RankedGoalSummary {
        opportunity: OpportunityKey {
            goal_key: ranked.offer.key,
            anchor: ranked.offer.anchor,
        },
        priority_class: ranked.priority_class,
        motive_score: ranked.motive_score,
        motive_source_contributions: motive_source_contributions_for_summary(ranked),
        provenance: ranked.provenance.clone(),
        source_reliability_discount: ranked.source_reliability_discount.clone(),
        competition_discount: ranked.competition_discount.clone(),
        source_composite: ranked.source_composite,
        feasibility: ranked.feasibility,
        acquisition_quantity: ranked.offer.acquisition_quantity,
        artifact_axes: None,
    }
}

fn motive_source_contributions_for_summary(
    ranked: &AgendaEntry,
) -> Vec<(worldwake_core::MotiveSourceRef, u32)> {
    ranked.motive_source_contributions.clone()
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

fn same_goal_search_failure_incidents(
    current_plan: Option<&PlannedPlan>,
    selected_plan: &PlannedPlan,
    selection_plans: &[SelectionCandidatePlan],
    current_place: Option<EntityId>,
    tick: Tick,
) -> Vec<OpportunityExpectationFailureIncident> {
    let Some(current_plan) = current_plan else {
        return Vec::new();
    };
    let Some(current_place) = current_place else {
        return Vec::new();
    };
    if selected_plan.goal != current_plan.goal
        || selected_plan.opportunity == current_plan.opportunity
        || current_plan.opportunity.anchor != OpportunityAnchor::Place(current_place)
    {
        return Vec::new();
    }

    let current_failed_search = selection_plans.iter().any(|plan| {
        plan.searched_opportunity == current_plan.opportunity && plan.found_plan.is_none()
    });
    if !current_failed_search {
        return Vec::new();
    }

    let selected_sibling_found = selection_plans.iter().any(|plan| {
        plan.searched_opportunity == selected_plan.opportunity && plan.found_plan.is_some()
    });
    if !selected_sibling_found {
        return Vec::new();
    }

    let (Some(source), Some(expectation_kind)) =
        (current_plan.committed_source, current_plan.expectation_kind)
    else {
        return Vec::new();
    };
    vec![OpportunityExpectationFailureIncident {
        opportunity: current_plan.opportunity,
        source,
        expectation_kind,
        detected_at_tick: tick,
        phase: ExpectationFailurePhase::Search,
        cause: ExpectationFailureCause::SameGoalSearchInfeasibleWhileSiblingSucceeded,
    }]
}

#[cfg(test)]
#[allow(clippy::too_many_arguments, clippy::trivially_copy_pass_by_ref)]
pub(super) fn build_candidate_plans(
    world: &worldwake_core::World,
    scheduler: &Scheduler,
    agent: worldwake_core::EntityId,
    ranked_candidates: &OrderedRanked<'_>,
    committed_opportunity: Option<OpportunityKey>,
    discrepancy_memory: &DiscrepancyMemory,
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
) -> CandidatePlanningPass {
    build_candidate_plans_with_route_preference(
        world,
        scheduler,
        agent,
        ranked_candidates,
        committed_opportunity,
        discrepancy_memory,
        blocked_memory,
        current_tick,
        cognitive,
        execution_budget,
        semantics_table,
        action_defs,
        action_handlers,
        recipe_registry,
        collect_rejections,
        collect_expansion_summaries,
        exhaustion_cache,
        None,
    )
}

#[allow(clippy::too_many_arguments, clippy::trivially_copy_pass_by_ref)]
pub(super) fn build_candidate_plans_with_route_preference(
    world: &worldwake_core::World,
    scheduler: &Scheduler,
    agent: worldwake_core::EntityId,
    ranked_candidates: &OrderedRanked<'_>,
    committed_opportunity: Option<OpportunityKey>,
    discrepancy_memory: &DiscrepancyMemory,
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
    route_preference: Option<&RoutePreference>,
) -> CandidatePlanningPass {
    let opportunity_index = PerceivedOpportunityIndex::default();
    let view = runtime_belief_view(agent, world, scheduler, action_defs, recipe_registry);
    let operating_mode = derive_operating_mode(&view, agent, ranked_candidates);
    build_candidate_plans_with_opportunity_index(
        world,
        scheduler,
        agent,
        ranked_candidates,
        committed_opportunity,
        discrepancy_memory,
        blocked_memory,
        current_tick,
        cognitive,
        execution_budget,
        semantics_table,
        action_defs,
        action_handlers,
        recipe_registry,
        collect_rejections,
        collect_expansion_summaries,
        exhaustion_cache,
        &opportunity_index,
        route_preference,
        operating_mode,
    )
}

#[allow(clippy::too_many_arguments, clippy::trivially_copy_pass_by_ref)]
fn build_candidate_plans_with_opportunity_index(
    world: &worldwake_core::World,
    scheduler: &Scheduler,
    agent: worldwake_core::EntityId,
    ranked_candidates: &OrderedRanked<'_>,
    committed_opportunity: Option<OpportunityKey>,
    discrepancy_memory: &DiscrepancyMemory,
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
    opportunity_index: &PerceivedOpportunityIndex,
    route_preference: Option<&RoutePreference>,
    operating_mode: worldwake_core::OperatingMode,
) -> CandidatePlanningPass {
    build_candidate_plans_with_sources(
        world,
        scheduler,
        agent,
        ranked_candidates,
        committed_opportunity,
        discrepancy_memory,
        blocked_memory,
        current_tick,
        cognitive,
        execution_budget,
        semantics_table,
        action_defs,
        action_handlers,
        recipe_registry,
        collect_rejections,
        collect_expansion_summaries,
        exhaustion_cache,
        &BTreeMap::new(),
        opportunity_index,
        route_preference,
        operating_mode,
    )
}

#[allow(clippy::too_many_arguments, clippy::trivially_copy_pass_by_ref)]
pub(super) fn build_candidate_plans_with_sources(
    world: &worldwake_core::World,
    scheduler: &Scheduler,
    agent: worldwake_core::EntityId,
    ranked_candidates: &OrderedRanked<'_>,
    committed_opportunity: Option<OpportunityKey>,
    discrepancy_memory: &DiscrepancyMemory,
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
    candidate_sources: &BTreeMap<OpportunityKey, CandidateSource>,
    opportunity_index: &PerceivedOpportunityIndex,
    route_preference: Option<&RoutePreference>,
    operating_mode: worldwake_core::OperatingMode,
) -> CandidatePlanningPass {
    let view = runtime_belief_view(agent, world, scheduler, action_defs, recipe_registry);
    let route_preference_profile =
        route_preference.and_then(|_| ProfileBeliefView::route_preference_profile(&view, agent));
    let mut admitted_candidates: Vec<_> = ranked_candidates
        .iter()
        .filter(|c| {
            opportunity_admitted_by_exhaustion(
                exhaustion_cache,
                OpportunityKey {
                    goal_key: c.offer.key,
                    anchor: c.offer.anchor,
                },
                current_tick,
            )
        })
        .cloned()
        .collect();
    let admitted_candidates = crate::ranking::sort_in_place(&mut admitted_candidates);
    let probe_context = feasibility_probe::ProbeContext {
        belief_view: &view,
        discrepancy_memory,
        blocker_memory: blocked_memory,
        semantics_table,
        action_defs,
        action_handlers,
        current_tick,
        agent,
        agent_place: SpatialBeliefView::effective_place(&view, agent),
    };
    let portfolio_weights = ProfileBeliefView::portfolio_weights_profile(&view, agent);
    let portfolio = assemble_portfolio(
        &admitted_candidates,
        committed_opportunity,
        &portfolio_weights,
        operating_mode,
        |ranked| feasibility_probe::probe(ranked, &probe_context),
    );
    let plausible_slots = portfolio
        .plausible_slots_by_score(&portfolio_weights)
        .into_iter()
        .map(|(kind, _slot)| kind)
        .collect::<Vec<_>>();
    // Opportunities the feasibility probe has already rejected. Excluding
    // them from the search budget is the portfolio's budget-saving function
    // (FND-20): no point searching a candidate whose belief-grounded
    // prerequisites we already know are not met.
    //
    // We do NOT exclude the agent's committed opportunity even when the
    // probe rejects it. "Rejected" here simply means "no productive
    // belief-grounded work is possible right now"; for a goal the agent
    // is already pursuing (`runtime.current_plan` still targets it) that
    // often means the plan reached its terminal (e.g. cargo is already at
    // its destination) rather than the goal being truly infeasible.
    // Searching the committed goal preserves the intention across ticks
    // (FND-21) — the downstream selection logic sees "same goal, no new
    // plan" and keeps `active_goal` pinned instead of defecting.
    let rejected_opportunities: BTreeSet<OpportunityKey> = portfolio
        .slots
        .values()
        .filter_map(|slot| {
            matches!(
                slot.feasibility,
                crate::agent_tick::portfolio::FeasibilityVerdict::RejectedBeforeSearch { .. }
            )
            .then_some(OpportunityKey {
                goal_key: slot.ranked.offer.key,
                anchor: slot.ranked.offer.anchor,
            })
        })
        .collect();
    // Search order follows `ranking::compare_ranked_goals` — the same
    // composite preference used everywhere else in the decision cycle —
    // rather than re-prioritising by slot category. The portfolio's role
    // here is trace/diagnostic plus probe-based budget protection; re-sorting
    // the search order by slot category caused higher-motive non-survival
    // goals (e.g. `ExploreLocation` under hunger pressure) to be deferred
    // behind lower-motive survival slot winners, regressing main's
    // "highest-preference first" behaviour.
    let search_order: Vec<OpportunityKey> = admitted_candidates
        .iter()
        .filter_map(|ranked| {
            let opp = OpportunityKey {
                goal_key: ranked.offer.key,
                anchor: ranked.offer.anchor,
            };
            (!rejected_opportunities.contains(&opp)).then_some(opp)
        })
        .collect();
    let candidate_cap = usize::from(portfolio_weights.max_plans_for_mode(operating_mode));

    // All candidates filtered by exhausted-goal skip set or probe — no snapshot needed.
    if search_order.is_empty() {
        return CandidatePlanningPass {
            portfolio,
            plausible_slots,
            search_order,
            plans: Vec::new(),
            snapshot_admissions: None,
            snapshot_cache_counters: None,
            planning_state_cache_counters: None,
        };
    }

    let admitted_by_opportunity = admitted_candidates
        .iter()
        .cloned()
        .map(|ranked| {
            (
                OpportunityKey {
                    goal_key: ranked.offer.key,
                    anchor: ranked.offer.anchor,
                },
                ranked,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut results = Vec::with_capacity(search_order.len().min(candidate_cap));
    let mut snapshot_admissions = Vec::new();
    let mut snapshot_cache_counters = SnapshotCacheCounters::default();
    let mut planning_state_cache_counters = PlanningStateCacheCounters::default();
    let mut snapshot_count = 0usize;
    let mut planning_state_count = 0usize;
    let mut continue_same_goal_after_found = None;
    for opportunity in search_order.iter().take(candidate_cap) {
        let ranked = admitted_by_opportunity
            .get(opportunity)
            .expect("search order must map back to an admitted opportunity");
        if let Some(found_goal) = continue_same_goal_after_found
            && ranked.offer.key != found_goal
        {
            break;
        }
        let mut rejections = Vec::new();
        let mut expansions = Vec::new();
        let mut trace_metadata = SearchTraceMetadata::default();
        let snapshot = build_planning_snapshot_with_blocked_facility_uses_and_route_preference(
            &view,
            agent,
            &ranked.offer.evidence_entities,
            &ranked.offer.evidence_places,
            cognitive.snapshot_travel_horizon,
            blocked_memory,
            current_tick,
            ranked.offer.key.kind.relevant_op_kinds(),
            route_preference,
            route_preference_profile.as_ref(),
        );
        let opportunity = OpportunityKey {
            goal_key: ranked.offer.key,
            anchor: ranked.offer.anchor,
        };
        snapshot_admissions.extend(snapshot_admission_trace_entries(opportunity, &snapshot));
        let candidate_source = candidate_sources
            .get(&opportunity)
            .copied()
            .unwrap_or(CandidateSource::Emitter);
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
        let result = search_plan_with_trace_metadata_and_source(
            &snapshot,
            &ranked.offer,
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
            candidate_source,
            opportunity_index,
        );
        let mut result = match result {
            PlanSearchResult::Found(plan)
                if plan.steps.first().is_some_and(|step| {
                    step.op_kind == crate::PlannerOpKind::Harvest
                        && matches!(
                            ranked.offer.key.kind,
                            GoalKind::AcquireCommodity { .. } | GoalKind::RestockCommodity { .. }
                        )
                        && matches!(
                            ranked.offer.anchor,
                            OpportunityAnchor::Place(place)
                                if SpatialBeliefView::effective_place(&view, agent) == Some(place)
                        )
                        && !revalidate_next_step(
                            &view,
                            agent,
                            step,
                            &crate::MaterializationBindings::default(),
                            action_defs,
                            action_handlers,
                        )
                }) =>
            {
                PlanSearchResult::Unsupported
            }
            other => other,
        };
        if let PlanSearchResult::Found(plan) = &mut result {
            plan.method_id = trace_metadata
                .method_trace
                .as_ref()
                .and_then(|trace| trace.method_id);
        }
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
        snapshot_cache_counters.add_assign(snapshot.snapshot_cache_counters());
        snapshot_count += 1;
        if let Some(counters) = trace_metadata.planning_state_cache_counters {
            planning_state_cache_counters.add_assign(counters);
            planning_state_count += 1;
        }
        results.push(CandidatePlanSearch {
            opportunity,
            result,
            perceived_cost,
            skeleton_source: trace_metadata.skeleton_source.clone(),
            trace_metadata,
            binding_rejections: rejections,
            expansion_summaries: expansions,
        });
        if found_blocks_later_goals {
            continue_same_goal_after_found = Some(opportunity.goal_key);
        }
    }
    CandidatePlanningPass {
        portfolio,
        plausible_slots,
        search_order,
        plans: results,
        snapshot_admissions: (!snapshot_admissions.is_empty()).then_some(snapshot_admissions),
        snapshot_cache_counters: (snapshot_count > 0).then_some(snapshot_cache_counters),
        planning_state_cache_counters: (planning_state_count > 0)
            .then_some(planning_state_cache_counters),
    }
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
    plausible_opportunities: &[OpportunityKey],
    candidate_plan_cap: u8,
    plans: &[CandidatePlanSearch],
) -> Option<SameGoalPlanningTrace> {
    if plausible_opportunities.is_empty() {
        return None;
    }

    let admitted_cap = usize::from(candidate_plan_cap);
    let candidate_cap_hit = plausible_opportunities.len() > admitted_cap;
    let continuation_trigger = plans
        .iter()
        .find(|plan| plan.result.is_found())
        .map(|plan| plan.opportunity);
    let stop_reason =
        if let Some(found_goal) = continuation_trigger.map(|opportunity| opportunity.goal_key) {
            if let Some(next_candidate) = plausible_opportunities
                .iter()
                .take(admitted_cap)
                .skip(plans.len())
                .find(|candidate| candidate.goal_key != found_goal)
            {
                SameGoalPlanningStopReason::EncounteredDifferentGoal {
                    next_goal: next_candidate.goal_key,
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

fn ranked_goal_for_opportunity<'a>(
    ranked_candidates: &'a OrderedRanked<'a>,
    opportunity: OpportunityKey,
) -> Option<&'a AgendaEntry> {
    ranked_candidates.iter().find(|candidate| {
        candidate.offer.key == opportunity.goal_key && candidate.offer.anchor == opportunity.anchor
    })
}

fn summarize_snapshot_continuation(
    current_opportunity: OpportunityKey,
    ranked_candidates: &OrderedRanked<'_>,
    planning_switch_margin: Permille,
) -> SnapshotContinuationTrace {
    let top = ranked_candidates.first();
    let current = ranked_goal_for_opportunity(ranked_candidates, current_opportunity);
    let top_opportunity = top.map(|ranked| OpportunityKey {
        goal_key: ranked.offer.key,
        anchor: ranked.offer.anchor,
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
    ranked_candidates: &OrderedRanked<'_>,
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
                if let Some(commodity) = acquisition_exhaustion_signal_commodity(
                    &plan.opportunity.goal_key.kind,
                    &plan.result,
                    recipe_registry,
                ) {
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

fn write_budget_exhausted_partial_plan_segments(
    agenda_state: &mut AgendaState,
    ranked_candidates: &OrderedRanked<'_>,
    plans: &[CandidatePlanSearch],
    tick: Tick,
    cognitive: &CognitiveProfile,
) -> usize {
    let mut written = 0usize;
    for plan in plans {
        let PlanSearchResult::BudgetExhausted { expansions_used } = plan.result else {
            continue;
        };
        let remaining_skeleton = plan
            .skeleton_source
            .as_ref()
            .map(|source| source.remaining_skeleton.clone());
        let Some(candidate) = ranked_candidates.iter().find(|candidate| {
            candidate.offer.key == plan.opportunity.goal_key
                && candidate.offer.anchor == plan.opportunity.anchor
        }) else {
            continue;
        };

        let mut entry = candidate.clone();
        entry.phase = AgendaPhase::Suspended;
        entry.last_reconsidered_tick = tick;
        entry.revival_trigger = None;
        entry.kill_condition = KillCondition::External;
        entry.partial_plan_segment = Some(budget_exhausted_partial_plan_segment(
            candidate.offer.clone(),
            u32::from(expansions_used),
            u32::from(cognitive.max_node_expansions),
            remaining_skeleton,
            tick,
            written
                .try_into()
                .expect("partial segment counter exceeded u16 in one planning tick"),
            cognitive,
        ));
        agenda_state.pending.remove(&entry.key);
        if agenda_state
            .committed
            .as_ref()
            .is_some_and(|committed| committed.key == entry.key)
        {
            agenda_state.committed = None;
        }
        agenda_state.suspended.insert(entry.key, entry);
        written += 1;
    }
    written
}

fn write_information_barrier_partial_plan_segment(
    agenda_state: &mut AgendaState,
    ranked_candidates: &OrderedRanked<'_>,
    selected_plan: &PlannedPlan,
    plans: &[CandidatePlanSearch],
    tick: Tick,
    cognitive: &CognitiveProfile,
) -> bool {
    let PlanTerminalKind::InformationBarrier { topic } = selected_plan.terminal_kind else {
        return false;
    };
    if matches!(
        selected_plan.goal.kind,
        GoalKind::AskWitness { .. } | GoalKind::ShareBelief { .. }
    ) {
        return false;
    }
    let Some(candidate) = ranked_candidates.iter().find(|candidate| {
        candidate.offer.key == selected_plan.opportunity.goal_key
            && candidate.offer.anchor == selected_plan.opportunity.anchor
    }) else {
        return false;
    };
    let remaining_skeleton = plans
        .iter()
        .find(|plan| plan.opportunity == selected_plan.opportunity)
        .and_then(|plan| plan.skeleton_source.as_ref())
        .map(|source| source.remaining_skeleton.clone());
    let Some(segment) = information_barrier_partial_plan_segment(
        candidate.offer.clone(),
        topic,
        remaining_skeleton,
        tick,
        next_partial_plan_segment_counter(agenda_state, tick),
        cognitive,
    ) else {
        return false;
    };

    let mut entry = candidate.clone();
    entry.phase = AgendaPhase::Suspended;
    entry.last_reconsidered_tick = tick;
    entry.revival_trigger = None;
    entry.kill_condition = KillCondition::External;
    entry.partial_plan_segment = Some(segment);
    agenda_state.pending.remove(&entry.key);
    if agenda_state
        .committed
        .as_ref()
        .is_some_and(|committed| committed.key == entry.key)
    {
        agenda_state.committed = None;
    }
    agenda_state.suspended.insert(entry.key, entry);
    true
}

fn next_partial_plan_segment_counter(agenda_state: &AgendaState, tick: Tick) -> u16 {
    agenda_state
        .suspended
        .values()
        .filter_map(|entry| entry.partial_plan_segment.as_ref())
        .filter(|segment| segment.created_tick == tick)
        .map(|segment| segment.id.local_counter)
        .max()
        .map_or(0, |counter| counter.saturating_add(1))
}

fn frontier_exhaustion_entry(
    goal_kind: &GoalKind,
    invalidation_conditions: Vec<crate::ExhaustionInvalidationCondition>,
    baseline: crate::ExhaustionBaseline,
    tick: Tick,
    cognitive: &CognitiveProfile,
) -> ExhaustionEntry {
    match GoalDispatchKey::from_goal_kind(goal_kind)
        .declaration()
        .frontier_exhaustion_strategy
    {
        FrontierExhaustionStrategy::CooldownRetry => ExhaustionEntry::budget_retry_pending(
            invalidation_conditions,
            baseline,
            tick,
            cognitive,
        ),
        FrontierExhaustionStrategy::PermanentUntilInvalidator => {
            ExhaustionEntry::frontier_exhausted(invalidation_conditions, baseline)
        }
    }
}

fn acquisition_exhaustion_signal_commodity(
    goal_kind: &GoalKind,
    result: &crate::PlanSearchResult,
    recipe_registry: &RecipeRegistry,
) -> Option<worldwake_core::CommodityKind> {
    match result {
        crate::PlanSearchResult::BudgetExhausted { .. } => {
            goal_kind.target_commodity(recipe_registry)
        }
        crate::PlanSearchResult::FrontierExhausted { .. } => match goal_kind {
            GoalKind::AcquireCommodity {
                commodity,
                purpose: worldwake_core::CommodityPurpose::SelfConsume,
                ..
            } => Some(*commodity),
            _ => None,
        },
        crate::PlanSearchResult::Found(_) | crate::PlanSearchResult::Unsupported => None,
    }
}

fn has_pending_budget_retry(runtime: &AgentDecisionRuntime, current_tick: Tick) -> bool {
    runtime
        .exhaustion_cache
        .values()
        .any(|entry| entry.is_retry_eligible(current_tick))
}

fn score_gap(committed_motive: u32, rejected_motive: u32) -> i32 {
    let gap = i64::from(committed_motive) - i64::from(rejected_motive);
    gap.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

fn ranked_goal_comparison_dimension_tag(
    dimension: crate::ranking::RankedGoalComparisonDimension,
) -> worldwake_core::RankedGoalComparisonDimensionTag {
    use crate::ranking::RankedGoalComparisonDimension as Source;
    use worldwake_core::RankedGoalComparisonDimensionTag as Tag;

    match dimension {
        Source::PriorityClass => Tag::PriorityClass,
        Source::SubstitutePreferenceOrder => Tag::SubstitutePreferenceOrder,
        Source::MotiveScore => Tag::MotiveScore,
        Source::SourceComposite => Tag::SourceComposite,
        Source::Feasibility => Tag::Feasibility,
        Source::GoalSpecificity => Tag::GoalSpecificity,
        Source::OpportunityStrength => Tag::OpportunityStrength,
        Source::ShareBeliefTopicOrder => Tag::ShareBeliefTopicOrder,
        Source::GoalKindOrder => Tag::GoalKindOrder,
        Source::CommodityKey => Tag::CommodityKey,
        Source::EntityKey => Tag::EntityKey,
        Source::PlaceKey => Tag::PlaceKey,
    }
}

fn rejection_dimension(
    rejected: &AgendaEntry,
    committed: Option<&AgendaEntry>,
) -> Option<worldwake_core::RankedGoalComparisonDimensionTag> {
    let committed = committed?;
    crate::ranking::explain_ranked_goal_order(rejected, committed)
        .map(|comparison| ranked_goal_comparison_dimension_tag(comparison.decisive_dimension))
}

pub(super) fn build_rejected_alternatives(
    ranked_candidates: &OrderedRanked<'_>,
    portfolio: &Portfolio,
    committed_goal: worldwake_core::GoalKey,
    committed_motive: u32,
    max_alternatives: u8,
) -> Vec<worldwake_core::RejectedAlternativeSummary> {
    #[derive(Clone, Copy)]
    struct RejectedGoal<'a> {
        goal_key: worldwake_core::GoalKey,
        motive_score: u32,
        rejection_reason: worldwake_core::GoalRejectionReason,
        entry: &'a AgendaEntry,
    }

    let mut rejected_by_goal = BTreeMap::<worldwake_core::GoalKey, RejectedGoal>::new();
    let committed_entry = ranked_candidates
        .iter()
        .find(|candidate| candidate.offer.key == committed_goal);
    for slot in portfolio.slots.values() {
        if matches!(
            slot.feasibility,
            FeasibilityVerdict::RejectedBeforeSearch { .. }
        ) {
            rejected_by_goal.insert(
                slot.ranked.offer.key,
                RejectedGoal {
                    goal_key: slot.ranked.offer.key,
                    motive_score: slot.ranked.motive_score,
                    rejection_reason: worldwake_core::GoalRejectionReason::FeasibilityProbeFailed,
                    entry: &slot.ranked,
                },
            );
        }
    }

    for candidate in ranked_candidates
        .iter()
        .filter(|candidate| candidate.offer.key != committed_goal)
    {
        rejected_by_goal
            .entry(candidate.offer.key)
            .and_modify(|existing| {
                if matches!(
                    existing.rejection_reason,
                    worldwake_core::GoalRejectionReason::LowerMotive
                ) && candidate.motive_score > existing.motive_score
                {
                    existing.motive_score = candidate.motive_score;
                    existing.entry = candidate;
                }
            })
            .or_insert(RejectedGoal {
                goal_key: candidate.offer.key,
                motive_score: candidate.motive_score,
                rejection_reason: worldwake_core::GoalRejectionReason::LowerMotive,
                entry: candidate,
            });
    }

    let mut rejected = rejected_by_goal.into_values().collect::<Vec<_>>();
    rejected.sort_unstable_by(|left, right| {
        right
            .motive_score
            .cmp(&left.motive_score)
            .then_with(|| left.goal_key.cmp(&right.goal_key))
    });
    rejected.truncate(usize::from(max_alternatives));
    rejected
        .into_iter()
        .map(|rejected| worldwake_core::RejectedAlternativeSummary {
            goal_key: rejected.goal_key,
            rejection_reason: rejected.rejection_reason,
            score_gap: score_gap(committed_motive, rejected.motive_score),
            rejection_dimension: rejection_dimension(rejected.entry, committed_entry),
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn emit_plan_selection_events(
    event_log: &mut EventLog,
    tick: Tick,
    agent: EntityId,
    runtime: &AgentDecisionRuntime,
    view: &dyn DecisionHistoryContextView,
    ranked_candidates: &OrderedRanked<'_>,
    portfolio: &Portfolio,
    current_goal_before_selection: Option<worldwake_core::GoalKey>,
    selected_plan: &PlannedPlan,
    max_alternatives: u8,
    prepared_frame: Option<&IntentionFrame>,
) {
    let assumptions = prepared_frame.map_or(&[][..], |frame| frame.assumptions.as_slice());
    if current_goal_before_selection != Some(selected_plan.goal) {
        let committed = ranked_candidates
            .iter()
            .find(|candidate| candidate.offer.key == selected_plan.goal)
            .expect("selected plan must map to a ranked goal");
        emit_decision_event(
            event_log,
            tick,
            agent,
            EventTag::GoalCommitted,
            DecisionEventPayload::GoalCommitted(worldwake_core::GoalCommittedPayload {
                agent,
                goal_key: selected_plan.goal,
                motive_score: committed.motive_score,
                decisive_motive_sources: committed.offer.motive_sources.clone(),
                rejected_alternatives: build_rejected_alternatives(
                    ranked_candidates,
                    portfolio,
                    selected_plan.goal,
                    committed.motive_score,
                    max_alternatives,
                ),
                assumptions: assumptions_to_refs(
                    assumptions,
                    max_alternatives,
                    Some(selected_plan),
                ),
                testimony_trust_context: testimony_trust_context_for_plan(
                    runtime,
                    view,
                    agent,
                    selected_plan,
                ),
                route_preference_context: route_preference_context_for_plan(
                    runtime,
                    view,
                    agent,
                    selected_plan,
                    tick,
                ),
            }),
        );
    }
    emit_decision_event(
        event_log,
        tick,
        agent,
        EventTag::PlanAdopted,
        DecisionEventPayload::PlanAdopted(PlanAdoptedPayload {
            agent,
            goal_key: selected_plan.goal,
            plan_step_count: selected_plan
                .steps
                .len()
                .try_into()
                .expect("plan step count exceeds u16"),
            assumptions: assumptions_to_refs(assumptions, max_alternatives, Some(selected_plan)),
        }),
    );
}

fn testimony_trust_context_for_plan(
    runtime: &AgentDecisionRuntime,
    view: &dyn DecisionHistoryContextView,
    agent: EntityId,
    selected_plan: &PlannedPlan,
) -> Vec<TestimonyTrustSummary> {
    let GoalKind::AskWitness { witness, topic } = &selected_plan.goal.kind else {
        return Vec::new();
    };
    let Some(profile) = view.testimony_trust_profile(agent) else {
        return Vec::new();
    };
    let topic = belief_topic_to_topic_scope(topic);
    let key = TestimonyReliabilityKey {
        source: *witness,
        topic,
    };
    runtime
        .testimony_reliability
        .get(&key)
        .map(|entry| {
            vec![TestimonyTrustSummary {
                source: *witness,
                topic,
                trust: entry.trust(&profile, topic),
                observations: entry.observations(),
            }]
        })
        .unwrap_or_default()
}

fn route_preference_context_for_plan(
    runtime: &AgentDecisionRuntime,
    view: &dyn DecisionHistoryContextView,
    agent: EntityId,
    selected_plan: &PlannedPlan,
    tick: Tick,
) -> Vec<RoutePreferenceSummary> {
    let Some(profile) = view.route_preference_profile(agent) else {
        return Vec::new();
    };
    let mut current_place = SpatialBeliefView::effective_place(view, agent);
    let mut summaries = BTreeMap::new();

    for step in &selected_plan.steps {
        if step.op_kind != crate::PlannerOpKind::Travel {
            continue;
        }
        let Some(from) = current_place else {
            continue;
        };
        let Some(to) = step.primary_target() else {
            continue;
        };
        let segment = RouteSegment::new(from, to);
        current_place = Some(to);
        let Some(entry) = runtime.route_preference.get(&segment) else {
            continue;
        };
        summaries.entry(segment).or_insert(RoutePreferenceSummary {
            segment,
            preference: entry.preference(&profile, tick),
            last_safe_tick: entry.last_safe_tick,
            last_dangerous_tick: entry.last_dangerous_tick,
        });
    }

    summaries.into_values().collect()
}

fn clear_committed_plan_state(
    runtime: &mut AgentDecisionRuntime,
    jc: &mut Option<IntentionFrame>,
    facility_intents: &mut worldwake_core::ContentionIntents,
) {
    if jc.is_some() {
        runtime.last_frame_clear_reason = Some(worldwake_core::FrameClearReason::LostPlan);
    }
    *jc = None;
    facility_intents.intents.clear();
    runtime.materialization_bindings.clear();
    runtime.current_plan = None;
    runtime.current_step_index = 0;
    runtime.step_in_flight = false;
    runtime.pending_repair_context = None;
    runtime.accepted_repair = None;
}

#[allow(clippy::too_many_arguments)]
fn apply_committed_rejection_lifecycle(
    world: &worldwake_core::World,
    scheduler: &Scheduler,
    runtime: &mut AgentDecisionRuntime,
    agenda_state: &mut AgendaState,
    jc: &mut Option<IntentionFrame>,
    facility_intents: &mut worldwake_core::ContentionIntents,
    agent: EntityId,
    current_tick: Tick,
    action_defs: &worldwake_sim::ActionDefRegistry,
    recipe_registry: &RecipeRegistry,
    discrepancy_memory: &mut DiscrepancyMemory,
    slot: &crate::agent_tick::portfolio::PortfolioSlot,
) -> bool {
    if !matches!(
        slot.feasibility,
        FeasibilityVerdict::RejectedBeforeSearch { .. }
    ) {
        return false;
    }

    let Some(profile) = world.get_component_agenda_profile(agent) else {
        return false;
    };
    let view = runtime_belief_view(agent, world, scheduler, action_defs, recipe_registry);
    match classify_rejection(
        agent,
        &slot.feasibility,
        &slot.ranked.offer,
        &view,
        current_tick,
        profile.revive_cooldown_ticks,
    ) {
        RejectionLifecycle::Satisfied => {
            clear_committed_plan_state(runtime, jc, facility_intents);
            if let Some(mut goal) = agenda_state.committed.take() {
                goal.phase = AgendaPhase::Suspended;
                goal.revival_trigger = None;
                goal.kill_condition = KillCondition::External;
                agenda_state.suspended.insert(goal.key, goal);
            }
            true
        }
        RejectionLifecycle::InfeasibleUntil { trigger } => {
            clear_committed_plan_state(runtime, jc, facility_intents);
            if let Some(mut goal) = agenda_state.committed.take() {
                goal.phase = AgendaPhase::Pending;
                goal.revival_trigger = Some(trigger);
                goal.kill_condition = KillCondition::External;
                agenda_state.pending.insert(goal.key, goal);
            }
            true
        }
        RejectionLifecycle::Dead => {
            clear_committed_plan_state(runtime, jc, facility_intents);
            if let FeasibilityVerdict::RejectedBeforeSearch { reason } = slot.feasibility {
                discrepancy_memory.record(worldwake_core::DiscrepancyEntry {
                    scope: worldwake_core::BlockerKey {
                        goal_key: slot.ranked.offer.key,
                        place: match slot.ranked.offer.anchor {
                            OpportunityAnchor::Place(place) => Some(place),
                            OpportunityAnchor::Entity(_) | OpportunityAnchor::None => {
                                slot.ranked.offer.key.place
                            }
                        },
                        target: match slot.ranked.offer.anchor {
                            OpportunityAnchor::Entity(target) => Some(target),
                            OpportunityAnchor::Place(_) | OpportunityAnchor::None => {
                                slot.ranked.offer.key.entity
                            }
                        },
                        action_def: None,
                    }
                    .into(),
                    discrepancy: reason,
                    observed_tick: current_tick,
                    expires_tick: Tick(current_tick.0.saturating_add(1)),
                    clearing_condition: worldwake_core::DiscrepancyClearing::TtlExpiry,
                    source_event: None,
                });
            }
            agenda_state.committed = None;
            true
        }
    }
}

fn opportunity_anchor_entity(anchor: OpportunityAnchor) -> Option<EntityId> {
    match anchor {
        OpportunityAnchor::Place(entity) | OpportunityAnchor::Entity(entity) => Some(entity),
        OpportunityAnchor::None => None,
    }
}

fn plans_share_repair_intent(
    failed_plan: &PlannedPlan,
    selected_plan: &PlannedPlan,
    recipe_registry: &RecipeRegistry,
) -> bool {
    failed_plan.goal == selected_plan.goal
        || failed_plan
            .goal
            .kind
            .target_commodity(recipe_registry)
            .zip(selected_plan.goal.kind.target_commodity(recipe_registry))
            .is_some_and(|(failed, selected)| failed == selected)
}

fn repair_trade_counterparty(plan: &PlannedPlan) -> Option<EntityId> {
    plan.steps
        .iter()
        .find(|step| step.op_kind == crate::PlannerOpKind::Trade)
        .and_then(|step| {
            step.payload_override
                .as_ref()
                .and_then(ActionPayload::as_trade)
                .map(|payload| payload.counterparty)
                .or_else(|| step.targets.first().copied().and_then(authoritative_target))
        })
}

fn pending_trigger_from_failed_plan(plan: &PlannedPlan) -> Option<RevivalTrigger> {
    let counterparty = repair_trade_counterparty(plan)?;
    let place = match plan.opportunity.anchor {
        OpportunityAnchor::Place(place) | OpportunityAnchor::Entity(place) => Some(place),
        OpportunityAnchor::None => plan
            .goal
            .place
            .or_else(|| plan.steps.iter().find_map(|step| step.target_place)),
    }?;
    Some(RevivalTrigger::CounterpartyAvailable {
        counterparty,
        place,
    })
}

fn park_committed_goal_from_failed_repair(
    runtime: &mut AgentDecisionRuntime,
    agenda_state: &mut AgendaState,
) {
    let Some(trigger) = runtime
        .pending_repair_context
        .as_ref()
        .and_then(|pending| pending_trigger_from_failed_plan(&pending.failed_plan))
    else {
        return;
    };
    let Some(mut goal) = agenda_state.committed.take() else {
        return;
    };
    goal.phase = AgendaPhase::Pending;
    goal.revival_trigger = Some(trigger);
    goal.kill_condition = KillCondition::External;
    agenda_state.pending.insert(goal.key, goal);
}

fn repair_route_signature(plan: &PlannedPlan) -> Vec<EntityId> {
    plan.steps
        .iter()
        .filter(|step| step.op_kind == crate::PlannerOpKind::Travel)
        .filter_map(|step| step.targets.first().copied().and_then(authoritative_target))
        .collect()
}

fn refresh_resume_payload_from_live_affordance(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    step: &mut PlannedStep,
    bindings: &crate::MaterializationBindings,
    action_defs: &worldwake_sim::ActionDefRegistry,
    action_handlers: &ActionHandlerRegistry,
) {
    let Some(def) = action_defs.get(step.def_id) else {
        return;
    };
    let Some(handler) = action_handlers.get(def.handler) else {
        return;
    };
    if !matches!(def.payload, ActionPayload::None) || step.payload_override.is_none() {
        return;
    }
    let Some(targets) =
        crate::resolve_planning_targets_with(&step.targets, |id| bindings.resolve(id))
    else {
        return;
    };
    if (handler.affordance_payloads)(def, actor, &targets, view).is_empty() {
        return;
    }

    let single_def = BTreeSet::from([step.def_id]);
    let mut matching_affordances =
        get_affordances_for_defs(view, actor, action_defs, action_handlers, &single_def)
            .into_iter()
            .filter(|affordance| affordance.bound_targets == targets);
    let Some(rebound_payload) = matching_affordances
        .next()
        .and_then(|affordance| affordance.payload_override)
    else {
        return;
    };
    if matching_affordances.next().is_some() {
        return;
    }
    step.payload_override = Some(rebound_payload);
}

fn refresh_resume_plan_payloads_from_live_affordances(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    plan: &mut PlannedPlan,
    bindings: &crate::MaterializationBindings,
    action_defs: &worldwake_sim::ActionDefRegistry,
    action_handlers: &ActionHandlerRegistry,
) {
    for step in &mut plan.steps {
        refresh_resume_payload_from_live_affordance(
            view,
            actor,
            step,
            bindings,
            action_defs,
            action_handlers,
        );
    }
}

fn classify_accepted_repair(
    runtime: &AgentDecisionRuntime,
    selected_plan: &PlannedPlan,
    recipe_registry: &RecipeRegistry,
) -> Option<AcceptedRepairProvenance> {
    let pending = runtime.pending_repair_context.as_ref()?;
    let failed_plan = &pending.failed_plan;
    if !plans_share_repair_intent(failed_plan, selected_plan, recipe_registry) {
        return None;
    }

    match (&failed_plan.goal.kind, &selected_plan.goal.kind) {
        (
            GoalKind::ProduceCommodity {
                recipe_id: failed_recipe,
            },
            GoalKind::ProduceCommodity {
                recipe_id: selected_recipe,
            },
        ) if failed_recipe != selected_recipe
            && failed_plan
                .goal
                .kind
                .target_commodity(recipe_registry)
                .zip(selected_plan.goal.kind.target_commodity(recipe_registry))
                .is_some() =>
        {
            return Some(AcceptedRepairProvenance {
                goal_key: selected_plan.goal,
                repair_kind: RepairKind::RebindTarget,
                substitute_target: None,
                substitute_recipe: Some(*selected_recipe),
                records_repair_memory: false,
            });
        }
        _ => {}
    }

    if let (Some(failed_counterparty), Some(selected_counterparty)) = (
        repair_trade_counterparty(failed_plan),
        repair_trade_counterparty(selected_plan),
    ) && failed_counterparty != selected_counterparty
    {
        return Some(AcceptedRepairProvenance {
            goal_key: selected_plan.goal,
            repair_kind: RepairKind::RebindTarget,
            substitute_target: Some(selected_counterparty),
            substitute_recipe: None,
            records_repair_memory: false,
        });
    }

    if failed_plan.opportunity.anchor != selected_plan.opportunity.anchor
        && let Some(substitute_target) = opportunity_anchor_entity(selected_plan.opportunity.anchor)
    {
        return Some(AcceptedRepairProvenance {
            goal_key: selected_plan.goal,
            repair_kind: RepairKind::RebindTarget,
            substitute_target: Some(substitute_target),
            substitute_recipe: None,
            records_repair_memory: true,
        });
    }

    let failed_route = repair_route_signature(failed_plan);
    let selected_route = repair_route_signature(selected_plan);
    if !failed_route.is_empty()
        && !selected_route.is_empty()
        && failed_route != selected_route
        && failed_plan.opportunity.anchor == selected_plan.opportunity.anchor
        && repair_trade_counterparty(failed_plan) == repair_trade_counterparty(selected_plan)
    {
        return Some(AcceptedRepairProvenance {
            goal_key: selected_plan.goal,
            repair_kind: RepairKind::ReplaceProvider,
            substitute_target: None,
            substitute_recipe: None,
            records_repair_memory: false,
        });
    }

    None
}

fn resume_pending_repair_plan(
    runtime: &mut AgentDecisionRuntime,
    agenda_state: &AgendaState,
    ranked_candidates: &OrderedRanked<'_>,
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    action_defs: &worldwake_sim::ActionDefRegistry,
    action_handlers: &ActionHandlerRegistry,
) -> bool {
    let Some(committed) = agenda_state.committed.as_ref() else {
        return false;
    };
    let Some(pending) = runtime.pending_repair_context.as_ref() else {
        return false;
    };
    let Some(trigger) = pending_trigger_from_failed_plan(&pending.failed_plan) else {
        return false;
    };
    if pending.failed_plan.goal != committed.key.goal_key
        || committed.phase != AgendaPhase::Committed
        || committed.revival_trigger.as_ref() != Some(&trigger)
    {
        return false;
    }

    let mut resumed_plan = pending.failed_plan.clone();
    refresh_resume_plan_payloads_from_live_affordances(
        view,
        agent,
        &mut resumed_plan,
        &runtime.materialization_bindings,
        action_defs,
        action_handlers,
    );
    runtime.current_plan = Some(resumed_plan);
    runtime.current_step_index = usize::from(pending.failed_step_index);
    runtime.step_in_flight = false;
    runtime.accepted_repair = None;
    runtime.last_priority_class = ranked_candidates
        .iter()
        .find(|candidate| candidate.key == committed.key)
        .map(|candidate| candidate.priority_class)
        .or(Some(committed.priority_class));
    true
}

#[allow(clippy::too_many_arguments)]
fn adopt_selected_plan(
    world: &mut worldwake_core::World,
    event_log: &mut EventLog,
    runtime: &mut AgentDecisionRuntime,
    agenda_state: &mut AgendaState,
    jc: &mut Option<IntentionFrame>,
    facility_intents: &mut worldwake_core::ContentionIntents,
    agent: EntityId,
    ranked_candidates: &OrderedRanked<'_>,
    mut selected_plan: PlannedPlan,
    recipe_registry: &RecipeRegistry,
    tick: Tick,
    cognitive: &CognitiveProfile,
    prepared_frame: Option<IntentionFrame>,
    current_place: EntityId,
) {
    runtime.materialization_bindings.clear();
    facility_intents.intents.clear();
    runtime.accepted_repair = classify_accepted_repair(runtime, &selected_plan, recipe_registry);
    runtime.pending_repair_context = None;
    agenda_state.committed = ranked_candidates
        .iter()
        .find(|candidate| candidate.key == selected_plan.opportunity)
        .map(|candidate| AgendaEntry::committed_from(candidate, tick));
    if (selected_plan.committed_source.is_none() || selected_plan.expectation_kind.is_none())
        && let Some(candidate) = ranked_candidates
            .iter()
            .find(|candidate| candidate.key == selected_plan.opportunity)
    {
        if selected_plan.committed_source.is_none() {
            selected_plan.committed_source = committed_source_for_offer(&candidate.offer);
        }
        if selected_plan.expectation_kind.is_none() {
            selected_plan.expectation_kind = expectation_kind_for_offer(&candidate.offer);
        }
    }
    *jc = prepared_frame;
    let _ = persist_expectation_store_update(world, event_log, agent, tick, |store| {
        let expired = expire_plan_step_expectations(store);
        let wrote = write_plan_step_expectations(
            store,
            agent,
            current_place,
            &selected_plan,
            tick,
            cognitive.expectation_tolerance_ticks,
        );
        expired || wrote
    });
    runtime.current_plan = Some(selected_plan);
    runtime.current_step_index = 0;
    runtime.step_in_flight = false;
    runtime.last_priority_class = ranked_candidates
        .iter()
        .find(|candidate| {
            Some(candidate.offer.key) == agenda_state.committed.as_ref().map(|ag| ag.key.goal_key)
        })
        .map(|candidate| candidate.priority_class);
}

#[allow(clippy::too_many_arguments)]
fn clear_current_plan(
    world: &mut worldwake_core::World,
    event_log: &mut EventLog,
    runtime: &mut AgentDecisionRuntime,
    agenda_state: &mut AgendaState,
    jc: &mut Option<IntentionFrame>,
    facility_intents: &mut worldwake_core::ContentionIntents,
    ranked_candidates: &OrderedRanked<'_>,
    agent: EntityId,
    tick: Tick,
) {
    if jc.is_some() {
        runtime.last_frame_clear_reason = Some(worldwake_core::FrameClearReason::LostPlan);
    }
    *jc = None;
    runtime.materialization_bindings.clear();
    facility_intents.intents.clear();
    let _ = persist_expectation_store_update(world, event_log, agent, tick, |store| {
        expire_plan_step_expectations(store)
    });
    if runtime.pending_repair_context.is_none()
        && let Some(failed_plan) = runtime.current_plan.clone()
    {
        runtime.pending_repair_context = Some(PendingRepairContext {
            failed_plan,
            failed_step_index: runtime
                .current_step_index
                .try_into()
                .expect("failed repair step index exceeds u16"),
        });
    }
    runtime.current_plan = None;
    runtime.current_step_index = 0;
    runtime.step_in_flight = false;
    runtime.accepted_repair = None;
    park_committed_goal_from_failed_repair(runtime, agenda_state);
    runtime.last_priority_class = ranked_candidates
        .first()
        .map(|candidate| candidate.priority_class);
}

#[cfg(test)]
#[allow(clippy::too_many_arguments, clippy::trivially_copy_pass_by_ref)]
pub(super) fn plan_and_validate_next_step(
    world: &mut worldwake_core::World,
    event_log: &mut EventLog,
    scheduler: &Scheduler,
    runtime: &mut AgentDecisionRuntime,
    agenda_state: &mut AgendaState,
    jc: &mut Option<IntentionFrame>,
    facility_intents: &mut worldwake_core::ContentionIntents,
    agent: worldwake_core::EntityId,
    ranked_candidates: &OrderedRanked<'_>,
    discrepancy_memory: &mut DiscrepancyMemory,
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
    let opportunity_index = PerceivedOpportunityIndex::default();
    plan_and_validate_next_step_with_opportunity_index(
        world,
        event_log,
        scheduler,
        runtime,
        agenda_state,
        jc,
        facility_intents,
        agent,
        ranked_candidates,
        discrepancy_memory,
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
        &opportunity_index,
    )
}

#[allow(clippy::too_many_arguments, clippy::trivially_copy_pass_by_ref)]
fn plan_and_validate_next_step_with_opportunity_index(
    world: &mut worldwake_core::World,
    event_log: &mut EventLog,
    scheduler: &Scheduler,
    runtime: &mut AgentDecisionRuntime,
    agenda_state: &mut AgendaState,
    jc: &mut Option<IntentionFrame>,
    facility_intents: &mut worldwake_core::ContentionIntents,
    agent: worldwake_core::EntityId,
    ranked_candidates: &OrderedRanked<'_>,
    discrepancy_memory: &mut DiscrepancyMemory,
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
    opportunity_index: &PerceivedOpportunityIndex,
) -> (Option<PlannedStep>, Option<bool>) {
    let planning_start = Instant::now();
    let result = (|| {
        let active_goal_key = agenda_state.committed.as_ref().map(|ag| ag.key.goal_key);
        let committed_opportunity = runtime
            .current_plan
            .as_ref()
            .map(|plan| plan.opportunity)
            .filter(|opportunity| Some(opportunity.goal_key) == active_goal_key);
        let should_plan = !runtime.dirty.is_empty() || has_pending_budget_retry(runtime, tick);
        if should_plan {
            // This read view is scoped to planning so world mutation can happen
            // afterwards at the plan-adoption / plan-clear seam.
            let view = runtime_belief_view(agent, world, scheduler, action_defs, recipe_registry);
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
            runtime.operating_mode = derive_operating_mode(&view, agent, ranked_candidates);

            let plans = build_candidate_plans_with_opportunity_index(
                world,
                scheduler,
                agent,
                ranked_candidates,
                committed_opportunity,
                discrepancy_memory,
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
                opportunity_index,
                Some(&runtime.route_preference),
                runtime.operating_mode,
            );

            // Record newly exhausted goals for next tick.
            let _ = record_exhausted_goals(
                runtime,
                &view,
                agent,
                recipe_registry,
                &plans.plans,
                tick,
                cognitive,
            );
            let _ = write_budget_exhausted_partial_plan_segments(
                agenda_state,
                ranked_candidates,
                &plans.plans,
                tick,
                cognitive,
            );
            for plan in &plans.plans {
                if plan.result.is_found() {
                    runtime.exhaustion_cache.remove(&plan.opportunity);
                }
            }
            let committed_rejection_parked = committed_opportunity
                .and_then(|opportunity| {
                    plans.portfolio.slots.values().find(|slot| {
                        slot.ranked.offer.key == opportunity.goal_key
                            && slot.ranked.offer.anchor == opportunity.anchor
                    })
                })
                .is_some_and(|slot| {
                    apply_committed_rejection_lifecycle(
                        world,
                        scheduler,
                        runtime,
                        agenda_state,
                        jc,
                        facility_intents,
                        agent,
                        tick,
                        action_defs,
                        recipe_registry,
                        discrepancy_memory,
                        slot,
                    )
                });

            let selection_plans = selection_candidates(&plans.plans);

            if !committed_rejection_parked {
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
                    let failed_sources = {
                        let current_place = SpatialBeliefView::effective_place(&view, agent);
                        same_goal_search_failure_incidents(
                            runtime.current_plan.as_ref(),
                            &selected_plan,
                            &selection_plans,
                            current_place,
                            tick,
                        )
                    };
                    if !failed_sources.is_empty() {
                        let committed_source_before_invalidation = runtime
                            .current_plan
                            .as_ref()
                            .and_then(|plan| plan.committed_source);
                        let applied_failures =
                            super::apply_source_reliability_failure_observations(
                                world,
                                event_log,
                                agent,
                                tick,
                                &failed_sources,
                            )
                            .expect("planning-stage source reliability persistence should succeed");
                        let invalidated =
                            super::invalidate_committed_source_after_reliability_failure(
                                runtime,
                                jc.as_ref(),
                                facility_intents,
                                discrepancy_memory,
                                &applied_failures,
                                tick,
                                cognitive.structural_block_ticks,
                            );
                        super::emit_source_expectation_failure_events(
                            event_log,
                            agent,
                            &failed_sources,
                            &applied_failures,
                            if invalidated {
                                committed_source_before_invalidation
                            } else {
                                None
                            },
                            cognitive.decision_history_alternatives,
                        );
                    }
                    let refreshed_view =
                        runtime_belief_view(agent, world, scheduler, action_defs, recipe_registry);
                    let mut prepared_frame =
                        update_frame_for_adopted_plan(jc.as_ref(), &selected_plan, tick, runtime);
                    if let Some(frame) = prepared_frame.as_mut() {
                        let completion_tick =
                            plan_completion_tick_for_adoption(&selected_plan, tick);
                        frame.assumptions = populate_assumptions(
                            frame,
                            agent,
                            &refreshed_view,
                            tick,
                            completion_tick,
                        );
                    }
                    emit_plan_selection_events(
                        event_log,
                        tick,
                        agent,
                        runtime,
                        &refreshed_view,
                        ranked_candidates,
                        &plans.portfolio,
                        active_goal_key,
                        &selected_plan,
                        cognitive.decision_history_alternatives,
                        prepared_frame.as_ref(),
                    );
                    if write_information_barrier_partial_plan_segment(
                        agenda_state,
                        ranked_candidates,
                        &selected_plan,
                        &plans.plans,
                        tick,
                        cognitive,
                    ) {
                        runtime.current_plan = None;
                        runtime.current_step_index = 0;
                        runtime.step_in_flight = false;
                        runtime.materialization_bindings.clear();
                        facility_intents.intents.clear();
                        *jc = None;
                        runtime.accepted_repair = None;
                        runtime.pending_repair_context = None;
                        runtime.last_priority_class = ranked_candidates
                            .iter()
                            .find(|candidate| candidate.key == selected_plan.opportunity)
                            .map(|candidate| candidate.priority_class);
                        return (None, None);
                    }
                    let current_place = SpatialBeliefView::effective_place(&refreshed_view, agent)
                        .expect("plan adoption expects actor to have an effective place");
                    adopt_selected_plan(
                        world,
                        event_log,
                        runtime,
                        agenda_state,
                        jc,
                        facility_intents,
                        agent,
                        ranked_candidates,
                        selected_plan,
                        recipe_registry,
                        tick,
                        cognitive,
                        prepared_frame,
                        current_place,
                    );
                } else if !resume_pending_repair_plan(
                    runtime,
                    agenda_state,
                    ranked_candidates,
                    &view,
                    agent,
                    action_defs,
                    action_handlers,
                ) {
                    clear_current_plan(
                        world,
                        event_log,
                        runtime,
                        agenda_state,
                        jc,
                        facility_intents,
                        ranked_candidates,
                        agent,
                        tick,
                    );
                }
            }
            runtime.dirty = DirtySet::default();
        }

        let view = runtime_belief_view(agent, world, scheduler, action_defs, recipe_registry);
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
/// Returns `(next_step, valid, plan_continued, trace surfaces, pending_tracker_increments)`.
#[cfg(test)]
#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref
)]
pub(super) fn plan_and_validate_next_step_traced(
    world: &mut worldwake_core::World,
    event_log: &mut EventLog,
    scheduler: &Scheduler,
    runtime: &mut AgentDecisionRuntime,
    agenda_state: &mut AgendaState,
    jc: &mut Option<IntentionFrame>,
    facility_intents: &mut worldwake_core::ContentionIntents,
    agent: worldwake_core::EntityId,
    ranked_candidates: &OrderedRanked<'_>,
    discrepancy_memory: &mut DiscrepancyMemory,
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
    candidate_sources: &BTreeMap<OpportunityKey, CandidateSource>,
) -> PlanningStepTraceResult {
    let opportunity_index = PerceivedOpportunityIndex::default();
    plan_and_validate_next_step_traced_with_opportunity_index(
        world,
        event_log,
        scheduler,
        runtime,
        agenda_state,
        jc,
        facility_intents,
        agent,
        ranked_candidates,
        discrepancy_memory,
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
        tracing,
        previous_goal,
        recipe_registry,
        candidate_sources,
        &opportunity_index,
    )
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref
)]
pub(super) fn plan_and_validate_next_step_traced_with_opportunity_index(
    world: &mut worldwake_core::World,
    event_log: &mut EventLog,
    scheduler: &Scheduler,
    runtime: &mut AgentDecisionRuntime,
    agenda_state: &mut AgendaState,
    jc: &mut Option<IntentionFrame>,
    facility_intents: &mut worldwake_core::ContentionIntents,
    agent: worldwake_core::EntityId,
    ranked_candidates: &OrderedRanked<'_>,
    discrepancy_memory: &mut DiscrepancyMemory,
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
    candidate_sources: &BTreeMap<OpportunityKey, CandidateSource>,
    opportunity_index: &PerceivedOpportunityIndex,
) -> PlanningStepTraceResult {
    if !tracing {
        let (step, valid) = plan_and_validate_next_step_with_opportunity_index(
            world,
            event_log,
            scheduler,
            runtime,
            agenda_state,
            jc,
            facility_intents,
            agent,
            ranked_candidates,
            discrepancy_memory,
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
            opportunity_index,
        );
        return (
            step,
            valid,
            false,
            None,
            None,
            None,
            None,
            None,
            None,
            BTreeSet::new(),
        );
    }

    // Traced path: inline the logic to capture intermediate results.
    let mut plan_search_trace = PlanSearchTrace {
        attempts: Vec::new(),
        same_goal_trace: None,
    };
    let mut portfolio_trace = None;
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
    let mut snapshot_admissions = None;
    let mut snapshot_cache_counters = None;
    let mut planning_state_cache_counters = None;

    let should_plan = !runtime.dirty.is_empty() || has_pending_budget_retry(runtime, tick);
    if should_plan {
        // This read view is scoped to planning so world mutation can happen
        // afterwards at the plan-adoption / plan-clear seam.
        let view = runtime_belief_view(agent, world, scheduler, action_defs, recipe_registry);
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
                        None,
                        None,
                        None,
                        None,
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

        let committed_opportunity = runtime
            .current_plan
            .as_ref()
            .map(|plan| plan.opportunity)
            .filter(|opportunity| {
                Some(opportunity.goal_key)
                    == agenda_state
                        .committed
                        .as_ref()
                        .map(|active_goal| active_goal.key.goal_key)
            });
        runtime.operating_mode = derive_operating_mode(&view, agent, ranked_candidates);

        let plans = build_candidate_plans_with_sources(
            world,
            scheduler,
            agent,
            ranked_candidates,
            committed_opportunity,
            discrepancy_memory,
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
            candidate_sources,
            opportunity_index,
            Some(&runtime.route_preference),
            runtime.operating_mode,
        );
        portfolio_trace = Some(plans.portfolio_trace());
        snapshot_admissions.clone_from(&plans.snapshot_admissions);
        snapshot_cache_counters = plans.snapshot_cache_counters;
        planning_state_cache_counters = plans.planning_state_cache_counters;

        pending_tracker_increments = record_exhausted_goals(
            runtime,
            &view,
            agent,
            recipe_registry,
            &plans.plans,
            tick,
            cognitive,
        );
        let _ = write_budget_exhausted_partial_plan_segments(
            agenda_state,
            ranked_candidates,
            &plans.plans,
            tick,
            cognitive,
        );
        for plan in &plans.plans {
            if plan.result.is_found() {
                runtime.exhaustion_cache.remove(&plan.opportunity);
            }
        }

        let known_entities: BTreeSet<EntityId> = view
            .known_entity_beliefs(agent)
            .into_iter()
            .map(|(entity, _)| entity)
            .collect();
        for plan in &plans.plans {
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
            plans.search_opportunities(),
            ProfileBeliefView::portfolio_weights_profile(&view, agent)
                .max_plans_for_mode(runtime.operating_mode),
            &plans.plans,
        );

        let committed_rejection_parked = committed_opportunity
            .and_then(|opportunity| {
                plans.portfolio.slots.values().find(|slot| {
                    slot.ranked.offer.key == opportunity.goal_key
                        && slot.ranked.offer.anchor == opportunity.anchor
                })
            })
            .is_some_and(|slot| {
                apply_committed_rejection_lifecycle(
                    world,
                    scheduler,
                    runtime,
                    agenda_state,
                    jc,
                    facility_intents,
                    agent,
                    tick,
                    action_defs,
                    recipe_registry,
                    discrepancy_memory,
                    slot,
                )
            });

        let selection_plans = selection_candidates(&plans.plans);
        let current_goal_before_selection =
            agenda_state.committed.as_ref().map(|ag| ag.key.goal_key);

        if !committed_rejection_parked {
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
                let failed_sources = {
                    let current_place = SpatialBeliefView::effective_place(&view, agent);
                    same_goal_search_failure_incidents(
                        runtime.current_plan.as_ref(),
                        &selected_plan,
                        &selection_plans,
                        current_place,
                        tick,
                    )
                };
                if !failed_sources.is_empty() {
                    let committed_source_before_invalidation = runtime
                        .current_plan
                        .as_ref()
                        .and_then(|plan| plan.committed_source);
                    let applied_failures = super::apply_source_reliability_failure_observations(
                        world,
                        event_log,
                        agent,
                        tick,
                        &failed_sources,
                    )
                    .expect("planning-stage source reliability persistence should succeed");
                    let invalidated = super::invalidate_committed_source_after_reliability_failure(
                        runtime,
                        jc.as_ref(),
                        facility_intents,
                        discrepancy_memory,
                        &applied_failures,
                        tick,
                        cognitive.structural_block_ticks,
                    );
                    super::emit_source_expectation_failure_events(
                        event_log,
                        agent,
                        &failed_sources,
                        &applied_failures,
                        if invalidated {
                            committed_source_before_invalidation
                        } else {
                            None
                        },
                        cognitive.decision_history_alternatives,
                    );
                }
                let refreshed_view =
                    runtime_belief_view(agent, world, scheduler, action_defs, recipe_registry);
                let mut prepared_frame =
                    update_frame_for_adopted_plan(jc.as_ref(), &selected_plan, tick, runtime);
                if let Some(frame) = prepared_frame.as_mut() {
                    let completion_tick = plan_completion_tick_for_adoption(&selected_plan, tick);
                    frame.assumptions =
                        populate_assumptions(frame, agent, &refreshed_view, tick, completion_tick);
                }
                emit_plan_selection_events(
                    event_log,
                    tick,
                    agent,
                    runtime,
                    &refreshed_view,
                    ranked_candidates,
                    &plans.portfolio,
                    current_goal_before_selection,
                    &selected_plan,
                    cognitive.decision_history_alternatives,
                    prepared_frame.as_ref(),
                );
                let selected_goal = selected_plan.goal;
                let selected_opportunity = selected_plan.opportunity;
                let selected_plan_source = determine_selected_plan_source(
                    selected_opportunity,
                    current_goal_before_selection,
                    &selection_plans,
                );
                let search_provenance =
                    matches!(selected_plan_source, SelectedPlanSource::SearchSelection)
                        .then(|| summarize_search_provenance(&plans.plans, selected_opportunity))
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
                    let prev_rank = ranked_candidates.iter().find(|c| c.offer.key == prev);
                    let new_rank = ranked_candidates
                        .iter()
                        .find(|c| c.offer.key == selected_goal);
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

                if write_information_barrier_partial_plan_segment(
                    agenda_state,
                    ranked_candidates,
                    &selected_plan,
                    &plans.plans,
                    tick,
                    cognitive,
                ) {
                    runtime.current_plan = None;
                    runtime.current_step_index = 0;
                    runtime.step_in_flight = false;
                    runtime.materialization_bindings.clear();
                    facility_intents.intents.clear();
                    *jc = None;
                    runtime.accepted_repair = None;
                    runtime.pending_repair_context = None;
                    runtime.last_priority_class = ranked_candidates
                        .iter()
                        .find(|candidate| candidate.key == selected_plan.opportunity)
                        .map(|candidate| candidate.priority_class);
                    runtime.dirty = DirtySet::default();
                    let view =
                        runtime_belief_view(agent, world, scheduler, action_defs, recipe_registry);
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
                    return (
                        next_step,
                        next_step_valid,
                        plan_continued,
                        Some(plan_search_trace),
                        Some(selection_trace),
                        portfolio_trace,
                        snapshot_admissions,
                        snapshot_cache_counters,
                        planning_state_cache_counters,
                        pending_tracker_increments,
                    );
                }

                let current_place = SpatialBeliefView::effective_place(&refreshed_view, agent)
                    .expect("plan adoption expects actor to have an effective place");
                adopt_selected_plan(
                    world,
                    event_log,
                    runtime,
                    agenda_state,
                    jc,
                    facility_intents,
                    agent,
                    ranked_candidates,
                    selected_plan,
                    recipe_registry,
                    tick,
                    cognitive,
                    prepared_frame,
                    current_place,
                );
            } else if !resume_pending_repair_plan(
                runtime,
                agenda_state,
                ranked_candidates,
                &view,
                agent,
                action_defs,
                action_handlers,
            ) {
                clear_current_plan(
                    world,
                    event_log,
                    runtime,
                    agenda_state,
                    jc,
                    facility_intents,
                    ranked_candidates,
                    agent,
                    tick,
                );
            }
        }
        runtime.dirty = DirtySet::default();
    }

    let view = runtime_belief_view(agent, world, scheduler, action_defs, recipe_registry);
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
        portfolio_trace,
        snapshot_admissions,
        snapshot_cache_counters,
        planning_state_cache_counters,
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
        goal_budget: trace_metadata.goal_budget,
        strategic_budget: trace_metadata.strategic_budget.clone(),
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
        method_trace: trace_metadata.method_trace.clone(),
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
        CandidatePlanSearch, found_plan_blocks_later_goals, frontier_exhaustion_entry,
        has_pending_budget_retry, plan_completion_tick_for_adoption, plan_search_result_to_trace,
        planning_time_target_belief_presence, record_exhausted_goals, selected_plan_value,
        summarize_ranked_goal, summarize_selected_plan, summarize_snapshot_continuation,
        write_budget_exhausted_partial_plan_segments,
        write_information_barrier_partial_plan_segment,
    };
    use crate::{
        AgendaEntry, AgendaPhase, AgendaState, AgentDecisionRuntime, DirtySet, ExhaustionEntry,
        ExhaustionInvalidationCondition, ExhaustionRetryState, ExpectationFailureCause,
        ExpectationFailurePhase, GoalDispatchKey, GoalKey, GoalKind, GoalOffer, GoalPriorityClass,
        KillCondition, OpportunityAnchor, OpportunityExpectationKind, OpportunityKey,
        PlanSearchResult, PlanTerminalKind, PlannedPlan, PlannedStep, PlannerOpKind,
        PlanningEntityRef, ProfileFixture, RevivalTrigger, SourceCompositeRank,
        agent_tick::portfolio::{FeasibilityVerdict, Portfolio, PortfolioSlot, SlotKind},
        build_semantics_table,
        decision_trace::{
            CompetitionDiscount, SnapshotContinuationOutcome, SourceReliabilityDiscount,
            TargetBeliefPresence,
        },
        feasibility::FeasibilityHint,
        goal_schema::{FrontierExhaustionStrategy, GoalDispatchKeySchemaExt},
        plan_selection::SelectionCandidatePlan,
        search::{PartialPlanSkeletonSource, SearchTraceMetadata},
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        AcquisitionQuantity, ActionDefId, ActionDomain, BodyCostPerTick, CauseRef,
        CognitiveProfile, CommodityKind, CommodityPurpose, ContentionIntents, ControlSource,
        DecisionEventPayload, EntityId, EventId, EventLog, EventTag, EventView, ExecutionBudget,
        FrameAssumption, GoalCommittedPayload, GoalRejectionReason, HomeostaticNeedId,
        HomeostaticNeeds, MerchandiseProfile, MotiveSource, MotiveSourceRef, PerceptionSource,
        Permille, Place, PlanAdoptedPayload, Quantity, RankedGoalComparisonDimensionTag,
        RepairKind, RoutePreferenceProfile, RoutePreferenceSummary, RouteSegment, SourceKey,
        TellTopic, TestimonyTrustProfile, TestimonyTrustSummary, Tick, TopicScope, Topology,
        TravelEdge, TravelEdgeId, VisibilitySpec, WitnessData, WorkstationTag, World, WorldTxn,
        build_believed_entity_state, build_prototype_world,
    };
    use worldwake_sim::{
        ActionDef, ActionDefRegistry, ActionError, ActionExecutionContext, ActionHandler,
        ActionHandlerId, ActionHandlerRegistry, ActionPayload, ActionProgress, ActionState,
        BindingStrictness, CommitOutcome, DeterministicRng, DurationExpr, Interruptibility,
        PerAgentBeliefView, Precondition, ProfileBeliefView, RecipeDefinition, RecipeRegistry,
        RuntimeBeliefView, Scheduler, SpatialBeliefView, SystemManifest, TargetSpec,
    };
    use worldwake_systems::build_full_action_registries;

    fn consume_goal(commodity: CommodityKind) -> GoalKey {
        GoalKey::from(GoalKind::ConsumeOwnedCommodity { commodity })
    }

    fn ordered(ranked: &[AgendaEntry]) -> crate::ranking::OrderedRanked<'_> {
        crate::ranking::OrderedRanked::from_sorted_for_test(ranked)
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

    fn information_barrier_plan(opportunity: OpportunityKey, subject: EntityId) -> PlannedPlan {
        PlannedPlan::new(
            opportunity,
            opportunity.goal_key,
            Vec::new(),
            PlanTerminalKind::InformationBarrier {
                topic: TellTopic::EntityBelief { subject },
            },
        )
    }

    #[derive(Default)]
    struct SelectionContextView {
        effective_places: BTreeMap<EntityId, EntityId>,
        testimony_profile: Option<TestimonyTrustProfile>,
        route_profile: Option<RoutePreferenceProfile>,
    }

    impl ProfileBeliefView for SelectionContextView {
        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            None
        }

        fn drive_thresholds(&self, _agent: EntityId) -> Option<worldwake_core::DriveThresholds> {
            None
        }

        fn metabolism_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::MetabolismProfile> {
            None
        }

        fn testimony_trust_profile(&self, _agent: EntityId) -> Option<TestimonyTrustProfile> {
            self.testimony_profile.clone()
        }

        fn route_preference_profile(&self, _agent: EntityId) -> Option<RoutePreferenceProfile> {
            self.route_profile.clone()
        }
    }

    impl SpatialBeliefView for SelectionContextView {
        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.effective_places.get(&entity).copied()
        }

        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }

        fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn adjacent_places(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn route_exists(&self, _from: EntityId, _to: EntityId) -> bool {
            false
        }

        fn in_transit_state(&self, _entity: EntityId) -> Option<worldwake_core::InTransitOnEdge> {
            None
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            _place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            Vec::new()
        }
    }

    fn cognitive(reasoning: &ProfileFixture) -> CognitiveProfile {
        CognitiveProfile {
            max_candidates_per_expansion: CognitiveProfile::default().max_candidates_per_expansion,
            max_plan_depth: reasoning.max_plan_depth,
            max_travel_candidates_per_expansion: CognitiveProfile::default()
                .max_travel_candidates_per_expansion,
            snapshot_travel_horizon: reasoning.snapshot_travel_horizon,
            max_node_expansions: reasoning.max_node_expansions,
            switch_margin: reasoning.switch_margin,
            planning_switch_margin: CognitiveProfile::default().planning_switch_margin,
            transient_block_ticks: reasoning.transient_block_ticks,
            structural_block_ticks: reasoning.structural_block_ticks,
            stale_belief_backoff_ticks: CognitiveProfile::default().stale_belief_backoff_ticks,
            contradicted_belief_backoff_ticks: CognitiveProfile::default()
                .contradicted_belief_backoff_ticks,
            improper_state_backoff_ticks: CognitiveProfile::default().improper_state_backoff_ticks,
            missing_observation_backoff_ticks: CognitiveProfile::default()
                .missing_observation_backoff_ticks,
            no_legal_binding_backoff_ticks: CognitiveProfile::default()
                .no_legal_binding_backoff_ticks,
            counterparty_refusal_backoff_ticks: CognitiveProfile::default()
                .counterparty_refusal_backoff_ticks,
            route_unknown_backoff_ticks: CognitiveProfile::default().route_unknown_backoff_ticks,
            route_segment_blocker_ticks: CognitiveProfile::default().route_segment_blocker_ticks,
            counterparty_blocker_ticks: CognitiveProfile::default().counterparty_blocker_ticks,
            search_exhaustion_backoff_ticks: CognitiveProfile::default()
                .search_exhaustion_backoff_ticks,
            partial_drift_backoff_ticks: CognitiveProfile::default().partial_drift_backoff_ticks,
            expectation_tolerance_ticks: CognitiveProfile::default().expectation_tolerance_ticks,
            guard_min_confidence_ceiling: CognitiveProfile::default().guard_min_confidence_ceiling,
            repair_memory_ticks: CognitiveProfile::default().repair_memory_ticks,
            learned_opportunity_memory_ticks: CognitiveProfile::default()
                .learned_opportunity_memory_ticks,
            survey_memory_capacity: CognitiveProfile::default().survey_memory_capacity,
            survey_memory_retention_ticks: CognitiveProfile::default()
                .survey_memory_retention_ticks,
            initial_cooldown_ticks: reasoning.initial_cooldown_ticks,
            max_cooldown_ticks: reasoning.max_cooldown_ticks,
            landmark_extraction_depth: CognitiveProfile::default().landmark_extraction_depth,
            use_ff_heuristic: CognitiveProfile::default().use_ff_heuristic,
            decision_history_alternatives: CognitiveProfile::default()
                .decision_history_alternatives,
            detour_budget_permille: CognitiveProfile::default().detour_budget_permille,
            compile_opportunity_cap: CognitiveProfile::default().compile_opportunity_cap,
            repair_budget_fraction: CognitiveProfile::default().repair_budget_fraction,
            causal_links_per_step_cap: CognitiveProfile::default().causal_links_per_step_cap,
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
    ) -> GoalOffer {
        GoalOffer {
            key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            anchor,
            evidence_entities,
            evidence_places,
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        }
    }

    fn searched_plan(opportunity: OpportunityKey, result: PlanSearchResult) -> CandidatePlanSearch {
        CandidatePlanSearch {
            opportunity,
            result,
            perceived_cost: None,
            skeleton_source: None,
            trace_metadata: SearchTraceMetadata::default(),
            binding_rejections: Vec::new(),
            expansion_summaries: Vec::new(),
        }
    }

    fn searched_plan_with_skeleton(
        opportunity: OpportunityKey,
        result: PlanSearchResult,
        source: PartialPlanSkeletonSource,
    ) -> CandidatePlanSearch {
        CandidatePlanSearch {
            skeleton_source: Some(source.clone()),
            trace_metadata: SearchTraceMetadata {
                skeleton_source: Some(source),
                ..SearchTraceMetadata::default()
            },
            ..searched_plan(opportunity, result)
        }
    }

    #[test]
    fn candidate_plan_search_retains_partial_plan_skeleton_source() {
        let opportunity = consume_opportunity(CommodityKind::Bread, OpportunityAnchor::None);
        let source = PartialPlanSkeletonSource {
            remaining_skeleton: vec![crate::PlannedSkeletonStep {
                op: PlannerOpKind::Trade,
                target_template: crate::htn::PayloadTemplate::FromContext,
                expected_pre: vec![crate::htn::BeliefPredicate::SellerKnown {
                    commodity: crate::htn::CommodityTemplate::Fixed(CommodityKind::Bread),
                }],
            }],
        };

        let plan = CandidatePlanSearch {
            opportunity,
            result: PlanSearchResult::BudgetExhausted { expansions_used: 1 },
            perceived_cost: None,
            skeleton_source: Some(source.clone()),
            trace_metadata: SearchTraceMetadata {
                skeleton_source: Some(source.clone()),
                ..SearchTraceMetadata::default()
            },
            binding_rejections: Vec::new(),
            expansion_summaries: Vec::new(),
        };

        assert_eq!(plan.skeleton_source, Some(source));
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

    #[allow(clippy::unnecessary_wraps)]
    fn noop_start(
        _def: &ActionDef,
        _instance: &mut worldwake_sim::ActionInstance,
        _context: &ActionExecutionContext<'_>,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<Option<ActionState>, ActionError> {
        Ok(None)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_tick(
        _def: &ActionDef,
        _instance: &mut worldwake_sim::ActionInstance,
        _context: &ActionExecutionContext<'_>,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        Ok(ActionProgress::Complete)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_commit(
        _def: &ActionDef,
        _instance: &worldwake_sim::ActionInstance,
        _context: &ActionExecutionContext<'_>,
        _event_log: &EventLog,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<CommitOutcome, ActionError> {
        Ok(CommitOutcome::empty())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_abort(
        _def: &ActionDef,
        _instance: &worldwake_sim::ActionInstance,
        _context: &ActionExecutionContext<'_>,
        _reason: &worldwake_sim::AbortReason,
        _event_log: &EventLog,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    fn resume_trade_payload_override_is_valid(
        def: &ActionDef,
        actor: EntityId,
        targets: &[EntityId],
        payload: &ActionPayload,
        _view: &dyn RuntimeBeliefView,
    ) -> bool {
        if def.name != "trade:resume-test" {
            return false;
        }
        let Some(payload) = payload.as_trade() else {
            return false;
        };
        let Some(counterparty) = targets.first().copied() else {
            return false;
        };
        payload.counterparty == counterparty
            && actor != counterparty
            && payload.sale_lot == entity(600)
            && payload.offered_commodity == CommodityKind::Coin
            && payload.requested_quantity == Quantity(1)
            && (Quantity(1)..=Quantity(4)).contains(&payload.offered_quantity)
    }

    fn build_resume_trade_registry() -> (ActionDefRegistry, ActionHandlerRegistry) {
        let mut registry = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(
            ActionHandler::new(noop_start, noop_tick, noop_commit, noop_abort)
                .with_affordance_payloads(|_, actor, targets, _| {
                    let Some(counterparty) = targets.first().copied() else {
                        return Vec::new();
                    };
                    if actor == counterparty {
                        return Vec::new();
                    }
                    vec![ActionPayload::Trade(worldwake_sim::TradeActionPayload {
                        counterparty,
                        sale_lot: entity(600),
                        offered_commodity: CommodityKind::Coin,
                        offered_quantity: Quantity(3),
                        requested_quantity: Quantity(1),
                    })]
                })
                .with_payload_override_validator(resume_trade_payload_override_is_valid),
        );
        registry.register(ActionDef {
            id: ActionDefId(0),
            name: "trade:resume-test".to_string(),
            domain: worldwake_core::ActionDomain::Trade,
            actor_constraints: vec![worldwake_sim::Constraint::ActorAlive],
            targets: vec![TargetSpec::EntityAtActorPlace {
                kind: worldwake_core::EntityKind::Agent,
            }],
            preconditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetKind {
                    target_index: 0,
                    kind: worldwake_core::EntityKind::Agent,
                },
            ],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        });
        (registry, handlers)
    }

    #[test]
    fn adopt_selected_plan_populates_expected_commodity_assumption_immediately() {
        let origin = entity(91);
        let orchard = entity(92);
        let source = entity(93);
        let mut world = World::new(cargo_topology(origin, orchard)).unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Hungry", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, origin).unwrap();
            commit_txn(txn);
            agent
        };
        let (defs, _handlers, recipes) = build_full_registries();
        let scheduler = Scheduler::new(SystemManifest::canonical());
        let mut event_log = EventLog::new();
        let mut runtime = AgentDecisionRuntime::default();
        let mut agenda_state = AgendaState::default();
        let mut frame = None;
        let mut facility_intents = ContentionIntents::default();
        let ranked_candidates = vec![ranked_goal(acquire_goal(
            CommodityKind::Apple,
            OpportunityAnchor::Place(orchard),
            BTreeSet::from([source]),
            BTreeSet::from([orchard]),
        ))];
        let goal = ranked_candidates[0].offer.key;
        let selected_plan = PlannedPlan::new(
            OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::Place(orchard),
            },
            goal,
            vec![PlannedStep {
                def_id: ActionDefId(1),
                targets: vec![PlanningEntityRef::Authoritative(orchard)],
                target_place: None,
                payload_override: None,
                op_kind: PlannerOpKind::Travel,
                estimated_ticks: 1,
                is_materialization_barrier: false,
                expected_materializations: Vec::new(),
                guard: None,
                expectations: Vec::new(),
            }],
            PlanTerminalKind::GoalSatisfied,
        );
        let (prepared_frame, current_place) = {
            let view = super::runtime_belief_view(agent, &world, &scheduler, &defs, &recipes);
            let mut prepared_frame = super::update_frame_for_adopted_plan(
                frame.as_ref(),
                &selected_plan,
                Tick(5),
                &mut runtime,
            );
            if let Some(frame) = prepared_frame.as_mut() {
                let completion_tick = plan_completion_tick_for_adoption(&selected_plan, Tick(5));
                frame.assumptions =
                    super::populate_assumptions(frame, agent, &view, Tick(5), completion_tick);
            }
            let current_place = worldwake_sim::SpatialBeliefView::effective_place(&view, agent)
                .expect("adopted test agent should have an effective place");
            (prepared_frame, current_place)
        };

        super::adopt_selected_plan(
            &mut world,
            &mut event_log,
            &mut runtime,
            &mut agenda_state,
            &mut frame,
            &mut facility_intents,
            agent,
            &ordered(&ranked_candidates),
            selected_plan,
            &recipes,
            Tick(5),
            &CognitiveProfile::default(),
            prepared_frame,
            current_place,
        );

        let frame = frame.expect("adopting a plan should create an intention frame");
        assert!(
            frame
                .assumptions
                .contains(&FrameAssumption::CommodityAvailableAt {
                    commodity: CommodityKind::Apple,
                    place: orchard,
                })
        );
        assert_eq!(
            runtime
                .current_plan
                .as_ref()
                .and_then(|plan| plan.committed_source),
            Some(SourceKey {
                entity: source,
                commodity: CommodityKind::Apple,
            })
        );
        assert_eq!(
            runtime
                .current_plan
                .as_ref()
                .and_then(|plan| plan.expectation_kind),
            Some(OpportunityExpectationKind::AcquireCommodityFromConcreteSource)
        );
    }

    #[test]
    fn adopt_selected_plan_leaves_expectation_kind_empty_without_concrete_source() {
        let origin = entity(101);
        let orchard = entity(102);
        let mut world = World::new(cargo_topology(origin, orchard)).unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Hungry", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, origin).unwrap();
            commit_txn(txn);
            agent
        };
        let (defs, _handlers, recipes) = build_full_registries();
        let scheduler = Scheduler::new(SystemManifest::canonical());
        let mut event_log = EventLog::new();
        let mut runtime = AgentDecisionRuntime::default();
        let mut agenda_state = AgendaState::default();
        let mut frame = None;
        let mut facility_intents = ContentionIntents::default();
        let ranked_candidates = vec![ranked_goal(acquire_goal(
            CommodityKind::Apple,
            OpportunityAnchor::Place(orchard),
            BTreeSet::new(),
            BTreeSet::from([orchard]),
        ))];
        let goal = ranked_candidates[0].offer.key;
        let selected_plan = PlannedPlan::new(
            OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::Place(orchard),
            },
            goal,
            vec![travel_step(orchard)],
            PlanTerminalKind::GoalSatisfied,
        );
        let (prepared_frame, current_place) = {
            let view = super::runtime_belief_view(agent, &world, &scheduler, &defs, &recipes);
            let mut prepared_frame = super::update_frame_for_adopted_plan(
                frame.as_ref(),
                &selected_plan,
                Tick(5),
                &mut runtime,
            );
            if let Some(frame) = prepared_frame.as_mut() {
                let completion_tick = plan_completion_tick_for_adoption(&selected_plan, Tick(5));
                frame.assumptions =
                    super::populate_assumptions(frame, agent, &view, Tick(5), completion_tick);
            }
            let current_place = worldwake_sim::SpatialBeliefView::effective_place(&view, agent)
                .expect("adopted test agent should have an effective place");
            (prepared_frame, current_place)
        };

        super::adopt_selected_plan(
            &mut world,
            &mut event_log,
            &mut runtime,
            &mut agenda_state,
            &mut frame,
            &mut facility_intents,
            agent,
            &ordered(&ranked_candidates),
            selected_plan,
            &recipes,
            Tick(5),
            &CognitiveProfile::default(),
            prepared_frame,
            current_place,
        );

        assert_eq!(
            runtime
                .current_plan
                .as_ref()
                .and_then(|plan| plan.committed_source),
            None
        );
        assert_eq!(
            runtime
                .current_plan
                .as_ref()
                .and_then(|plan| plan.expectation_kind),
            None
        );
    }

    #[test]
    fn same_goal_search_failure_incidents_emit_search_incident_for_committed_source() {
        let place = entity(500);
        let sibling_place = entity(501);
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let current_plan = PlannedPlan::new(
            OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::Place(place),
            },
            goal,
            vec![travel_step(place)],
            PlanTerminalKind::GoalSatisfied,
        )
        .with_committed_source(Some(SourceKey {
            entity: place,
            commodity: CommodityKind::Apple,
        }))
        .with_expectation_kind(Some(
            OpportunityExpectationKind::AcquireCommodityFromConcreteSource,
        ));
        let selected_plan = PlannedPlan::new(
            OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::Place(sibling_place),
            },
            goal,
            vec![travel_step(sibling_place)],
            PlanTerminalKind::GoalSatisfied,
        );
        let selection_plans = vec![
            SelectionCandidatePlan {
                searched_opportunity: current_plan.opportunity,
                found_plan: None,
                perceived_cost: None,
            },
            SelectionCandidatePlan {
                searched_opportunity: selected_plan.opportunity,
                found_plan: Some(selected_plan.clone()),
                perceived_cost: None,
            },
        ];

        let incidents = super::same_goal_search_failure_incidents(
            Some(&current_plan),
            &selected_plan,
            &selection_plans,
            Some(place),
            Tick(11),
        );

        assert_eq!(
            incidents,
            vec![crate::OpportunityExpectationFailureIncident {
                opportunity: current_plan.opportunity,
                source: SourceKey {
                    entity: place,
                    commodity: CommodityKind::Apple,
                },
                expectation_kind: OpportunityExpectationKind::AcquireCommodityFromConcreteSource,
                detected_at_tick: Tick(11),
                phase: ExpectationFailurePhase::Search,
                cause: ExpectationFailureCause::SameGoalSearchInfeasibleWhileSiblingSucceeded,
            }]
        );
    }

    fn travel_step(destination: EntityId) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(1),
            targets: vec![PlanningEntityRef::Authoritative(destination)],
            target_place: Some(destination),
            payload_override: None,
            op_kind: PlannerOpKind::Travel,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        }
    }

    fn trade_step(counterparty: EntityId) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(2),
            targets: vec![PlanningEntityRef::Authoritative(counterparty)],
            target_place: None,
            payload_override: Some(ActionPayload::Trade(worldwake_sim::TradeActionPayload {
                counterparty,
                sale_lot: entity(600),
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(1),
                requested_quantity: Quantity(1),
            })),
            op_kind: PlannerOpKind::Trade,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        }
    }

    fn craft_step(recipe_id: worldwake_core::RecipeId) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(3),
            targets: vec![PlanningEntityRef::Authoritative(entity(700))],
            target_place: Some(entity(700)),
            payload_override: Some(ActionPayload::Craft(worldwake_sim::CraftActionPayload {
                recipe_id,
                required_workstation_tag: WorkstationTag::Mill,
                inputs: vec![(CommodityKind::Grain, Quantity(2))],
                outputs: vec![(CommodityKind::Bread, Quantity(1))],
                required_tool_kinds: Vec::new(),
            })),
            op_kind: PlannerOpKind::Craft,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        }
    }

    fn runtime_with_failed_plan(plan: PlannedPlan) -> AgentDecisionRuntime {
        AgentDecisionRuntime {
            pending_repair_context: Some(crate::PendingRepairContext {
                failed_plan: plan,
                failed_step_index: 0,
            }),
            ..AgentDecisionRuntime::default()
        }
    }

    #[test]
    fn classify_accepted_repair_prefers_alternate_merchant_over_anchor_change() {
        let (_defs, _handlers, recipes) = build_full_registries();
        let market_a = entity(201);
        let market_b = entity(202);
        let seller_a = entity(301);
        let seller_b = entity(302);
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let failed_plan = PlannedPlan::new(
            OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::Place(market_a),
            },
            goal,
            vec![travel_step(market_a), trade_step(seller_a)],
            PlanTerminalKind::GoalSatisfied,
        );
        let selected_plan = PlannedPlan::new(
            OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::Place(market_b),
            },
            goal,
            vec![travel_step(market_b), trade_step(seller_b)],
            PlanTerminalKind::GoalSatisfied,
        );

        let repair = super::classify_accepted_repair(
            &runtime_with_failed_plan(failed_plan),
            &selected_plan,
            &recipes,
        )
        .expect("merchant replacement should classify as a repair");

        assert_eq!(repair.goal_key, goal);
        assert_eq!(repair.repair_kind, RepairKind::RebindTarget);
        assert_eq!(repair.substitute_target, Some(seller_b));
        assert_eq!(repair.substitute_recipe, None);
        assert!(!repair.records_repair_memory);
    }

    #[test]
    fn classify_accepted_repair_detects_alternate_recipe_for_same_output() {
        let mut recipes = RecipeRegistry::new();
        let recipe_a = recipes.register(RecipeDefinition {
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
        let recipe_b = recipes.register(RecipeDefinition {
            name: "Bake Bread Fast".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(1))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(2).unwrap(),
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
        let failed_goal = GoalKey::from(GoalKind::ProduceCommodity {
            recipe_id: recipe_a,
        });
        let selected_goal = GoalKey::from(GoalKind::ProduceCommodity {
            recipe_id: recipe_b,
        });
        let failed_plan = PlannedPlan::new(
            opportunity(failed_goal),
            failed_goal,
            vec![craft_step(recipe_a)],
            PlanTerminalKind::GoalSatisfied,
        );
        let selected_plan = PlannedPlan::new(
            opportunity(selected_goal),
            selected_goal,
            vec![craft_step(recipe_b)],
            PlanTerminalKind::GoalSatisfied,
        );

        let repair = super::classify_accepted_repair(
            &runtime_with_failed_plan(failed_plan),
            &selected_plan,
            &recipes,
        )
        .expect("recipe replacement should classify as a repair");

        assert_eq!(repair.goal_key, selected_goal);
        assert_eq!(repair.repair_kind, RepairKind::RebindTarget);
        assert_eq!(repair.substitute_target, None);
        assert_eq!(repair.substitute_recipe, Some(recipe_b));
        assert!(!repair.records_repair_memory);
    }

    #[test]
    fn classify_accepted_repair_detects_alternate_route_for_same_anchor() {
        let (_defs, _handlers, recipes) = build_full_registries();
        let market = entity(401);
        let waypoint_a = entity(402);
        let waypoint_b = entity(403);
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let failed_plan = PlannedPlan::new(
            OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::Place(market),
            },
            goal,
            vec![travel_step(waypoint_a), travel_step(market)],
            PlanTerminalKind::GoalSatisfied,
        );
        let selected_plan = PlannedPlan::new(
            OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::Place(market),
            },
            goal,
            vec![travel_step(waypoint_b), travel_step(market)],
            PlanTerminalKind::GoalSatisfied,
        );

        let repair = super::classify_accepted_repair(
            &runtime_with_failed_plan(failed_plan),
            &selected_plan,
            &recipes,
        )
        .expect("route replacement should classify as a repair");

        assert_eq!(repair.goal_key, goal);
        assert_eq!(repair.repair_kind, RepairKind::ReplaceProvider);
        assert_eq!(repair.substitute_target, None);
        assert_eq!(repair.substitute_recipe, None);
        assert!(!repair.records_repair_memory);
    }

    #[test]
    fn pending_trigger_from_failed_plan_uses_trade_counterparty_and_place() {
        let market = entity(501);
        let seller = entity(502);
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let failed_plan = PlannedPlan::new(
            OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::Place(market),
            },
            goal,
            vec![travel_step(market), trade_step(seller)],
            PlanTerminalKind::GoalSatisfied,
        );

        assert_eq!(
            super::pending_trigger_from_failed_plan(&failed_plan),
            Some(RevivalTrigger::CounterpartyAvailable {
                counterparty: seller,
                place: market,
            })
        );
    }

    #[test]
    fn clear_current_plan_parks_committed_trade_goal_into_pending_repair() {
        let market = entity(601);
        let seller = entity(602);
        let mut world = World::new(cargo_topology(entity(600), market)).unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Buyer", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, market).unwrap();
            commit_txn(txn);
            agent
        };
        let mut event_log = EventLog::new();
        let failed_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let failed_plan = PlannedPlan::new(
            OpportunityKey {
                goal_key: failed_goal,
                anchor: OpportunityAnchor::Place(market),
            },
            failed_goal,
            vec![trade_step(seller)],
            PlanTerminalKind::GoalSatisfied,
        );
        let mut runtime = AgentDecisionRuntime {
            current_plan: Some(failed_plan.clone()),
            pending_repair_context: Some(crate::PendingRepairContext {
                failed_plan,
                failed_step_index: 0,
            }),
            ..AgentDecisionRuntime::default()
        };
        let mut agenda_state = AgendaState {
            committed: Some(AgendaEntry::committed_from(
                &ranked_goal(acquire_goal(
                    CommodityKind::Bread,
                    OpportunityAnchor::Place(market),
                    BTreeSet::from([seller]),
                    BTreeSet::from([market]),
                )),
                Tick(7),
            )),
            ..AgendaState::default()
        };
        let mut frame = None;
        let mut facility_intents = ContentionIntents::default();
        let ranked_candidates = vec![ranked_goal(acquire_goal(
            CommodityKind::Bread,
            OpportunityAnchor::Place(market),
            BTreeSet::from([seller]),
            BTreeSet::from([market]),
        ))];

        super::clear_current_plan(
            &mut world,
            &mut event_log,
            &mut runtime,
            &mut agenda_state,
            &mut frame,
            &mut facility_intents,
            &ordered(&ranked_candidates),
            agent,
            Tick(8),
        );

        assert!(runtime.current_plan.is_none());
        assert!(frame.is_none());
        assert!(facility_intents.intents.is_empty());
        assert!(agenda_state.committed.is_none());
        let pending = agenda_state
            .pending
            .values()
            .find(|entry| entry.key.goal_key == failed_goal)
            .expect("failed trade commitment should park in pending");
        assert_eq!(pending.phase, AgendaPhase::Pending);
        assert_eq!(
            pending.revival_trigger,
            Some(RevivalTrigger::CounterpartyAvailable {
                counterparty: seller,
                place: market,
            })
        );
        assert_eq!(pending.kill_condition, KillCondition::External);
    }

    #[test]
    fn clear_current_plan_snapshots_current_trade_plan_for_pending_repair() {
        let market = entity(611);
        let seller = entity(612);
        let mut world = World::new(cargo_topology(entity(610), market)).unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Buyer", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, market).unwrap();
            commit_txn(txn);
            agent
        };
        let mut event_log = EventLog::new();
        let failed_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let failed_plan = PlannedPlan::new(
            OpportunityKey {
                goal_key: failed_goal,
                anchor: OpportunityAnchor::Place(market),
            },
            failed_goal,
            vec![trade_step(seller)],
            PlanTerminalKind::GoalSatisfied,
        );
        let mut runtime = AgentDecisionRuntime {
            current_plan: Some(failed_plan.clone()),
            ..AgentDecisionRuntime::default()
        };
        let mut agenda_state = AgendaState {
            committed: Some(AgendaEntry::committed_from(
                &ranked_goal(acquire_goal(
                    CommodityKind::Bread,
                    OpportunityAnchor::Place(market),
                    BTreeSet::from([seller]),
                    BTreeSet::from([market]),
                )),
                Tick(7),
            )),
            ..AgendaState::default()
        };
        let mut frame = None;
        let mut facility_intents = ContentionIntents::default();
        let ranked_candidates = vec![ranked_goal(acquire_goal(
            CommodityKind::Bread,
            OpportunityAnchor::Place(market),
            BTreeSet::from([seller]),
            BTreeSet::from([market]),
        ))];

        super::clear_current_plan(
            &mut world,
            &mut event_log,
            &mut runtime,
            &mut agenda_state,
            &mut frame,
            &mut facility_intents,
            &ordered(&ranked_candidates),
            agent,
            Tick(8),
        );

        assert_eq!(
            runtime
                .pending_repair_context
                .as_ref()
                .map(|context| &context.failed_plan),
            Some(&failed_plan),
            "planning-path clear should preserve the failed trade plan for later revival"
        );
        let pending = agenda_state
            .pending
            .values()
            .find(|entry| entry.key.goal_key == failed_goal)
            .expect("failed trade commitment should park in pending");
        assert_eq!(
            pending.revival_trigger,
            Some(RevivalTrigger::CounterpartyAvailable {
                counterparty: seller,
                place: market,
            })
        );
    }

    #[test]
    fn resume_pending_repair_plan_restores_failed_trade_plan_when_counterparty_trigger_revives() {
        let market = entity(603);
        let mut world = World::new(cargo_topology(entity(602), market)).unwrap();
        let (agent, seller) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Buyer", ControlSource::Ai).unwrap();
            let seller = txn.create_agent("Seller", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, market).unwrap();
            txn.set_ground_location(seller, market).unwrap();
            commit_txn(txn);
            (agent, seller)
        };
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let failed_trade_step = PlannedStep {
            def_id: ActionDefId(0),
            targets: vec![PlanningEntityRef::Authoritative(seller)],
            target_place: None,
            payload_override: Some(ActionPayload::Trade(worldwake_sim::TradeActionPayload {
                counterparty: seller,
                sale_lot: entity(600),
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(1),
                requested_quantity: Quantity(1),
            })),
            op_kind: PlannerOpKind::Trade,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        };
        let failed_plan = PlannedPlan::new(
            OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::Place(market),
            },
            goal,
            vec![failed_trade_step],
            PlanTerminalKind::GoalSatisfied,
        );
        let mut runtime = AgentDecisionRuntime {
            pending_repair_context: Some(crate::PendingRepairContext {
                failed_plan: failed_plan.clone(),
                failed_step_index: 0,
            }),
            ..AgentDecisionRuntime::default()
        };
        let committed = AgendaEntry::committed_from(
            &ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(market),
                BTreeSet::from([seller]),
                BTreeSet::from([market]),
            )),
            Tick(9),
        );
        let mut committed = committed;
        committed.revival_trigger = Some(RevivalTrigger::CounterpartyAvailable {
            counterparty: seller,
            place: market,
        });
        let agenda_state = AgendaState {
            committed: Some(committed),
            ..AgendaState::default()
        };
        let ranked_candidates = vec![ranked_goal(acquire_goal(
            CommodityKind::Bread,
            OpportunityAnchor::Place(market),
            BTreeSet::from([seller]),
            BTreeSet::from([market]),
        ))];
        let (defs, handlers) = build_resume_trade_registry();
        let scheduler = Scheduler::new(SystemManifest::canonical());
        let recipes = RecipeRegistry::new();
        let view = super::runtime_belief_view(agent, &world, &scheduler, &defs, &recipes);

        assert!(super::resume_pending_repair_plan(
            &mut runtime,
            &agenda_state,
            &ordered(&ranked_candidates),
            &view,
            agent,
            &defs,
            &handlers,
        ));
        assert_eq!(
            runtime
                .current_plan
                .as_ref()
                .and_then(|plan| plan.steps.first())
                .and_then(|step| step.payload_override.as_ref())
                .and_then(ActionPayload::as_trade)
                .map(|payload| payload.offered_quantity),
            Some(Quantity(3))
        );
        assert_eq!(runtime.current_step_index, 0);
        assert!(!runtime.step_in_flight);
    }

    fn ranked_goal(goal: GoalOffer) -> AgendaEntry {
        AgendaEntry {
            key: worldwake_core::OpportunityKey {
                goal_key: goal.key,
                anchor: goal.anchor,
            },
            offer: goal,
            priority_class: GoalPriorityClass::High,
            motive_score: 100,
            motive_source_contributions: Vec::new(),
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            source_composite: None,
            feasibility: FeasibilityHint::Likely,
            partial_plan_segment: None,
            phase: crate::AgendaPhase::Pending,
            origin: crate::AgendaOrigin::NeedDrive,
            introduced_tick: Tick(0),
            last_reconsidered_tick: Tick(0),
            revival_trigger: None,
            kill_condition: crate::KillCondition::External,
        }
    }

    #[test]
    fn emit_plan_selection_events_records_commit_then_adoption_with_truncation() {
        let selected_goal = GoalKey::from(GoalKind::Sleep);
        let runner_up = GoalKey::from(GoalKind::Wash);
        let third = GoalKey::from(GoalKind::Relieve);
        let fourth = GoalKey::from(GoalKind::ReduceDanger);
        let decisive_source = worldwake_core::MotiveSourceRef {
            source: worldwake_core::MotiveSource::NeedPressure {
                need: HomeostaticNeedId::Fatigue,
            },
            introduced_tick: Tick(7),
        };
        let ranked_candidates = vec![
            AgendaEntry {
                offer: GoalOffer {
                    key: selected_goal,
                    anchor: OpportunityAnchor::None,
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                    obligation_source: None,
                    commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                    required_information_gaps: Vec::new(),
                    invalidators: Vec::new(),
                    learned_expectation_refs: Vec::new(),
                    motive_sources: vec![decisive_source.clone()],
                    acquisition_quantity: None,
                },
                priority_class: GoalPriorityClass::High,
                motive_score: 120,
                motive_source_contributions: Vec::new(),
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                source_composite: None,
                feasibility: FeasibilityHint::Likely,
                partial_plan_segment: None,
                key: worldwake_core::OpportunityKey {
                    goal_key: selected_goal,
                    anchor: OpportunityAnchor::None,
                },
                phase: crate::AgendaPhase::Pending,
                origin: crate::AgendaOrigin::NeedDrive,
                introduced_tick: Tick(0),
                last_reconsidered_tick: Tick(0),
                revival_trigger: None,
                kill_condition: crate::KillCondition::External,
            },
            AgendaEntry {
                offer: GoalOffer {
                    key: runner_up,
                    anchor: OpportunityAnchor::None,
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                    obligation_source: None,
                    commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                    required_information_gaps: Vec::new(),
                    invalidators: Vec::new(),
                    learned_expectation_refs: Vec::new(),
                    motive_sources: Vec::new(),
                    acquisition_quantity: None,
                },
                priority_class: GoalPriorityClass::High,
                motive_score: 110,
                motive_source_contributions: Vec::new(),
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                source_composite: None,
                feasibility: FeasibilityHint::Likely,
                partial_plan_segment: None,
                key: worldwake_core::OpportunityKey {
                    goal_key: runner_up,
                    anchor: OpportunityAnchor::None,
                },
                phase: crate::AgendaPhase::Pending,
                origin: crate::AgendaOrigin::NeedDrive,
                introduced_tick: Tick(0),
                last_reconsidered_tick: Tick(0),
                revival_trigger: None,
                kill_condition: crate::KillCondition::External,
            },
            AgendaEntry {
                offer: GoalOffer {
                    key: third,
                    anchor: OpportunityAnchor::None,
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                    obligation_source: None,
                    commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                    required_information_gaps: Vec::new(),
                    invalidators: Vec::new(),
                    learned_expectation_refs: Vec::new(),
                    motive_sources: Vec::new(),
                    acquisition_quantity: None,
                },
                priority_class: GoalPriorityClass::High,
                motive_score: 90,
                motive_source_contributions: Vec::new(),
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                source_composite: None,
                feasibility: FeasibilityHint::Likely,
                partial_plan_segment: None,
                key: worldwake_core::OpportunityKey {
                    goal_key: third,
                    anchor: OpportunityAnchor::None,
                },
                phase: crate::AgendaPhase::Pending,
                origin: crate::AgendaOrigin::NeedDrive,
                introduced_tick: Tick(0),
                last_reconsidered_tick: Tick(0),
                revival_trigger: None,
                kill_condition: crate::KillCondition::External,
            },
            AgendaEntry {
                offer: GoalOffer {
                    key: fourth,
                    anchor: OpportunityAnchor::None,
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                    obligation_source: None,
                    commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                    required_information_gaps: Vec::new(),
                    invalidators: Vec::new(),
                    learned_expectation_refs: Vec::new(),
                    motive_sources: Vec::new(),
                    acquisition_quantity: None,
                },
                priority_class: GoalPriorityClass::High,
                motive_score: 80,
                motive_source_contributions: Vec::new(),
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                source_composite: None,
                feasibility: FeasibilityHint::Likely,
                partial_plan_segment: None,
                key: worldwake_core::OpportunityKey {
                    goal_key: fourth,
                    anchor: OpportunityAnchor::None,
                },
                phase: crate::AgendaPhase::Pending,
                origin: crate::AgendaOrigin::NeedDrive,
                introduced_tick: Tick(0),
                last_reconsidered_tick: Tick(0),
                revival_trigger: None,
                kill_condition: crate::KillCondition::External,
            },
        ];
        let first_place = entity(50);
        let second_place = entity(51);
        let selected_plan = PlannedPlan::new(
            opportunity(selected_goal),
            selected_goal,
            vec![
                PlannedStep {
                    def_id: ActionDefId(1),
                    targets: vec![PlanningEntityRef::Authoritative(first_place)],
                    target_place: Some(first_place),
                    payload_override: None,
                    op_kind: PlannerOpKind::Travel,
                    estimated_ticks: 1,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                    guard: None,
                    expectations: Vec::new(),
                },
                PlannedStep {
                    def_id: ActionDefId(1),
                    targets: vec![PlanningEntityRef::Authoritative(second_place)],
                    target_place: Some(second_place),
                    payload_override: None,
                    op_kind: PlannerOpKind::Travel,
                    estimated_ticks: 1,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                    guard: None,
                    expectations: Vec::new(),
                },
            ],
            PlanTerminalKind::SearchBudgetExhausted {
                budget_consumed: 0,
                budget_total: 0,
            },
        );
        let mut event_log = EventLog::new();
        let agent = entity(1);
        let tick = Tick(9);
        let runtime = AgentDecisionRuntime::default();
        let view = SelectionContextView::default();
        let frame = worldwake_core::IntentionFrame {
            goal: selected_goal,
            domain: worldwake_core::IntentionDomain::Generic,
            assumptions: vec![FrameAssumption::RouteExists {
                from: first_place,
                to: second_place,
            }],
            state: worldwake_core::FrameState::Active,
            established_at: tick,
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 3,
            motive_refs: Vec::new(),
            resume_conditions: Vec::new(),
            abandon_conditions: Vec::new(),
            explicit_claims: Vec::new(),
            causal_links: Vec::new(),
        };

        super::emit_plan_selection_events(
            &mut event_log,
            tick,
            agent,
            &runtime,
            &view,
            &ordered(&ranked_candidates),
            &Portfolio {
                slots: BTreeMap::new(),
            },
            None,
            &selected_plan,
            2,
            Some(&frame),
        );

        let tick_events = event_log.events_at_tick(tick);
        assert_eq!(tick_events.len(), 2);
        let commit = event_log.get(tick_events[0]).unwrap();
        let adopt = event_log.get(tick_events[1]).unwrap();
        assert!(commit.tags().contains(&EventTag::GoalCommitted));
        assert!(adopt.tags().contains(&EventTag::PlanAdopted));
        assert_eq!(
            commit.decision_payload(),
            Some(&DecisionEventPayload::GoalCommitted(GoalCommittedPayload {
                agent,
                goal_key: selected_goal,
                motive_score: 120,
                decisive_motive_sources: vec![decisive_source],
                rejected_alternatives: vec![
                    worldwake_core::RejectedAlternativeSummary {
                        goal_key: runner_up,
                        rejection_reason: GoalRejectionReason::LowerMotive,
                        score_gap: 10,
                        rejection_dimension: Some(RankedGoalComparisonDimensionTag::MotiveScore),
                    },
                    worldwake_core::RejectedAlternativeSummary {
                        goal_key: third,
                        rejection_reason: GoalRejectionReason::LowerMotive,
                        score_gap: 30,
                        rejection_dimension: Some(RankedGoalComparisonDimensionTag::MotiveScore),
                    },
                ],
                assumptions: vec![worldwake_core::PlanAssumptionRef {
                    assumption: FrameAssumption::RouteExists {
                        from: first_place,
                        to: second_place,
                    },
                    introduced_at_step: 1,
                }],
                testimony_trust_context: Vec::new(),
                route_preference_context: Vec::new(),
            }))
        );
        assert_eq!(
            adopt.decision_payload(),
            Some(&DecisionEventPayload::PlanAdopted(PlanAdoptedPayload {
                agent,
                goal_key: selected_goal,
                plan_step_count: 2,
                assumptions: vec![worldwake_core::PlanAssumptionRef {
                    assumption: FrameAssumption::RouteExists {
                        from: first_place,
                        to: second_place,
                    },
                    introduced_at_step: 1,
                }],
            }))
        );
    }

    #[test]
    fn emit_plan_selection_events_records_learned_contexts_for_committed_goal() {
        let agent = entity(1);
        let witness = entity(2);
        let subject = entity(3);
        let first_place = entity(50);
        let second_place = entity(51);
        let topic = TellTopic::EntityBelief { subject };
        let selected_goal = GoalKey::from(GoalKind::AskWitness { witness, topic });
        let runner_up = GoalKey::from(GoalKind::Sleep);
        let ranked_candidates = vec![
            ranked_goal(GoalOffer {
                key: selected_goal,
                anchor: OpportunityAnchor::None,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                motive_sources: Vec::new(),
                acquisition_quantity: None,
            }),
            ranked_goal(GoalOffer {
                key: runner_up,
                anchor: OpportunityAnchor::None,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                motive_sources: Vec::new(),
                acquisition_quantity: None,
            }),
        ];
        let selected_plan = PlannedPlan::new(
            opportunity(selected_goal),
            selected_goal,
            vec![PlannedStep {
                def_id: ActionDefId(1),
                targets: vec![PlanningEntityRef::Authoritative(second_place)],
                target_place: Some(second_place),
                payload_override: None,
                op_kind: PlannerOpKind::Travel,
                estimated_ticks: 1,
                is_materialization_barrier: false,
                expected_materializations: Vec::new(),
                guard: None,
                expectations: Vec::new(),
            }],
            PlanTerminalKind::SearchBudgetExhausted {
                budget_consumed: 0,
                budget_total: 0,
            },
        );

        let tick = Tick(20);
        let mut runtime = AgentDecisionRuntime::default();
        runtime.testimony_reliability.record_refutation(
            worldwake_core::TestimonyReliabilityKey {
                source: witness,
                topic: TopicScope::GeneralFact,
            },
            EventId(77),
            Tick(19),
        );
        runtime
            .route_preference
            .record_safe(RouteSegment::new(first_place, second_place), tick);
        let mut effective_places = BTreeMap::new();
        effective_places.insert(agent, first_place);
        let view = SelectionContextView {
            effective_places,
            testimony_profile: Some(TestimonyTrustProfile {
                minimum_observations: 1,
                ..TestimonyTrustProfile::default()
            }),
            route_profile: Some(RoutePreferenceProfile {
                minimum_traversals: 1,
                ..RoutePreferenceProfile::default()
            }),
        };
        let mut event_log = EventLog::new();

        super::emit_plan_selection_events(
            &mut event_log,
            tick,
            agent,
            &runtime,
            &view,
            &ordered(&ranked_candidates),
            &Portfolio {
                slots: BTreeMap::new(),
            },
            None,
            &selected_plan,
            2,
            None,
        );

        let tick_events = event_log.events_at_tick(tick);
        let commit = event_log.get(tick_events[0]).unwrap();
        let Some(DecisionEventPayload::GoalCommitted(payload)) = commit.decision_payload() else {
            panic!("first decision event should be a goal commit");
        };
        assert_eq!(
            payload.testimony_trust_context,
            vec![TestimonyTrustSummary {
                source: witness,
                topic: TopicScope::GeneralFact,
                trust: Permille::new_unchecked(300),
                observations: 1,
            }]
        );
        assert_eq!(
            payload.route_preference_context,
            vec![RoutePreferenceSummary {
                segment: RouteSegment::new(first_place, second_place),
                preference: Permille::new_unchecked(700),
                last_safe_tick: Some(tick),
                last_dangerous_tick: None,
            }]
        );
    }

    #[test]
    fn build_rejected_alternatives_records_decisive_dimensions() {
        let selected_goal = GoalKey::from(GoalKind::Sleep);
        let lower_motive_goal = GoalKey::from(GoalKind::Relieve);
        let lower_feasibility_goal = GoalKey::from(GoalKind::Wash);
        let selected = ranked_goal(GoalOffer {
            key: selected_goal,
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        });
        let mut lower_motive = ranked_goal(GoalOffer {
            key: lower_motive_goal,
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        });
        lower_motive.motive_score = 80;
        let mut lower_feasibility = ranked_goal(GoalOffer {
            key: lower_feasibility_goal,
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        });
        lower_feasibility.feasibility = FeasibilityHint::Uncertain;
        let ranked_candidates = vec![selected, lower_feasibility, lower_motive];

        let rejected = super::build_rejected_alternatives(
            &ordered(&ranked_candidates),
            &Portfolio {
                slots: BTreeMap::new(),
            },
            selected_goal,
            100,
            4,
        );

        assert_eq!(
            rejected,
            vec![
                worldwake_core::RejectedAlternativeSummary {
                    goal_key: lower_feasibility_goal,
                    rejection_reason: GoalRejectionReason::LowerMotive,
                    score_gap: 0,
                    rejection_dimension: Some(RankedGoalComparisonDimensionTag::Feasibility),
                },
                worldwake_core::RejectedAlternativeSummary {
                    goal_key: lower_motive_goal,
                    rejection_reason: GoalRejectionReason::LowerMotive,
                    score_gap: 20,
                    rejection_dimension: Some(RankedGoalComparisonDimensionTag::MotiveScore),
                },
            ]
        );
    }

    #[test]
    fn portfolio_assembly_always_runs_when_plan_cap_is_one() {
        fn ranked_slot_goal(
            kind: GoalKind,
            motive_score: u32,
            anchor: OpportunityAnchor,
        ) -> AgendaEntry {
            AgendaEntry {
                offer: GoalOffer {
                    key: GoalKey::from(kind),
                    anchor,
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                    obligation_source: None,
                    commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                    required_information_gaps: Vec::new(),
                    invalidators: Vec::new(),
                    learned_expectation_refs: Vec::new(),
                    motive_sources: Vec::new(),
                    acquisition_quantity: None,
                },
                priority_class: GoalPriorityClass::High,
                motive_score,
                motive_source_contributions: Vec::new(),
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                source_composite: None,
                feasibility: FeasibilityHint::Likely,
                partial_plan_segment: None,
                key: worldwake_core::OpportunityKey {
                    goal_key: GoalKey::from(kind),
                    anchor,
                },
                phase: crate::AgendaPhase::Pending,
                origin: crate::AgendaOrigin::NeedDrive,
                introduced_tick: Tick(0),
                last_reconsidered_tick: Tick(0),
                revival_trigger: None,
                kill_condition: crate::KillCondition::External,
            }
        }

        fn with_motive(mut ranked: AgendaEntry, source: MotiveSource, weight: u32) -> AgendaEntry {
            ranked.motive_source_contributions = vec![(
                MotiveSourceRef {
                    source,
                    introduced_tick: Tick(0),
                },
                weight,
            )];
            ranked
        }

        let posting_place = entity(40);
        let mut ranked = vec![
            with_motive(
                ranked_slot_goal(GoalKind::Sleep, 900, OpportunityAnchor::None),
                MotiveSource::NeedPressure {
                    need: HomeostaticNeedId::Hunger,
                },
                900,
            ),
            with_motive(
                ranked_slot_goal(
                    GoalKind::PostNotice {
                        posting: worldwake_core::ArtifactPostingContext {
                            posting_place,
                            issuing_authority: None,
                            expires_at: Some(Tick(5)),
                            jurisdiction: Some(posting_place),
                        },
                        topic: worldwake_core::NoticeTopic::ThreatWarning {
                            place: posting_place,
                        },
                    },
                    700,
                    OpportunityAnchor::Place(posting_place),
                ),
                MotiveSource::OfficeDuty {
                    office: posting_place,
                },
                700,
            ),
            with_motive(
                ranked_slot_goal(
                    GoalKind::SellCommodity {
                        commodity: CommodityKind::Apple,
                    },
                    500,
                    OpportunityAnchor::Place(entity(41)),
                ),
                MotiveSource::Greed {
                    opportunity: OpportunityKey {
                        goal_key: GoalKey::from(GoalKind::SellCommodity {
                            commodity: CommodityKind::Apple,
                        }),
                        anchor: OpportunityAnchor::Place(entity(41)),
                    },
                },
                500,
            ),
        ];
        let ranked = crate::ranking::sort_in_place(&mut ranked);
        let portfolio = super::assemble_portfolio(
            &ranked,
            None,
            &worldwake_core::PortfolioWeightsProfile::default(),
            worldwake_core::OperatingMode::Normal,
            |_| FeasibilityVerdict::Plausible,
        );
        let plausible_slots = portfolio
            .plausible_slots_by_score(&worldwake_core::PortfolioWeightsProfile::default())
            .into_iter()
            .map(|(kind, _)| kind)
            .collect::<Vec<_>>();
        let pass = super::CandidatePlanningPass {
            portfolio,
            plausible_slots,
            search_order: vec![OpportunityKey {
                goal_key: GoalKey::from(GoalKind::Sleep),
                anchor: OpportunityAnchor::None,
            }],
            plans: vec![searched_plan(
                OpportunityKey {
                    goal_key: GoalKey::from(GoalKind::Sleep),
                    anchor: OpportunityAnchor::None,
                },
                PlanSearchResult::FrontierExhausted { expansions_used: 1 },
            )],
            snapshot_admissions: None,
            snapshot_cache_counters: None,
            planning_state_cache_counters: None,
        };

        let trace = pass.portfolio_trace();
        assert_eq!(trace.slots.len(), 3);
        assert_eq!(trace.slots_attempted, 1);
        assert!(trace.slots.contains_key(&SlotKind::NeedSurvival));
        assert!(trace.slots.contains_key(&SlotKind::ObligationDuty));
        assert!(trace.slots.contains_key(&SlotKind::EconomicOpportunity));
    }

    #[test]
    fn infeasible_top_two_rejected_feasible_third_commits_same_tick() {
        let selected_goal = GoalKey::from(GoalKind::SellCommodity {
            commodity: CommodityKind::Apple,
        });
        let survival_goal = GoalKey::from(GoalKind::Sleep);
        let posting_place = entity(44);
        let commitment_goal = GoalKey::from(GoalKind::PostNotice {
            posting: worldwake_core::ArtifactPostingContext {
                posting_place,
                issuing_authority: None,
                expires_at: Some(Tick(10)),
                jurisdiction: Some(posting_place),
            },
            topic: worldwake_core::NoticeTopic::ThreatWarning {
                place: posting_place,
            },
        });
        let ranked_candidates = vec![
            AgendaEntry {
                offer: GoalOffer {
                    key: survival_goal,
                    anchor: OpportunityAnchor::None,
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                    obligation_source: None,
                    commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                    required_information_gaps: Vec::new(),
                    invalidators: Vec::new(),
                    learned_expectation_refs: Vec::new(),
                    motive_sources: Vec::new(),
                    acquisition_quantity: None,
                },
                priority_class: GoalPriorityClass::High,
                motive_score: 900,
                motive_source_contributions: Vec::new(),
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                source_composite: None,
                feasibility: FeasibilityHint::Likely,
                partial_plan_segment: None,
                key: worldwake_core::OpportunityKey {
                    goal_key: survival_goal,
                    anchor: OpportunityAnchor::None,
                },

                phase: crate::AgendaPhase::Pending,
                origin: crate::AgendaOrigin::NeedDrive,
                introduced_tick: Tick(0),
                last_reconsidered_tick: Tick(0),
                revival_trigger: None,
                kill_condition: crate::KillCondition::External,
            },
            AgendaEntry {
                offer: GoalOffer {
                    key: commitment_goal,
                    anchor: OpportunityAnchor::Place(posting_place),
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                    obligation_source: None,
                    commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                    required_information_gaps: Vec::new(),
                    invalidators: Vec::new(),
                    learned_expectation_refs: Vec::new(),
                    motive_sources: Vec::new(),
                    acquisition_quantity: None,
                },
                priority_class: GoalPriorityClass::High,
                motive_score: 800,
                motive_source_contributions: Vec::new(),
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                source_composite: None,
                feasibility: FeasibilityHint::Likely,
                partial_plan_segment: None,
                key: worldwake_core::OpportunityKey {
                    goal_key: commitment_goal,
                    anchor: OpportunityAnchor::Place(posting_place),
                },

                phase: crate::AgendaPhase::Pending,
                origin: crate::AgendaOrigin::NeedDrive,
                introduced_tick: Tick(0),
                last_reconsidered_tick: Tick(0),
                revival_trigger: None,
                kill_condition: crate::KillCondition::External,
            },
            AgendaEntry {
                offer: GoalOffer {
                    key: selected_goal,
                    anchor: OpportunityAnchor::Place(entity(45)),
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                    obligation_source: None,
                    commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                    required_information_gaps: Vec::new(),
                    invalidators: Vec::new(),
                    learned_expectation_refs: Vec::new(),
                    motive_sources: Vec::new(),
                    acquisition_quantity: None,
                },
                priority_class: GoalPriorityClass::High,
                motive_score: 600,
                motive_source_contributions: Vec::new(),
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                source_composite: None,
                feasibility: FeasibilityHint::Likely,
                partial_plan_segment: None,
                key: worldwake_core::OpportunityKey {
                    goal_key: selected_goal,
                    anchor: OpportunityAnchor::Place(entity(45)),
                },

                phase: crate::AgendaPhase::Pending,
                origin: crate::AgendaOrigin::NeedDrive,
                introduced_tick: Tick(0),
                last_reconsidered_tick: Tick(0),
                revival_trigger: None,
                kill_condition: crate::KillCondition::External,
            },
        ];
        let _selected_plan = PlannedPlan::new(
            opportunity(selected_goal),
            selected_goal,
            vec![PlannedStep {
                def_id: ActionDefId(7),
                targets: Vec::new(),
                target_place: None,
                payload_override: None,
                op_kind: PlannerOpKind::StockManagement,
                estimated_ticks: 1,
                is_materialization_barrier: false,
                expected_materializations: Vec::new(),
                guard: None,
                expectations: Vec::new(),
            }],
            PlanTerminalKind::GoalSatisfied,
        );
        let portfolio = Portfolio {
            slots: BTreeMap::from([
                (
                    SlotKind::NeedSurvival,
                    PortfolioSlot {
                        ranked: ranked_candidates[0].clone(),
                        feasibility: FeasibilityVerdict::RejectedBeforeSearch {
                            reason: worldwake_core::Discrepancy::MissingObservation,
                        },
                    },
                ),
                (
                    SlotKind::ObligationDuty,
                    PortfolioSlot {
                        ranked: ranked_candidates[1].clone(),
                        feasibility: FeasibilityVerdict::RejectedBeforeSearch {
                            reason: worldwake_core::Discrepancy::RouteUnknown,
                        },
                    },
                ),
                (
                    SlotKind::EconomicOpportunity,
                    PortfolioSlot {
                        ranked: ranked_candidates[2].clone(),
                        feasibility: FeasibilityVerdict::Plausible,
                    },
                ),
            ]),
        };
        let rejected = super::build_rejected_alternatives(
            &ordered(&ranked_candidates),
            &portfolio,
            selected_goal,
            600,
            4,
        );
        assert_eq!(
            rejected,
            vec![
                worldwake_core::RejectedAlternativeSummary {
                    goal_key: survival_goal,
                    rejection_reason: GoalRejectionReason::FeasibilityProbeFailed,
                    score_gap: -300,
                    rejection_dimension: Some(RankedGoalComparisonDimensionTag::MotiveScore),
                },
                worldwake_core::RejectedAlternativeSummary {
                    goal_key: commitment_goal,
                    rejection_reason: GoalRejectionReason::FeasibilityProbeFailed,
                    score_gap: -200,
                    rejection_dimension: Some(RankedGoalComparisonDimensionTag::MotiveScore),
                },
            ]
        );
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
                skeleton_source: None,
                strategic_budget: Some(crate::decision_trace::StrategicBudgetTrace {
                    stages_count: 1,
                    budget_total: 6,
                    budget_used: 2,
                    exhausted: false,
                }),
                method_trace: None,
                goal_budget: worldwake_core::GoalPlanningBudget::PRODUCTION,
                planning_state_cache_counters: None,
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
            trace.goal_budget,
            worldwake_core::GoalPlanningBudget::PRODUCTION
        );
        assert_eq!(
            trace
                .strategic_budget
                .as_ref()
                .map(|budget| budget.budget_used),
            Some(2)
        );
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
    ) -> AgendaEntry {
        AgendaEntry {
            offer: GoalOffer {
                key: opportunity.goal_key,
                anchor: opportunity.anchor,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                motive_sources: Vec::new(),
                acquisition_quantity: None,
            },
            priority_class,
            motive_score,
            motive_source_contributions: Vec::new(),
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            source_composite: None,
            feasibility: FeasibilityHint::Likely,
            partial_plan_segment: None,
            key: worldwake_core::OpportunityKey {
                goal_key: opportunity.goal_key,
                anchor: opportunity.anchor,
            },
            phase: crate::AgendaPhase::Pending,
            origin: crate::AgendaOrigin::NeedDrive,
            introduced_tick: Tick(0),
            last_reconsidered_tick: Tick(0),
            revival_trigger: None,
            kill_condition: crate::KillCondition::External,
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
    fn summarize_ranked_goal_preserves_source_composite() {
        let goal = acquire_goal(
            CommodityKind::Bread,
            OpportunityAnchor::Place(place_entity(40)),
            BTreeSet::from([entity(9)]),
            BTreeSet::new(),
        );
        let mut ranked = ranked_goal(goal);
        ranked.source_composite = Some(SourceCompositeRank {
            source_entity: entity(9),
            commodity: CommodityKind::Bread,
            trust_factor_permille: 900,
            wait_factor_permille: 800,
            capacity_factor_permille: 1200,
            composite_permille: 864,
        });

        let summary = summarize_ranked_goal(&ranked);

        assert_eq!(summary.source_composite, ranked.source_composite);
    }

    #[test]
    fn summarize_ranked_goal_preserves_acquisition_quantity() {
        // S127QUAAWAACQ-009: the per-emission `AcquisitionQuantity` set on
        // `GoalOffer.acquisition_quantity` at candidate emission time must
        // reach the decision-trace `RankedGoalSummary` so consumers can
        // inspect `desired_min` / `desired_target` / `horizon_ticks`
        // without re-deriving from agent state.
        let goal = acquire_goal(
            CommodityKind::Bread,
            OpportunityAnchor::Place(place_entity(40)),
            BTreeSet::new(),
            BTreeSet::new(),
        );
        let mut ranked = ranked_goal(goal);
        let expected = AcquisitionQuantity {
            desired_min: std::num::NonZeroU16::new(2).unwrap(),
            desired_target: std::num::NonZeroU16::new(5).unwrap(),
            horizon_ticks: std::num::NonZeroU32::new(123).unwrap(),
        };
        ranked.offer.acquisition_quantity = Some(expected);

        let summary = summarize_ranked_goal(&ranked);

        assert_eq!(summary.acquisition_quantity, Some(expected));
    }

    #[test]
    fn summarize_ranked_goal_populates_motive_source_contributions() {
        let mut goal = acquire_goal(
            CommodityKind::Bread,
            OpportunityAnchor::Place(place_entity(40)),
            BTreeSet::new(),
            BTreeSet::new(),
        );
        let source = worldwake_core::MotiveSourceRef {
            source: worldwake_core::MotiveSource::NeedPressure {
                need: HomeostaticNeedId::Hunger,
            },
            introduced_tick: Tick(12),
        };
        goal.motive_sources = vec![source.clone()];
        let ranked = ranked_goal_with_score(
            OpportunityKey {
                goal_key: goal.key,
                anchor: goal.anchor,
            },
            GoalPriorityClass::High,
            42,
        );
        let mut ranked = AgendaEntry {
            offer: goal,
            ..ranked
        };
        ranked.motive_score = 42;
        ranked.motive_source_contributions = vec![(source.clone(), 42)];

        let summary = summarize_ranked_goal(&ranked);

        assert_eq!(summary.motive_source_contributions, vec![(source, 42)]);
    }

    #[test]
    fn summarize_selected_plan_preserves_side_benefit_trace_fields() {
        let market = place_entity(40);
        let orchard = place_entity(41);
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
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
                    target_place: None,
                    payload_override: None,
                    op_kind: crate::PlannerOpKind::Travel,
                    estimated_ticks: 3,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                    guard: None,
                    expectations: Vec::new(),
                },
                PlannedStep {
                    def_id: ActionDefId(2),
                    targets: vec![crate::PlanningEntityRef::Authoritative(orchard)],
                    target_place: None,
                    payload_override: None,
                    op_kind: crate::PlannerOpKind::Travel,
                    estimated_ticks: 2,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                    guard: None,
                    expectations: Vec::new(),
                },
            ],
            PlanTerminalKind::GoalSatisfied,
        );
        let ranked_candidates = vec![
            AgendaEntry {
                offer: GoalOffer {
                    key: goal,
                    anchor: OpportunityAnchor::None,
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                    obligation_source: None,
                    commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                    required_information_gaps: Vec::new(),
                    invalidators: Vec::new(),
                    learned_expectation_refs: Vec::new(),
                    motive_sources: Vec::new(),
                    acquisition_quantity: None,
                },
                priority_class: GoalPriorityClass::High,
                motive_score: 800,
                motive_source_contributions: Vec::new(),
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                source_composite: None,
                feasibility: FeasibilityHint::Likely,
                partial_plan_segment: None,
                key: worldwake_core::OpportunityKey {
                    goal_key: goal,
                    anchor: OpportunityAnchor::None,
                },

                phase: crate::AgendaPhase::Pending,
                origin: crate::AgendaOrigin::NeedDrive,
                introduced_tick: Tick(0),
                last_reconsidered_tick: Tick(0),
                revival_trigger: None,
                kill_condition: crate::KillCondition::External,
            },
            AgendaEntry {
                offer: GoalOffer {
                    key: GoalKey::from(GoalKind::SellCommodity {
                        commodity: CommodityKind::Apple,
                    }),
                    anchor: OpportunityAnchor::Place(market),
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                    obligation_source: None,
                    commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                    required_information_gaps: Vec::new(),
                    invalidators: Vec::new(),
                    learned_expectation_refs: Vec::new(),
                    motive_sources: Vec::new(),
                    acquisition_quantity: None,
                },
                priority_class: GoalPriorityClass::Low,
                motive_score: 300,
                motive_source_contributions: Vec::new(),
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                source_composite: None,
                feasibility: FeasibilityHint::Likely,
                partial_plan_segment: None,
                key: worldwake_core::OpportunityKey {
                    goal_key: goal,
                    anchor: OpportunityAnchor::Place(market),
                },

                phase: crate::AgendaPhase::Pending,
                origin: crate::AgendaOrigin::NeedDrive,
                introduced_tick: Tick(0),
                last_reconsidered_tick: Tick(0),
                revival_trigger: None,
                kill_condition: crate::KillCondition::External,
            },
        ];
        let plan_value = selected_plan_value(
            &ordered(&ranked_candidates),
            &plan,
            Permille::new(100).unwrap(),
        )
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
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
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
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        });

        let exact = super::summarize_step(
            &PlannedStep {
                def_id: ActionDefId(0),
                targets: vec![],
                target_place: None,
                payload_override: None,
                op_kind: crate::PlannerOpKind::Heal,
                estimated_ticks: 3,
                is_materialization_barrier: false,
                expected_materializations: vec![],
                guard: None,
                expectations: Vec::new(),
            },
            &action_defs,
        );
        let fungible = super::summarize_step(
            &PlannedStep {
                def_id: ActionDefId(1),
                targets: vec![],
                target_place: None,
                payload_override: None,
                op_kind: crate::PlannerOpKind::Consume,
                estimated_ticks: 2,
                is_materialization_barrier: false,
                expected_materializations: vec![],
                guard: None,
                expectations: Vec::new(),
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
            &ordered(&[
                ranked_goal_with_score(top, GoalPriorityClass::High, 900),
                ranked_goal_with_score(current, GoalPriorityClass::High, 800),
            ]),
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
            &ordered(&[
                ranked_goal_with_score(top, GoalPriorityClass::High, 950),
                ranked_goal_with_score(current, GoalPriorityClass::High, 800),
            ]),
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
            &ordered(&[
                ranked_goal_with_score(top, GoalPriorityClass::Critical, 820),
                ranked_goal_with_score(current, GoalPriorityClass::High, 1000),
            ]),
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
            &ordered(&[ranked_goal_with_score(top, GoalPriorityClass::High, 900)]),
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
            &ordered(&[
                ranked_goal_with_score(top, GoalPriorityClass::High, 801),
                ranked_goal_with_score(current, GoalPriorityClass::High, 800),
            ]),
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
            ranked_goal(GoalOffer {
                key: GoalKey::from(GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                }),
                anchor: worldwake_core::OpportunityAnchor::None,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                motive_sources: Vec::new(),
                acquisition_quantity: None,
            }),
            ranked_goal(GoalOffer {
                key: GoalKey::from(GoalKind::Sleep),
                anchor: worldwake_core::OpportunityAnchor::Place(market),
                evidence_entities: BTreeSet::from([seller]),
                evidence_places: BTreeSet::from([market]),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                motive_sources: Vec::new(),
                acquisition_quantity: None,
            }),
        ];
        let budget = ProfileFixture {
            snapshot_travel_horizon: 0,
            ..ProfileFixture::default()
        };

        let plans = super::build_candidate_plans(
            &world,
            &scheduler,
            agent,
            &ordered(&ranked_candidates),
            None,
            &worldwake_core::DiscrepancyMemory::default(),
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
                quantity: AcquisitionQuantity::single(),
            })
        );
        assert!(
            !first.result.is_found(),
            "AcquireCommodity(Bread) search should not be able to use the remote seller evidence attached only to a different admitted candidate"
        );
    }

    #[test]
    fn portfolio_admission_prefers_strongest_same_slot_candidate_before_ranked_fallback() {
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
        // Candidates are pre-sorted by `ranking::compare_ranked_goals`:
        // the same-place/local-evidence candidate ranks ahead of the remote
        // one when everything else is equal. Portfolio admission must honour
        // that sort order rather than re-tiebreaking internally.
        let ranked_candidates = vec![
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(origin),
                BTreeSet::from([bread]),
                BTreeSet::from([origin]),
            )),
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(market),
                BTreeSet::new(),
                BTreeSet::from([market]),
            )),
        ];
        let budget = ProfileFixture {
            snapshot_travel_horizon: 4,
            ..ProfileFixture::default()
        };

        let plans = super::build_candidate_plans(
            &world,
            &scheduler,
            agent,
            &ordered(&ranked_candidates),
            None,
            &worldwake_core::DiscrepancyMemory::default(),
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

        assert_eq!(plans.len(), 2);
        assert_eq!(
            plans[0].opportunity.anchor,
            OpportunityAnchor::Place(origin)
        );
        assert!(
            !matches!(plans[0].result, PlanSearchResult::Unsupported),
            "the strongest same-slot opportunity should lead the search order after portfolio admission"
        );
        assert_eq!(
            plans[1].opportunity.anchor,
            OpportunityAnchor::Place(market)
        );
        assert!(
            !matches!(plans[1].result, PlanSearchResult::Unsupported),
            "remaining candidate budget should still search the next admitted ranked opportunity"
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
            ..ProfileFixture::default()
        };
        let exhausted = OpportunityKey {
            goal_key: ranked_candidates[0].offer.key,
            anchor: ranked_candidates[0].offer.anchor,
        };
        let exhaustion_cache = BTreeMap::from([(
            exhausted,
            ExhaustionEntry::frontier_exhausted(Vec::new(), crate::ExhaustionBaseline::default()),
        )]);

        let plans = super::build_candidate_plans(
            &world,
            &scheduler,
            agent,
            &ordered(&ranked_candidates),
            None,
            &worldwake_core::DiscrepancyMemory::default(),
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
                    target_place: None,
                    payload_override: None,
                    op_kind: crate::PlannerOpKind::Travel,
                    estimated_ticks: 2,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                    guard: None,
                    expectations: Vec::new(),
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
            PlanTerminalKind::SearchBudgetExhausted {
                budget_consumed: 0,
                budget_total: 0,
            },
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
            PlanTerminalKind::SearchBudgetExhausted {
                budget_consumed: 0,
                budget_total: 0,
            },
        );

        assert!(found_plan_blocks_later_goals(&barrier_plan));
    }

    #[test]
    fn satisfied_and_combat_found_plans_block_later_goals() {
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
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
    fn traced_planning_records_portfolio_led_same_slot_attempt_order() {
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
        // Local-evidence candidate is sorted ahead by `ranking::compare_ranked_goals`
        // under real conditions; pass the list in that pre-sorted order.
        let ranked_candidates = vec![
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(origin),
                BTreeSet::from([bread]),
                BTreeSet::from([origin]),
            )),
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(market),
                BTreeSet::new(),
                BTreeSet::from([market]),
            )),
        ];
        let budget = ProfileFixture {
            snapshot_travel_horizon: 4,
            ..ProfileFixture::default()
        };
        let mut runtime = AgentDecisionRuntime {
            dirty: DirtySet::NO_PLAN,
            ..AgentDecisionRuntime::default()
        };
        let mut agenda_state = AgendaState::default();
        let mut frame = None;
        let mut facility_intents = worldwake_core::ContentionIntents::default();

        let mut event_log = EventLog::new();
        let (_, _, _, plan_search_trace, _, _, _, _, _, _) =
            super::plan_and_validate_next_step_traced(
                &mut world,
                &mut event_log,
                &scheduler,
                &mut runtime,
                &mut agenda_state,
                &mut frame,
                &mut facility_intents,
                agent,
                &ordered(&ranked_candidates),
                &mut worldwake_core::DiscrepancyMemory::default(),
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
                &BTreeMap::new(),
            );

        let plan_search_trace =
            plan_search_trace.expect("traced planning should record attempt order");
        let attempts = &plan_search_trace.attempts;
        assert_eq!(attempts.len(), 2);
        assert_eq!(
            attempts[0].opportunity_anchor,
            OpportunityAnchor::Place(origin)
        );
        assert!(
            !matches!(
                attempts[0].outcome,
                crate::decision_trace::PlanSearchOutcome::Unsupported
            ),
            "the admitted portfolio slot should remain a real search attempt even when it finds no plan"
        );
        assert_eq!(
            attempts[1].opportunity_anchor,
            OpportunityAnchor::Place(market)
        );
        assert!(
            !matches!(
                attempts[1].outcome,
                crate::decision_trace::PlanSearchOutcome::Unsupported
            ),
            "remaining candidate budget should still trace later ranked opportunities"
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
        let sleep_goal = GoalOffer {
            key: GoalKey::from(GoalKind::Sleep),
            anchor: OpportunityAnchor::Place(entity(63)),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };
        let _ranked_candidates = [
            ranked_goal(GoalOffer {
                key: goal,
                anchor: market.anchor,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                motive_sources: Vec::new(),
                acquisition_quantity: None,
            }),
            ranked_goal(GoalOffer {
                key: goal,
                anchor: orchard.anchor,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                motive_sources: Vec::new(),
                acquisition_quantity: None,
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
                &[
                    market,
                    orchard,
                    OpportunityKey {
                        goal_key: sleep_goal.key,
                        anchor: sleep_goal.anchor,
                    }
                ],
                3,
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
        let plans = vec![
            searched_plan(
                OpportunityKey {
                    goal_key: GoalKey::from(GoalKind::Sleep),
                    anchor: OpportunityAnchor::None,
                },
                PlanSearchResult::FrontierExhausted { expansions_used: 1 },
            ),
            searched_plan(
                OpportunityKey {
                    goal_key: GoalKey::from(GoalKind::PostNotice {
                        posting: worldwake_core::ArtifactPostingContext {
                            posting_place: entity(52),
                            issuing_authority: None,
                            expires_at: Some(Tick(5)),
                            jurisdiction: Some(entity(52)),
                        },
                        topic: worldwake_core::NoticeTopic::ThreatWarning { place: entity(52) },
                    }),
                    anchor: OpportunityAnchor::Place(entity(52)),
                },
                PlanSearchResult::FrontierExhausted { expansions_used: 1 },
            ),
        ];
        assert_eq!(
            super::summarize_same_goal_planning_trace(
                &[
                    OpportunityKey {
                        goal_key: GoalKey::from(GoalKind::Sleep),
                        anchor: OpportunityAnchor::None,
                    },
                    OpportunityKey {
                        goal_key: GoalKey::from(GoalKind::PostNotice {
                            posting: worldwake_core::ArtifactPostingContext {
                                posting_place: entity(52),
                                issuing_authority: None,
                                expires_at: Some(Tick(5)),
                                jurisdiction: Some(entity(52)),
                            },
                            topic: worldwake_core::NoticeTopic::ThreatWarning { place: entity(52) },
                        }),
                        anchor: OpportunityAnchor::Place(entity(52)),
                    },
                    OpportunityKey {
                        goal_key: GoalKey::from(GoalKind::SellCommodity {
                            commodity: CommodityKind::Apple,
                        }),
                        anchor: OpportunityAnchor::Place(entity(53)),
                    },
                ],
                2,
                &plans,
            ),
            Some(crate::SameGoalPlanningTrace {
                continuation_trigger: None,
                stop_reason: crate::SameGoalPlanningStopReason::ReachedCandidatePlanCap,
            })
        );
    }

    #[test]
    fn summarize_same_goal_planning_trace_uses_same_portfolio_admission_as_candidate_plans() {
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
        // Order this list as `ranking::compare_ranked_goals` would: the
        // fresh same-place candidate leads, and retry-ready/cooling-down
        // variants trail in sort order. `assemble_portfolio` trusts that
        // sort order, so the test mirrors it here explicitly.
        let ranked_candidates = vec![
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(fresh),
                BTreeSet::new(),
                BTreeSet::from([fresh]),
            )),
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(retry_ready),
                BTreeSet::new(),
                BTreeSet::from([retry_ready]),
            )),
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(cooling_down),
                BTreeSet::new(),
                BTreeSet::from([cooling_down]),
            )),
            ranked_goal(acquire_goal(
                CommodityKind::Bread,
                OpportunityAnchor::Place(frontier),
                BTreeSet::new(),
                BTreeSet::from([frontier]),
            )),
        ];
        // After the reordering above, indices map to:
        //   [0] fresh (no exhaustion entry — always admitted)
        //   [1] retry_ready (BudgetRetry, next_retry=10, eligible at tick 10)
        //   [2] cooling_down (BudgetRetry, next_retry=20, still cooling at tick 10)
        //   [3] frontier (FrontierExhausted — suppressed from planning)
        let exhaustion_cache = BTreeMap::from([
            (
                OpportunityKey {
                    goal_key: ranked_candidates[3].offer.key,
                    anchor: ranked_candidates[3].offer.anchor,
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
                    goal_key: ranked_candidates[2].offer.key,
                    anchor: ranked_candidates[2].offer.anchor,
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
                    goal_key: ranked_candidates[1].offer.key,
                    anchor: ranked_candidates[1].offer.anchor,
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
            max_node_expansions: 0,
            ..ProfileFixture::default()
        };

        let plans = super::build_candidate_plans(
            &world,
            &scheduler,
            agent,
            &ordered(&ranked_candidates),
            None,
            &worldwake_core::DiscrepancyMemory::default(),
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
                    goal_key: ranked_candidates[0].offer.key,
                    anchor: ranked_candidates[0].offer.anchor,
                },
                OpportunityKey {
                    goal_key: ranked_candidates[1].offer.key,
                    anchor: ranked_candidates[1].offer.anchor,
                },
            ]
        );
        assert_eq!(
            super::summarize_same_goal_planning_trace(
                &plans.plausible_opportunities(),
                worldwake_core::PortfolioWeightsProfile::default()
                    .max_plans_for_mode(worldwake_core::OperatingMode::Normal),
                &plans.plans,
            ),
            Some(crate::SameGoalPlanningTrace {
                continuation_trigger: None,
                stop_reason: crate::SameGoalPlanningStopReason::ExhaustedAdmittedOpportunities,
            }),
            "the trace summary should see the same portfolio-admitted opportunities as real planning after exhaustion filtering and same-slot collapse"
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
    fn write_budget_exhausted_partial_plan_segments_suspends_ranked_goal_with_segment() {
        let opportunity = consume_opportunity(CommodityKind::Bread, OpportunityAnchor::None);
        let ranked = vec![ranked_goal_with_score(
            opportunity,
            GoalPriorityClass::Medium,
            50,
        )];
        let skeleton = vec![crate::PlannedSkeletonStep {
            op: PlannerOpKind::Trade,
            target_template: crate::htn::PayloadTemplate::FromContext,
            expected_pre: vec![crate::htn::BeliefPredicate::SellerKnown {
                commodity: crate::htn::CommodityTemplate::Fixed(CommodityKind::Bread),
            }],
        }];
        let plans = vec![searched_plan_with_skeleton(
            opportunity,
            PlanSearchResult::BudgetExhausted {
                expansions_used: 12,
            },
            PartialPlanSkeletonSource {
                remaining_skeleton: skeleton.clone(),
            },
        )];
        let mut cognitive = cognitive(&ProfileFixture {
            max_node_expansions: 99,
            ..ProfileFixture::default()
        });
        cognitive.search_exhaustion_backoff_ticks = 7;
        let mut agenda_state = AgendaState::default();
        agenda_state
            .pending
            .insert(ranked[0].key, ranked[0].clone());

        let written = write_budget_exhausted_partial_plan_segments(
            &mut agenda_state,
            &ordered(&ranked),
            &plans,
            Tick(30),
            &cognitive,
        );

        assert_eq!(written, 1);
        assert!(!agenda_state.pending.contains_key(&opportunity));
        let suspended = agenda_state
            .suspended
            .get(&opportunity)
            .expect("budget-exhausted goal should be suspended with a segment");
        assert_eq!(suspended.phase, AgendaPhase::Suspended);
        let segment = suspended.partial_plan_segment.as_ref().unwrap();
        assert_eq!(segment.goal, ranked[0].offer);
        assert_eq!(
            segment.terminal_barrier,
            PlanTerminalKind::SearchBudgetExhausted {
                budget_consumed: 12,
                budget_total: 99,
            }
        );
        assert_eq!(
            segment.resume_conditions,
            vec![worldwake_core::IntentionResumeCondition::TickElapsed(7)]
        );
        assert_eq!(
            segment.abandon_conditions,
            vec![worldwake_core::IntentionAbandonCondition::PatienceExhausted]
        );
        assert_eq!(segment.remaining_skeleton, Some(skeleton));
    }

    #[test]
    fn write_information_barrier_partial_plan_segment_suspends_selected_goal_with_skeleton() {
        let subject = place_entity(42);
        let opportunity = consume_opportunity(CommodityKind::Bread, OpportunityAnchor::None);
        let ranked = vec![ranked_goal_with_score(
            opportunity,
            GoalPriorityClass::Medium,
            50,
        )];
        let skeleton = vec![crate::PlannedSkeletonStep {
            op: PlannerOpKind::Trade,
            target_template: crate::htn::PayloadTemplate::FromContext,
            expected_pre: vec![crate::htn::BeliefPredicate::TargetLastSeenKnown {
                target: crate::htn::EntityTemplate::Fixed(subject),
            }],
        }];
        let selected_plan = information_barrier_plan(opportunity, subject);
        let plans = vec![searched_plan_with_skeleton(
            opportunity,
            PlanSearchResult::Found(Box::new(selected_plan.clone())),
            PartialPlanSkeletonSource {
                remaining_skeleton: skeleton.clone(),
            },
        )];
        let mut agenda_state = AgendaState::default();
        agenda_state
            .pending
            .insert(ranked[0].key, ranked[0].clone());

        let written = write_information_barrier_partial_plan_segment(
            &mut agenda_state,
            &ordered(&ranked),
            &selected_plan,
            &plans,
            Tick(31),
            &CognitiveProfile::default(),
        );

        assert!(written);
        assert!(!agenda_state.pending.contains_key(&opportunity));
        let suspended = agenda_state
            .suspended
            .get(&opportunity)
            .expect("information-barrier goal should be suspended with a segment");
        assert_eq!(suspended.phase, AgendaPhase::Suspended);
        let segment = suspended.partial_plan_segment.as_ref().unwrap();
        assert_eq!(segment.goal, ranked[0].offer);
        assert_eq!(
            segment.terminal_barrier,
            PlanTerminalKind::InformationBarrier {
                topic: TellTopic::EntityBelief { subject },
            }
        );
        assert_eq!(segment.remaining_skeleton, Some(skeleton));
        assert_eq!(
            segment.resume_conditions,
            vec![
                worldwake_core::IntentionResumeCondition::BeliefStatusChanged {
                    subject,
                    target_status: worldwake_core::BeliefStatusTag::Certain,
                }
            ]
        );
    }

    #[test]
    fn write_information_barrier_partial_plan_segment_allows_missing_skeleton_source() {
        let subject = place_entity(42);
        let opportunity = consume_opportunity(CommodityKind::Bread, OpportunityAnchor::None);
        let ranked = vec![ranked_goal_with_score(
            opportunity,
            GoalPriorityClass::Medium,
            50,
        )];
        let selected_plan = information_barrier_plan(opportunity, subject);
        let plans = vec![searched_plan(
            opportunity,
            PlanSearchResult::Found(Box::new(selected_plan.clone())),
        )];
        let mut agenda_state = AgendaState::default();
        agenda_state
            .pending
            .insert(ranked[0].key, ranked[0].clone());

        let written = write_information_barrier_partial_plan_segment(
            &mut agenda_state,
            &ordered(&ranked),
            &selected_plan,
            &plans,
            Tick(31),
            &CognitiveProfile::default(),
        );

        assert!(written);
        let segment = agenda_state
            .suspended
            .get(&opportunity)
            .and_then(|entry| entry.partial_plan_segment.as_ref())
            .expect("information-barrier segment should still be lawful without a source");
        assert_eq!(segment.remaining_skeleton, None);
    }

    #[test]
    fn write_information_barrier_partial_plan_segment_does_not_suspend_ask_witness_companion() {
        let subject = place_entity(42);
        let witness = place_entity(43);
        let opportunity = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::AskWitness {
                witness,
                topic: TellTopic::EntityBelief { subject },
            }),
            anchor: OpportunityAnchor::Entity(witness),
        };
        let ranked = vec![ranked_goal_with_score(
            opportunity,
            GoalPriorityClass::Medium,
            50,
        )];
        let selected_plan = information_barrier_plan(opportunity, subject);
        let plans = vec![searched_plan(
            opportunity,
            PlanSearchResult::Found(Box::new(selected_plan.clone())),
        )];
        let mut agenda_state = AgendaState::default();
        agenda_state
            .pending
            .insert(ranked[0].key, ranked[0].clone());

        let written = write_information_barrier_partial_plan_segment(
            &mut agenda_state,
            &ordered(&ranked),
            &selected_plan,
            &plans,
            Tick(31),
            &CognitiveProfile::default(),
        );

        assert!(!written);
        assert!(agenda_state.pending.contains_key(&opportunity));
        assert!(!agenda_state.suspended.contains_key(&opportunity));
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
    fn frontier_exhaustion_entry_uses_strategy_dispatch() {
        let cognitive = cognitive(&ProfileFixture::default());

        let cooldown = frontier_exhaustion_entry(
            &GoalKind::Sleep,
            Vec::new(),
            crate::ExhaustionBaseline::default(),
            Tick(9),
            &cognitive,
        );
        assert_eq!(
            GoalDispatchKey::from_goal_kind(&GoalKind::Sleep)
                .declaration()
                .frontier_exhaustion_strategy,
            FrontierExhaustionStrategy::CooldownRetry
        );
        assert_eq!(
            cooldown.retry_state,
            ExhaustionRetryState::BudgetRetryPending
        );
        assert!(cooldown.next_retry_tick.is_some());
        assert!(!cooldown.suppresses_planning());

        let permanent = frontier_exhaustion_entry(
            &GoalKind::Wash,
            Vec::new(),
            crate::ExhaustionBaseline::default(),
            Tick(9),
            &cognitive,
        );
        assert_eq!(
            GoalDispatchKey::from_goal_kind(&GoalKind::Wash)
                .declaration()
                .frontier_exhaustion_strategy,
            FrontierExhaustionStrategy::PermanentUntilInvalidator
        );
        assert_eq!(
            permanent.retry_state,
            ExhaustionRetryState::FrontierExhausted
        );
        assert_eq!(permanent.next_retry_tick, None);
        assert!(permanent.suppresses_planning());
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
    fn record_exhausted_goals_records_patrol_frontier_exhaustion_as_budget_retry() {
        let place = entity(11);
        let goal = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::Patrol { place }),
            anchor: OpportunityAnchor::Place(place),
        };
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
        assert!(entry.next_retry_tick.is_some());
    }

    #[test]
    fn record_exhausted_goals_records_self_consume_acquire_frontier_exhaustion_as_retry() {
        let goal = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            anchor: OpportunityAnchor::Place(place_entity(3)),
        };
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

        assert_eq!(
            tracker_increments,
            BTreeSet::from([
                worldwake_core::HomeostaticNeedId::Thirst,
                worldwake_core::HomeostaticNeedId::Dirtiness,
            ])
        );
        let entry = runtime.exhaustion_cache.get(&goal).unwrap();
        assert_eq!(entry.retry_state, ExhaustionRetryState::BudgetRetryPending);
        assert!(!entry.suppresses_planning());
        assert!(
            entry.next_retry_tick.is_some(),
            "self-consume acquisition frontier exhaustion should retry and feed exploration fallback"
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
            goal_key: ranked_candidates[0].offer.key,
            anchor: ranked_candidates[0].offer.anchor,
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
            &ordered(&ranked_candidates),
            None,
            &worldwake_core::DiscrepancyMemory::default(),
            &worldwake_core::BlockerMemory::default(),
            Tick(10),
            &cognitive(&ProfileFixture {
                snapshot_travel_horizon: 4,
                max_node_expansions: 128,
                ..ProfileFixture::default()
            }),
            &execution_budget(&ProfileFixture {
                snapshot_travel_horizon: 4,
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
                goal_key: ranked_candidates[0].offer.key,
                anchor: ranked_candidates[0].offer.anchor,
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
            &ordered(&ranked_candidates),
            None,
            &worldwake_core::DiscrepancyMemory::default(),
            &worldwake_core::BlockerMemory::default(),
            Tick(10),
            &cognitive(&ProfileFixture {
                snapshot_travel_horizon: 4,
                ..ProfileFixture::default()
            }),
            &execution_budget(&ProfileFixture {
                snapshot_travel_horizon: 4,
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
            goal_key: ranked_candidates[0].offer.key,
            anchor: ranked_candidates[0].offer.anchor,
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
            &ordered(&ranked_candidates),
            None,
            &worldwake_core::DiscrepancyMemory::default(),
            &worldwake_core::BlockerMemory::default(),
            Tick(10),
            &cognitive(&ProfileFixture {
                snapshot_travel_horizon: 4,
                ..ProfileFixture::default()
            }),
            &execution_budget(&ProfileFixture {
                snapshot_travel_horizon: 4,
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
