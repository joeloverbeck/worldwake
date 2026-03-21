use crate::planner_ops::planner_only_candidates;
use crate::{
    apply_hypothetical_transition, GoalKindPlannerExt, GroundedGoal, PlanTerminalKind, PlannedPlan,
    PlannedStep, PlannerOpKind, PlannerOpSemantics, PlanningBudget, PlanningEntityRef,
    PlanningSnapshot, PlanningState,
};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use worldwake_core::{ActionDefId, EntityId, GoalKind};
use worldwake_sim::{
    get_affordances_for_defs, ActionDefRegistry, ActionHandlerRegistry, ActionPayload, Affordance,
    QueueForFacilityUsePayload, RecipeRegistry, RuntimeBeliefView,
};

#[derive(Clone)]
struct SearchNode<'snapshot> {
    state: PlanningState<'snapshot>,
    steps: Vec<PlannedStep>,
    total_estimated_ticks: u32,
    /// A* heuristic: minimum travel ticks from the actor's current simulated
    /// position to the nearest goal-relevant place.  Zero when already at a
    /// goal-relevant place, when no spatial guidance is available, or when the
    /// actor's place cannot be resolved.
    heuristic_ticks: u32,
}

struct FrontierEntry<'snapshot> {
    node: SearchNode<'snapshot>,
}

#[derive(Clone)]
struct SearchCandidate {
    def_id: ActionDefId,
    authoritative_targets: Vec<EntityId>,
    planning_targets: Vec<PlanningEntityRef>,
    payload_override: Option<ActionPayload>,
}

impl<'snapshot> FrontierEntry<'snapshot> {
    fn new(node: SearchNode<'snapshot>) -> Self {
        Self { node }
    }

    fn into_node(self) -> SearchNode<'snapshot> {
        self.node
    }
}

impl PartialEq for FrontierEntry<'_> {
    fn eq(&self, other: &Self) -> bool {
        compare_search_nodes(&self.node, &other.node) == Ordering::Equal
    }
}

impl Eq for FrontierEntry<'_> {}

impl PartialOrd for FrontierEntry<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FrontierEntry<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_search_nodes(&other.node, &self.node)
    }
}

/// Outcome of a plan search for one goal.
///
/// Replaces the previous `Option<PlannedPlan>` return type to preserve
/// failure-mode information needed by both diagnostics and tracing.
#[derive(Clone, Debug)]
pub enum PlanSearchResult {
    /// A valid plan was found.
    Found(PlannedPlan),
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
            Self::Found(plan) => Some(plan),
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
    mut binding_rejections: Option<&mut Vec<crate::decision_trace::BindingRejection>>,
    mut expansion_summaries: Option<&mut Vec<crate::decision_trace::SearchExpansionSummary>>,
) -> PlanSearchResult {
    if unsupported_goal(&goal.key.kind) {
        return PlanSearchResult::Unsupported;
    }

    let mut frontier = BinaryHeap::new();
    frontier.push(FrontierEntry::new(root_node(
        snapshot, goal, recipes, budget,
    )));
    let mut expansions = 0u16;
    let mut best_barrier: Option<PlannedPlan> = None;

    while let Some(node) = frontier.pop().map(FrontierEntry::into_node) {
        if goal.key.kind.is_satisfied(&node.state) {
            return PlanSearchResult::Found(PlannedPlan::new(
                goal.key,
                node.steps,
                PlanTerminalKind::GoalSatisfied,
            ));
        }
        if node.steps.len() >= usize::from(budget.max_plan_depth) {
            continue;
        }
        if expansions >= budget.max_node_expansions {
            if let Some(barrier_plan) = best_barrier {
                return PlanSearchResult::Found(barrier_plan);
            }
            return PlanSearchResult::BudgetExhausted {
                expansions_used: expansions,
            };
        }
        expansions = expansions.saturating_add(1);

        let depth = node.steps.len() as u8;

        let mut candidates = search_candidates(
            goal,
            &node,
            semantics_table,
            registry,
            handlers,
            binding_rejections.as_deref_mut(),
        );
        let mut travel_pruning = None;
        if let Some(current_place) =
            node.state
                .effective_place_ref(PlanningEntityRef::Authoritative(
                    node.state.snapshot().actor(),
                ))
        {
            let combined_places = combined_relevant_places(goal, &node.state, recipes, budget);
            travel_pruning = prune_travel_away_from_goal(
                &mut candidates,
                current_place,
                &combined_places,
                snapshot,
                semantics_table,
            );
        }

        let candidates_generated = candidates.len() as u16;

        let mut terminal_successors = Vec::new();
        let mut successors = Vec::new();
        let mut candidates_skipped = 0u16;
        for candidate in candidates {
            let Some((terminal, successor)) = build_successor(
                goal,
                semantics_table,
                registry,
                &node,
                &candidate,
                recipes,
                budget,
            ) else {
                candidates_skipped += 1;
                continue;
            };
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
                                candidates_generated,
                                candidates_skipped,
                                terminal_successors: terminal_count,
                                non_terminal_before_beam,
                                non_terminal_after_beam: non_terminal_before_beam, // no truncation happened yet
                                found_goal_satisfied,
                                travel_pruning: travel_pruning.clone(),
                            });
                        }
                        return PlanSearchResult::Found(PlannedPlan::new(
                            goal.key,
                            successor.steps,
                            terminal_kind,
                        ));
                    }
                    // ProgressBarrier is stored as a fallback — keep searching
                    // for a GoalSatisfied plan across deeper expansion levels.
                    PlanTerminalKind::ProgressBarrier => {
                        if best_barrier.is_none() {
                            best_barrier =
                                Some(PlannedPlan::new(goal.key, successor.steps, terminal_kind));
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
                candidates_generated,
                candidates_skipped,
                terminal_successors: terminal_count,
                non_terminal_before_beam,
                non_terminal_after_beam,
                found_goal_satisfied,
                travel_pruning,
            });
        }

        for (terminal, successor) in successors {
            if let Some(terminal_kind) = terminal {
                return PlanSearchResult::Found(PlannedPlan::new(
                    goal.key,
                    successor.steps,
                    terminal_kind,
                ));
            }
            frontier.push(FrontierEntry::new(successor));
        }
    }

    if let Some(barrier_plan) = best_barrier {
        return PlanSearchResult::Found(barrier_plan);
    }
    PlanSearchResult::FrontierExhausted {
        expansions_used: expansions,
    }
}

/// Compute the A* heuristic: minimum travel ticks from the actor's current
/// simulated position to the nearest goal-relevant place.  Returns 0 when
/// the actor is already at a goal-relevant place, when no spatial guidance
/// is available (empty `goal_relevant_places`), or when the actor's place
/// cannot be resolved.
fn compute_heuristic(
    snapshot: &PlanningSnapshot,
    state: &PlanningState<'_>,
    goal_relevant_places: &[EntityId],
) -> u32 {
    if goal_relevant_places.is_empty() {
        return 0;
    }
    let actor = state.snapshot().actor();
    state
        .effective_place_ref(PlanningEntityRef::Authoritative(actor))
        .and_then(|place| snapshot.min_travel_ticks_to_any(place, goal_relevant_places))
        .unwrap_or(0)
}

fn combined_relevant_places(
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
    budget: &PlanningBudget,
) -> Vec<EntityId> {
    let mut places = goal.key.kind.goal_relevant_places(state, recipes);
    let prerequisite_places = goal.key.kind.prerequisite_places(state, recipes, budget);
    for place in prerequisite_places {
        if !places.contains(&place) {
            places.push(place);
        }
    }
    places
}

fn root_node<'snapshot>(
    snapshot: &'snapshot PlanningSnapshot,
    goal: &GroundedGoal,
    recipes: &RecipeRegistry,
    budget: &PlanningBudget,
) -> SearchNode<'snapshot> {
    let state = PlanningState::new(snapshot);
    let combined_places = combined_relevant_places(goal, &state, recipes, budget);
    let heuristic_ticks = compute_heuristic(snapshot, &state, &combined_places);
    SearchNode {
        state,
        steps: Vec::new(),
        total_estimated_ticks: 0,
        heuristic_ticks,
    }
}

/// Removes travel candidates that move the actor farther from every
/// goal-relevant place.  Non-travel candidates are never pruned.
/// When `goal_places` is empty, the function is a no-op.
///
/// When the actor is already at one goal-relevant place, pruning continues
/// against the remaining relevant places so search can leave the current place
/// without broadening into arbitrary detours.
fn prune_travel_away_from_goal(
    candidates: &mut Vec<SearchCandidate>,
    current_place: EntityId,
    goal_places: &[EntityId],
    snapshot: &PlanningSnapshot,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
) -> Option<crate::decision_trace::TravelPruningTrace> {
    if goal_places.is_empty() {
        return None;
    }
    let current_min = snapshot
        .min_travel_ticks_to_any(current_place, goal_places)
        .unwrap_or(u32::MAX);
    let effective_goal_places = if current_min == 0 {
        let alternatives = goal_places
            .iter()
            .copied()
            .filter(|place| *place != current_place)
            .collect::<Vec<_>>();
        if alternatives.is_empty() {
            return None;
        }
        alternatives
    } else {
        goal_places.to_vec()
    };
    let current_min = snapshot
        .min_travel_ticks_to_any(current_place, &effective_goal_places)
        .unwrap_or(u32::MAX);
    let mut retained = Vec::new();
    let mut pruned = Vec::new();
    let mut kept_candidates = Vec::with_capacity(candidates.len());

    for candidate in candidates.drain(..) {
        let Some(semantics) = semantics_table.get(&candidate.def_id) else {
            kept_candidates.push(candidate);
            continue;
        };
        if semantics.op_kind != PlannerOpKind::Travel {
            kept_candidates.push(candidate);
            continue;
        }
        let Some(destination) = candidate.authoritative_targets.first().copied() else {
            kept_candidates.push(candidate);
            continue;
        };

        let remaining_travel_ticks = snapshot
            .min_travel_ticks_to_any(destination, &effective_goal_places)
            .unwrap_or(u32::MAX);
        let successor = crate::decision_trace::TravelSuccessorTrace {
            destination,
            remaining_travel_ticks,
        };
        if remaining_travel_ticks <= current_min {
            retained.push(successor);
            kept_candidates.push(candidate);
        } else {
            pruned.push(successor);
        }
    }

    *candidates = kept_candidates;
    if retained.is_empty() && pruned.is_empty() {
        return None;
    }
    retained.sort_by_key(|successor| successor.destination);
    pruned.sort_by_key(|successor| successor.destination);
    Some(crate::decision_trace::TravelPruningTrace {
        current_place,
        current_remaining_travel_ticks: current_min,
        retained,
        pruned,
    })
}

fn build_successor<'snapshot>(
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    node: &SearchNode<'snapshot>,
    candidate: &SearchCandidate,
    recipes: &RecipeRegistry,
    budget: &PlanningBudget,
) -> Option<(Option<PlanTerminalKind>, SearchNode<'snapshot>)> {
    let def = registry.get(candidate.def_id)?;
    let semantics = semantics_table.get(&candidate.def_id)?;
    if !goal
        .key
        .kind
        .relevant_op_kinds()
        .contains(&semantics.op_kind)
    {
        return None;
    }

    let actor = node.state.snapshot().actor();
    let payload_override = goal
        .key
        .kind
        .build_payload_override(
            candidate.payload_override.as_ref(),
            &node.state,
            &candidate.authoritative_targets,
            def,
            semantics,
        )
        .ok()?;
    let effective_payload = payload_override.as_ref().unwrap_or(&def.payload);
    let duration = node.state.estimate_duration(
        actor,
        &def.duration,
        &candidate.authoritative_targets,
        effective_payload,
    )?;
    let estimated_ticks = duration.ticks();

    let transition = apply_hypothetical_transition(
        goal,
        semantics,
        node.state.clone(),
        &candidate.planning_targets,
        payload_override.as_ref(),
    )?;
    let step = PlannedStep {
        def_id: candidate.def_id,
        targets: transition.targets,
        payload_override,
        op_kind: semantics.op_kind,
        estimated_ticks,
        is_materialization_barrier: semantics.is_materialization_barrier,
        expected_materializations: transition.expected_materializations,
    };
    let terminal = terminal_kind(goal, &transition.state, &step);
    if !semantics.may_appear_mid_plan && terminal.is_none() {
        return None;
    }
    let total_estimated_ticks = node.total_estimated_ticks.checked_add(estimated_ticks)?;
    let combined_places = combined_relevant_places(goal, &transition.state, recipes, budget);
    let heuristic_ticks =
        compute_heuristic(node.state.snapshot(), &transition.state, &combined_places);
    let mut steps = node.steps.clone();
    steps.push(step);

    Some((
        terminal,
        SearchNode {
            state: transition.state,
            steps,
            total_estimated_ticks,
            heuristic_ticks,
        },
    ))
}

fn relevant_action_defs(
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
) -> BTreeSet<ActionDefId> {
    let relevant_ops = goal.key.kind.relevant_op_kinds();
    semantics_table
        .iter()
        .filter(|(_, sem)| relevant_ops.contains(&sem.op_kind))
        .map(|(def_id, _)| *def_id)
        .collect()
}

fn search_candidates(
    goal: &GroundedGoal,
    node: &SearchNode<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    binding_rejections: Option<&mut Vec<crate::decision_trace::BindingRejection>>,
) -> Vec<SearchCandidate> {
    let relevant_defs = relevant_action_defs(goal, semantics_table);
    let mut candidates = get_affordances_for_defs(
        &node.state,
        node.state.snapshot().actor(),
        registry,
        handlers,
        &relevant_defs,
    )
    .into_iter()
    .flat_map(|affordance| {
        search_candidates_from_affordance(goal, &node.state, registry, &affordance)
    })
    .collect::<Vec<_>>();
    candidates.extend(
        planner_only_candidates(&node.state, semantics_table)
            .into_iter()
            .map(search_candidate_from_planner),
    );
    candidates
        .retain(|candidate| !candidate_uses_blocked_facility_use(candidate, &node.state, registry));
    // Reject candidates whose authoritative targets violate goal binding.
    // When a rejection collector is provided, record rejected candidates for traces.
    if let Some(rejections) = binding_rejections {
        candidates.retain(|candidate| {
            let Some(semantics) = semantics_table.get(&candidate.def_id) else {
                return true;
            };
            let passes = goal
                .key
                .kind
                .matches_binding(&candidate.authoritative_targets, semantics.op_kind);
            if !passes {
                let required_target = goal.key.entity.or(goal.key.place);
                rejections.push(crate::decision_trace::BindingRejection {
                    def_id: candidate.def_id,
                    rejected_targets: candidate.authoritative_targets.clone(),
                    required_target,
                });
            }
            passes
        });
    } else {
        candidates.retain(|candidate| {
            let Some(semantics) = semantics_table.get(&candidate.def_id) else {
                return true;
            };
            goal.key
                .kind
                .matches_binding(&candidate.authoritative_targets, semantics.op_kind)
        });
    }
    candidates
}

fn candidate_uses_blocked_facility_use(
    candidate: &SearchCandidate,
    state: &PlanningState<'_>,
    registry: &ActionDefRegistry,
) -> bool {
    let Some(facility) = candidate.authoritative_targets.first().copied() else {
        return false;
    };
    let Some(intended_action) = intended_exclusive_action(candidate, registry) else {
        return false;
    };

    state.is_facility_use_blocked(facility, intended_action)
}

fn intended_exclusive_action(
    candidate: &SearchCandidate,
    registry: &ActionDefRegistry,
) -> Option<ActionDefId> {
    if let Some(payload) = candidate
        .payload_override
        .as_ref()
        .and_then(ActionPayload::as_queue_for_facility_use)
    {
        return Some(payload.intended_action);
    }

    let payload = candidate
        .payload_override
        .as_ref()
        .or_else(|| registry.get(candidate.def_id).map(|def| &def.payload))?;
    matches!(payload, ActionPayload::Harvest(_) | ActionPayload::Craft(_))
        .then_some(candidate.def_id)
}

fn search_candidates_from_affordance(
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    registry: &ActionDefRegistry,
    affordance: &Affordance,
) -> Vec<SearchCandidate> {
    let planning_targets = affordance
        .bound_targets
        .iter()
        .copied()
        .map(PlanningEntityRef::Authoritative)
        .collect::<Vec<_>>();
    let base = SearchCandidate {
        def_id: affordance.def_id,
        authoritative_targets: affordance.bound_targets.clone(),
        planning_targets,
        payload_override: affordance.payload_override.clone(),
    };

    let Some(def) = registry.get(affordance.def_id) else {
        return vec![base];
    };
    if def.name != "queue_for_facility_use" {
        return vec![base];
    }
    if base.payload_override.is_some() {
        return vec![base];
    }

    let Some(facility) = affordance.bound_targets.first().copied() else {
        return Vec::new();
    };
    if state
        .snapshot()
        .entities
        .get(&facility)
        .and_then(|entity| entity.facility_queue.as_ref())
        .is_none()
    {
        return Vec::new();
    }
    let Some((workstation_tag, intended_actions)) =
        queue_intended_actions_for(goal, state, registry, facility)
    else {
        return Vec::new();
    };
    if state.is_actor_queued_at_facility(facility) {
        return Vec::new();
    }

    intended_actions
        .into_iter()
        .filter(|action_id| !state.has_actor_facility_grant(facility, *action_id))
        .map(|action_id| SearchCandidate {
            payload_override: Some(ActionPayload::QueueForFacilityUse(
                QueueForFacilityUsePayload {
                    intended_action: action_id,
                },
            )),
            ..base.clone()
        })
        .filter(|candidate| {
            registry
                .get(candidate.def_id)
                .is_some_and(|_| state.workstation_tag(facility) == Some(workstation_tag))
        })
        .collect()
}

fn queue_intended_actions_for(
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    registry: &ActionDefRegistry,
    facility: EntityId,
) -> Option<(worldwake_core::WorkstationTag, Vec<ActionDefId>)> {
    let workstation_tag = state.workstation_tag(facility)?;
    let actions = match goal.key.kind {
        GoalKind::ProduceCommodity { recipe_id } => registry
            .iter()
            .filter_map(|def| {
                let payload = def.payload.as_craft()?;
                (payload.recipe_id == recipe_id
                    && payload.required_workstation_tag == workstation_tag)
                    .then_some(def.id)
            })
            .collect::<Vec<_>>(),
        GoalKind::AcquireCommodity { commodity, .. }
        | GoalKind::ConsumeOwnedCommodity { commodity }
        | GoalKind::RestockCommodity { commodity } => registry
            .iter()
            .filter_map(|def| {
                if let Some(payload) = def.payload.as_harvest() {
                    return (payload.output_commodity == commodity
                        && payload.required_workstation_tag == workstation_tag)
                        .then_some(def.id);
                }
                def.payload.as_craft().and_then(|payload| {
                    (payload.required_workstation_tag == workstation_tag
                        && payload.outputs.iter().any(|(output, quantity)| {
                            *output == commodity && *quantity > worldwake_core::Quantity(0)
                        }))
                    .then_some(def.id)
                })
            })
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    (!actions.is_empty()).then_some((workstation_tag, actions))
}

fn search_candidate_from_planner(
    candidate: crate::planner_ops::PlannerSyntheticCandidate,
) -> SearchCandidate {
    SearchCandidate {
        def_id: candidate.def_id,
        authoritative_targets: Vec::new(),
        planning_targets: candidate.targets,
        payload_override: candidate.payload_override,
    }
}

fn unsupported_goal(goal: &GoalKind) -> bool {
    matches!(goal, GoalKind::SellCommodity { .. })
}

fn compare_search_nodes(left: &SearchNode<'_>, right: &SearchNode<'_>) -> Ordering {
    let left_f = left
        .total_estimated_ticks
        .saturating_add(left.heuristic_ticks);
    let right_f = right
        .total_estimated_ticks
        .saturating_add(right.heuristic_ticks);
    left_f
        .cmp(&right_f)
        .then_with(|| left.total_estimated_ticks.cmp(&right.total_estimated_ticks))
        .then_with(|| left.steps.len().cmp(&right.steps.len()))
        .then_with(|| left.steps.cmp(&right.steps))
}

fn terminal_kind(
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    step: &PlannedStep,
) -> Option<PlanTerminalKind> {
    if matches!(step.op_kind, PlannerOpKind::Attack | PlannerOpKind::Defend) {
        return Some(PlanTerminalKind::CombatCommitment);
    }
    if goal.key.kind.is_satisfied(state) {
        return Some(PlanTerminalKind::GoalSatisfied);
    }
    goal.key
        .kind
        .is_progress_barrier(step)
        .then_some(PlanTerminalKind::ProgressBarrier)
}

#[cfg(test)]
mod tests {
    use super::{
        build_successor, combined_relevant_places, compare_search_nodes, compute_heuristic,
        prune_travel_away_from_goal, root_node, search_candidate_from_planner, search_candidates,
        search_candidates_from_affordance, search_plan, FrontierEntry, SearchCandidate, SearchNode,
    };
    use crate::goal_model::GoalKindPlannerExt;
    use crate::planner_ops::planner_only_candidates;
    use crate::{
        build_planning_snapshot, build_planning_snapshot_with_blocked_facility_uses,
        build_semantics_table, CommodityPurpose, GoalKey, GoalKind, GroundedGoal, PlanTerminalKind,
        PlannedStep, PlannerOpKind, PlannerOpSemantics, PlannerTransitionKind, PlanningBudget,
        PlanningEntityRef, PlanningSnapshot, PlanningState,
    };
    use std::cmp::Ordering;
    use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_believed_entity_state, build_prototype_world, prototype_place_entity,
        test_utils::sample_trade_disposition_profile, ActionDefId, BlockedIntent,
        BlockedIntentMemory, BlockingFact, BodyCostPerTick, CarryCapacity, CauseRef, CombatProfile,
        CommodityConsumableProfile, CommodityKind, ControlSource, DemandMemory, DemandObservation,
        DemandObservationReason, DeprivationExposure, DriveThresholds, EntityId, EntityKind,
        EventLog, ExclusiveFacilityPolicy, FacilityUseQueue, GrantedFacilityUse, HomeostaticNeeds,
        InTransitOnEdge, KnownRecipes, LoadUnits, MerchandiseProfile, MetabolismProfile,
        PerceptionSource, Permille, Place, PrototypePlace, Quantity, RecipeId, ResourceSource,
        Tick, TickRange, Topology, TradeDispositionProfile, TravelEdge, TravelEdgeId,
        UniqueItemKind, VisibilitySpec, WitnessData, WorkstationMarker, WorkstationTag, World,
        WorldTxn, Wound,
    };
    use worldwake_sim::{
        estimate_duration_from_beliefs, ActionDefRegistry, ActionPayload, Affordance, DurationExpr,
        PerAgentBeliefView, QueueForFacilityUsePayload, RecipeDefinition, RecipeRegistry,
        RuntimeBeliefView, TransportActionPayload,
    };
    use worldwake_systems::build_full_action_registries;

    #[derive(Default)]
    struct TestBeliefView {
        alive: BTreeSet<EntityId>,
        kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        entities_at: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessions: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessors: BTreeMap<EntityId, EntityId>,
        controllable: BTreeSet<(EntityId, EntityId)>,
        adjacent: BTreeMap<EntityId, Vec<(EntityId, NonZeroU32)>>,
        lot_commodities: BTreeMap<EntityId, CommodityKind>,
        consumable_profiles: BTreeMap<EntityId, CommodityConsumableProfile>,
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        carry_capacities: BTreeMap<EntityId, LoadUnits>,
        entity_loads: BTreeMap<EntityId, LoadUnits>,
        needs: BTreeMap<EntityId, HomeostaticNeeds>,
        thresholds: BTreeMap<EntityId, DriveThresholds>,
        trade_profiles: BTreeMap<EntityId, TradeDispositionProfile>,
        merchandise_profiles: BTreeMap<EntityId, MerchandiseProfile>,
        demand_memory: BTreeMap<EntityId, Vec<DemandObservation>>,
        hostiles: BTreeMap<EntityId, Vec<EntityId>>,
        attackers: BTreeMap<EntityId, Vec<EntityId>>,
    }

    impl RuntimeBeliefView for TestBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.contains(&entity)
        }
        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.kinds.get(&entity).copied()
        }
        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.effective_places.get(&entity).copied()
        }
        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }
        fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.entities_at.get(&place).cloned().unwrap_or_default()
        }
        fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId> {
            self.direct_possessions
                .get(&holder)
                .cloned()
                .unwrap_or_default()
        }
        fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
            self.adjacent_places_with_travel_ticks(place)
                .into_iter()
                .map(|(place, _)| place)
                .collect()
        }
        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
            false
        }
        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
        }
        fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
            self.commodity_quantities
                .get(&(holder, kind))
                .copied()
                .unwrap_or(Quantity(0))
        }
        fn controlled_commodity_quantity_at_place(
            &self,
            actor: EntityId,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Quantity {
            self.local_controlled_lots_for(actor, place, commodity)
                .into_iter()
                .fold(Quantity(0), |total, entity| {
                    let quantity = self
                        .commodity_quantities
                        .get(&(entity, commodity))
                        .copied()
                        .unwrap_or(Quantity(0));
                    Quantity(total.0 + quantity.0)
                })
        }
        fn local_controlled_lots_for(
            &self,
            actor: EntityId,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Vec<EntityId> {
            let mut entities = self.entities_at(place);
            entities.extend(self.direct_possessions(actor));
            entities.sort();
            entities.dedup();
            entities
                .into_iter()
                .filter(|entity| self.item_lot_commodity(*entity) == Some(commodity))
                .filter(|entity| self.can_control(actor, *entity))
                .collect()
        }
        fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> {
            self.lot_commodities.get(&entity).copied()
        }
        fn item_lot_consumable_profile(
            &self,
            entity: EntityId,
        ) -> Option<CommodityConsumableProfile> {
            self.consumable_profiles.get(&entity).copied()
        }
        fn direct_container(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }
        fn direct_possessor(&self, entity: EntityId) -> Option<EntityId> {
            self.direct_possessors.get(&entity).copied()
        }
        fn believed_owner_of(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }
        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }
        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }
        fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
            self.controllable.contains(&(actor, entity))
        }
        fn has_control(&self, entity: EntityId) -> bool {
            self.kinds.get(&entity) == Some(&EntityKind::Agent)
        }
        fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.carry_capacities.get(&entity).copied()
        }
        fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.entity_loads.get(&entity).copied()
        }
        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }
        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
            Vec::new()
        }
        fn is_dead(&self, entity: EntityId) -> bool {
            !self.is_alive(entity)
        }
        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }
        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
        }
        fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
            self.needs.get(&agent).copied()
        }
        fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
            self.thresholds.get(&agent).copied()
        }
        fn belief_confidence_policy(
            &self,
            _agent: EntityId,
        ) -> worldwake_core::BeliefConfidencePolicy {
            worldwake_core::BeliefConfidencePolicy::default()
        }
        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            Some(MetabolismProfile::default())
        }
        fn trade_disposition_profile(&self, agent: EntityId) -> Option<TradeDispositionProfile> {
            self.trade_profiles.get(&agent).cloned()
        }
        fn travel_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::TravelDispositionProfile> {
            None
        }
        fn combat_profile(&self, _agent: EntityId) -> Option<CombatProfile> {
            Some(CombatProfile::new(
                pm(1000),
                pm(700),
                pm(620),
                pm(580),
                pm(80),
                pm(25),
                pm(18),
                pm(120),
                pm(35),
                NonZeroU32::new(6).unwrap(),
                NonZeroU32::new(10).unwrap(),
            ))
        }
        fn wounds(&self, _agent: EntityId) -> Vec<Wound> {
            Vec::new()
        }
        fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId> {
            self.hostiles.get(&agent).cloned().unwrap_or_default()
        }
        fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId> {
            self.attackers.get(&agent).cloned().unwrap_or_default()
        }
        fn agents_selling_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.entities_at(place)
                .into_iter()
                .filter(|entity| {
                    self.merchandise_profiles
                        .get(entity)
                        .is_some_and(|profile| profile.sale_kinds.contains(&commodity))
                })
                .collect()
        }
        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            Vec::new()
        }
        fn matching_workstations_at(
            &self,
            _place: EntityId,
            _tag: WorkstationTag,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation> {
            self.demand_memory.get(&agent).cloned().unwrap_or_default()
        }
        fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile> {
            self.merchandise_profiles.get(&agent).cloned()
        }
        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn in_transit_state(&self, _entity: EntityId) -> Option<InTransitOnEdge> {
            None
        }
        fn adjacent_places_with_travel_ticks(
            &self,
            place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            self.adjacent.get(&place).cloned().unwrap_or_default()
        }
        fn estimate_duration(
            &self,
            actor: EntityId,
            duration: &DurationExpr,
            targets: &[EntityId],
            payload: &ActionPayload,
        ) -> Option<worldwake_sim::ActionDuration> {
            estimate_duration_from_beliefs(self, actor, duration, targets, payload)
        }
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn sync_all_beliefs(world: &mut World, observer: EntityId, observed_tick: Tick) {
        let snapshots = world
            .entities()
            .filter(|entity| *entity != observer)
            .filter_map(|entity| {
                build_believed_entity_state(
                    world,
                    entity,
                    observed_tick,
                    PerceptionSource::DirectObservation,
                )
                .map(|state| (entity, state))
            })
            .collect::<Vec<_>>();
        let mut store = world
            .get_component_agent_belief_store(observer)
            .cloned()
            .expect("observer must have AgentBeliefStore");
        store.known_entities.clear();
        for (entity, state) in snapshots {
            store.update_entity(entity, state);
        }
        let mut txn = WorldTxn::new(
            world,
            observed_tick,
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        txn.set_component_agent_belief_store(observer, store)
            .expect("observer belief store should remain writable");
        let mut event_log = EventLog::new();
        let _ = txn.commit(&mut event_log);
    }

    fn build_registry() -> (ActionDefRegistry, worldwake_sim::ActionHandlerRegistry) {
        let recipes = RecipeRegistry::new();
        let registries = build_full_action_registries(&recipes).unwrap();
        (registries.defs, registries.handlers)
    }

    fn build_registry_with_recipes(
        recipes: &RecipeRegistry,
    ) -> (ActionDefRegistry, worldwake_sim::ActionHandlerRegistry) {
        let registries = build_full_action_registries(recipes).unwrap();
        (registries.defs, registries.handlers)
    }

    fn harvest_apple_recipe() -> RecipeDefinition {
        RecipeDefinition {
            name: "Harvest Apples".to_string(),
            inputs: vec![],
            outputs: vec![(CommodityKind::Apple, Quantity(2))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::OrchardRow),
            required_tool_kinds: vec![],
            body_cost_per_tick: BodyCostPerTick::new(pm(3), pm(2), pm(5), pm(1)),
        }
    }

    fn harvest_apple_recipe_variant(name: &str, output_quantity: u32) -> RecipeDefinition {
        RecipeDefinition {
            name: name.to_string(),
            inputs: vec![],
            outputs: vec![(CommodityKind::Apple, Quantity(output_quantity))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::OrchardRow),
            required_tool_kinds: vec![],
            body_cost_per_tick: BodyCostPerTick::new(pm(3), pm(2), pm(5), pm(1)),
        }
    }

    fn insert_hungry_actor(view: &mut TestBeliefView, actor: EntityId) {
        view.kinds.insert(actor, EntityKind::Agent);
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
    }

    fn insert_consumable_lot(
        view: &mut TestBeliefView,
        actor: EntityId,
        lot: EntityId,
        place: EntityId,
        commodity: CommodityKind,
        entities_at_place: &mut Vec<EntityId>,
    ) {
        view.alive.insert(lot);
        view.kinds.insert(lot, EntityKind::ItemLot);
        view.effective_places.insert(lot, place);
        view.controllable.insert((actor, lot));
        view.lot_commodities.insert(lot, commodity);
        view.commodity_quantities
            .insert((lot, commodity), Quantity(1));
        view.consumable_profiles
            .insert(lot, commodity.spec().consumable_profile.unwrap());
        entities_at_place.push(lot);
    }

    fn insert_bread_lot(
        view: &mut TestBeliefView,
        actor: EntityId,
        bread: EntityId,
        place: EntityId,
        entities_at_place: &mut Vec<EntityId>,
    ) {
        insert_consumable_lot(
            view,
            actor,
            bread,
            place,
            CommodityKind::Bread,
            entities_at_place,
        );
    }

    fn consume_goal(commodity: CommodityKind) -> GroundedGoal {
        GroundedGoal {
            key: GoalKey::from(worldwake_core::GoalKind::ConsumeOwnedCommodity { commodity }),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
        }
    }

    fn acquire_goal_with_purpose(
        commodity: CommodityKind,
        purpose: CommodityPurpose,
    ) -> GroundedGoal {
        GroundedGoal {
            key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity { commodity, purpose }),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
        }
    }

    fn acquire_goal(commodity: CommodityKind) -> GroundedGoal {
        acquire_goal_with_purpose(commodity, CommodityPurpose::SelfConsume)
    }

    fn sample_step(
        def_id: u32,
        op_kind: PlannerOpKind,
        estimated_ticks: u32,
        targets: Vec<EntityId>,
    ) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(def_id),
            targets: targets
                .into_iter()
                .map(PlanningEntityRef::Authoritative)
                .collect(),
            payload_override: None,
            op_kind,
            estimated_ticks,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
        }
    }

    fn frontier_test_node(
        snapshot: &PlanningSnapshot,
        total_estimated_ticks: u32,
        steps: Vec<PlannedStep>,
    ) -> SearchNode<'_> {
        SearchNode {
            state: PlanningState::new(snapshot),
            steps,
            total_estimated_ticks,
            heuristic_ticks: 0,
        }
    }

    fn pickup_node(
        commodity: CommodityKind,
        quantity: Quantity,
        carry_capacity: LoadUnits,
    ) -> (
        SearchNode<'static>,
        EntityId,
        EntityId,
        EntityId,
        ActionDefRegistry,
        worldwake_sim::ActionHandlerRegistry,
    ) {
        let actor = entity(1);
        let place = entity(10);
        let lot = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, place, lot]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(lot, EntityKind::ItemLot);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(lot, place);
        view.entities_at.insert(place, vec![actor, lot]);
        view.controllable.insert((actor, lot));
        view.lot_commodities.insert(lot, commodity);
        view.commodity_quantities.insert((lot, commodity), quantity);
        view.carry_capacities.insert(actor, carry_capacity);
        view.entity_loads.insert(actor, LoadUnits(0));
        view.entity_loads.insert(
            lot,
            LoadUnits(
                quantity
                    .0
                    .saturating_mul(worldwake_core::load_per_unit(commodity).0),
            ),
        );
        let snapshot = Box::leak(Box::new(build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([lot]),
            &BTreeSet::from([place]),
            1,
        )));

        let (registry, handlers) = build_registry();
        (
            SearchNode {
                state: PlanningState::new(snapshot),
                steps: Vec::new(),
                total_estimated_ticks: 0,
                heuristic_ticks: 0,
            },
            actor,
            place,
            lot,
            registry,
            handlers,
        )
    }

    #[test]
    fn search_returns_one_step_consume_plan_for_local_food() {
        let actor = entity(1);
        let town = entity(10);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(bread, town);
        view.entities_at.insert(town, vec![actor, bread]);
        view.controllable.insert((actor, bread));
        view.direct_possessions.insert(actor, vec![bread]);
        view.direct_possessors.insert(bread, actor);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityKind::Bread.spec().consumable_profile.unwrap(),
        );
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let plan = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .unwrap();

        assert_eq!(plan.terminal_kind, PlanTerminalKind::GoalSatisfied);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Consume);
    }

    #[test]
    fn search_frontier_heap_preserves_priority_tiebreaks() {
        let actor = entity(1);
        let town = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.entities_at.insert(town, vec![actor]);

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let mut frontier = BinaryHeap::new();
        frontier.push(FrontierEntry::new(frontier_test_node(
            &snapshot,
            5,
            vec![sample_step(4, PlannerOpKind::Travel, 5, vec![entity(24)])],
        )));
        frontier.push(FrontierEntry::new(frontier_test_node(
            &snapshot,
            3,
            vec![
                sample_step(1, PlannerOpKind::Travel, 1, vec![entity(21)]),
                sample_step(2, PlannerOpKind::Consume, 2, vec![entity(22)]),
            ],
        )));
        frontier.push(FrontierEntry::new(frontier_test_node(
            &snapshot,
            3,
            vec![sample_step(3, PlannerOpKind::Travel, 3, vec![entity(23)])],
        )));
        frontier.push(FrontierEntry::new(frontier_test_node(
            &snapshot,
            3,
            vec![sample_step(2, PlannerOpKind::Travel, 3, vec![entity(22)])],
        )));

        let popped = std::iter::from_fn(|| frontier.pop().map(FrontierEntry::into_node))
            .map(|node| node.steps)
            .collect::<Vec<_>>();

        assert_eq!(
            popped,
            vec![
                vec![sample_step(2, PlannerOpKind::Travel, 3, vec![entity(22)])],
                vec![sample_step(3, PlannerOpKind::Travel, 3, vec![entity(23)])],
                vec![
                    sample_step(1, PlannerOpKind::Travel, 1, vec![entity(21)]),
                    sample_step(2, PlannerOpKind::Consume, 2, vec![entity(22)]),
                ],
                vec![sample_step(4, PlannerOpKind::Travel, 5, vec![entity(24)])],
            ]
        );
    }

    #[test]
    fn search_returns_travel_then_consume_for_adjacent_food() {
        let actor = entity(1);
        let town = entity(10);
        let field = entity(11);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town, field, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(field, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(bread, field);
        view.entities_at.insert(town, vec![actor]);
        view.entities_at.insert(field, vec![bread]);
        view.controllable.insert((actor, bread));
        view.adjacent
            .insert(town, vec![(field, NonZeroU32::new(3).unwrap())]);
        view.adjacent
            .insert(field, vec![(town, NonZeroU32::new(3).unwrap())]);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(1));
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.consumable_profiles.insert(
            bread,
            CommodityKind::Bread.spec().consumable_profile.unwrap(),
        );
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let plan = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .unwrap();

        // ConsumeOwnedCommodity treats MoveCargo as a progress barrier because
        // the planner cannot model possession transfer. After pick_up commits,
        // the agent replans and finds eat as a 1-step GoalSatisfied plan.
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Travel);
        assert_eq!(plan.steps[1].op_kind, PlannerOpKind::MoveCargo);
        assert_eq!(plan.terminal_kind, PlanTerminalKind::ProgressBarrier);
    }

    #[test]
    fn search_returns_none_when_only_wrong_local_consumable_is_controllable() {
        let actor = entity(1);
        let town = entity(10);
        let water = entity(20);
        let mut view = TestBeliefView::default();
        let mut town_entities = vec![actor];
        view.alive.extend([actor, town]);
        insert_hungry_actor(&mut view, actor);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        insert_consumable_lot(
            &mut view,
            actor,
            water,
            town,
            CommodityKind::Water,
            &mut town_entities,
        );
        view.entities_at.insert(town, town_entities);

        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        // Protects the search_plan -> apply_hypothetical_transition seam for consume targets.
        let plan = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        );

        assert!(!plan.is_found());
    }

    #[test]
    fn search_returns_travel_then_trade_barrier_for_reachable_seller() {
        let actor = entity(1);
        let town = entity(10);
        let market = entity(11);
        let seller = entity(2);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, seller, town, market]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(seller, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(market, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(seller, market);
        view.entities_at.insert(town, vec![actor]);
        view.entities_at.insert(market, vec![seller]);
        view.adjacent
            .insert(town, vec![(market, NonZeroU32::new(4).unwrap())]);
        view.adjacent
            .insert(market, vec![(town, NonZeroU32::new(4).unwrap())]);
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        view.trade_profiles
            .insert(actor, sample_trade_disposition_profile());
        view.merchandise_profiles.insert(
            seller,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(market),
            },
        );
        view.commodity_quantities
            .insert((actor, CommodityKind::Coin), Quantity(3));
        view.commodity_quantities
            .insert((seller, CommodityKind::Bread), Quantity(2));
        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }),
            evidence_entities: BTreeSet::from([seller]),
            evidence_places: BTreeSet::from([market]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            1,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .unwrap();

        assert_eq!(plan.terminal_kind, PlanTerminalKind::ProgressBarrier);
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Travel);
        assert_eq!(plan.steps[1].op_kind, PlannerOpKind::Trade);
        assert!(matches!(
            plan.steps[1].payload_override,
            Some(ActionPayload::Trade(_))
        ));
    }

    #[test]
    fn search_prefers_local_trade_barrier_over_cheaper_nonterminal_travel_options() {
        let actor = entity(1);
        let seller = entity(2);
        let town = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, seller, town]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(seller, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(seller, town);
        view.entities_at.insert(town, vec![actor, seller]);
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        view.trade_profiles
            .insert(actor, sample_trade_disposition_profile());
        view.merchandise_profiles.insert(
            seller,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(town),
            },
        );
        view.commodity_quantities
            .insert((actor, CommodityKind::Coin), Quantity(3));
        view.commodity_quantities
            .insert((seller, CommodityKind::Bread), Quantity(2));

        for offset in 0..9 {
            let branch = entity(20 + offset);
            view.alive.insert(branch);
            view.kinds.insert(branch, EntityKind::Place);
            view.adjacent
                .entry(town)
                .or_default()
                .push((branch, NonZeroU32::new(1).unwrap()));
            view.adjacent
                .entry(branch)
                .or_default()
                .push((town, NonZeroU32::new(1).unwrap()));
        }

        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }),
            evidence_entities: BTreeSet::from([seller]),
            evidence_places: BTreeSet::from([town]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            1,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .expect("local trade barrier should not be pruned by cheaper travel branches");

        assert_eq!(plan.terminal_kind, PlanTerminalKind::ProgressBarrier);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Trade);
        assert!(matches!(
            plan.steps[0].payload_override,
            Some(ActionPayload::Trade(_))
        ));
    }

    #[test]
    fn search_returns_trade_barrier_for_recipe_input_acquire_goal() {
        let actor = entity(1);
        let seller = entity(2);
        let town = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, seller, town]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(seller, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(seller, town);
        view.entities_at.insert(town, vec![actor, seller]);
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        view.trade_profiles
            .insert(actor, sample_trade_disposition_profile());
        view.merchandise_profiles.insert(
            seller,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Firewood]),
                home_market: Some(town),
            },
        );
        view.commodity_quantities
            .insert((actor, CommodityKind::Coin), Quantity(3));
        view.commodity_quantities
            .insert((seller, CommodityKind::Firewood), Quantity(1));

        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
                commodity: CommodityKind::Firewood,
                purpose: CommodityPurpose::RecipeInput(RecipeId(0)),
            }),
            evidence_entities: BTreeSet::from([seller]),
            evidence_places: BTreeSet::from([town]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            1,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .expect("local recipe-input acquire goal should plan through trade");

        assert_eq!(plan.terminal_kind, PlanTerminalKind::ProgressBarrier);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Trade);
        assert!(matches!(
            plan.steps[0].payload_override,
            Some(ActionPayload::Trade(_))
        ));
    }

    #[test]
    fn search_respects_plan_depth_budget() {
        let actor = entity(1);
        let town = entity(10);
        let field = entity(11);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town, field, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(field, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(bread, field);
        view.entities_at.insert(town, vec![actor]);
        view.entities_at.insert(field, vec![bread]);
        view.controllable.insert((actor, bread));
        view.adjacent
            .insert(town, vec![(field, NonZeroU32::new(3).unwrap())]);
        view.adjacent
            .insert(field, vec![(town, NonZeroU32::new(3).unwrap())]);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityKind::Bread.spec().consumable_profile.unwrap(),
        );
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let budget = PlanningBudget {
            max_plan_depth: 1,
            ..PlanningBudget::default()
        };
        let plan = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &budget,
            &RecipeRegistry::new(),
            None,
            None,
        );

        assert!(!plan.is_found());
    }

    #[test]
    fn search_returns_none_when_node_expansion_budget_is_exhausted() {
        let actor = entity(1);
        let town = entity(10);
        let field = entity(11);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town, field, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(field, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(bread, field);
        view.entities_at.insert(town, vec![actor]);
        view.entities_at.insert(field, vec![bread]);
        view.controllable.insert((actor, bread));
        view.adjacent
            .insert(town, vec![(field, NonZeroU32::new(3).unwrap())]);
        view.adjacent
            .insert(field, vec![(town, NonZeroU32::new(3).unwrap())]);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityKind::Bread.spec().consumable_profile.unwrap(),
        );
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let budget = PlanningBudget {
            max_node_expansions: 0,
            ..PlanningBudget::default()
        };
        let plan = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &budget,
            &RecipeRegistry::new(),
            None,
            None,
        );

        assert!(!plan.is_found());
    }

    #[test]
    fn search_beam_width_1_prunes_viable_slower_branch() {
        let actor = entity(1);
        let town = entity(10);
        let dead_end = entity(11);
        let pantry = entity(12);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        let mut pantry_entities = Vec::new();
        view.alive.extend([actor, town, dead_end, pantry]);
        insert_hungry_actor(&mut view, actor);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(dead_end, EntityKind::Place);
        view.kinds.insert(pantry, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.entities_at.insert(town, vec![actor]);
        view.entities_at.insert(dead_end, Vec::new());
        insert_bread_lot(&mut view, actor, bread, pantry, &mut pantry_entities);
        view.entities_at.insert(pantry, pantry_entities);
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.adjacent.insert(
            town,
            vec![
                (dead_end, NonZeroU32::new(1).unwrap()),
                (pantry, NonZeroU32::new(3).unwrap()),
            ],
        );

        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let narrow_beam_plan = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget {
                beam_width: 1,
                ..PlanningBudget::default()
            },
            &RecipeRegistry::new(),
            None,
            None,
        );
        let wide_beam_plan = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget {
                beam_width: 2,
                ..PlanningBudget::default()
            },
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .unwrap();

        assert!(!narrow_beam_plan.is_found());
        assert_eq!(
            wide_beam_plan.terminal_kind,
            PlanTerminalKind::ProgressBarrier
        );
        assert_eq!(wide_beam_plan.steps.len(), 2);
        assert_eq!(wide_beam_plan.steps[0].op_kind, PlannerOpKind::Travel);
        assert_eq!(wide_beam_plan.steps[1].op_kind, PlannerOpKind::MoveCargo);
        assert_eq!(
            wide_beam_plan.steps[0].targets,
            vec![PlanningEntityRef::Authoritative(pantry)]
        );
    }

    #[test]
    fn search_beam_width_widening_keeps_more_successors() {
        let actor = entity(1);
        let town = entity(10);
        let dead_end_a = entity(11);
        let dead_end_b = entity(12);
        let pantry = entity(13);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        let mut pantry_entities = Vec::new();
        view.alive
            .extend([actor, town, dead_end_a, dead_end_b, pantry]);
        insert_hungry_actor(&mut view, actor);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(dead_end_a, EntityKind::Place);
        view.kinds.insert(dead_end_b, EntityKind::Place);
        view.kinds.insert(pantry, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.entities_at.insert(town, vec![actor]);
        view.entities_at.insert(dead_end_a, Vec::new());
        view.entities_at.insert(dead_end_b, Vec::new());
        insert_bread_lot(&mut view, actor, bread, pantry, &mut pantry_entities);
        view.entities_at.insert(pantry, pantry_entities);
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.adjacent.insert(
            town,
            vec![
                (dead_end_a, NonZeroU32::new(1).unwrap()),
                (dead_end_b, NonZeroU32::new(2).unwrap()),
                (pantry, NonZeroU32::new(3).unwrap()),
            ],
        );

        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let beam_two_plan = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget {
                beam_width: 2,
                ..PlanningBudget::default()
            },
            &RecipeRegistry::new(),
            None,
            None,
        );
        let beam_three_plan = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget {
                beam_width: 3,
                ..PlanningBudget::default()
            },
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .unwrap();

        assert!(!beam_two_plan.is_found());
        assert_eq!(
            beam_three_plan.terminal_kind,
            PlanTerminalKind::ProgressBarrier
        );
        assert_eq!(beam_three_plan.steps.len(), 2);
        assert_eq!(
            beam_three_plan.steps[0].targets,
            vec![PlanningEntityRef::Authoritative(pantry)]
        );
    }

    #[test]
    fn search_returns_none_when_large_beam_still_exhausts_node_budget() {
        let actor = entity(1);
        let town = entity(10);
        let dead_end_a = entity(11);
        let dead_end_b = entity(12);
        let pantry = entity(13);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        let mut pantry_entities = Vec::new();
        view.alive
            .extend([actor, town, dead_end_a, dead_end_b, pantry]);
        insert_hungry_actor(&mut view, actor);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(dead_end_a, EntityKind::Place);
        view.kinds.insert(dead_end_b, EntityKind::Place);
        view.kinds.insert(pantry, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.entities_at.insert(town, vec![actor]);
        view.entities_at.insert(dead_end_a, Vec::new());
        view.entities_at.insert(dead_end_b, Vec::new());
        insert_bread_lot(&mut view, actor, bread, pantry, &mut pantry_entities);
        view.entities_at.insert(pantry, pantry_entities);
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.adjacent.insert(
            town,
            vec![
                (dead_end_a, NonZeroU32::new(1).unwrap()),
                (dead_end_b, NonZeroU32::new(2).unwrap()),
                (pantry, NonZeroU32::new(3).unwrap()),
            ],
        );

        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let exhausted_plan = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget {
                beam_width: 3,
                max_node_expansions: 2,
                ..PlanningBudget::default()
            },
            &RecipeRegistry::new(),
            None,
            None,
        );
        let sufficient_budget_plan = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget {
                beam_width: 3,
                max_node_expansions: 6,
                ..PlanningBudget::default()
            },
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .unwrap();

        assert!(!exhausted_plan.is_found());
        assert_eq!(
            sufficient_budget_plan.terminal_kind,
            PlanTerminalKind::ProgressBarrier
        );
        assert_eq!(
            sufficient_budget_plan.steps[0].targets,
            vec![PlanningEntityRef::Authoritative(pantry)]
        );
    }

    #[test]
    fn search_returns_none_when_plan_depth_is_zero() {
        let actor = entity(1);
        let town = entity(10);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        let mut town_entities = vec![actor];
        view.alive.extend([actor, town]);
        insert_hungry_actor(&mut view, actor);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        insert_bread_lot(&mut view, actor, bread, town, &mut town_entities);
        view.entities_at.insert(town, town_entities);

        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let plan = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget {
                max_plan_depth: 0,
                ..PlanningBudget::default()
            },
            &RecipeRegistry::new(),
            None,
            None,
        );

        assert!(!plan.is_found());
    }

    #[test]
    fn search_rejects_branch_when_duration_estimation_fails() {
        let actor = entity(1);
        let town = entity(10);
        let market = entity(11);
        let seller = entity(2);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, seller, town, market]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(seller, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(seller, market);
        view.entities_at.insert(town, vec![actor]);
        view.entities_at.insert(market, vec![seller]);
        view.adjacent
            .insert(town, vec![(market, NonZeroU32::new(3).unwrap())]);
        view.adjacent
            .insert(market, vec![(town, NonZeroU32::new(3).unwrap())]);
        view.merchandise_profiles.insert(
            seller,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(market),
            },
        );
        view.commodity_quantities
            .insert((actor, CommodityKind::Coin), Quantity(3));
        view.commodity_quantities
            .insert((seller, CommodityKind::Bread), Quantity(2));
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        let goal = GroundedGoal {
            key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }),
            evidence_entities: BTreeSet::from([seller]),
            evidence_places: BTreeSet::from([market]),
        };

        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            1,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        );

        assert!(!plan.is_found());
    }

    #[test]
    fn search_returns_pick_up_goal_satisfaction_for_local_unpossessed_food_lot() {
        let actor = entity(1);
        let town = entity(10);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(bread, town);
        view.entities_at.insert(town, vec![actor, bread]);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityKind::Bread.spec().consumable_profile.unwrap(),
        );
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(1));
        view.carry_capacities.insert(actor, LoadUnits(4));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.entity_loads.insert(bread, LoadUnits(1));
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());

        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: acquire_goal(CommodityKind::Bread).key,
            evidence_entities: BTreeSet::from([bread]),
            evidence_places: BTreeSet::from([town]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            1,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .unwrap();

        assert_eq!(plan.terminal_kind, PlanTerminalKind::GoalSatisfied);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::MoveCargo);
    }

    #[test]
    fn search_returns_pick_up_goal_satisfaction_for_local_commodity_lot() {
        let actor = entity(1);
        let town = entity(10);
        let medicine = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town, medicine]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(medicine, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(medicine, town);
        view.entities_at.insert(town, vec![actor, medicine]);
        view.lot_commodities
            .insert(medicine, CommodityKind::Medicine);
        view.commodity_quantities
            .insert((medicine, CommodityKind::Medicine), Quantity(1));
        view.carry_capacities.insert(actor, LoadUnits(2));
        view.entity_loads.insert(actor, LoadUnits(0));

        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: acquire_goal_with_purpose(CommodityKind::Medicine, CommodityPurpose::SelfConsume)
                .key,
            evidence_entities: BTreeSet::from([medicine]),
            evidence_places: BTreeSet::from([town]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            1,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .unwrap();

        assert_eq!(plan.terminal_kind, PlanTerminalKind::GoalSatisfied);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::MoveCargo);
    }

    #[test]
    fn search_returns_partial_pick_up_goal_satisfaction_for_local_food_lot() {
        let actor = entity(1);
        let town = entity(10);
        let apples = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town, apples]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(apples, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(apples, town);
        view.entities_at.insert(town, vec![actor, apples]);
        view.lot_commodities.insert(apples, CommodityKind::Apple);
        view.consumable_profiles.insert(
            apples,
            CommodityKind::Apple.spec().consumable_profile.unwrap(),
        );
        view.commodity_quantities
            .insert((apples, CommodityKind::Apple), Quantity(2));
        view.carry_capacities.insert(actor, LoadUnits(1));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.entity_loads.insert(apples, LoadUnits(2));
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());

        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: acquire_goal(CommodityKind::Apple).key,
            evidence_entities: BTreeSet::from([apples]),
            evidence_places: BTreeSet::from([town]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            1,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .unwrap();

        assert_eq!(plan.terminal_kind, PlanTerminalKind::GoalSatisfied);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::MoveCargo);
        assert_eq!(
            plan.steps[0].targets,
            vec![PlanningEntityRef::Authoritative(apples)]
        );
        assert!(!plan.steps[0].expected_materializations.is_empty());
    }

    #[test]
    fn cargo_search_finds_pickup_then_travel_plan() {
        let actor = entity(1);
        let origin = entity(10);
        let destination = entity(11);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, origin, destination, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(origin, EntityKind::Place);
        view.kinds.insert(destination, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, origin);
        view.effective_places.insert(bread, origin);
        view.entities_at.insert(origin, vec![actor, bread]);
        view.entities_at.insert(destination, Vec::new());
        view.adjacent
            .insert(origin, vec![(destination, NonZeroU32::new(2).unwrap())]);
        view.adjacent
            .insert(destination, vec![(origin, NonZeroU32::new(2).unwrap())]);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(2));
        view.controllable.insert((actor, bread));
        view.carry_capacities.insert(actor, LoadUnits(4));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.entity_loads.insert(bread, LoadUnits(2));
        view.thresholds.insert(actor, DriveThresholds::default());
        view.merchandise_profiles.insert(
            actor,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(destination),
            },
        );
        view.demand_memory.insert(
            actor,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(2),
                place: destination,
                tick: Tick(1),
                counterparty: None,
                reason: worldwake_core::DemandObservationReason::WantedToBuyButNoSeller,
            }],
        );

        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination,
            }),
            evidence_entities: BTreeSet::from([bread]),
            evidence_places: BTreeSet::from([origin, destination]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            1,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .unwrap();

        assert_eq!(plan.terminal_kind, PlanTerminalKind::GoalSatisfied);
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::MoveCargo);
        assert_eq!(
            plan.steps[0].payload_override,
            Some(ActionPayload::Transport(TransportActionPayload {
                quantity: Quantity(2),
            }))
        );
        assert_eq!(plan.steps[1].op_kind, PlannerOpKind::Travel);
    }

    #[test]
    fn cargo_search_handles_partial_pickup_split_before_travel() {
        let actor = entity(1);
        let origin = entity(10);
        let destination = entity(11);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, origin, destination, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(origin, EntityKind::Place);
        view.kinds.insert(destination, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, origin);
        view.effective_places.insert(bread, origin);
        view.entities_at.insert(origin, vec![actor, bread]);
        view.entities_at.insert(destination, Vec::new());
        view.adjacent
            .insert(origin, vec![(destination, NonZeroU32::new(2).unwrap())]);
        view.adjacent
            .insert(destination, vec![(origin, NonZeroU32::new(2).unwrap())]);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(3));
        view.controllable.insert((actor, bread));
        view.carry_capacities.insert(actor, LoadUnits(3));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.entity_loads.insert(bread, LoadUnits(3));
        view.thresholds.insert(actor, DriveThresholds::default());
        view.merchandise_profiles.insert(
            actor,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(destination),
            },
        );
        view.demand_memory.insert(
            actor,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(2),
                place: destination,
                tick: Tick(1),
                counterparty: None,
                reason: worldwake_core::DemandObservationReason::WantedToBuyButNoSeller,
            }],
        );

        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination,
            }),
            evidence_entities: BTreeSet::from([bread]),
            evidence_places: BTreeSet::from([origin, destination]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            1,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .unwrap();

        assert_eq!(plan.terminal_kind, PlanTerminalKind::GoalSatisfied);
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::MoveCargo);
        assert_eq!(
            plan.steps[0].payload_override,
            Some(ActionPayload::Transport(TransportActionPayload {
                quantity: Quantity(2),
            }))
        );
        assert_eq!(
            plan.steps[0].targets,
            vec![PlanningEntityRef::Authoritative(bread)]
        );
        assert!(!plan.steps[0].expected_materializations.is_empty());
        assert_eq!(plan.steps[1].op_kind, PlannerOpKind::Travel);
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn authoritative_partial_cargo_pickup_can_reach_goal_satisfaction() {
        let origin = entity(10);
        let destination = entity(11);
        let mut topology = Topology::new();
        topology
            .add_place(
                origin,
                Place {
                    name: "Origin".to_string(),
                    capacity: None,
                    tags: BTreeSet::new(),
                },
            )
            .unwrap();
        topology
            .add_place(
                destination,
                Place {
                    name: "Destination".to_string(),
                    capacity: None,
                    tags: BTreeSet::new(),
                },
            )
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(1), origin, destination, 2, None).unwrap())
            .unwrap();
        topology
            .add_edge(TravelEdge::new(TravelEdgeId(2), destination, origin, 2, None).unwrap())
            .unwrap();

        let mut world = World::new(topology).unwrap();
        let actor;
        let bread;
        {
            let mut txn = WorldTxn::new(
                &mut world,
                Tick(1),
                CauseRef::Bootstrap,
                None,
                None,
                VisibilitySpec::SamePlace,
                WitnessData::default(),
            );
            actor = txn.create_agent("Mira", ControlSource::Ai).unwrap();
            bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(3))
                .unwrap();
            txn.set_ground_location(actor, origin).unwrap();
            txn.set_ground_location(bread, origin).unwrap();
            txn.set_owner(bread, actor).unwrap();
            txn.set_component_homeostatic_needs(actor, HomeostaticNeeds::default())
                .unwrap();
            txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
                .unwrap();
            txn.set_component_drive_thresholds(actor, DriveThresholds::default())
                .unwrap();
            txn.set_component_metabolism_profile(actor, MetabolismProfile::default())
                .unwrap();
            txn.set_component_carry_capacity(actor, CarryCapacity(LoadUnits(3)))
                .unwrap();
            txn.set_component_merchandise_profile(
                actor,
                MerchandiseProfile {
                    sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                    home_market: Some(destination),
                },
            )
            .unwrap();
            txn.set_component_demand_memory(
                actor,
                DemandMemory {
                    observations: vec![DemandObservation {
                        commodity: CommodityKind::Bread,
                        quantity: Quantity(2),
                        place: destination,
                        tick: Tick(1),
                        counterparty: None,
                        reason: DemandObservationReason::WantedToBuyButNoSeller,
                    }],
                },
            )
            .unwrap();
            let mut event_log = EventLog::new();
            let _ = txn.commit(&mut event_log);
        }
        sync_all_beliefs(&mut world, actor, Tick(1));

        let view = PerAgentBeliefView::from_world(actor, &world);
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination,
            }),
            evidence_entities: BTreeSet::from([bread]),
            evidence_places: BTreeSet::from([origin, destination]),
        };
        let (registry, handlers) = build_registry();
        let semantics = build_semantics_table(&registry);
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            1,
        );
        let node = SearchNode {
            state: PlanningState::new(&snapshot),
            steps: Vec::new(),
            total_estimated_ticks: 0,
            heuristic_ticks: 0,
        };

        let initial_candidates =
            search_candidates(&goal, &node, &semantics, &registry, &handlers, None);
        let pick_up = initial_candidates
            .iter()
            .find(|candidate| {
                registry
                    .get(candidate.def_id)
                    .is_some_and(|def| def.name == "pick_up")
            })
            .expect("authoritative snapshot should expose cargo pick_up");
        let (terminal, after_pick_up) = build_successor(
            &goal,
            &semantics,
            &registry,
            &node,
            pick_up,
            &RecipeRegistry::new(),
            &PlanningBudget::default(),
        )
        .unwrap();
        assert_eq!(terminal, None);
        assert_eq!(
            after_pick_up.steps[0].targets,
            vec![PlanningEntityRef::Authoritative(bread)]
        );
        assert!(!after_pick_up.steps[0].expected_materializations.is_empty());

        let follow_up_candidates = search_candidates(
            &goal,
            &after_pick_up,
            &semantics,
            &registry,
            &handlers,
            None,
        );
        let travel = follow_up_candidates
            .iter()
            .find(|candidate| {
                registry
                    .get(candidate.def_id)
                    .is_some_and(|def| def.name == "travel")
                    && candidate.authoritative_targets == vec![destination]
            })
            .expect("partial cargo successor should expose travel to destination");
        let (terminal, _) = build_successor(
            &goal,
            &semantics,
            &registry,
            &after_pick_up,
            travel,
            &RecipeRegistry::new(),
            &PlanningBudget::default(),
        )
        .unwrap();

        assert_eq!(terminal, Some(PlanTerminalKind::GoalSatisfied));
    }

    #[test]
    fn search_uses_hypothetical_movement_to_reduce_local_danger() {
        let actor = entity(1);
        let attacker = entity(2);
        let town = entity(10);
        let refuge = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, attacker, town, refuge]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(attacker, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(refuge, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(attacker, town);
        view.entities_at.insert(town, vec![actor, attacker]);
        view.entities_at.insert(refuge, Vec::new());
        view.adjacent
            .insert(town, vec![(refuge, NonZeroU32::new(2).unwrap())]);
        view.adjacent
            .insert(refuge, vec![(town, NonZeroU32::new(2).unwrap())]);
        view.thresholds.insert(actor, DriveThresholds::default());
        view.hostiles.insert(actor, vec![attacker]);
        view.attackers.insert(actor, vec![attacker]);
        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: GoalKey::from(worldwake_core::GoalKind::ReduceDanger),
            evidence_entities: BTreeSet::from([attacker]),
            evidence_places: BTreeSet::from([town, refuge]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            1,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .unwrap();

        assert_eq!(plan.steps.len(), 1);
        assert!(matches!(
            (plan.steps[0].op_kind, plan.terminal_kind),
            (PlannerOpKind::Travel, PlanTerminalKind::GoalSatisfied)
                | (PlannerOpKind::Defend, PlanTerminalKind::CombatCommitment)
        ));
    }

    #[test]
    fn search_marks_leaf_combat_as_combat_commitment() {
        let actor = entity(1);
        let attacker = entity(2);
        let town = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, attacker, town]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(attacker, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(attacker, town);
        view.entities_at.insert(town, vec![actor, attacker]);
        view.thresholds.insert(actor, DriveThresholds::default());
        view.hostiles.insert(actor, vec![attacker]);
        view.attackers.insert(actor, vec![attacker]);

        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: GoalKey::from(worldwake_core::GoalKind::ReduceDanger),
            evidence_entities: BTreeSet::from([attacker]),
            evidence_places: BTreeSet::from([town]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            0,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .unwrap();

        assert!(matches!(
            plan.steps[0].op_kind,
            PlannerOpKind::Attack | PlannerOpKind::Defend
        ));
        assert_eq!(plan.terminal_kind, PlanTerminalKind::CombatCommitment);
    }

    #[test]
    fn build_successor_estimates_defend_ticks_from_combat_profile() {
        let actor = entity(1);
        let attacker = entity(2);
        let town = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, attacker, town]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(attacker, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(attacker, town);
        view.entities_at.insert(town, vec![actor, attacker]);
        view.thresholds.insert(actor, DriveThresholds::default());
        view.hostiles.insert(actor, vec![attacker]);
        view.attackers.insert(actor, vec![attacker]);

        let (registry, _handlers) = build_registry();
        let semantics_table = build_semantics_table(&registry);
        let defend = registry.iter().find(|def| def.name == "defend").unwrap();
        let goal = GroundedGoal {
            key: GoalKey::from(worldwake_core::GoalKind::ReduceDanger),
            evidence_entities: BTreeSet::from([attacker]),
            evidence_places: BTreeSet::from([town]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            0,
        );
        let node = SearchNode {
            state: PlanningState::new(&snapshot),
            steps: Vec::new(),
            total_estimated_ticks: 0,
            heuristic_ticks: 0,
        };
        let candidate = SearchCandidate {
            def_id: defend.id,
            authoritative_targets: Vec::new(),
            planning_targets: Vec::new(),
            payload_override: None,
        };

        let (_, successor) = build_successor(
            &goal,
            &semantics_table,
            &registry,
            &node,
            &candidate,
            &RecipeRegistry::new(),
            &PlanningBudget::default(),
        )
        .unwrap();

        assert_eq!(successor.steps.len(), 1);
        assert_eq!(successor.steps[0].op_kind, PlannerOpKind::Defend);
        assert_eq!(successor.steps[0].estimated_ticks, 10);
        assert_eq!(successor.total_estimated_ticks, 10);
    }

    #[test]
    fn build_successor_uses_transition_metadata_for_partial_pickup() {
        let (node, _actor, _place, lot, registry, _handlers) =
            pickup_node(CommodityKind::Water, Quantity(3), LoadUnits(4));
        let semantics_table = build_semantics_table(&registry);
        let goal = acquire_goal(CommodityKind::Water);
        let pick_up = registry.iter().find(|def| def.name == "pick_up").unwrap();

        let candidate = SearchCandidate {
            def_id: pick_up.id,
            authoritative_targets: vec![lot],
            planning_targets: vec![PlanningEntityRef::Authoritative(lot)],
            payload_override: None,
        };
        let (_, successor) = build_successor(
            &goal,
            &semantics_table,
            &registry,
            &node,
            &candidate,
            &RecipeRegistry::new(),
            &PlanningBudget::default(),
        )
        .unwrap();

        let step = &successor.steps[0];
        assert_eq!(step.targets, vec![PlanningEntityRef::Authoritative(lot)]);
        assert_eq!(step.expected_materializations.len(), 1);
        assert_eq!(
            step.expected_materializations[0].tag,
            worldwake_sim::MaterializationTag::SplitOffLot
        );
    }

    #[test]
    fn search_adds_put_down_candidate_for_directly_possessed_hypothetical_lot() {
        let (node, _actor, _place, lot, registry, _handlers) =
            pickup_node(CommodityKind::Water, Quantity(3), LoadUnits(4));
        let semantics_table = build_semantics_table(&registry);
        let goal = acquire_goal(CommodityKind::Water);
        let pick_up = registry.iter().find(|def| def.name == "pick_up").unwrap();

        let candidate = SearchCandidate {
            def_id: pick_up.id,
            authoritative_targets: vec![lot],
            planning_targets: vec![PlanningEntityRef::Authoritative(lot)],
            payload_override: None,
        };
        let (_, successor) = build_successor(
            &goal,
            &semantics_table,
            &registry,
            &node,
            &candidate,
            &RecipeRegistry::new(),
            &PlanningBudget::default(),
        )
        .unwrap();

        let candidates = planner_only_candidates(&successor.state, &semantics_table)
            .into_iter()
            .map(search_candidate_from_planner)
            .collect::<Vec<_>>();
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].authoritative_targets.is_empty());
        assert_eq!(candidates[0].payload_override, None);
        assert!(matches!(
            candidates[0].planning_targets.as_slice(),
            [PlanningEntityRef::Hypothetical(_)]
        ));
        let put_down = registry.iter().find(|def| def.name == "put_down").unwrap();
        assert_eq!(candidates[0].def_id, put_down.id);
    }

    #[test]
    fn search_finds_restock_progress_barrier_from_branchy_market_hub() {
        let village_square = prototype_place_entity(PrototypePlace::VillageSquare);
        let orchard_farm = prototype_place_entity(PrototypePlace::OrchardFarm);
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, orchard_row) = {
            let mut txn = WorldTxn::new(
                &mut world,
                Tick(1),
                CauseRef::Bootstrap,
                None,
                None,
                VisibilitySpec::SamePlace,
                WitnessData::default(),
            );
            let actor = txn.create_agent("Merchant", ControlSource::Ai).unwrap();
            let orchard_row = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(actor, village_square).unwrap();
            txn.set_ground_location(orchard_row, orchard_farm).unwrap();
            txn.set_component_homeostatic_needs(actor, HomeostaticNeeds::default())
                .unwrap();
            txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
                .unwrap();
            txn.set_component_drive_thresholds(actor, DriveThresholds::default())
                .unwrap();
            txn.set_component_metabolism_profile(actor, MetabolismProfile::default())
                .unwrap();
            txn.set_component_carry_capacity(actor, CarryCapacity(LoadUnits(50)))
                .unwrap();
            txn.set_component_known_recipes(actor, KnownRecipes::with([RecipeId(0)]))
                .unwrap();
            txn.set_component_workstation_marker(
                orchard_row,
                WorkstationMarker(WorkstationTag::OrchardRow),
            )
            .unwrap();
            txn.set_component_resource_source(
                orchard_row,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(10),
                    max_quantity: Quantity(10),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                },
            )
            .unwrap();
            let mut event_log = EventLog::new();
            let _ = txn.commit(&mut event_log);
            (actor, orchard_row)
        };

        let mut recipes = RecipeRegistry::new();
        recipes.register(harvest_apple_recipe());
        sync_all_beliefs(&mut world, actor, Tick(1));

        let (registry, handlers) = build_registry_with_recipes(&recipes);
        let semantics = build_semantics_table(&registry);
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::RestockCommodity {
                commodity: CommodityKind::Apple,
            }),
            evidence_entities: BTreeSet::from([orchard_row]),
            evidence_places: BTreeSet::from([village_square, orchard_farm]),
        };
        let view = PerAgentBeliefView::from_world(actor, &world);
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            PlanningBudget::default().snapshot_travel_horizon,
        );

        let plan = search_plan(
            &snapshot,
            &goal,
            &semantics,
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .expect("default search budget should find the branchy market-hub restock route");

        assert_eq!(plan.terminal_kind, PlanTerminalKind::ProgressBarrier);
        assert_eq!(plan.steps.len(), 4);
        assert_eq!(
            plan.steps.last().map(|step| step.op_kind),
            Some(PlannerOpKind::Harvest)
        );
    }

    struct ExclusiveOrchardFixture {
        world: World,
        actor: EntityId,
        orchard_farm: EntityId,
        orchard_row: EntityId,
        harvest_action: ActionDefId,
        registry: ActionDefRegistry,
        handlers: worldwake_sim::ActionHandlerRegistry,
        semantics: BTreeMap<ActionDefId, PlannerOpSemantics>,
    }

    fn build_exclusive_orchard_fixture(granted: bool) -> ExclusiveOrchardFixture {
        let orchard_farm = prototype_place_entity(PrototypePlace::OrchardFarm);
        let mut recipes = RecipeRegistry::new();
        recipes.register(harvest_apple_recipe());
        let (registry, handlers) = build_registry_with_recipes(&recipes);
        let harvest_action = registry
            .iter()
            .find(|def| def.name == "harvest:Harvest Apples")
            .map(|def| def.id)
            .expect("harvest action should be registered");
        let semantics = build_semantics_table(&registry);
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, orchard_row) = {
            let mut txn = WorldTxn::new(
                &mut world,
                Tick(1),
                CauseRef::Bootstrap,
                None,
                None,
                VisibilitySpec::SamePlace,
                WitnessData::default(),
            );
            let actor = txn.create_agent("Merchant", ControlSource::Ai).unwrap();
            let orchard_row = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(actor, orchard_farm).unwrap();
            txn.set_ground_location(orchard_row, orchard_farm).unwrap();
            txn.set_component_homeostatic_needs(actor, HomeostaticNeeds::default())
                .unwrap();
            txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
                .unwrap();
            txn.set_component_drive_thresholds(actor, DriveThresholds::default())
                .unwrap();
            txn.set_component_metabolism_profile(actor, MetabolismProfile::default())
                .unwrap();
            txn.set_component_carry_capacity(actor, CarryCapacity(LoadUnits(50)))
                .unwrap();
            txn.set_component_known_recipes(actor, KnownRecipes::with([RecipeId(0)]))
                .unwrap();
            txn.set_component_workstation_marker(
                orchard_row,
                WorkstationMarker(WorkstationTag::OrchardRow),
            )
            .unwrap();
            txn.set_component_resource_source(
                orchard_row,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(10),
                    max_quantity: Quantity(10),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                },
            )
            .unwrap();
            txn.set_component_exclusive_facility_policy(
                orchard_row,
                ExclusiveFacilityPolicy {
                    grant_hold_ticks: NonZeroU32::new(3).unwrap(),
                },
            )
            .unwrap();
            let granted = granted.then_some(GrantedFacilityUse {
                actor,
                intended_action: harvest_action,
                granted_at: Tick(2),
                expires_at: Tick(5),
            });
            txn.set_component_facility_use_queue(
                orchard_row,
                FacilityUseQueue {
                    granted,
                    ..FacilityUseQueue::default()
                },
            )
            .unwrap();
            let mut event_log = EventLog::new();
            let _ = txn.commit(&mut event_log);
            (actor, orchard_row)
        };

        sync_all_beliefs(&mut world, actor, Tick(2));

        ExclusiveOrchardFixture {
            world,
            actor,
            orchard_farm,
            orchard_row,
            harvest_action,
            registry,
            handlers,
            semantics,
        }
    }

    fn enqueue_actor_for_exclusive_fixture(fixture: &mut ExclusiveOrchardFixture, queued_at: Tick) {
        let mut txn = WorldTxn::new(
            &mut fixture.world,
            queued_at,
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        let mut queue = txn
            .get_component_facility_use_queue(fixture.orchard_row)
            .cloned()
            .expect("exclusive fixture should include queue state");
        queue
            .enqueue(fixture.actor, fixture.harvest_action, queued_at)
            .expect("fixture actor should be queueable");
        txn.set_component_facility_use_queue(fixture.orchard_row, queue)
            .unwrap();
        let mut event_log = EventLog::new();
        let _ = txn.commit(&mut event_log);
        sync_all_beliefs(&mut fixture.world, fixture.actor, queued_at);
    }

    #[test]
    fn search_queues_before_harvest_at_exclusive_facility_without_grant() {
        let fixture = build_exclusive_orchard_fixture(false);

        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::RestockCommodity {
                commodity: CommodityKind::Apple,
            }),
            evidence_entities: BTreeSet::from([fixture.orchard_row]),
            evidence_places: BTreeSet::from([fixture.orchard_farm]),
        };
        let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
        let snapshot = build_planning_snapshot(
            &view,
            fixture.actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            PlanningBudget::default().snapshot_travel_horizon,
        );

        let plan = search_plan(
            &snapshot,
            &goal,
            &fixture.semantics,
            &fixture.registry,
            &fixture.handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .expect("exclusive orchard should yield a queue barrier plan");

        assert_eq!(plan.terminal_kind, PlanTerminalKind::ProgressBarrier);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::QueueForFacilityUse);
        assert_eq!(
            plan.steps[0].payload_override,
            Some(ActionPayload::QueueForFacilityUse(
                QueueForFacilityUsePayload {
                    intended_action: fixture.harvest_action,
                },
            ))
        );
        assert_eq!(
            plan.steps[0].targets,
            vec![PlanningEntityRef::Authoritative(fixture.orchard_row)]
        );
    }

    #[test]
    fn search_skips_queue_when_matching_grant_is_already_active() {
        let fixture = build_exclusive_orchard_fixture(true);

        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::RestockCommodity {
                commodity: CommodityKind::Apple,
            }),
            evidence_entities: BTreeSet::from([fixture.orchard_row]),
            evidence_places: BTreeSet::from([fixture.orchard_farm]),
        };
        let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
        let snapshot = build_planning_snapshot(
            &view,
            fixture.actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            PlanningBudget::default().snapshot_travel_horizon,
        );

        let plan = search_plan(
            &snapshot,
            &goal,
            &fixture.semantics,
            &fixture.registry,
            &fixture.handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .expect("matching grant should allow direct harvest plan");

        assert_eq!(plan.terminal_kind, PlanTerminalKind::ProgressBarrier);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Harvest);
        assert_eq!(
            plan.steps[0]
                .payload_override
                .as_ref()
                .and_then(ActionPayload::as_harvest),
            Some(&worldwake_sim::HarvestActionPayload {
                recipe_id: RecipeId(0),
                required_workstation_tag: WorkstationTag::OrchardRow,
                output_commodity: CommodityKind::Apple,
                output_quantity: Quantity(2),
                required_tool_kinds: Vec::new(),
            })
        );
        assert_ne!(plan.steps[0].op_kind, PlannerOpKind::QueueForFacilityUse);
    }

    #[test]
    fn search_does_not_offer_duplicate_queue_candidate_when_actor_is_already_queued() {
        let mut fixture = build_exclusive_orchard_fixture(false);
        enqueue_actor_for_exclusive_fixture(&mut fixture, Tick(2));

        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::RestockCommodity {
                commodity: CommodityKind::Apple,
            }),
            evidence_entities: BTreeSet::from([fixture.orchard_row]),
            evidence_places: BTreeSet::from([fixture.orchard_farm]),
        };
        let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
        let snapshot = build_planning_snapshot(
            &view,
            fixture.actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            PlanningBudget::default().snapshot_travel_horizon,
        );
        let queue_def = fixture
            .registry
            .iter()
            .find(|def| def.name == "queue_for_facility_use")
            .map(|def| def.id)
            .expect("queue action should be registered");

        let candidates = search_candidates(
            &goal,
            &root_node(
                &snapshot,
                &goal,
                &RecipeRegistry::new(),
                &PlanningBudget::default(),
            ),
            &fixture.semantics,
            &fixture.registry,
            &fixture.handlers,
            None,
        );

        assert!(!candidates.iter().any(|candidate| {
            candidate.def_id == queue_def
                && candidate.authoritative_targets == vec![fixture.orchard_row]
        }));
    }

    #[test]
    fn search_filters_blocked_facility_use_from_queue_candidates() {
        let fixture = build_exclusive_orchard_fixture(false);
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::RestockCommodity {
                commodity: CommodityKind::Apple,
            }),
            evidence_entities: BTreeSet::from([fixture.orchard_row]),
            evidence_places: BTreeSet::from([fixture.orchard_farm]),
        };
        let blocked = BlockedIntentMemory {
            intents: vec![BlockedIntent {
                goal_key: goal.key,
                blocking_fact: BlockingFact::ExclusiveFacilityUnavailable,
                related_entity: Some(fixture.orchard_row),
                related_place: Some(fixture.orchard_farm),
                related_action: Some(fixture.harvest_action),
                observed_tick: Tick(2),
                expires_tick: Tick(20),
            }],
        };
        let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
        let snapshot = build_planning_snapshot_with_blocked_facility_uses(
            &view,
            fixture.actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            PlanningBudget::default().snapshot_travel_horizon,
            &blocked,
            Tick(3),
        );
        let queue_def = fixture
            .registry
            .iter()
            .find(|def| def.name == "queue_for_facility_use")
            .map(|def| def.id)
            .expect("queue action should be registered");

        let candidates = search_candidates(
            &goal,
            &root_node(
                &snapshot,
                &goal,
                &RecipeRegistry::new(),
                &PlanningBudget::default(),
            ),
            &fixture.semantics,
            &fixture.registry,
            &fixture.handlers,
            None,
        );

        assert!(!candidates.iter().any(|candidate| {
            candidate.def_id == queue_def
                && candidate.authoritative_targets == vec![fixture.orchard_row]
        }));
    }

    #[test]
    fn search_keeps_other_facility_paths_when_one_exclusive_pair_is_blocked() {
        let mut fixture = build_exclusive_orchard_fixture(false);
        let second_orchard = {
            let mut txn = WorldTxn::new(
                &mut fixture.world,
                Tick(2),
                CauseRef::Bootstrap,
                None,
                None,
                VisibilitySpec::SamePlace,
                WitnessData::default(),
            );
            let orchard_row = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(orchard_row, fixture.orchard_farm)
                .unwrap();
            txn.set_component_workstation_marker(
                orchard_row,
                WorkstationMarker(WorkstationTag::OrchardRow),
            )
            .unwrap();
            txn.set_component_resource_source(
                orchard_row,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(10),
                    max_quantity: Quantity(10),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                },
            )
            .unwrap();
            txn.set_component_exclusive_facility_policy(
                orchard_row,
                ExclusiveFacilityPolicy {
                    grant_hold_ticks: NonZeroU32::new(3).unwrap(),
                },
            )
            .unwrap();
            txn.set_component_facility_use_queue(orchard_row, FacilityUseQueue::default())
                .unwrap();
            let mut event_log = EventLog::new();
            let _ = txn.commit(&mut event_log);
            orchard_row
        };
        sync_all_beliefs(&mut fixture.world, fixture.actor, Tick(2));
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::RestockCommodity {
                commodity: CommodityKind::Apple,
            }),
            evidence_entities: BTreeSet::from([fixture.orchard_row, second_orchard]),
            evidence_places: BTreeSet::from([fixture.orchard_farm]),
        };
        let blocked = BlockedIntentMemory {
            intents: vec![BlockedIntent {
                goal_key: goal.key,
                blocking_fact: BlockingFact::ExclusiveFacilityUnavailable,
                related_entity: Some(fixture.orchard_row),
                related_place: Some(fixture.orchard_farm),
                related_action: Some(fixture.harvest_action),
                observed_tick: Tick(2),
                expires_tick: Tick(20),
            }],
        };
        let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
        let snapshot = build_planning_snapshot_with_blocked_facility_uses(
            &view,
            fixture.actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            PlanningBudget::default().snapshot_travel_horizon,
            &blocked,
            Tick(3),
        );

        let plan = search_plan(
            &snapshot,
            &goal,
            &fixture.semantics,
            &fixture.registry,
            &fixture.handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .expect("second facility should still yield a queue-backed plan");

        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::QueueForFacilityUse);
        assert_eq!(
            plan.steps[0]
                .targets
                .first()
                .copied()
                .and_then(crate::authoritative_target),
            Some(second_orchard)
        );
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn queue_affordance_expands_to_one_candidate_per_matching_intended_action() {
        let orchard_farm = prototype_place_entity(PrototypePlace::OrchardFarm);
        let mut recipes = RecipeRegistry::new();
        recipes.register(harvest_apple_recipe_variant("Harvest Apples Alpha", 2));
        recipes.register(harvest_apple_recipe_variant("Harvest Apples Beta", 1));
        let (registry, _handlers) = build_registry_with_recipes(&recipes);
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, orchard_row) = {
            let mut txn = WorldTxn::new(
                &mut world,
                Tick(1),
                CauseRef::Bootstrap,
                None,
                None,
                VisibilitySpec::SamePlace,
                WitnessData::default(),
            );
            let actor = txn.create_agent("Merchant", ControlSource::Ai).unwrap();
            let orchard_row = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(actor, orchard_farm).unwrap();
            txn.set_ground_location(orchard_row, orchard_farm).unwrap();
            txn.set_component_homeostatic_needs(actor, HomeostaticNeeds::default())
                .unwrap();
            txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
                .unwrap();
            txn.set_component_drive_thresholds(actor, DriveThresholds::default())
                .unwrap();
            txn.set_component_metabolism_profile(actor, MetabolismProfile::default())
                .unwrap();
            txn.set_component_carry_capacity(actor, CarryCapacity(LoadUnits(50)))
                .unwrap();
            txn.set_component_known_recipes(actor, KnownRecipes::with([RecipeId(0), RecipeId(1)]))
                .unwrap();
            txn.set_component_workstation_marker(
                orchard_row,
                WorkstationMarker(WorkstationTag::OrchardRow),
            )
            .unwrap();
            txn.set_component_resource_source(
                orchard_row,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(10),
                    max_quantity: Quantity(10),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                },
            )
            .unwrap();
            txn.set_component_exclusive_facility_policy(
                orchard_row,
                ExclusiveFacilityPolicy {
                    grant_hold_ticks: NonZeroU32::new(3).unwrap(),
                },
            )
            .unwrap();
            txn.set_component_facility_use_queue(orchard_row, FacilityUseQueue::default())
                .unwrap();
            let mut event_log = EventLog::new();
            let _ = txn.commit(&mut event_log);
            (actor, orchard_row)
        };
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::RestockCommodity {
                commodity: CommodityKind::Apple,
            }),
            evidence_entities: BTreeSet::from([orchard_row]),
            evidence_places: BTreeSet::from([orchard_farm]),
        };
        {
            let mut store = world
                .get_component_agent_belief_store(actor)
                .cloned()
                .expect("actor must have AgentBeliefStore");
            store.update_entity(
                orchard_row,
                build_believed_entity_state(
                    &world,
                    orchard_row,
                    Tick(0),
                    PerceptionSource::DirectObservation,
                )
                .expect("orchard facility should build a believed snapshot"),
            );
            let mut txn = WorldTxn::new(
                &mut world,
                Tick(0),
                CauseRef::Bootstrap,
                None,
                None,
                VisibilitySpec::Hidden,
                WitnessData::default(),
            );
            txn.set_component_agent_belief_store(actor, store)
                .expect("test should keep belief stores writable");
            let mut event_log = EventLog::new();
            let _ = txn.commit(&mut event_log);
        }
        let view = PerAgentBeliefView::from_world(actor, &world);
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            PlanningBudget::default().snapshot_travel_horizon,
        );
        let state = PlanningState::new(&snapshot);
        let affordance = Affordance {
            actor,
            def_id: registry
                .iter()
                .find(|def| def.name == "queue_for_facility_use")
                .map(|def| def.id)
                .expect("queue action should be registered"),
            bound_targets: vec![orchard_row],
            payload_override: None,
            explanation: None,
        };

        let queue_candidates =
            search_candidates_from_affordance(&goal, &state, &registry, &affordance);

        assert_eq!(queue_candidates.len(), 2);
        let intended_actions = queue_candidates
            .iter()
            .filter_map(|candidate| {
                candidate
                    .payload_override
                    .as_ref()
                    .and_then(ActionPayload::as_queue_for_facility_use)
                    .map(|payload| payload.intended_action)
            })
            .collect::<BTreeSet<_>>();
        let expected_actions = registry
            .iter()
            .filter(|def| {
                matches!(def.payload.as_harvest(), Some(payload)
                    if payload.output_commodity == CommodityKind::Apple
                        && payload.required_workstation_tag == WorkstationTag::OrchardRow)
            })
            .map(|def| def.id)
            .collect::<BTreeSet<_>>();
        assert_eq!(intended_actions, expected_actions);
    }

    // ── A* heuristic tests ──────────────────────────────────────────────

    /// Build a 3-place chain: `place_a` --3--> `place_b` --5--> `place_c`
    /// Actor starts at `place_a`.
    fn build_chain_heuristic_view() -> (TestBeliefView, EntityId, EntityId, EntityId, EntityId) {
        let actor = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let place_c = entity(12);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, place_a, place_b, place_c]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place_a, EntityKind::Place);
        view.kinds.insert(place_b, EntityKind::Place);
        view.kinds.insert(place_c, EntityKind::Place);
        view.effective_places.insert(actor, place_a);
        view.entities_at.insert(place_a, vec![actor]);
        view.entities_at.insert(place_b, Vec::new());
        view.entities_at.insert(place_c, Vec::new());
        // A --3--> B --5--> C (bidirectional)
        view.adjacent
            .insert(place_a, vec![(place_b, NonZeroU32::new(3).unwrap())]);
        view.adjacent.insert(
            place_b,
            vec![
                (place_a, NonZeroU32::new(3).unwrap()),
                (place_c, NonZeroU32::new(5).unwrap()),
            ],
        );
        view.adjacent
            .insert(place_c, vec![(place_b, NonZeroU32::new(5).unwrap())]);
        (view, actor, place_a, place_b, place_c)
    }

    fn build_branching_care_view() -> (
        TestBeliefView,
        EntityId,
        EntityId,
        EntityId,
        EntityId,
        EntityId,
    ) {
        let actor = entity(1);
        let patient = entity(2);
        let current_place = entity(10);
        let patient_place = entity(11);
        let medicine_place = entity(12);
        let medicine = entity(20);

        let mut view = TestBeliefView::default();
        view.alive.extend([
            actor,
            patient,
            current_place,
            patient_place,
            medicine_place,
            medicine,
        ]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(patient, EntityKind::Agent);
        view.kinds.insert(current_place, EntityKind::Place);
        view.kinds.insert(patient_place, EntityKind::Place);
        view.kinds.insert(medicine_place, EntityKind::Place);
        view.kinds.insert(medicine, EntityKind::ItemLot);
        view.effective_places.insert(actor, current_place);
        view.effective_places.insert(patient, patient_place);
        view.effective_places.insert(medicine, medicine_place);
        view.entities_at.insert(current_place, vec![actor]);
        view.entities_at.insert(patient_place, vec![patient]);
        view.entities_at.insert(medicine_place, vec![medicine]);
        view.adjacent.insert(
            current_place,
            vec![
                (patient_place, NonZeroU32::new(2).unwrap()),
                (medicine_place, NonZeroU32::new(2).unwrap()),
            ],
        );
        view.adjacent.insert(
            patient_place,
            vec![(current_place, NonZeroU32::new(2).unwrap())],
        );
        view.adjacent.insert(
            medicine_place,
            vec![(current_place, NonZeroU32::new(2).unwrap())],
        );
        view.controllable.insert((actor, medicine));
        view.lot_commodities
            .insert(medicine, CommodityKind::Medicine);
        view.commodity_quantities
            .insert((medicine, CommodityKind::Medicine), Quantity(1));
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.entity_loads.insert(
            medicine,
            LoadUnits(worldwake_core::load_per_unit(CommodityKind::Medicine).0),
        );

        (
            view,
            actor,
            patient,
            current_place,
            patient_place,
            medicine_place,
        )
    }

    #[test]
    fn heuristic_is_zero_when_actor_at_goal_relevant_place() {
        let (view, actor, place_a, _place_b, _place_c) = build_chain_heuristic_view();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::from([place_a]),
            3,
        );
        let state = PlanningState::new(&snapshot);
        assert_eq!(compute_heuristic(&snapshot, &state, &[place_a]), 0);
    }

    #[test]
    fn heuristic_equals_shortest_path_distance_to_goal_place() {
        let (view, actor, _place_a, _place_b, place_c) = build_chain_heuristic_view();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::from([place_c]),
            3,
        );
        // Actor at place_a, goal at place_c: shortest path is A->B(3)+B->C(5)=8
        let state = PlanningState::new(&snapshot);
        assert_eq!(compute_heuristic(&snapshot, &state, &[place_c]), 8);
    }

    #[test]
    fn heuristic_picks_nearest_among_multiple_goal_places() {
        let (view, actor, _place_a, place_b, place_c) = build_chain_heuristic_view();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::from([place_b, place_c]),
            3,
        );
        // Actor at place_a: B is 3 ticks, C is 8 ticks → min is 3
        let state = PlanningState::new(&snapshot);
        assert_eq!(compute_heuristic(&snapshot, &state, &[place_b, place_c]), 3);
    }

    #[test]
    fn heuristic_is_zero_when_goal_relevant_places_empty() {
        let (view, actor, _place_a, _place_b, _place_c) = build_chain_heuristic_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 3);
        let state = PlanningState::new(&snapshot);
        assert_eq!(compute_heuristic(&snapshot, &state, &[]), 0);
    }

    #[test]
    fn compare_search_nodes_orders_by_f_cost() {
        let (view, actor, place_a, _place_b, _place_c) = build_chain_heuristic_view();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::from([place_a]),
            3,
        );
        // Node with lower f = g + h should come first.
        let low_f = SearchNode {
            state: PlanningState::new(&snapshot),
            steps: Vec::new(),
            total_estimated_ticks: 2,
            heuristic_ticks: 1, // f = 3
        };
        let high_f = SearchNode {
            state: PlanningState::new(&snapshot),
            steps: Vec::new(),
            total_estimated_ticks: 3,
            heuristic_ticks: 2, // f = 5
        };
        assert_eq!(compare_search_nodes(&low_f, &high_f), Ordering::Less);
        assert_eq!(compare_search_nodes(&high_f, &low_f), Ordering::Greater);
    }

    #[test]
    fn compare_search_nodes_equal_f_prefers_lower_g() {
        let (view, actor, place_a, _place_b, _place_c) = build_chain_heuristic_view();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::from([place_a]),
            3,
        );
        // Both f = 5, but different g. Prefer lower g (less committed cost).
        let low_g = SearchNode {
            state: PlanningState::new(&snapshot),
            steps: Vec::new(),
            total_estimated_ticks: 2,
            heuristic_ticks: 3, // f = 5, g = 2
        };
        let high_g = SearchNode {
            state: PlanningState::new(&snapshot),
            steps: Vec::new(),
            total_estimated_ticks: 3,
            heuristic_ticks: 2, // f = 5, g = 3
        };
        assert_eq!(compare_search_nodes(&low_g, &high_g), Ordering::Less);
    }

    #[test]
    fn search_with_empty_goal_places_degrades_to_uniform_cost() {
        // When goal_relevant_places is empty, all heuristic_ticks are 0,
        // so ordering matches pure g-cost (the pre-A* behavior).
        let (view, actor, place_a, _place_b, _place_c) = build_chain_heuristic_view();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::from([place_a]),
            3,
        );
        let node_a = SearchNode {
            state: PlanningState::new(&snapshot),
            steps: Vec::new(),
            total_estimated_ticks: 5,
            heuristic_ticks: 0,
        };
        let node_b = SearchNode {
            state: PlanningState::new(&snapshot),
            steps: Vec::new(),
            total_estimated_ticks: 3,
            heuristic_ticks: 0,
        };
        // Pure g-cost: node_b (3) < node_a (5)
        assert_eq!(compare_search_nodes(&node_b, &node_a), Ordering::Less);
    }

    // ── Travel pruning tests ──────────────────────────────────────────────

    /// Build a hub topology for pruning tests:
    ///
    ///   north(13) --3-- hub(10) --5-- east(11)
    ///                     |
    ///                     4
    ///                     |
    ///                   south(12)
    ///
    /// Actor starts at hub. `goal_store(14)` is adjacent to east(11) at cost 2.
    fn build_hub_pruning_view() -> (
        TestBeliefView,
        EntityId,
        EntityId,
        EntityId,
        EntityId,
        EntityId,
        EntityId,
    ) {
        let actor = entity(1);
        let hub = entity(10);
        let east = entity(11);
        let south = entity(12);
        let north = entity(13);
        let goal_store = entity(14);

        let mut view = TestBeliefView::default();
        view.alive
            .extend([actor, hub, east, south, north, goal_store]);
        view.kinds.insert(actor, EntityKind::Agent);
        for &place in &[hub, east, south, north, goal_store] {
            view.kinds.insert(place, EntityKind::Place);
        }
        view.effective_places.insert(actor, hub);
        view.entities_at.insert(hub, vec![actor]);

        view.adjacent.insert(
            hub,
            vec![
                (east, NonZeroU32::new(5).unwrap()),
                (south, NonZeroU32::new(4).unwrap()),
                (north, NonZeroU32::new(3).unwrap()),
            ],
        );
        view.adjacent.insert(
            east,
            vec![
                (hub, NonZeroU32::new(5).unwrap()),
                (goal_store, NonZeroU32::new(2).unwrap()),
            ],
        );
        view.adjacent
            .insert(south, vec![(hub, NonZeroU32::new(4).unwrap())]);
        view.adjacent
            .insert(north, vec![(hub, NonZeroU32::new(3).unwrap())]);
        view.adjacent
            .insert(goal_store, vec![(east, NonZeroU32::new(2).unwrap())]);

        (view, actor, hub, east, south, north, goal_store)
    }

    fn make_travel_candidate(def_id: ActionDefId, destination: EntityId) -> SearchCandidate {
        SearchCandidate {
            def_id,
            authoritative_targets: vec![destination],
            planning_targets: vec![PlanningEntityRef::Authoritative(destination)],
            payload_override: None,
        }
    }

    fn make_non_travel_candidate(def_id: ActionDefId, target: EntityId) -> SearchCandidate {
        SearchCandidate {
            def_id,
            authoritative_targets: vec![target],
            planning_targets: vec![PlanningEntityRef::Authoritative(target)],
            payload_override: None,
        }
    }

    fn travel_semantics() -> PlannerOpSemantics {
        PlannerOpSemantics {
            op_kind: PlannerOpKind::Travel,
            may_appear_mid_plan: true,
            is_materialization_barrier: false,
            transition_kind: PlannerTransitionKind::GoalModelFallback,
            relevant_goal_kinds: &[],
        }
    }

    fn harvest_semantics() -> PlannerOpSemantics {
        PlannerOpSemantics {
            op_kind: PlannerOpKind::Harvest,
            may_appear_mid_plan: true,
            is_materialization_barrier: true,
            transition_kind: PlannerTransitionKind::GoalModelFallback,
            relevant_goal_kinds: &[],
        }
    }

    #[test]
    fn prune_travel_keeps_only_toward_goal() {
        // Actor at hub, goal at goal_store.
        // hub→east (dist to goal_store: 5+2=7 via east) vs hub distance = 5+2=7
        // Actually: hub→east: east is 2 ticks from goal_store. hub is 5+2=7.
        // hub→south: south is dead-end, dist to goal_store = 4+5+2 (back through hub) or None if no path.
        // hub→north: north is dead-end similarly.
        // So only east should survive.
        let (view, actor, hub, east, south, north, goal_store) = build_hub_pruning_view();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::from([hub, east, south, north, goal_store]),
            5,
        );

        let travel_east_id = ActionDefId(100);
        let travel_south_id = ActionDefId(101);
        let travel_north_id = ActionDefId(102);

        let mut semantics_table = BTreeMap::new();
        semantics_table.insert(travel_east_id, travel_semantics());
        semantics_table.insert(travel_south_id, travel_semantics());
        semantics_table.insert(travel_north_id, travel_semantics());

        let mut candidates = vec![
            make_travel_candidate(travel_east_id, east),
            make_travel_candidate(travel_south_id, south),
            make_travel_candidate(travel_north_id, north),
        ];

        let pruning = prune_travel_away_from_goal(
            &mut candidates,
            hub,
            &[goal_store],
            &snapshot,
            &semantics_table,
        )
        .expect("deterministic hub pruning should return a structured pruning summary");

        // Only travel to east should survive (dest_min=2 < current_min=7).
        // south and north are dead-ends farther from goal.
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].def_id, travel_east_id);
        assert_eq!(pruning.current_place, hub);
        assert_eq!(pruning.current_remaining_travel_ticks, 7);
        assert_eq!(
            pruning.retained,
            vec![crate::decision_trace::TravelSuccessorTrace {
                destination: east,
                remaining_travel_ticks: 2,
            }]
        );
        assert_eq!(
            pruning.pruned,
            vec![
                crate::decision_trace::TravelSuccessorTrace {
                    destination: south,
                    remaining_travel_ticks: 11,
                },
                crate::decision_trace::TravelSuccessorTrace {
                    destination: north,
                    remaining_travel_ticks: 10,
                },
            ]
        );
    }

    #[test]
    fn prune_travel_noop_when_goal_places_empty() {
        let (view, actor, hub, east, south, north, _goal_store) = build_hub_pruning_view();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::from([hub, east, south, north]),
            5,
        );

        let travel_east_id = ActionDefId(100);
        let travel_south_id = ActionDefId(101);

        let mut semantics_table = BTreeMap::new();
        semantics_table.insert(travel_east_id, travel_semantics());
        semantics_table.insert(travel_south_id, travel_semantics());

        let mut candidates = vec![
            make_travel_candidate(travel_east_id, east),
            make_travel_candidate(travel_south_id, south),
        ];

        prune_travel_away_from_goal(&mut candidates, hub, &[], &snapshot, &semantics_table);

        assert_eq!(
            candidates.len(),
            2,
            "no candidates should be pruned when goal_places is empty"
        );
    }

    #[test]
    fn prune_travel_never_prunes_non_travel_actions() {
        let (view, actor, hub, east, south, north, goal_store) = build_hub_pruning_view();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::from([hub, east, south, north, goal_store]),
            5,
        );

        let harvest_id = ActionDefId(200);
        let trade_id = ActionDefId(201);

        let mut semantics_table = BTreeMap::new();
        semantics_table.insert(harvest_id, harvest_semantics());
        semantics_table.insert(
            trade_id,
            PlannerOpSemantics {
                op_kind: PlannerOpKind::Trade,
                may_appear_mid_plan: true,
                is_materialization_barrier: true,
                transition_kind: PlannerTransitionKind::GoalModelFallback,
                relevant_goal_kinds: &[],
            },
        );

        let mut candidates = vec![
            make_non_travel_candidate(harvest_id, south),
            make_non_travel_candidate(trade_id, north),
        ];

        prune_travel_away_from_goal(
            &mut candidates,
            hub,
            &[goal_store],
            &snapshot,
            &semantics_table,
        );

        assert_eq!(
            candidates.len(),
            2,
            "non-travel candidates must never be pruned"
        );
    }

    #[test]
    fn prune_travel_retains_equal_distance() {
        // Linear topology: A --3--> B --3--> C
        // Actor at B, goal at C. dist(B,C) = 3. dist(A,C) = 6.
        // Travel to A: dest_min=6 > current_min=3 → pruned.
        // Travel to C: dest_min=0 <= current_min=3 → retained.
        // But also test equal distance: if there were a D where dist(D,C) = 3,
        // it should be retained (dest_min == current_min).
        //
        // We use the chain view: A --3--> B --5--> C
        // Actor at B. dist(B,C)=5. dist(A,C)=8.
        // Travel to A: dest_min=8 > 5 → pruned.
        // Travel to C: dest_min=0 <= 5 → retained.
        let (view, actor, place_a, place_b, place_c) = build_chain_heuristic_view();
        // Move actor to place_b for this test.
        let mut view = view;
        view.effective_places.insert(actor, place_b);
        view.entities_at.insert(place_a, Vec::new());
        view.entities_at.insert(place_b, vec![actor]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::from([place_a, place_b, place_c]),
            3,
        );

        let retreat_travel_id = ActionDefId(100);
        let goalward_travel_id = ActionDefId(101);

        let mut semantics_table = BTreeMap::new();
        semantics_table.insert(retreat_travel_id, travel_semantics());
        semantics_table.insert(goalward_travel_id, travel_semantics());

        let mut candidates = vec![
            make_travel_candidate(retreat_travel_id, place_a),
            make_travel_candidate(goalward_travel_id, place_c),
        ];

        prune_travel_away_from_goal(
            &mut candidates,
            place_b,
            &[place_c],
            &snapshot,
            &semantics_table,
        );

        // Travel to C is retained (closer), travel to A is pruned (farther).
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].def_id, goalward_travel_id);
    }

    #[test]
    fn prune_travel_retains_only_path_forward_in_linear_topology() {
        // Chain: A --3--> B --5--> C
        // Actor at A, goal at C. Only one travel option: A→B.
        // dist(A,C) = 8, dist(B,C) = 5. 5 <= 8 → retained.
        let (view, actor, _place_a, place_b, place_c) = build_chain_heuristic_view();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::from([place_b, place_c]),
            3,
        );

        let travel_b_id = ActionDefId(100);
        let mut semantics_table = BTreeMap::new();
        semantics_table.insert(travel_b_id, travel_semantics());

        let mut candidates = vec![make_travel_candidate(travel_b_id, place_b)];

        let place_a = entity(10); // actor is at place_a
        prune_travel_away_from_goal(
            &mut candidates,
            place_a,
            &[place_c],
            &snapshot,
            &semantics_table,
        );

        assert_eq!(candidates.len(), 1, "only path forward must be retained");
        assert_eq!(candidates[0].def_id, travel_b_id);
    }

    #[test]
    fn prune_travel_at_goal_place_still_prunes_against_alternative_places() {
        // When the actor is already at one goal-relevant place but needs to
        // leave for another, pruning should keep only routes that progress
        // toward the alternative relevant place.
        let (view, actor, hub, east, south, north, goal_store) = build_hub_pruning_view();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::new(),
            &BTreeSet::from([hub, east, south, north, goal_store]),
            5,
        );

        let travel_east_id = ActionDefId(100);
        let travel_south_id = ActionDefId(101);
        let travel_north_id = ActionDefId(102);

        let mut semantics_table = BTreeMap::new();
        semantics_table.insert(travel_east_id, travel_semantics());
        semantics_table.insert(travel_south_id, travel_semantics());
        semantics_table.insert(travel_north_id, travel_semantics());

        let mut candidates = vec![
            make_travel_candidate(travel_east_id, east),
            make_travel_candidate(travel_south_id, south),
            make_travel_candidate(travel_north_id, north),
        ];

        let pruning = prune_travel_away_from_goal(
            &mut candidates,
            hub,
            &[hub, goal_store],
            &snapshot,
            &semantics_table,
        )
        .expect("alternative-place pruning should produce a trace");

        assert_eq!(
            candidates.len(),
            1,
            "only the route that progresses toward the alternative relevant place should survive"
        );
        assert_eq!(candidates[0].def_id, travel_east_id);
        assert_eq!(pruning.current_place, hub);
        assert_eq!(pruning.current_remaining_travel_ticks, 7);
    }

    #[test]
    fn combined_places_include_remote_medicine_lot_for_treat_wounds() {
        let (view, actor, patient, _current_place, patient_place, medicine_place) =
            build_branching_care_view();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([patient]),
            &BTreeSet::from([patient_place, medicine_place]),
            2,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::TreatWounds { patient }),
            evidence_entities: BTreeSet::from([patient]),
            evidence_places: BTreeSet::from([patient_place, medicine_place]),
        };

        let places = combined_relevant_places(
            &goal,
            &state,
            &RecipeRegistry::new(),
            &PlanningBudget::default(),
        );

        assert!(places.contains(&patient_place));
        assert!(places.contains(&medicine_place));
    }

    #[test]
    fn combined_places_drop_medicine_place_after_hypothetical_pick_up() {
        let (view, actor, patient, _current_place, patient_place, medicine_place) =
            build_branching_care_view();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([patient]),
            &BTreeSet::from([patient_place, medicine_place]),
            2,
        );
        let (registry, handlers) = build_registry();
        let semantics = build_semantics_table(&registry);
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::TreatWounds { patient }),
            evidence_entities: BTreeSet::from([patient]),
            evidence_places: BTreeSet::from([patient_place, medicine_place]),
        };
        let node = SearchNode {
            state: PlanningState::new(&snapshot).move_actor_to(medicine_place),
            steps: Vec::new(),
            total_estimated_ticks: 0,
            heuristic_ticks: 0,
        };

        let pick_up = search_candidates(&goal, &node, &semantics, &registry, &handlers, None)
            .into_iter()
            .find(|candidate| {
                registry
                    .get(candidate.def_id)
                    .is_some_and(|def| def.name == "pick_up")
            })
            .expect("moved actor should expose a medicine pick_up candidate");

        let (_, successor) = build_successor(
            &goal,
            &semantics,
            &registry,
            &node,
            &pick_up,
            &RecipeRegistry::new(),
            &PlanningBudget::default(),
        )
        .expect("hypothetical pick_up should build a successor");

        let places = combined_relevant_places(
            &goal,
            &successor.state,
            &RecipeRegistry::new(),
            &PlanningBudget::default(),
        );

        assert_eq!(places, vec![patient_place]);
    }

    #[test]
    fn prune_travel_retains_remote_medicine_branch_for_treat_wounds() {
        let (view, actor, patient, current_place, patient_place, medicine_place) =
            build_branching_care_view();
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([patient]),
            &BTreeSet::from([patient_place, medicine_place]),
            2,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::TreatWounds { patient }),
            evidence_entities: BTreeSet::from([patient]),
            evidence_places: BTreeSet::from([patient_place, medicine_place]),
        };
        let goal_places = combined_relevant_places(
            &goal,
            &state,
            &RecipeRegistry::new(),
            &PlanningBudget::default(),
        );

        let travel_patient_id = ActionDefId(500);
        let travel_medicine_id = ActionDefId(501);
        let mut semantics_table = BTreeMap::new();
        semantics_table.insert(travel_patient_id, travel_semantics());
        semantics_table.insert(travel_medicine_id, travel_semantics());

        let mut candidates = vec![
            make_travel_candidate(travel_patient_id, patient_place),
            make_travel_candidate(travel_medicine_id, medicine_place),
        ];

        prune_travel_away_from_goal(
            &mut candidates,
            current_place,
            &goal_places,
            &snapshot,
            &semantics_table,
        );

        assert_eq!(candidates.len(), 2);
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.authoritative_targets == vec![medicine_place]),
            "remote medicine travel should remain available for TreatWounds"
        );
    }

    #[test]
    fn treat_wounds_search_candidates_include_pick_up_at_medicine_location() {
        let (mut view, actor, patient, _current_place, patient_place, medicine_place) =
            build_branching_care_view();
        view.effective_places.insert(actor, medicine_place);
        view.entities_at
            .insert(medicine_place, vec![actor, entity(20)]);
        view.entities_at.insert(entity(10), Vec::new());

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([patient]),
            &BTreeSet::from([patient_place, medicine_place]),
            2,
        );
        let (registry, handlers) = build_registry();
        let semantics = build_semantics_table(&registry);
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::TreatWounds { patient }),
            evidence_entities: BTreeSet::from([patient]),
            evidence_places: BTreeSet::from([patient_place, medicine_place]),
        };
        let node = root_node(
            &snapshot,
            &goal,
            &RecipeRegistry::new(),
            &PlanningBudget::default(),
        );

        let candidates = search_candidates(&goal, &node, &semantics, &registry, &handlers, None);

        assert!(
            candidates.iter().any(|candidate| {
                registry
                    .get(candidate.def_id)
                    .is_some_and(|def| def.name == "pick_up")
            }),
            "TreatWounds should consider pick_up when remote medicine is co-located"
        );
    }

    // ── S03PLATARIDE-004: Search integration tests for exact target binding ──

    #[test]
    fn test_binding_two_corpses_same_place() {
        let actor = entity(1);
        let corpse_x = entity(2);
        let corpse_y = entity(3);
        let town = entity(10);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town]);
        // corpse_x and corpse_y are NOT in alive → is_dead returns true
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(corpse_x, EntityKind::Agent);
        view.kinds.insert(corpse_y, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(corpse_x, town);
        view.effective_places.insert(corpse_y, town);
        view.entities_at
            .insert(town, vec![actor, corpse_x, corpse_y]);
        view.thresholds.insert(actor, DriveThresholds::default());
        // Corpses must have commodities so LootCorpse is not immediately satisfied.
        view.commodity_quantities
            .insert((corpse_x, CommodityKind::Coin), Quantity(3));
        view.commodity_quantities
            .insert((corpse_y, CommodityKind::Coin), Quantity(2));

        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::LootCorpse { corpse: corpse_x }),
            evidence_entities: BTreeSet::from([corpse_x, corpse_y]),
            evidence_places: BTreeSet::from([town]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            0,
        );
        let mut rejections = Vec::new();
        let result = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            Some(&mut rejections),
            None,
        );

        let plan = result.into_plan().expect("search should find a loot plan");
        // LootCorpse is a progress barrier — the Loot step is the terminal step.
        let loot_step = plan
            .steps
            .iter()
            .find(|s| s.op_kind == PlannerOpKind::Loot)
            .expect("plan should contain a Loot step");
        assert!(
            loot_step
                .targets
                .iter()
                .any(|t| matches!(t, PlanningEntityRef::Authoritative(id) if *id == corpse_x)),
            "Loot step must target corpse X"
        );
        assert!(
            !loot_step
                .targets
                .iter()
                .any(|t| matches!(t, PlanningEntityRef::Authoritative(id) if *id == corpse_y)),
            "Loot step must NOT target corpse Y"
        );
        assert!(
            !rejections.is_empty(),
            "wrong-target loot affordance for corpse Y should be rejected"
        );
        assert!(
            rejections
                .iter()
                .any(|r| r.rejected_targets.contains(&corpse_y)),
            "binding rejections must include corpse Y"
        );
    }

    #[test]
    fn test_binding_two_hostiles_same_place() {
        let actor = entity(1);
        let hostile_a = entity(2);
        let hostile_b = entity(3);
        let town = entity(10);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, hostile_a, hostile_b, town]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(hostile_a, EntityKind::Agent);
        view.kinds.insert(hostile_b, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(hostile_a, town);
        view.effective_places.insert(hostile_b, town);
        view.entities_at
            .insert(town, vec![actor, hostile_a, hostile_b]);
        view.thresholds.insert(actor, DriveThresholds::default());
        view.hostiles.insert(actor, vec![hostile_a, hostile_b]);
        view.attackers.insert(actor, vec![hostile_a, hostile_b]);

        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::EngageHostile { target: hostile_a }),
            evidence_entities: BTreeSet::from([hostile_a, hostile_b]),
            evidence_places: BTreeSet::from([town]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            0,
        );
        let mut rejections = Vec::new();
        let result = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            Some(&mut rejections),
            None,
        );

        let plan = result
            .into_plan()
            .expect("search should find an attack plan");
        let attack_step = plan
            .steps
            .iter()
            .find(|s| s.op_kind == PlannerOpKind::Attack)
            .expect("plan should contain an Attack step");
        assert!(
            attack_step
                .targets
                .iter()
                .any(|t| matches!(t, PlanningEntityRef::Authoritative(id) if *id == hostile_a)),
            "Attack step must target hostile A"
        );
        assert!(
            !attack_step
                .targets
                .iter()
                .any(|t| matches!(t, PlanningEntityRef::Authoritative(id) if *id == hostile_b)),
            "Attack step must NOT target hostile B"
        );
        assert!(
            rejections
                .iter()
                .any(|r| r.rejected_targets.contains(&hostile_b)),
            "binding rejections must include hostile B"
        );
    }

    #[test]
    fn test_binding_flexible_goal_unaffected() {
        let actor = entity(1);
        let town = entity(10);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.entities_at.insert(town, vec![actor]);
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(0), pm(0), pm(800), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());

        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::Sleep),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::from([town]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            0,
        );
        let mut rejections = Vec::new();
        let result = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            Some(&mut rejections),
            None,
        );

        let plan = result.into_plan().expect("search should find a sleep plan");
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Sleep);
        assert!(
            rejections.is_empty(),
            "flexible Sleep goal must not produce binding rejections, got: {rejections:?}"
        );
    }

    #[test]
    fn test_binding_rejection_trace_populated() {
        let actor = entity(1);
        let corpse_x = entity(2);
        let corpse_y = entity(3);
        let town = entity(10);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(corpse_x, EntityKind::Agent);
        view.kinds.insert(corpse_y, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(corpse_x, town);
        view.effective_places.insert(corpse_y, town);
        view.entities_at
            .insert(town, vec![actor, corpse_x, corpse_y]);
        view.thresholds.insert(actor, DriveThresholds::default());
        // Corpses must have commodities so LootCorpse is not immediately satisfied.
        view.commodity_quantities
            .insert((corpse_x, CommodityKind::Coin), Quantity(3));
        view.commodity_quantities
            .insert((corpse_y, CommodityKind::Coin), Quantity(2));

        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::LootCorpse { corpse: corpse_x }),
            evidence_entities: BTreeSet::from([corpse_x, corpse_y]),
            evidence_places: BTreeSet::from([town]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            0,
        );
        let mut rejections = Vec::new();
        let _ = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            Some(&mut rejections),
            None,
        );

        // Verify BindingRejection fields are populated correctly.
        let corpse_y_rejection = rejections
            .iter()
            .find(|r| r.rejected_targets.contains(&corpse_y))
            .expect("should have a rejection for corpse Y");

        // def_id should reference the loot action.
        let loot_def = registry
            .iter()
            .find(|d| d.name == "loot")
            .expect("loot action must be registered");
        assert_eq!(
            corpse_y_rejection.def_id, loot_def.id,
            "rejected def_id should match the loot action"
        );

        // required_target should be corpse_x (the goal's canonical target).
        assert_eq!(
            corpse_y_rejection.required_target,
            Some(corpse_x),
            "required_target should be the goal's canonical corpse"
        );
    }

    /// With the deferred `ProgressBarrier` mechanism, a `GoalSatisfied` plan at
    /// depth 2 (`Travel` + `pick_up`) is preferred over a `ProgressBarrier` (`Trade`)
    /// at depth 1.  Before the deferral change, the search would greedily
    /// return the `Trade` `ProgressBarrier` without exploring deeper.
    #[test]
    fn search_defers_progress_barrier_and_prefers_goal_satisfied_at_deeper_level() {
        let actor = entity(1);
        let seller = entity(2);
        let town = entity(10);
        let market = entity(11);
        let bread = entity(20);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, seller, town, market, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(seller, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(market, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(seller, town);
        view.effective_places.insert(bread, market);
        view.entities_at.insert(town, vec![actor, seller]);
        view.entities_at.insert(market, vec![bread]);
        view.adjacent
            .insert(town, vec![(market, NonZeroU32::new(2).unwrap())]);
        view.adjacent
            .insert(market, vec![(town, NonZeroU32::new(2).unwrap())]);
        // Actor has coins for Trade and carry capacity for pick_up.
        view.commodity_quantities
            .insert((actor, CommodityKind::Coin), Quantity(3));
        view.carry_capacities.insert(actor, LoadUnits(4));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.entity_loads.insert(bread, LoadUnits(1));
        // Seller has bread merchandise.
        view.merchandise_profiles.insert(
            seller,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(town),
            },
        );
        view.trade_profiles
            .insert(actor, sample_trade_disposition_profile());
        // Ground bread lot at market.
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(1));
        // Needs/thresholds for the acquire goal context.
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());

        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }),
            evidence_entities: BTreeSet::from([seller, bread]),
            evidence_places: BTreeSet::from([town, market]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            1,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .expect("should find a plan");

        // The search should prefer GoalSatisfied (Travel + pick_up) over the
        // deferred Trade ProgressBarrier.
        assert_eq!(
            plan.terminal_kind,
            PlanTerminalKind::GoalSatisfied,
            "deferred barrier should yield to GoalSatisfied at deeper level"
        );
        assert!(
            plan.steps.len() >= 2,
            "plan should include Travel + pick_up"
        );
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Travel);
        assert_eq!(plan.steps[1].op_kind, PlannerOpKind::MoveCargo);
    }

    /// When only a `ProgressBarrier` exists and no `GoalSatisfied` is reachable,
    /// the deferred barrier is returned as a fallback after the frontier is
    /// exhausted.
    #[test]
    fn search_returns_deferred_barrier_as_fallback_after_frontier_exhaustion() {
        let actor = entity(1);
        let seller = entity(2);
        let town = entity(10);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, seller, town]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(seller, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(seller, town);
        view.entities_at.insert(town, vec![actor, seller]);
        view.commodity_quantities
            .insert((actor, CommodityKind::Coin), Quantity(3));
        view.merchandise_profiles.insert(
            seller,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(town),
            },
        );
        view.trade_profiles
            .insert(actor, sample_trade_disposition_profile());
        view.commodity_quantities
            .insert((seller, CommodityKind::Bread), Quantity(2));
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());

        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }),
            evidence_entities: BTreeSet::from([seller]),
            evidence_places: BTreeSet::from([town]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            1,
        );
        let plan = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None,
        )
        .into_plan()
        .expect("deferred barrier should be returned as fallback");

        assert_eq!(
            plan.terminal_kind,
            PlanTerminalKind::ProgressBarrier,
            "barrier fallback should be returned after frontier exhaustion"
        );
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Trade);
    }

    /// When the node expansion budget is exhausted but a `ProgressBarrier` was
    /// found earlier, the barrier plan is returned instead of `BudgetExhausted`.
    #[test]
    fn search_returns_deferred_barrier_on_budget_exhaustion() {
        let actor = entity(1);
        let seller = entity(2);
        let town = entity(10);
        let market = entity(11);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, seller, town, market]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(seller, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(market, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(seller, town);
        view.entities_at.insert(town, vec![actor, seller]);
        view.entities_at.insert(market, vec![]);
        view.adjacent
            .insert(town, vec![(market, NonZeroU32::new(2).unwrap())]);
        view.adjacent
            .insert(market, vec![(town, NonZeroU32::new(2).unwrap())]);
        view.commodity_quantities
            .insert((actor, CommodityKind::Coin), Quantity(3));
        view.merchandise_profiles.insert(
            seller,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(town),
            },
        );
        view.trade_profiles
            .insert(actor, sample_trade_disposition_profile());
        view.commodity_quantities
            .insert((seller, CommodityKind::Bread), Quantity(2));
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());

        let (registry, handlers) = build_registry();
        let goal = GroundedGoal {
            key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }),
            evidence_entities: BTreeSet::from([seller]),
            evidence_places: BTreeSet::from([town]),
        };
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &goal.evidence_entities,
            &goal.evidence_places,
            1,
        );

        // Tight budget: only 2 expansions.  Expansion 1 finds the Trade
        // ProgressBarrier (deferred).  Expansion 2 exhausts the budget.
        let tight_budget = PlanningBudget {
            max_node_expansions: 2,
            ..PlanningBudget::default()
        };
        let result = search_plan(
            &snapshot,
            &goal,
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &tight_budget,
            &RecipeRegistry::new(),
            None,
            None,
        );

        // Should return the deferred barrier, not BudgetExhausted.
        let plan = result
            .into_plan()
            .expect("deferred barrier should be returned on budget exhaustion");
        assert_eq!(
            plan.terminal_kind,
            PlanTerminalKind::ProgressBarrier,
            "barrier found before budget exhaustion should be returned"
        );
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Trade);
    }

    #[test]
    fn test_binding_empty_targets_planner_only_bypass() {
        let actor = entity(1);
        let corpse_x = entity(2);
        let town = entity(10);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(corpse_x, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(corpse_x, town);
        view.entities_at.insert(town, vec![actor, corpse_x]);
        view.thresholds.insert(actor, DriveThresholds::default());

        let (registry, _handlers) = build_registry();
        let semantics_table = build_semantics_table(&registry);
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([corpse_x]),
            &BTreeSet::from([town]),
            0,
        );
        let state = PlanningState::new(&snapshot);

        // Generate planner-only synthetic candidates and convert to search candidates.
        let planner_candidates: Vec<SearchCandidate> =
            planner_only_candidates(&state, &semantics_table)
                .into_iter()
                .map(search_candidate_from_planner)
                .collect();

        // Every planner-only candidate has empty authoritative_targets after conversion.
        for candidate in &planner_candidates {
            assert!(
                candidate.authoritative_targets.is_empty(),
                "planner-only candidate should have empty authoritative_targets"
            );
        }

        // Verify matches_binding returns true for all of them, even with
        // an exact-bound goal like LootCorpse.
        let goal = GoalKind::LootCorpse { corpse: corpse_x };
        for candidate in &planner_candidates {
            for semantics in semantics_table.values() {
                assert!(
                    goal.matches_binding(&candidate.authoritative_targets, semantics.op_kind),
                    "empty authoritative_targets must bypass binding for any op kind"
                );
            }
        }
    }

    // ── Expansion summary trace tests ──────────────────────────────

    #[test]
    fn search_expansion_summaries_collected_when_tracing_enabled() {
        // Simple 1-step consume plan: actor has bread locally.
        let actor = entity(1);
        let town = entity(10);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town, bread]);
        insert_hungry_actor(&mut view, actor);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(bread, town);
        view.entities_at.insert(town, vec![actor, bread]);
        view.controllable.insert((actor, bread));
        view.direct_possessions.insert(actor, vec![bread]);
        view.direct_possessors.insert(bread, actor);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityKind::Bread.spec().consumable_profile.unwrap(),
        );
        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);

        let mut summaries = Vec::new();
        let result = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            Some(&mut summaries),
        );

        assert!(result.is_found(), "plan should be found");
        assert!(
            !summaries.is_empty(),
            "expansion summaries should be non-empty when tracing is enabled"
        );
        // Depth should start at 0.
        assert_eq!(summaries[0].depth, 0);
        // At least one candidate was generated.
        assert!(summaries[0].candidates_generated > 0);
    }

    #[test]
    fn search_expansion_summaries_empty_when_tracing_disabled() {
        // Same setup as above but with tracing disabled (None).
        let actor = entity(1);
        let town = entity(10);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town, bread]);
        insert_hungry_actor(&mut view, actor);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(bread, town);
        view.entities_at.insert(town, vec![actor, bread]);
        view.controllable.insert((actor, bread));
        view.direct_possessions.insert(actor, vec![bread]);
        view.direct_possessors.insert(bread, actor);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityKind::Bread.spec().consumable_profile.unwrap(),
        );
        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);

        let result = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget::default(),
            &RecipeRegistry::new(),
            None,
            None, // tracing disabled
        );

        assert!(result.is_found(), "plan should be found");
        // No summaries collector was passed — zero-cost path.
    }

    #[test]
    fn beam_truncation_visible_in_expansion_summary() {
        // Setup: actor at town with 2 adjacent places (dead_end, pantry).
        // beam_width=1 forces truncation of one non-terminal successor.
        let actor = entity(1);
        let town = entity(10);
        let dead_end = entity(11);
        let pantry = entity(12);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        let mut pantry_entities = Vec::new();
        view.alive.extend([actor, town, dead_end, pantry]);
        insert_hungry_actor(&mut view, actor);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(dead_end, EntityKind::Place);
        view.kinds.insert(pantry, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.entities_at.insert(town, vec![actor]);
        view.entities_at.insert(dead_end, Vec::new());
        insert_bread_lot(&mut view, actor, bread, pantry, &mut pantry_entities);
        view.entities_at.insert(pantry, pantry_entities);
        view.carry_capacities.insert(actor, LoadUnits(10));
        view.adjacent.insert(
            town,
            vec![
                (dead_end, NonZeroU32::new(1).unwrap()),
                (pantry, NonZeroU32::new(3).unwrap()),
            ],
        );

        let (registry, handlers) = build_registry();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);

        let mut summaries = Vec::new();
        let _result = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget {
                beam_width: 1,
                ..PlanningBudget::default()
            },
            &RecipeRegistry::new(),
            None,
            Some(&mut summaries),
        );

        // The first expansion (depth 0) should show beam truncation:
        // at least 2 travel candidates before beam, truncated to 1.
        assert!(
            !summaries.is_empty(),
            "should have at least one expansion summary"
        );
        let first = &summaries[0];
        assert_eq!(first.depth, 0);
        assert!(
            first.non_terminal_before_beam > first.non_terminal_after_beam,
            "beam truncation should be visible: before={} after={}",
            first.non_terminal_before_beam,
            first.non_terminal_after_beam,
        );
        assert_eq!(
            first.non_terminal_after_beam, 1,
            "beam_width=1 should leave exactly 1 non-terminal successor"
        );
    }
}
