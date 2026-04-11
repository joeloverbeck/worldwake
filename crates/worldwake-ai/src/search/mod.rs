mod candidates;
mod frontier;
mod heuristic;
pub(crate) mod landmarks;
pub(crate) mod strategic;
mod transition;

use crate::{
    GoalKindPlannerExt, GroundedGoal, PlanTerminalKind, PlannedPlan, PlannedStep,
    PlannerOpSemantics, PlanningEntityRef, PlanningSnapshot, PlanningState,
    shared_collections::SharedVec,
};
#[cfg(test)]
use candidates::search_candidate_from_planner;
use candidates::{
    SearchCandidate, root_candidate_payload_status, search_candidates,
    search_candidates_from_affordance, unsupported_goal,
};
use frontier::{DualFrontier, FrontierEntry, compare_search_nodes};
use heuristic::combined_relevant_places_with_guidance;
#[cfg(test)]
use heuristic::compute_heuristic;
#[cfg(test)]
use heuristic::{combined_relevant_places, root_node};
use heuristic::{
    combined_relevant_places_for_tactical, combined_relevant_places_with_guidance_for_tactical,
    compute_landmark_heuristic, prune_travel_away_from_goal, root_node_for_tactical,
};
use landmarks::{
    LandmarkSet, PlanningFact, extract_landmarks, goal_facts_from_goal, planning_facts_from_state,
    planning_operator_from_transition, preferred_operators,
};
use std::collections::BTreeMap;
#[cfg(test)]
use transition::build_successor;
use transition::build_successor_detailed;
use worldwake_core::{
    ActionDefId, BlockedIntentMemory, CognitiveProfile, CommodityKind, ExecutionBudget,
    OpportunityKey, Tick,
};
use worldwake_sim::{
    ActionDefRegistry, ActionHandlerRegistry, InventoryBeliefView, RecipeRegistry,
    SpatialBeliefView, get_affordances_for_defs,
};

#[derive(Clone, Debug, Default)]
pub(crate) struct SearchTraceMetadata {
    pub(crate) strategic_plan: Option<strategic::StrategicPlan>,
    pub(crate) tactical_goal: Option<String>,
    pub(crate) landmarks_extracted: u16,
    pub(crate) landmark_orderings: u16,
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
        goal: &GroundedGoal,
        step: Option<&strategic::StrategicStep>,
        snapshot: &PlanningSnapshot,
    ) -> Option<Self> {
        let step = step?;
        match step.sub_goal {
            strategic::TacticalSubGoal::SatisfyGoal => {
                travel_to_goal_supports_tactical_barrier(&goal.key.kind).then_some(
                    Self::TravelToGoal {
                        destination: step.destination,
                    },
                )
            }
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
        goal: &GroundedGoal,
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
    goal: &GroundedGoal,
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
    step.is_some_and(|step| matches!(step.sub_goal, strategic::TacticalSubGoal::ExploreWithBarrier))
        && tactical_goal.is_none()
}

fn travel_to_goal_supports_tactical_barrier(goal: &worldwake_core::GoalKind) -> bool {
    !matches!(goal, worldwake_core::GoalKind::Accuse { .. })
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
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    cognitive: &CognitiveProfile,
    execution_budget: &ExecutionBudget,
    recipes: &RecipeRegistry,
    blocked: &BlockedIntentMemory,
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
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    cognitive: &CognitiveProfile,
    execution_budget: &ExecutionBudget,
    recipes: &RecipeRegistry,
    blocked: &BlockedIntentMemory,
    current_tick: Tick,
    mut binding_rejections: Option<&mut Vec<crate::decision_trace::BindingRejection>>,
    mut expansion_summaries: Option<&mut Vec<crate::decision_trace::SearchExpansionSummary>>,
    mut trace_metadata: Option<&mut SearchTraceMetadata>,
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

    let strategic_plan = strategic::plan(snapshot, goal, execution_budget, recipes);
    let mut trace_state = SearchTraceMetadata {
        strategic_plan: strategic_plan
            .as_ref()
            .filter(|plan| !plan.steps.is_empty())
            .cloned(),
        ..SearchTraceMetadata::default()
    };
    let first_strategic_step = strategic_plan.as_ref().and_then(|plan| plan.steps.first());
    let tactical_goal = TacticalGoal::from_strategic_step(goal, first_strategic_step, snapshot);
    trace_state.tactical_goal = tactical_goal.as_ref().map(|tg| format!("{tg:?}"));
    if should_fail_fast_for_missing_tactical_goal(first_strategic_step, tactical_goal.as_ref()) {
        if let Some(ref mut meta) = trace_metadata {
            **meta = trace_state;
        }
        return PlanSearchResult::FrontierExhausted {
            expansions_used: 0,
        };
    }
    let mut frontier = DualFrontier::new(execution_budget.preferred_operator_boost);
    frontier.push_regular(FrontierEntry::new(root_node_for_tactical(
        snapshot,
        goal,
        recipes,
        execution_budget,
        tactical_goal.as_ref(),
    )));
    let mut landmark_set = LandmarkSet::empty();
    let mut expansions = 0u16;
    let mut best_barrier: Option<PlannedPlan> = None;

    while let Some(node) = frontier.pop() {
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
        let active_tactical_goal = if node.tactical_barrier_reached {
            None
        } else {
            tactical_goal
                .as_ref()
                .filter(|goal| !goal.progress_barrier_satisfied(&node.state))
        };
        if node.steps.len() >= usize::from(cognitive.max_plan_depth) {
            continue;
        }
        if expansions >= cognitive.max_node_expansions {
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
        if let Some(extra_candidates) = social_query_candidates(
            goal,
            &node,
            semantics_table,
            registry,
            handlers,
            active_tactical_goal,
        ) {
            candidates.extend(extra_candidates);
        }
        apply_tactical_candidate_filter(
            &mut candidates,
            goal,
            &node,
            semantics_table,
            active_tactical_goal,
        );
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
            travel_pruning = prune_travel_away_from_goal(
                &mut candidates,
                current_place,
                &combined_places.places,
                snapshot,
                semantics_table,
            );
        }

        let candidates_generated = candidates.len() as u16;
        if candidates_generated > cognitive.max_candidates_per_expansion {
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
            let (terminal, successor) = match build_successor_detailed(
                goal,
                semantics_table,
                registry,
                &node,
                &candidate,
                recipes,
                execution_budget,
                active_tactical_goal,
                &landmark_set,
            ) {
                Ok(result) => result,
                Err(reason) => {
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
            if let Some(trace_index) = candidate.trace_index
                && let Some(trace) = root_candidates.get_mut(trace_index)
            {
                trace.payload_status = root_candidate_payload_status(
                    candidate.payload_override.as_ref(),
                    successor
                        .steps
                        .as_slice()
                        .last()
                        .and_then(|step| step.payload_override.as_ref()),
                );
            }
            if let Some(terminal_kind) = terminal {
                terminal_successors.push((terminal_kind, successor));
            } else {
                successor_candidates.push(candidate.clone());
                successor_operators.push(planning_operator_from_transition(
                    &node.state,
                    &successor.state,
                ));
                successors.push((terminal, successor, false));
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
                                travel_pruning: travel_pruning.clone(),
                                prerequisite_guidance: summary_places
                                    .as_ref()
                                    .and_then(|summary| summary.guidance_trace.clone()),
                                root_candidates: root_candidates.clone(),
                                root_omissions: root_omissions.clone(),
                            });
                        }
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

        if !landmark_set.landmarks.is_empty() && !successors.is_empty() {
            let current_facts = planning_facts_from_state(&node.state);
            let preferred_indices = preferred_operators(
                &landmark_set,
                &current_facts,
                &successor_candidates,
                &successor_operators,
            );
            for (index, (_, _, preferred)) in successors.iter_mut().enumerate() {
                *preferred = preferred_indices.contains(&index);
            }
        }

        successors.sort_by(|left, right| compare_search_nodes(&left.1, &right.1));
        successors.truncate(usize::from(execution_budget.beam_width));

        let non_terminal_after_beam = successors.len() as u16;
        let preferred_candidates = successors
            .iter()
            .filter(|(_, _, preferred)| *preferred)
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
                travel_pruning,
                prerequisite_guidance: summary_places.and_then(|summary| summary.guidance_trace),
                root_candidates,
                root_omissions,
            });
        }
        if successors.iter().any(|(_, _, preferred)| *preferred) {
            frontier.trigger_boost();
        }
        for (terminal, successor, preferred) in successors {
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
            let entry = FrontierEntry::new(successor);
            if preferred {
                frontier.push_preferred(entry);
            } else {
                frontier.push_regular(entry);
            }
        }
    }

    if let Some(barrier_plan) = best_barrier {
        if let Some(ref mut meta) = trace_metadata {
            **meta = trace_state;
        }
        return PlanSearchResult::Found(Box::new(barrier_plan));
    }
    if let Some(ref mut meta) = trace_metadata {
        **meta = trace_state;
    }
    PlanSearchResult::FrontierExhausted {
        expansions_used: expansions,
    }
}

fn social_query_candidates(
    goal: &GroundedGoal,
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

fn apply_tactical_candidate_filter(
    candidates: &mut Vec<SearchCandidate>,
    goal: &GroundedGoal,
    node: &SearchNode<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    tactical_goal: Option<&TacticalGoal>,
) {
    let Some(tactical_goal) = tactical_goal else {
        return;
    };
    let actor_place = node.state.effective_place(node.state.snapshot().actor());
    candidates.retain(|candidate| match tactical_goal {
        TacticalGoal::AcquirePrerequisite { destination, .. } => {
            if actor_place == Some(*destination) {
                semantics_table
                    .get(&candidate.def_id)
                    .is_none_or(|semantics| semantics.op_kind != crate::PlannerOpKind::Travel)
            } else {
                travel_advances_toward_destination(node, candidate, semantics_table, *destination)
            }
        }
        TacticalGoal::Explore { destination } => {
            if actor_place == Some(*destination) {
                false
            } else {
                travel_advances_toward_destination(node, candidate, semantics_table, *destination)
            }
        }
        TacticalGoal::SocialQuery { destination, .. } => {
            if actor_place == Some(*destination) {
                semantics_table
                    .get(&candidate.def_id)
                    .is_some_and(|semantics| semantics.op_kind == crate::PlannerOpKind::AskWitness)
            } else {
                travel_advances_toward_destination(node, candidate, semantics_table, *destination)
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
                travel_advances_toward_destination(node, candidate, semantics_table, *destination)
            }
        }
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
