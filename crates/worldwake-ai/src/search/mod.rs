mod candidates;
mod frontier;
mod heuristic;
mod transition;

use crate::{
    shared_collections::SharedVec, GoalKindPlannerExt, GroundedGoal, PlanTerminalKind, PlannedPlan,
    PlannedStep, PlannerOpSemantics, PlanningBudget, PlanningEntityRef, PlanningSnapshot,
    PlanningState,
};
use candidates::{
    root_candidate_payload_status, search_candidates, unsupported_goal, SearchCandidate,
};
#[cfg(test)]
use candidates::{search_candidate_from_planner, search_candidates_from_affordance};
use frontier::{compare_search_nodes, FrontierEntry};
#[cfg(test)]
use heuristic::compute_heuristic;
use heuristic::{combined_relevant_places, prune_travel_away_from_goal, root_node};
use std::collections::{BTreeMap, BinaryHeap};
#[cfg(test)]
use transition::build_successor;
use transition::build_successor_detailed;
use worldwake_core::{ActionDefId, BlockedIntentMemory, OpportunityKey, Tick};
use worldwake_sim::{ActionDefRegistry, ActionHandlerRegistry, RecipeRegistry};

#[derive(Clone)]
struct SearchNode<'snapshot> {
    state: PlanningState<'snapshot>,
    steps: SharedVec<PlannedStep>,
    total_estimated_ticks: u32,
    search_cost: u32,
    /// A* heuristic: minimum perceived travel cost from the actor's current simulated
    /// position to the nearest goal-relevant place under the actor's perceived
    /// travel-cost model. Zero when already at a goal-relevant place, when no
    /// spatial guidance is available, or when the actor's place cannot be resolved.
    heuristic_ticks: u32,
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

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn search_plan(
    snapshot: &PlanningSnapshot,
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    budget: &PlanningBudget,
    recipes: &RecipeRegistry,
    blocked: &BlockedIntentMemory,
    current_tick: Tick,
    mut binding_rejections: Option<&mut Vec<crate::decision_trace::BindingRejection>>,
    mut expansion_summaries: Option<&mut Vec<crate::decision_trace::SearchExpansionSummary>>,
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

    let mut frontier = BinaryHeap::new();
    frontier.push(FrontierEntry::new(root_node(
        snapshot, goal, recipes, budget,
    )));
    let mut expansions = 0u16;
    let mut best_barrier: Option<PlannedPlan> = None;

    while let Some(node) = frontier.pop().map(FrontierEntry::into_node) {
        if goal.key.kind.is_satisfied(&node.state) {
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
        if node.steps.len() >= usize::from(budget.max_plan_depth) {
            continue;
        }
        if expansions >= budget.max_node_expansions {
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

        let mut candidates = search_candidates(
            goal,
            &node,
            semantics_table,
            registry,
            handlers,
            blocked,
            current_tick,
            binding_rejections.as_deref_mut(),
            record_root_candidates.then_some(&mut root_candidates),
            record_root_candidates.then_some(&mut root_omissions),
            &relevant_defs,
        );
        let combined_places = combined_relevant_places(goal, &node.state, recipes, budget);
        let mut travel_pruning = None;
        if let Some(current_place) =
            node.state
                .effective_place_ref(PlanningEntityRef::Authoritative(
                    node.state.snapshot().actor(),
                ))
        {
            travel_pruning = prune_travel_away_from_goal(
                &mut candidates,
                current_place,
                &combined_places.places,
                snapshot,
                semantics_table,
            );
        }

        let candidates_generated = candidates.len() as u16;

        let mut terminal_successors = Vec::new();
        let mut successors = Vec::new();
        let mut candidates_skipped = 0u16;
        for candidate in candidates {
            let (terminal, successor) = match build_successor_detailed(
                goal,
                semantics_table,
                registry,
                &node,
                &candidate,
                recipes,
                budget,
            ) {
                Ok(result) => result,
                Err(reason) => {
                    candidates_skipped += 1;
                    if let Some(trace_index) = candidate.trace_index {
                        if let Some(trace) = root_candidates.get_mut(trace_index) {
                            trace.outcome =
                                crate::decision_trace::RootCandidateOutcome::Skipped(reason);
                        }
                    }
                    continue;
                }
            };
            if let Some(trace_index) = candidate.trace_index {
                if let Some(trace) = root_candidates.get_mut(trace_index) {
                    trace.payload_status = root_candidate_payload_status(
                        candidate.payload_override.as_ref(),
                        successor
                            .steps
                            .as_slice()
                            .last()
                            .and_then(|step| step.payload_override.as_ref()),
                    );
                }
            }
            if let Some(terminal_kind) = terminal {
                terminal_successors.push((terminal_kind, successor));
            } else {
                successors.push((terminal, successor));
            }
        }

        let terminal_count = terminal_successors.len() as u16;
        let non_terminal_before_beam = successors.len() as u16;

        let mut found_goal_satisfied = false;

        if !terminal_successors.is_empty() {
            // Sort by cost so the best candidate of each kind is first.
            terminal_successors.sort_by(|left, right| compare_search_nodes(&left.1, &right.1));

            for (terminal_kind, successor) in terminal_successors {
                match terminal_kind {
                    // GoalSatisfied and CombatCommitment are returned immediately.
                    PlanTerminalKind::GoalSatisfied | PlanTerminalKind::CombatCommitment => {
                        found_goal_satisfied =
                            matches!(terminal_kind, PlanTerminalKind::GoalSatisfied);
                        if let Some(ref mut sink) = expansion_summaries {
                            sink.push(crate::decision_trace::SearchExpansionSummary {
                                depth,
                                remaining_travel_ticks: node.heuristic_ticks,
                                combined_places_count: combined_places.places.len() as u16,
                                prerequisite_places_count: combined_places
                                    .prerequisite_places_count,
                                candidates_generated,
                                candidates_skipped,
                                terminal_successors: terminal_count,
                                non_terminal_before_beam,
                                non_terminal_after_beam: non_terminal_before_beam, // no truncation happened yet
                                found_goal_satisfied,
                                travel_pruning: travel_pruning.clone(),
                                prerequisite_guidance: combined_places.guidance_trace.clone(),
                                root_candidates: root_candidates.clone(),
                                root_omissions: root_omissions.clone(),
                            });
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
        successors.sort_by(|left, right| compare_search_nodes(&left.1, &right.1));
        successors.truncate(usize::from(budget.beam_width));

        let non_terminal_after_beam = successors.len() as u16;

        if let Some(ref mut sink) = expansion_summaries {
            sink.push(crate::decision_trace::SearchExpansionSummary {
                depth,
                remaining_travel_ticks: node.heuristic_ticks,
                combined_places_count: combined_places.places.len() as u16,
                prerequisite_places_count: combined_places.prerequisite_places_count,
                candidates_generated,
                candidates_skipped,
                terminal_successors: terminal_count,
                non_terminal_before_beam,
                non_terminal_after_beam,
                found_goal_satisfied,
                travel_pruning,
                prerequisite_guidance: combined_places.guidance_trace,
                root_candidates,
                root_omissions,
            });
        }

        for (terminal, successor) in successors {
            if let Some(terminal_kind) = terminal {
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
            frontier.push(FrontierEntry::new(successor));
        }
    }

    if let Some(barrier_plan) = best_barrier {
        return PlanSearchResult::Found(Box::new(barrier_plan));
    }
    PlanSearchResult::FrontierExhausted {
        expansions_used: expansions,
    }
}

#[cfg(test)]
mod tests;
