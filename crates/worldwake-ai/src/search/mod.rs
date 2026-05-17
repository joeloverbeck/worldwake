mod candidates;
mod frontier;
mod heuristic;
pub(crate) mod landmarks;
pub(crate) mod strategic;
mod transition;

use crate::goal_schema::GoalDispatchKeySchemaExt;
use crate::opportunity_compiler::PerceivedOpportunityIndex;
use crate::{
    GoalKindPlannerExt, GoalOffer, PlanTerminalKind, PlannedPlan, PlannedStep, PlannerOpSemantics,
    PlanningEntityRef, PlanningSnapshot, PlanningState, PlanningStateCacheCounters,
    shared_collections::SharedVec,
};
#[cfg(test)]
use candidates::search_candidate_from_planner;
#[cfg(test)]
use candidates::search_candidates;
use candidates::{
    CandidateFilterTraceSinks, CandidateSearchContext, CandidateTraceSinks, CommodityFilterContext,
    SearchCandidate, apply_commodity_relevance_filter_with_expansion_trace, candidate_action_place,
    search_candidates_from_affordance, search_candidates_with_expansion_trace, unsupported_goal,
    update_expansion_candidate_outcome, update_root_candidate_outcome,
};
use frontier::{DualFrontier, FrontierEntry, compare_search_nodes};
use heuristic::combined_relevant_places_with_guidance;
#[cfg(test)]
use heuristic::prune_travel_away_from_goal;
#[cfg(test)]
use heuristic::{combined_relevant_places, root_node};
use heuristic::{
    combined_relevant_places_for_tactical, combined_relevant_places_with_guidance_for_tactical,
    compute_heuristic, compute_landmark_heuristic,
    prune_travel_away_from_goal_with_expansion_trace, root_node_for_tactical,
};
use landmarks::{
    LandmarkSet, PlanningFact, PlanningOperator, RelaxedPlanResult, compute_ff_heuristic,
    extract_landmarks, goal_facts_from_goal, planning_facts_from_state,
    planning_operator_from_transition, preferred_operators,
};
use std::collections::BTreeMap;
#[cfg(test)]
use transition::build_successor;
use transition::build_successor_detailed;
use worldwake_core::{
    ActionDefId, BlockerMemory, CognitiveProfile, CommodityKind, EntityKind, ExecutionBudget,
    GoalDispatchKey, GoalKind, GoalPlanningBudget, OpportunityKey, Tick,
};
use worldwake_sim::{
    ActionDefRegistry, ActionHandlerRegistry, EconomicBeliefView, EntityBeliefView,
    InventoryBeliefView, RecipeRegistry, SpatialBeliefView, get_affordances_for_defs,
};

#[derive(Clone, Debug)]
pub(crate) struct SearchTraceMetadata {
    pub(crate) strategic_plan: Option<strategic::StrategicPlan>,
    pub(crate) strategic_budget: Option<crate::decision_trace::StrategicBudgetTrace>,
    pub(crate) goal_budget: GoalPlanningBudget,
    pub(crate) planning_state_cache_counters: Option<PlanningStateCacheCounters>,
    pub(crate) tactical_goal: Option<String>,
    pub(crate) landmarks_extracted: u16,
    pub(crate) landmark_orderings: u16,
}

impl Default for SearchTraceMetadata {
    fn default() -> Self {
        Self {
            strategic_plan: None,
            strategic_budget: None,
            goal_budget: GoalPlanningBudget::TRAVEL_PURCHASE,
            planning_state_cache_counters: None,
            tactical_goal: None,
            landmarks_extracted: 0,
            landmark_orderings: 0,
        }
    }
}

#[derive(Clone)]
struct SearchNode<'snapshot> {
    state: PlanningState<'snapshot>,
    steps: SharedVec<PlannedStep>,
    total_estimated_ticks: u32,
    search_cost: u32,
    tactical_barrier_reached: bool,
    /// A* heuristic: minimum perceived travel cost from the actor's current simulated
    /// position to the nearest goal-relevant place under the actor's perceived
    /// travel-cost model. Zero when already at a goal-relevant place, when no
    /// spatial guidance is available, or when the actor's place cannot be resolved.
    heuristic_ticks: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum TacticalGoal {
    AcquirePrerequisite {
        commodity: CommodityKind,
        destination: worldwake_core::EntityId,
    },
    Explore {
        destination: worldwake_core::EntityId,
    },
    SocialQuery {
        commodity: CommodityKind,
        destination: worldwake_core::EntityId,
    },
    TravelToGoal {
        destination: worldwake_core::EntityId,
    },
}

impl TacticalGoal {
    fn from_strategic_step(
        goal: &GoalOffer,
        step: Option<&strategic::StrategicStep>,
        snapshot: &PlanningSnapshot,
    ) -> Option<Self> {
        let step = step?;
        match step.sub_goal {
            strategic::TacticalSubGoal::SatisfyGoal => travel_to_goal_supports_tactical_barrier(
                &goal.key.kind,
            )
            .then_some(Self::TravelToGoal {
                destination: step.destination,
            }),
            strategic::TacticalSubGoal::AcquirePrerequisite(commodity) => {
                Some(Self::AcquirePrerequisite {
                    commodity,
                    destination: step.destination,
                })
            }
            strategic::TacticalSubGoal::ExploreWithBarrier => Some(Self::Explore {
                destination: evidence_directed_destination(goal, step, snapshot),
            }),
            strategic::TacticalSubGoal::ExploreFallback => None,
            strategic::TacticalSubGoal::SocialQuery(commodity) => Some(Self::SocialQuery {
                commodity,
                destination: step.destination,
            }),
        }
    }

    fn progress_barrier_satisfied(&self, state: &PlanningState<'_>) -> bool {
        let actor = state.snapshot().actor();
        match self {
            Self::AcquirePrerequisite { commodity, .. } => {
                state.commodity_quantity(actor, *commodity) > worldwake_core::Quantity(0)
            }
            Self::Explore { destination } | Self::TravelToGoal { destination } => {
                state.effective_place(actor) == Some(*destination)
            }
            Self::SocialQuery { .. } => false,
        }
    }

    fn goal_facts(
        &self,
        goal: &GoalOffer,
        state: &PlanningState<'_>,
        recipes: &RecipeRegistry,
    ) -> std::collections::BTreeSet<PlanningFact> {
        match self {
            Self::AcquirePrerequisite { commodity, .. } => {
                std::collections::BTreeSet::from([PlanningFact::HasCommodity(*commodity)])
            }
            Self::Explore { destination } | Self::TravelToGoal { destination } => {
                std::collections::BTreeSet::from([PlanningFact::AtPlace(*destination)])
            }
            Self::SocialQuery { .. } => std::collections::BTreeSet::new(),
        }
        .into_iter()
        .chain(goal_facts_from_goal(goal, state, recipes))
        .collect()
    }
}

fn evidence_directed_destination(
    goal: &GoalOffer,
    step: &strategic::StrategicStep,
    snapshot: &PlanningSnapshot,
) -> worldwake_core::EntityId {
    let state = PlanningState::new(snapshot);
    let actor_place = state.effective_place(snapshot.actor());
    actor_place
        .into_iter()
        .flat_map(|current_place| {
            goal.evidence_places.iter().filter_map(move |place| {
                snapshot
                    .min_perceived_travel_cost_to_any(current_place, &[*place])
                    .map(|cost| (cost, *place))
            })
        })
        .min_by_key(|(cost, place)| (*cost, *place))
        .map_or(step.destination, |(_, place)| place)
}

fn should_fail_fast_for_missing_tactical_goal(
    step: Option<&strategic::StrategicStep>,
    tactical_goal: Option<&TacticalGoal>,
) -> bool {
    step.is_some_and(|step| {
        matches!(
            step.sub_goal,
            strategic::TacticalSubGoal::ExploreWithBarrier
        )
    }) && tactical_goal.is_none()
}

fn travel_to_goal_supports_tactical_barrier(goal: &worldwake_core::GoalKind) -> bool {
    !matches!(goal, worldwake_core::GoalKind::Accuse { .. })
}

fn anchored_place_constrains_execution(goal: &worldwake_core::GoalKind) -> bool {
    matches!(goal, worldwake_core::GoalKind::AcquireCommodity { .. })
}

#[allow(clippy::too_many_arguments, clippy::trivially_copy_pass_by_ref)]
fn apply_ff_heuristic_to_successors<'snapshot>(
    snapshot: &PlanningSnapshot,
    goal: &GoalOffer,
    node: &SearchNode<'snapshot>,
    cognitive: &CognitiveProfile,
    recipes: &RecipeRegistry,
    execution_budget: &ExecutionBudget,
    tactical_goal: Option<&TacticalGoal>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    successor_candidates: &[SearchCandidate],
    successor_operators: &[PlanningOperator],
    successors: &mut [(usize, Option<PlanTerminalKind>, SearchNode<'snapshot>, bool)],
) -> Option<RelaxedPlanResult> {
    if !cognitive.use_ff_heuristic || successor_operators.is_empty() {
        return None;
    }

    let goal_facts = tactical_goal
        .map(|tactical_goal| tactical_goal.goal_facts(goal, &node.state, recipes))
        .unwrap_or_default();
    if goal_facts.is_empty() {
        return None;
    }

    let current_facts = planning_facts_from_state(&node.state);
    let ff_result = compute_ff_heuristic(&current_facts, &goal_facts, successor_operators)?;

    for (index, (_, _, successor, preferred)) in successors.iter_mut().enumerate() {
        let combined_places = combined_relevant_places_for_tactical(
            goal,
            &successor.state,
            recipes,
            execution_budget,
            tactical_goal,
        );
        let spatial_heuristic =
            compute_heuristic(snapshot, &successor.state, &combined_places.places);
        successor.heuristic_ticks = spatial_heuristic.max(ff_result.h_ff);
        let helpful = ff_result.helpful_action_indices.contains(&index);
        *preferred = ff_helpful_candidate_is_preferred(
            snapshot,
            node,
            &successor.state,
            &successor_candidates[index],
            helpful,
            &combined_places.places,
            semantics_table,
        );
    }

    Some(ff_result)
}

fn ff_helpful_candidate_is_preferred(
    snapshot: &PlanningSnapshot,
    node: &SearchNode<'_>,
    successor_state: &PlanningState<'_>,
    candidate: &SearchCandidate,
    helpful: bool,
    goal_relevant_places: &[worldwake_core::EntityId],
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
) -> bool {
    if !helpful {
        return false;
    }
    let is_travel = semantics_table
        .get(&candidate.def_id)
        .is_some_and(|semantics| semantics.op_kind == crate::PlannerOpKind::Travel);
    if !is_travel {
        return true;
    }
    travel_candidate_is_goal_directed(
        snapshot,
        node,
        successor_state,
        goal_relevant_places,
        candidate,
    )
}

fn travel_candidate_is_goal_directed(
    snapshot: &PlanningSnapshot,
    node: &SearchNode<'_>,
    successor_state: &PlanningState<'_>,
    goal_relevant_places: &[worldwake_core::EntityId],
    candidate: &SearchCandidate,
) -> bool {
    if goal_relevant_places.is_empty() {
        return false;
    }
    let Some(current_place) = node.state.effective_place(node.state.snapshot().actor()) else {
        return false;
    };
    let Some(next_place) = successor_state.effective_place(successor_state.snapshot().actor())
    else {
        return false;
    };
    if candidate.authoritative_targets.first().copied() != Some(next_place) {
        return false;
    }
    let current_remaining = snapshot
        .min_perceived_travel_cost_to_any(current_place, goal_relevant_places)
        .unwrap_or(u32::MAX);
    let next_remaining = snapshot
        .min_perceived_travel_cost_to_any(next_place, goal_relevant_places)
        .unwrap_or(u32::MAX);
    next_remaining < current_remaining || goal_relevant_places.contains(&next_place)
}

fn projected_travel_candidate_cost(
    snapshot: &PlanningSnapshot,
    state: &PlanningState<'_>,
    goal_relevant_places: &[worldwake_core::EntityId],
    candidate: &SearchCandidate,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
) -> Option<u32> {
    let is_travel = semantics_table
        .get(&candidate.def_id)
        .is_some_and(|semantics| semantics.op_kind == crate::PlannerOpKind::Travel);
    if !is_travel {
        return None;
    }
    let current_place = state.effective_place(state.snapshot().actor())?;
    let destination = candidate.authoritative_targets.first().copied()?;
    let direct_cost = snapshot.direct_perceived_travel_cost(current_place, destination)?;
    if goal_relevant_places.is_empty() {
        return Some(direct_cost);
    }
    let remaining_cost = snapshot
        .min_perceived_travel_cost_to_any(destination, goal_relevant_places)
        .unwrap_or(u32::MAX);
    Some(direct_cost.saturating_add(remaining_cost))
}

#[allow(clippy::too_many_arguments)]
fn cap_travel_candidates(
    candidates: &mut Vec<SearchCandidate>,
    snapshot: &PlanningSnapshot,
    state: &PlanningState<'_>,
    goal_relevant_places: &[worldwake_core::EntityId],
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    max_travel_candidates_per_expansion: Option<u16>,
    expansion_candidates: &mut Option<&mut Vec<crate::decision_trace::ExpansionCandidateTrace>>,
    root_candidates: &mut Option<&mut Vec<crate::decision_trace::RootCandidateTrace>>,
) {
    let Some(cap) = max_travel_candidates_per_expansion else {
        return;
    };
    let cap_usize = usize::from(cap);
    let mut scored_travel_candidates = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            projected_travel_candidate_cost(
                snapshot,
                state,
                goal_relevant_places,
                candidate,
                semantics_table,
            )
            .map(|cost| {
                (
                    index,
                    cost,
                    candidate.authoritative_targets.first().copied(),
                    candidate.def_id,
                )
            })
        })
        .collect::<Vec<_>>();
    if scored_travel_candidates.len() <= cap_usize {
        return;
    }
    scored_travel_candidates
        .sort_by_key(|(_, cost, destination, def_id)| (*cost, *destination, *def_id));
    let retained_indices = scored_travel_candidates
        .iter()
        .take(cap_usize)
        .map(|(index, _, _, _)| *index)
        .collect::<std::collections::BTreeSet<_>>();
    let original = std::mem::take(candidates);
    let mut retained = Vec::with_capacity(original.len());
    for (index, candidate) in original.into_iter().enumerate() {
        let is_travel = semantics_table
            .get(&candidate.def_id)
            .is_some_and(|semantics| semantics.op_kind == crate::PlannerOpKind::Travel);
        if !is_travel || retained_indices.contains(&index) {
            retained.push(candidate);
            continue;
        }
        update_expansion_candidate_outcome(
            expansion_candidates,
            candidate.expansion_trace_index,
            crate::decision_trace::ExpansionCandidateOutcome::Filtered(
                crate::decision_trace::ExpansionCandidateFilterReason::TravelCandidateCap { cap },
            ),
        );
        update_root_candidate_outcome(
            root_candidates,
            candidate.trace_index,
            crate::decision_trace::RootCandidateOutcome::Filtered(
                crate::decision_trace::RootCandidateFilterReason::TravelCandidateCap { cap },
            ),
        );
    }
    *candidates = retained;
}

/// Outcome of a plan search for one goal.
///
/// Replaces the previous `Option<PlannedPlan>` return type to preserve
/// failure-mode information needed by both diagnostics and tracing.
#[derive(Clone, Debug)]
pub enum PlanSearchResult {
    /// A valid plan was found.
    Found(Box<PlannedPlan>),
    /// Goal kind is not supported by the planner.
    Unsupported,
    /// Node expansion budget was exhausted before finding a plan.
    BudgetExhausted { expansions_used: u16 },
    /// Search frontier was fully explored without finding a plan.
    FrontierExhausted { expansions_used: u16 },
}

impl PlanSearchResult {
    /// Extract the plan if found, discarding failure information.
    #[must_use]
    pub fn into_plan(self) -> Option<PlannedPlan> {
        match self {
            Self::Found(plan) => Some(*plan),
            _ => None,
        }
    }

    /// Returns `true` if a plan was found.
    #[must_use]
    pub fn is_found(&self) -> bool {
        matches!(self, Self::Found(_))
    }
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref
)]
pub fn search_plan(
    snapshot: &PlanningSnapshot,
    goal: &GoalOffer,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    cognitive: &CognitiveProfile,
    execution_budget: &ExecutionBudget,
    recipes: &RecipeRegistry,
    blocked: &BlockerMemory,
    current_tick: Tick,
    binding_rejections: Option<&mut Vec<crate::decision_trace::BindingRejection>>,
    expansion_summaries: Option<&mut Vec<crate::decision_trace::SearchExpansionSummary>>,
) -> PlanSearchResult {
    search_plan_with_trace_metadata(
        snapshot,
        goal,
        semantics_table,
        registry,
        handlers,
        cognitive,
        execution_budget,
        recipes,
        blocked,
        current_tick,
        binding_rejections,
        expansion_summaries,
        None,
    )
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref
)]
pub(crate) fn search_plan_with_trace_metadata(
    snapshot: &PlanningSnapshot,
    goal: &GoalOffer,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    cognitive: &CognitiveProfile,
    execution_budget: &ExecutionBudget,
    recipes: &RecipeRegistry,
    blocked: &BlockerMemory,
    current_tick: Tick,
    binding_rejections: Option<&mut Vec<crate::decision_trace::BindingRejection>>,
    expansion_summaries: Option<&mut Vec<crate::decision_trace::SearchExpansionSummary>>,
    trace_metadata: Option<&mut SearchTraceMetadata>,
) -> PlanSearchResult {
    let opportunity_index = PerceivedOpportunityIndex::default();
    search_plan_with_trace_metadata_and_opportunities(
        snapshot,
        goal,
        semantics_table,
        registry,
        handlers,
        cognitive,
        execution_budget,
        recipes,
        blocked,
        current_tick,
        binding_rejections,
        expansion_summaries,
        trace_metadata,
        &opportunity_index,
    )
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref
)]
pub(crate) fn search_plan_with_trace_metadata_and_opportunities(
    snapshot: &PlanningSnapshot,
    goal: &GoalOffer,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    cognitive: &CognitiveProfile,
    execution_budget: &ExecutionBudget,
    recipes: &RecipeRegistry,
    blocked: &BlockerMemory,
    current_tick: Tick,
    binding_rejections: Option<&mut Vec<crate::decision_trace::BindingRejection>>,
    expansion_summaries: Option<&mut Vec<crate::decision_trace::SearchExpansionSummary>>,
    trace_metadata: Option<&mut SearchTraceMetadata>,
    opportunity_index: &PerceivedOpportunityIndex,
) -> PlanSearchResult {
    search_plan_with_trace_metadata_and_source(
        snapshot,
        goal,
        semantics_table,
        registry,
        handlers,
        cognitive,
        execution_budget,
        recipes,
        blocked,
        current_tick,
        binding_rejections,
        expansion_summaries,
        trace_metadata,
        crate::decision_trace::CandidateSource::Emitter,
        opportunity_index,
    )
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref
)]
pub(crate) fn search_plan_with_trace_metadata_and_source(
    snapshot: &PlanningSnapshot,
    goal: &GoalOffer,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    cognitive: &CognitiveProfile,
    execution_budget: &ExecutionBudget,
    recipes: &RecipeRegistry,
    blocked: &BlockerMemory,
    current_tick: Tick,
    mut binding_rejections: Option<&mut Vec<crate::decision_trace::BindingRejection>>,
    mut expansion_summaries: Option<&mut Vec<crate::decision_trace::SearchExpansionSummary>>,
    mut trace_metadata: Option<&mut SearchTraceMetadata>,
    candidate_source: crate::decision_trace::CandidateSource,
    opportunity_index: &PerceivedOpportunityIndex,
) -> PlanSearchResult {
    if unsupported_goal(&goal.key.kind) {
        return PlanSearchResult::Unsupported;
    }
    let opportunity = OpportunityKey {
        goal_key: goal.key,
        anchor: goal.anchor,
    };

    // Pre-compute goal-relevant action defs once — invariant across expansions.
    let relevant_defs = candidates::relevant_action_defs(goal, semantics_table);

    let schema_budget = schema_budget_for_goal(&goal.key.kind);
    let strategic_result = strategic::plan_with_budget_trace(
        snapshot,
        goal,
        execution_budget,
        schema_budget.max_strategic_expansions,
        recipes,
    );
    let effective_budget = compose_effective_goal_budget(
        schema_budget,
        cognitive,
        *execution_budget,
        strategic_result.stages_count,
    );
    let mut trace_state = SearchTraceMetadata {
        strategic_plan: strategic_result
            .plan
            .as_ref()
            .filter(|plan| !plan.steps.is_empty())
            .cloned(),
        strategic_budget: strategic_result.budget_trace,
        goal_budget: effective_budget,
        ..SearchTraceMetadata::default()
    };
    let first_strategic_step = strategic_result
        .plan
        .as_ref()
        .and_then(|plan| plan.steps.first());
    let tactical_goal = TacticalGoal::from_strategic_step(goal, first_strategic_step, snapshot);
    trace_state.tactical_goal = tactical_goal.as_ref().map(|tg| format!("{tg:?}"));
    if should_fail_fast_for_missing_tactical_goal(first_strategic_step, tactical_goal.as_ref()) {
        if let Some(ref mut meta) = trace_metadata {
            **meta = trace_state;
        }
        return PlanSearchResult::FrontierExhausted { expansions_used: 0 };
    }
    let root = root_node_for_tactical(
        snapshot,
        goal,
        recipes,
        execution_budget,
        tactical_goal.as_ref(),
    );
    let cache_counter_source = root.state.clone();
    let mut frontier = DualFrontier::new(execution_budget.preferred_operator_boost());
    frontier.push_regular(FrontierEntry::new(root));
    let mut landmark_set = LandmarkSet::empty();
    let mut expansions = 0u16;
    let mut best_barrier: Option<PlannedPlan> = None;

    while let Some(node) = frontier.pop() {
        if root_goal_satisfaction_allowed(goal, &node.state)
            && goal.key.kind.is_satisfied(&node.state)
        {
            trace_state.planning_state_cache_counters = Some(cache_counter_source.cache_counters());
            if let Some(ref mut meta) = trace_metadata {
                **meta = trace_state.clone();
            }
            return PlanSearchResult::Found(
                PlannedPlan::new(
                    opportunity,
                    goal.key,
                    node.steps.into_vec(),
                    PlanTerminalKind::GoalSatisfied,
                )
                .into(),
            );
        }
        let active_tactical_goal = if node.tactical_barrier_reached {
            None
        } else {
            tactical_goal
                .as_ref()
                .filter(|goal| !goal.progress_barrier_satisfied(&node.state))
        };
        if node.steps.len() >= usize::from(effective_budget.max_depth) {
            continue;
        }
        if expansions >= effective_budget.max_node_expansions {
            trace_state.planning_state_cache_counters = Some(cache_counter_source.cache_counters());
            if let Some(ref mut meta) = trace_metadata {
                **meta = trace_state.clone();
            }
            if let Some(barrier_plan) = best_barrier {
                return PlanSearchResult::Found(Box::new(barrier_plan));
            }
            return PlanSearchResult::BudgetExhausted {
                expansions_used: expansions,
            };
        }
        expansions = expansions.saturating_add(1);

        let depth = node.steps.len() as u8;
        let record_root_candidates = depth == 0 && expansion_summaries.is_some();
        let mut root_candidates = Vec::new();
        let mut root_omissions = Vec::new();
        let mut expansion_candidates = Vec::new();
        let mut candidates = search_candidates_with_expansion_trace(
            goal,
            &node,
            CandidateSearchContext {
                semantics_table,
                registry,
                handlers,
                blocked,
                current_tick,
                relevant_defs: &relevant_defs,
                candidate_source,
            },
            CandidateTraceSinks {
                binding_rejections: binding_rejections.as_deref_mut(),
                expansion_candidates: Some(&mut expansion_candidates),
                root_candidates: record_root_candidates.then_some(&mut root_candidates),
                root_omissions: record_root_candidates.then_some(&mut root_omissions),
            },
        );
        if let Some(extra_candidates) = social_query_candidates(
            goal,
            &node,
            semantics_table,
            registry,
            handlers,
            active_tactical_goal,
        ) {
            candidates.extend(extra_candidates.into_iter().map(|mut candidate| {
                candidate.expansion_trace_index =
                    crate::search::candidates::push_expansion_candidate_trace(
                        &mut Some(&mut expansion_candidates),
                        crate::search::candidates::expansion_candidate_trace_from_candidate(
                            &candidate,
                            registry,
                            semantics_table,
                        ),
                    );
                candidate
            }));
        }
        apply_commodity_relevance_filter_with_expansion_trace(
            &mut candidates,
            goal,
            &node.state,
            CommodityFilterContext {
                tactical_goal: active_tactical_goal,
                semantics_table,
                registry,
                recipes,
            },
            CandidateFilterTraceSinks {
                expansion_candidates: Some(&mut expansion_candidates),
                root_candidates: record_root_candidates.then_some(&mut root_candidates),
            },
        );
        apply_tactical_candidate_filter_with_expansion_trace(
            &mut candidates,
            Some(&mut expansion_candidates),
            goal,
            &node,
            semantics_table,
            active_tactical_goal,
        );
        // NOTE: Futile root expansion detection was considered here but
        // deferred. When only Travel candidates survive at root, the planner
        // may still find affordances at destination locations (e.g., pick_up
        // at a remote pantry). Synthesis-level omissions alone cannot
        // distinguish "will never work" from "works after travel." The
        // recipe knowledge fix in Task 1 is the primary mitigation.
        let combined_places = if expansion_summaries.is_some() {
            combined_relevant_places_with_guidance_for_tactical(
                goal,
                &node.state,
                recipes,
                execution_budget,
                active_tactical_goal,
            )
        } else {
            combined_relevant_places_for_tactical(
                goal,
                &node.state,
                recipes,
                execution_budget,
                active_tactical_goal,
            )
        };
        let summary_places = expansion_summaries.as_ref().map(|_| {
            combined_relevant_places_with_guidance(goal, &node.state, recipes, execution_budget)
        });
        let mut travel_pruning = None;
        if let Some(current_place) =
            node.state
                .effective_place_ref(PlanningEntityRef::Authoritative(
                    node.state.snapshot().actor(),
                ))
        {
            travel_pruning = prune_travel_away_from_goal_with_expansion_trace(
                &mut candidates,
                Some(&mut expansion_candidates),
                current_place,
                &combined_places.places,
                snapshot,
                semantics_table,
                cognitive.detour_budget_permille,
                opportunity_index,
            );
        }
        let mut expansion_trace_sink = Some(&mut expansion_candidates);
        let mut root_candidate_sink = Some(&mut root_candidates);
        cap_travel_candidates(
            &mut candidates,
            snapshot,
            &node.state,
            &combined_places.places,
            semantics_table,
            cognitive.max_travel_candidates_per_expansion,
            &mut expansion_trace_sink,
            &mut root_candidate_sink,
        );

        let candidates_generated = candidates.len() as u16;
        if candidates_generated > cognitive.max_candidates_per_expansion {
            trace_state.planning_state_cache_counters = Some(cache_counter_source.cache_counters());
            if let Some(ref mut meta) = trace_metadata {
                **meta = trace_state.clone();
            }
            if let Some(barrier_plan) = best_barrier {
                return PlanSearchResult::Found(Box::new(barrier_plan));
            }
            return PlanSearchResult::BudgetExhausted {
                expansions_used: expansions,
            };
        }

        let mut terminal_successors = Vec::new();
        let mut successor_candidates = Vec::new();
        let mut successor_operators = Vec::new();
        let mut successors = Vec::new();
        let mut candidates_skipped = 0u16;
        for candidate in candidates {
            let Some(trace_index) = candidate.expansion_trace_index else {
                unreachable!(
                    "expansion candidate trace should exist before successor construction"
                );
            };
            let (terminal, successor, payload_status) = match build_successor_detailed(
                goal,
                semantics_table,
                registry,
                &node,
                &candidate,
                recipes,
                cognitive,
                execution_budget,
                active_tactical_goal,
                &landmark_set,
            ) {
                Ok(result) => result,
                Err(reason) => {
                    expansion_candidates[trace_index].outcome =
                        crate::decision_trace::ExpansionCandidateOutcome::Skipped(reason);
                    candidates_skipped += 1;
                    if let Some(trace_index) = candidate.trace_index
                        && let Some(trace) = root_candidates.get_mut(trace_index)
                    {
                        trace.outcome =
                            crate::decision_trace::RootCandidateOutcome::Skipped(reason);
                    }
                    continue;
                }
            };
            expansion_candidates[trace_index].payload_status = payload_status;
            if let Some(trace_index) = candidate.trace_index
                && let Some(trace) = root_candidates.get_mut(trace_index)
            {
                trace.payload_status = payload_status;
            }
            if let Some(terminal_kind) = terminal {
                expansion_candidates[trace_index].outcome =
                    crate::decision_trace::ExpansionCandidateOutcome::Terminal { terminal_kind };
                terminal_successors.push((trace_index, terminal_kind, successor));
            } else {
                successor_candidates.push(candidate.clone());
                successor_operators.push(planning_operator_from_transition(
                    &node.state,
                    &successor.state,
                ));
                successors.push((trace_index, terminal, successor, false));
            }
        }

        let terminal_count = terminal_successors.len() as u16;
        let non_terminal_before_beam = successors.len() as u16;
        let ff_result = apply_ff_heuristic_to_successors(
            snapshot,
            goal,
            &node,
            cognitive,
            recipes,
            execution_budget,
            active_tactical_goal,
            semantics_table,
            &successor_candidates,
            &successor_operators,
            &mut successors,
        );

        let mut found_goal_satisfied = false;
        if !terminal_successors.is_empty() {
            // Sort by cost so the best candidate of each kind is first.
            terminal_successors.sort_by(|left, right| compare_search_nodes(&left.2, &right.2));

            for (_, terminal_kind, successor) in terminal_successors {
                match terminal_kind {
                    // GoalSatisfied and CombatCommitment are returned immediately.
                    PlanTerminalKind::GoalSatisfied | PlanTerminalKind::CombatCommitment => {
                        found_goal_satisfied =
                            matches!(terminal_kind, PlanTerminalKind::GoalSatisfied);
                        if let Some(ref mut sink) = expansion_summaries {
                            let landmark_heuristic = if landmark_set.landmarks.is_empty() {
                                0
                            } else {
                                compute_landmark_heuristic(
                                    &landmark_set,
                                    &planning_facts_from_state(&node.state),
                                )
                            };
                            sink.push(crate::decision_trace::SearchExpansionSummary {
                                depth,
                                remaining_travel_ticks: node.heuristic_ticks,
                                combined_places_count: summary_places
                                    .as_ref()
                                    .map_or(combined_places.places.len() as u16, |summary| {
                                        summary.places.len() as u16
                                    }),
                                prerequisite_places_count: summary_places
                                    .as_ref()
                                    .map_or(combined_places.prerequisite_places_count, |summary| {
                                        summary.prerequisite_places_count
                                    }),
                                candidates_generated,
                                candidates_skipped,
                                terminal_successors: terminal_count,
                                non_terminal_before_beam,
                                non_terminal_after_beam: non_terminal_before_beam, // no truncation happened yet
                                found_goal_satisfied,
                                preferred_candidates: 0,
                                landmark_heuristic,
                                ff_heuristic: ff_result.as_ref().map(|result| result.h_ff),
                                helpful_action_count: ff_result
                                    .as_ref()
                                    .map_or(0, |result| result.helpful_action_indices.len() as u16),
                                travel_pruning: travel_pruning.clone(),
                                prerequisite_guidance: summary_places
                                    .as_ref()
                                    .and_then(|summary| summary.guidance_trace.clone()),
                                expansion_candidates: expansion_candidates.clone(),
                                root_candidates: root_candidates.clone(),
                                root_omissions: root_omissions.clone(),
                            });
                        }
                        if let Some(ref mut meta) = trace_metadata {
                            trace_state.planning_state_cache_counters =
                                Some(cache_counter_source.cache_counters());
                            **meta = trace_state.clone();
                        }
                        return PlanSearchResult::Found(
                            PlannedPlan::new(
                                opportunity,
                                goal.key,
                                successor.steps.into_vec(),
                                terminal_kind,
                            )
                            .into(),
                        );
                    }
                    // ProgressBarrier is stored as a fallback — keep searching
                    // for a GoalSatisfied plan across deeper expansion levels.
                    PlanTerminalKind::ProgressBarrier => {
                        if best_barrier.is_none() {
                            best_barrier = Some(PlannedPlan::new(
                                opportunity,
                                goal.key,
                                successor.steps.into_vec(),
                                terminal_kind,
                            ));
                        }
                    }
                }
            }
        }
        if landmark_set.landmarks.is_empty()
            && tactical_goal.is_some()
            && cognitive.landmark_extraction_depth > 0
        {
            let initial_facts = planning_facts_from_state(&node.state);
            let goal_facts = tactical_goal
                .as_ref()
                .map(|tactical_goal| tactical_goal.goal_facts(goal, &node.state, recipes))
                .unwrap_or_default();
            if !goal_facts.is_empty() {
                landmark_set = extract_landmarks(
                    &initial_facts,
                    &goal_facts,
                    &successor_operators,
                    cognitive.landmark_extraction_depth,
                );
                trace_state.landmarks_extracted =
                    landmark_set.landmarks.len().min(usize::from(u16::MAX)) as u16;
                trace_state.landmark_orderings =
                    landmark_set.orderings.len().min(usize::from(u16::MAX)) as u16;
            }
        }

        if ff_result.is_none() && !landmark_set.landmarks.is_empty() && !successors.is_empty() {
            let current_facts = planning_facts_from_state(&node.state);
            let preferred_indices = preferred_operators(
                &landmark_set,
                &current_facts,
                &successor_candidates,
                &successor_operators,
            );
            for (index, (_, _, _, preferred)) in successors.iter_mut().enumerate() {
                *preferred = preferred_indices.contains(&index);
            }
        }

        successors.sort_by(|left, right| compare_search_nodes(&left.2, &right.2));
        let retained_len = successors
            .len()
            .min(usize::from(execution_budget.beam_width()));
        for (index, (trace_index, _, _, preferred)) in successors.iter().enumerate() {
            expansion_candidates[*trace_index].outcome = if index < retained_len {
                crate::decision_trace::ExpansionCandidateOutcome::RetainedNonTerminal {
                    preferred: *preferred,
                }
            } else {
                crate::decision_trace::ExpansionCandidateOutcome::PrunedByBeam {
                    preferred: *preferred,
                }
            };
        }
        successors.truncate(retained_len);

        let non_terminal_after_beam = successors.len() as u16;
        let preferred_candidates = successors
            .iter()
            .filter(|(_, _, _, preferred)| *preferred)
            .count();
        let preferred_candidates = preferred_candidates.min(usize::from(u16::MAX)) as u16;
        let landmark_heuristic = if landmark_set.landmarks.is_empty() {
            0
        } else {
            compute_landmark_heuristic(&landmark_set, &planning_facts_from_state(&node.state))
        };

        if let Some(ref mut sink) = expansion_summaries {
            sink.push(crate::decision_trace::SearchExpansionSummary {
                depth,
                remaining_travel_ticks: node.heuristic_ticks,
                combined_places_count: summary_places
                    .as_ref()
                    .map_or(combined_places.places.len() as u16, |summary| {
                        summary.places.len() as u16
                    }),
                prerequisite_places_count: summary_places
                    .as_ref()
                    .map_or(combined_places.prerequisite_places_count, |summary| {
                        summary.prerequisite_places_count
                    }),
                candidates_generated,
                candidates_skipped,
                terminal_successors: terminal_count,
                non_terminal_before_beam,
                non_terminal_after_beam,
                found_goal_satisfied,
                preferred_candidates,
                landmark_heuristic,
                ff_heuristic: ff_result.as_ref().map(|result| result.h_ff),
                helpful_action_count: ff_result
                    .as_ref()
                    .map_or(0, |result| result.helpful_action_indices.len() as u16),
                travel_pruning,
                prerequisite_guidance: summary_places.and_then(|summary| summary.guidance_trace),
                expansion_candidates,
                root_candidates,
                root_omissions,
            });
        }
        if successors.iter().any(|(_, _, _, preferred)| *preferred) {
            frontier.trigger_boost();
        }
        for (_, terminal, successor, preferred) in successors {
            if let Some(terminal_kind) = terminal {
                trace_state.planning_state_cache_counters =
                    Some(cache_counter_source.cache_counters());
                if let Some(ref mut meta) = trace_metadata {
                    **meta = trace_state.clone();
                }
                return PlanSearchResult::Found(
                    PlannedPlan::new(
                        opportunity,
                        goal.key,
                        successor.steps.into_vec(),
                        terminal_kind,
                    )
                    .into(),
                );
            }
            let entry = FrontierEntry::new(successor);
            if preferred {
                frontier.push_preferred(entry);
            } else {
                frontier.push_regular(entry);
            }
        }
    }

    if let Some(barrier_plan) = best_barrier {
        trace_state.planning_state_cache_counters = Some(cache_counter_source.cache_counters());
        if let Some(ref mut meta) = trace_metadata {
            **meta = trace_state;
        }
        return PlanSearchResult::Found(Box::new(barrier_plan));
    }
    trace_state.planning_state_cache_counters = Some(cache_counter_source.cache_counters());
    if let Some(ref mut meta) = trace_metadata {
        **meta = trace_state;
    }
    PlanSearchResult::FrontierExhausted {
        expansions_used: expansions,
    }
}

fn schema_budget_for_goal(goal: &GoalKind) -> GoalPlanningBudget {
    GoalDispatchKey::from_goal_kind(goal)
        .declaration()
        .planning_budget
}

fn compose_effective_goal_budget(
    schema_budget: GoalPlanningBudget,
    cognitive: &CognitiveProfile,
    execution_budget: ExecutionBudget,
    strategic_stage_count: u16,
) -> GoalPlanningBudget {
    let strategic_expansion_ceiling =
        execution_budget.strategic_budget_for_stages(usize::from(strategic_stage_count));
    GoalPlanningBudget {
        max_depth: schema_budget.max_depth.min(cognitive.max_plan_depth),
        max_node_expansions: schema_budget
            .max_node_expansions
            .min(cognitive.max_node_expansions),
        repair_budget_fraction: schema_budget.repair_budget_fraction,
        max_strategic_expansions: schema_budget
            .max_strategic_expansions
            .min(u16::try_from(strategic_expansion_ceiling).unwrap_or(u16::MAX)),
    }
}

fn root_goal_satisfaction_allowed(goal: &GoalOffer, state: &PlanningState<'_>) -> bool {
    match &goal.key.kind {
        worldwake_core::GoalKind::AcquireCommodity {
            purpose: worldwake_core::CommodityPurpose::SelfConsume,
            ..
        } => false,
        worldwake_core::GoalKind::SellCommodity { commodity } => {
            let actor = state.snapshot().actor();
            let Some(place) = state.effective_place(actor) else {
                return true;
            };
            !goal.evidence_entities.iter().any(|entity| {
                state.entity_kind(*entity) == Some(EntityKind::ItemLot)
                    && state.item_lot_commodity(*entity) == Some(*commodity)
                    && state
                        .local_controlled_lots_for(actor, place, *commodity)
                        .contains(entity)
                    && !state.has_sale_listing(*entity)
            })
        }
        _ => true,
    }
}

fn social_query_candidates(
    goal: &GoalOffer,
    node: &SearchNode<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    tactical_goal: Option<&TacticalGoal>,
) -> Option<Vec<SearchCandidate>> {
    let TacticalGoal::SocialQuery { destination, .. } = tactical_goal? else {
        return None;
    };
    if node.state.effective_place(node.state.snapshot().actor()) != Some(*destination) {
        return None;
    }
    let ask_witness_defs = semantics_table
        .iter()
        .filter_map(|(def_id, semantics)| {
            (semantics.op_kind == crate::PlannerOpKind::AskWitness).then_some(*def_id)
        })
        .collect();
    Some(
        get_affordances_for_defs(
            &node.state,
            node.state.snapshot().actor(),
            registry,
            handlers,
            &ask_witness_defs,
        )
        .into_iter()
        .flat_map(|affordance| {
            search_candidates_from_affordance(goal, &node.state, registry, handlers, &affordance)
        })
        .collect(),
    )
}

#[cfg(test)]
fn apply_tactical_candidate_filter(
    candidates: &mut Vec<SearchCandidate>,
    goal: &GoalOffer,
    node: &SearchNode<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    tactical_goal: Option<&TacticalGoal>,
) {
    apply_tactical_candidate_filter_with_expansion_trace(
        candidates,
        None,
        goal,
        node,
        semantics_table,
        tactical_goal,
    );
}

fn apply_tactical_candidate_filter_with_expansion_trace(
    candidates: &mut Vec<SearchCandidate>,
    expansion_candidates: Option<&mut Vec<crate::decision_trace::ExpansionCandidateTrace>>,
    goal: &GoalOffer,
    node: &SearchNode<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    tactical_goal: Option<&TacticalGoal>,
) {
    let actor_place = node.state.effective_place(node.state.snapshot().actor());
    let anchored_place = match goal.anchor {
        worldwake_core::OpportunityAnchor::Place(place) => Some(place),
        worldwake_core::OpportunityAnchor::Entity(entity) => node.state.effective_place(entity),
        worldwake_core::OpportunityAnchor::None => None,
    };
    let Some(tactical_goal) = tactical_goal else {
        if !anchored_place_constrains_execution(&goal.key.kind) {
            return;
        }
        let Some(anchor_place) = anchored_place else {
            return;
        };
        let mut expansion_candidates = expansion_candidates;
        candidates.retain(|candidate| {
            let keep = if actor_place == Some(anchor_place) {
                candidate_action_place(candidate, node, semantics_table) == Some(anchor_place)
            } else {
                travel_advances_toward_destination(node, candidate, semantics_table, anchor_place)
            };
            if !keep {
                crate::search::candidates::update_expansion_candidate_outcome(
                    &mut expansion_candidates,
                    candidate.expansion_trace_index,
                    crate::decision_trace::ExpansionCandidateOutcome::Filtered(
                        crate::decision_trace::ExpansionCandidateFilterReason::TacticalGoalMismatch,
                    ),
                );
            }
            keep
        });
        return;
    };
    let mut expansion_candidates = expansion_candidates;
    candidates.retain(|candidate| {
        let keep = match tactical_goal {
            TacticalGoal::AcquirePrerequisite { destination, .. } => {
                if actor_place == Some(*destination) {
                    semantics_table
                        .get(&candidate.def_id)
                        .is_none_or(|semantics| semantics.op_kind != crate::PlannerOpKind::Travel)
                } else {
                    travel_advances_toward_destination(
                        node,
                        candidate,
                        semantics_table,
                        *destination,
                    )
                }
            }
            TacticalGoal::Explore { destination } => {
                if actor_place == Some(*destination) {
                    false
                } else {
                    travel_advances_toward_destination(
                        node,
                        candidate,
                        semantics_table,
                        *destination,
                    )
                }
            }
            TacticalGoal::SocialQuery { destination, .. } => {
                if actor_place == Some(*destination) {
                    semantics_table
                        .get(&candidate.def_id)
                        .is_some_and(|semantics| {
                            semantics.op_kind == crate::PlannerOpKind::AskWitness
                        })
                } else {
                    travel_advances_toward_destination(
                        node,
                        candidate,
                        semantics_table,
                        *destination,
                    )
                }
            }
            TacticalGoal::TravelToGoal { destination } => {
                if actor_place == Some(*destination) {
                    semantics_table
                        .get(&candidate.def_id)
                        .is_none_or(|semantics| semantics.op_kind != crate::PlannerOpKind::Travel)
                } else if node.steps.is_empty()
                    && semantics_table
                        .get(&candidate.def_id)
                        .is_some_and(|semantics| {
                            semantics.op_kind != crate::PlannerOpKind::Travel
                                && goal
                                    .key
                                    .kind
                                    .relevant_op_kinds()
                                    .contains(&semantics.op_kind)
                        })
                {
                    true
                } else {
                    travel_advances_toward_destination(
                        node,
                        candidate,
                        semantics_table,
                        *destination,
                    )
                }
            }
        };
        if !keep {
            crate::search::candidates::update_expansion_candidate_outcome(
                &mut expansion_candidates,
                candidate.expansion_trace_index,
                crate::decision_trace::ExpansionCandidateOutcome::Filtered(
                    crate::decision_trace::ExpansionCandidateFilterReason::TacticalGoalMismatch,
                ),
            );
        }
        keep
    });
}

fn travel_advances_toward_destination(
    node: &SearchNode<'_>,
    candidate: &SearchCandidate,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    destination: worldwake_core::EntityId,
) -> bool {
    let Some(semantics) = semantics_table.get(&candidate.def_id) else {
        return false;
    };
    if semantics.op_kind != crate::PlannerOpKind::Travel {
        return false;
    }
    let Some(current_place) = node.state.effective_place(node.state.snapshot().actor()) else {
        return false;
    };
    let Some(next_place) = candidate.authoritative_targets.first().copied() else {
        return false;
    };
    let current_remaining = node
        .state
        .snapshot()
        .min_perceived_travel_cost_to_any(current_place, &[destination])
        .unwrap_or(u32::MAX);
    let next_remaining = node
        .state
        .snapshot()
        .min_perceived_travel_cost_to_any(next_place, &[destination])
        .unwrap_or(u32::MAX);
    next_remaining < current_remaining
}

#[cfg(test)]
mod tests;
