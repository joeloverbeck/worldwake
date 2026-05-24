#![allow(dead_code)]

use crate::{
    GoalKindPlannerExt, GoalOffer, PlannedSkeletonStep, PlannerOpKind, PlanningSnapshot,
    PlanningState,
    decision_trace::{
        MethodPlanAttemptTrace, RejectedMethodTrace, StrategicBudgetTrace, StrategicFallbackReason,
        SubgoalAttemptOutcome, SubgoalAttemptResult,
    },
    htn::{
        ArtifactTemplate, ClaimRequirement, CommodityTemplate, EntityCriterion, EntityTemplate,
        LocationTemplate, MethodPrecondition, MethodSchema, MethodSelection, PayloadTemplate,
        PayloadValueTemplate, RecipeTemplate, SubgoalTemplate, build_method_registry,
        select_method_with_recipes,
    },
    planning_snapshot::AdmissionSource,
    search::PartialPlanSkeletonSource,
};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use worldwake_core::{
    AgentSchemaContextProfile, CommodityKind, EntityId, EntityKind, ExecutionBudget, GoalKind,
    MotiveSourceDiscriminant, MotiveSourceRef, OpportunityAnchor, Quantity, RecipeId,
    WorkstationTag,
};
use worldwake_sim::{
    EconomicBeliefView, EntityBeliefView, FacilityBeliefView, InventoryBeliefView,
    ProfileBeliefView, RecipeRegistry, SpatialBeliefView,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StrategicPlan {
    pub steps: Vec<StrategicStep>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StrategicSearchResult {
    pub plan: Option<StrategicPlan>,
    pub skeleton_source: Option<PartialPlanSkeletonSource>,
    pub budget_trace: Option<StrategicBudgetTrace>,
    pub stages_count: u16,
    pub method_trace: Option<MethodPlanAttemptTrace>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct StrategicStep {
    pub destination: EntityId,
    pub sub_goal: TacticalSubGoal,
    pub estimated_travel_ticks: u32,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum TacticalSubGoal {
    SatisfyGoal,
    AcquirePrerequisite(CommodityKind),
    ExploreWithBarrier,
    ExploreFallback,
    SocialQuery(CommodityKind),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum StrategicStageKind {
    Goal,
    Acquire(CommodityKind),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StrategicStage {
    kind: StrategicStageKind,
    places: Vec<EntityId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SearchNode {
    stage_index: usize,
    current_place: EntityId,
    total_cost: u32,
    steps: Vec<StrategicStep>,
}

impl Ord for SearchNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .total_cost
            .cmp(&self.total_cost)
            .then_with(|| other.stage_index.cmp(&self.stage_index))
            .then_with(|| other.steps.cmp(&self.steps))
    }
}

impl PartialOrd for SearchNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
pub(crate) fn plan(
    snapshot: &PlanningSnapshot,
    goal: &GoalOffer,
    execution_budget: &ExecutionBudget,
    recipes: &RecipeRegistry,
) -> Option<StrategicPlan> {
    plan_with_budget_trace(snapshot, goal, execution_budget, u16::MAX, recipes).plan
}

#[allow(clippy::trivially_copy_pass_by_ref)]
pub(crate) fn plan_with_budget_trace(
    snapshot: &PlanningSnapshot,
    goal: &GoalOffer,
    execution_budget: &ExecutionBudget,
    max_strategic_expansions: u16,
    recipes: &RecipeRegistry,
) -> StrategicSearchResult {
    let state = PlanningState::new(snapshot);
    if root_goal_satisfaction_allowed(goal, &state) && goal.key.kind.is_satisfied(&state) {
        return StrategicSearchResult {
            plan: Some(StrategicPlan { steps: Vec::new() }),
            skeleton_source: None,
            budget_trace: None,
            stages_count: 0,
            method_trace: None,
        };
    }

    let actor = snapshot.actor();
    let Some(actor_place) = state.effective_place(actor) else {
        return StrategicSearchResult {
            plan: None,
            skeleton_source: None,
            budget_trace: None,
            stages_count: 0,
            method_trace: None,
        };
    };
    let goal_places = goal_places(goal, &state, recipes);
    let missing_commodities = missing_commodities(goal, &state, recipes, actor_place, &goal_places);
    let query_commodity = social_query_commodity(goal, &missing_commodities, recipes);
    let method_registry = build_method_registry();
    let default_profile = AgentSchemaContextProfile::default();
    let profile = state
        .agent_schema_context_profile(actor)
        .unwrap_or(default_profile);
    let method_selection = select_method_with_recipes(
        actor,
        goal,
        &method_registry,
        &profile,
        &state,
        &goal.motive_sources,
        Some(recipes),
    );
    let stage_build = build_stages(
        &state,
        snapshot,
        goal,
        recipes,
        &method_selection,
        actor,
        actor_place,
        &goal_places,
        &missing_commodities,
        execution_budget.max_prerequisite_locations(),
    );
    let mut stages = stage_build.stages;

    if stages.is_empty() {
        if goal_places.contains(&actor_place) {
            return StrategicSearchResult {
                plan: Some(StrategicPlan { steps: Vec::new() }),
                skeleton_source: stage_build.skeleton_source,
                budget_trace: None,
                stages_count: 0,
                method_trace: stage_build.method_trace,
            };
        }
        let plan = exploration_plan(snapshot, actor_place, &goal.key.kind)
            .or_else(|| social_query_plan(snapshot, actor_place, query_commodity));
        return StrategicSearchResult {
            plan,
            skeleton_source: stage_build.skeleton_source,
            budget_trace: None,
            stages_count: 0,
            method_trace: stage_build.method_trace,
        };
    }

    let mut local_steps = Vec::new();
    consume_current_place_prerequisites(actor_place, &mut stages, &mut local_steps);

    if stages.is_empty() || matches_local_goal_stage(&stages, actor_place) {
        return StrategicSearchResult {
            plan: Some(StrategicPlan { steps: local_steps }),
            skeleton_source: stage_build.skeleton_source,
            budget_trace: None,
            stages_count: stages.len().min(usize::from(u16::MAX)) as u16,
            method_trace: stage_build.method_trace,
        };
    }

    let stages_count = stages.len().min(usize::from(u16::MAX)) as u16;
    let search_budget = execution_budget
        .strategic_budget_for_stages(stages.len())
        .min(usize::from(max_strategic_expansions));
    let mut frontier = BinaryHeap::new();
    frontier.push(SearchNode {
        stage_index: 0,
        current_place: actor_place,
        total_cost: 0,
        steps: local_steps,
    });
    let mut best_cost = BTreeMap::<(usize, EntityId), u32>::new();
    let mut expansions = 0usize;

    while let Some(node) = frontier.pop() {
        if node.stage_index >= stages.len() {
            return StrategicSearchResult {
                plan: Some(StrategicPlan { steps: node.steps }),
                skeleton_source: stage_build.skeleton_source,
                budget_trace: Some(strategic_budget_trace(
                    stages.len(),
                    search_budget,
                    expansions,
                    false,
                )),
                stages_count,
                method_trace: stage_build.method_trace,
            };
        }
        if expansions >= search_budget {
            break;
        }
        expansions = expansions.saturating_add(1);

        let state_key = (node.stage_index, node.current_place);
        if best_cost
            .get(&state_key)
            .is_some_and(|best| *best <= node.total_cost)
        {
            continue;
        }
        best_cost.insert(state_key, node.total_cost);

        let current_stage = &stages[node.stage_index];
        for destination in &current_stage.places {
            let Some(travel_ticks) =
                snapshot.min_perceived_travel_cost_to_any(node.current_place, &[*destination])
            else {
                continue;
            };
            let mut steps = node.steps.clone();
            steps.push(StrategicStep {
                destination: *destination,
                sub_goal: sub_goal_for_stage(current_stage),
                estimated_travel_ticks: travel_ticks,
            });
            frontier.push(SearchNode {
                stage_index: node.stage_index + 1,
                current_place: *destination,
                total_cost: node.total_cost.saturating_add(travel_ticks),
                steps,
            });
        }
    }

    StrategicSearchResult {
        plan: None,
        skeleton_source: stage_build.skeleton_source,
        budget_trace: Some(strategic_budget_trace(
            stages.len(),
            search_budget,
            expansions,
            expansions >= search_budget,
        )),
        stages_count,
        method_trace: stage_build.method_trace,
    }
}

fn strategic_budget_trace(
    stages_count: usize,
    budget_total: usize,
    budget_used: usize,
    exhausted: bool,
) -> StrategicBudgetTrace {
    StrategicBudgetTrace {
        stages_count: u16::try_from(stages_count).unwrap_or(u16::MAX),
        budget_total: u32::try_from(budget_total).unwrap_or(u32::MAX),
        budget_used: u32::try_from(budget_used).unwrap_or(u32::MAX),
        exhausted,
    }
}

fn goal_places(
    goal: &GoalOffer,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
) -> Vec<EntityId> {
    if let Some(place) = anchored_goal_place(goal, state) {
        return vec![place];
    }

    let mut places = goal.key.kind.goal_relevant_places(state, recipes);
    if places.is_empty() && matches!(goal.key.kind, GoalKind::SearchForMissing { .. }) {
        places.extend(goal.evidence_places.iter().copied());
    }
    places.sort_unstable();
    places.dedup();
    places
}

fn anchored_goal_place(goal: &GoalOffer, state: &PlanningState<'_>) -> Option<EntityId> {
    if !matches!(goal.key.kind, GoalKind::AcquireCommodity { .. }) {
        return None;
    }
    match goal.anchor {
        OpportunityAnchor::Place(place) => Some(place),
        OpportunityAnchor::Entity(entity) => state.effective_place(entity),
        OpportunityAnchor::None => None,
    }
}

fn missing_commodities(
    goal: &GoalOffer,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
    actor_place: EntityId,
    goal_places: &[EntityId],
) -> Vec<CommodityKind> {
    let actor = state.snapshot().actor();
    let mut commodities = match goal.key.kind {
        GoalKind::AcquireCommodity { commodity, .. } => {
            let needs_stage = anchored_goal_place(goal, state).is_none()
                && !goal_places.contains(&actor_place)
                && state.commodity_quantity(actor, commodity) == Quantity(0);
            needs_stage.then_some(commodity).into_iter().collect()
        }
        GoalKind::TreatWounds { .. } => (state.commodity_quantity(actor, CommodityKind::Medicine)
            == Quantity(0))
        .then_some(CommodityKind::Medicine)
        .into_iter()
        .collect(),
        GoalKind::ProduceCommodity { recipe_id } => recipes
            .get(recipe_id)
            .into_iter()
            .flat_map(|recipe| recipe.inputs.iter())
            .filter(|(commodity, required)| state.commodity_quantity(actor, *commodity) < *required)
            .map(|(commodity, _)| *commodity)
            .collect(),
        _ => Vec::new(),
    };
    commodities.sort_unstable();
    commodities.dedup();
    commodities
}

fn social_query_commodity(
    goal: &GoalOffer,
    missing_commodities: &[CommodityKind],
    recipes: &RecipeRegistry,
) -> Option<CommodityKind> {
    match goal.key.kind {
        GoalKind::ConsumeOwnedCommodity { .. }
        | GoalKind::AcquireCommodity { .. }
        | GoalKind::RestockCommodity { .. }
        | GoalKind::TreatWounds { .. } => goal.key.kind.target_commodity(recipes),
        GoalKind::ProduceCommodity { .. } => missing_commodities.first().copied(),
        _ => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StageBuildResult {
    stages: Vec<StrategicStage>,
    skeleton_source: Option<PartialPlanSkeletonSource>,
    method_trace: Option<MethodPlanAttemptTrace>,
}

#[allow(clippy::too_many_arguments)]
fn build_stages(
    state: &PlanningState<'_>,
    snapshot: &PlanningSnapshot,
    goal: &GoalOffer,
    recipes: &RecipeRegistry,
    method_selection: &MethodSelection<'_>,
    actor: EntityId,
    actor_place: EntityId,
    goal_places: &[EntityId],
    missing_commodities: &[CommodityKind],
    per_stage_limit: u8,
) -> StageBuildResult {
    let rejected_methods = rejected_method_traces(method_selection);
    if let Some(method) = method_selection.selected {
        let stages = method
            .subgoals
            .iter()
            .flat_map(|subgoal| {
                template_to_stages(
                    &subgoal.template,
                    state,
                    snapshot,
                    goal,
                    recipes,
                    actor,
                    actor_place,
                    goal_places,
                    per_stage_limit,
                )
            })
            .collect::<Vec<_>>();
        let mut stages = collapse_repeated_stages(stages);
        if !goal_places.is_empty()
            && !stages
                .iter()
                .any(|stage| matches!(stage.kind, StrategicStageKind::Goal))
        {
            stages.push(StrategicStage {
                kind: StrategicStageKind::Goal,
                places: goal_places.to_vec(),
            });
        }
        if !stages.is_empty() {
            return StageBuildResult {
                stages,
                skeleton_source: skeleton_source_for_method(method),
                method_trace: Some(method_trace(method, &goal.motive_sources, rejected_methods)),
            };
        }
    }

    let mut stages = missing_commodities
        .iter()
        .filter_map(|commodity| {
            let places = acquisition_places_for_commodity(
                state,
                snapshot,
                actor,
                actor_place,
                *commodity,
                per_stage_limit,
            );
            (!places.is_empty()).then_some(StrategicStage {
                kind: StrategicStageKind::Acquire(*commodity),
                places,
            })
        })
        .collect::<Vec<_>>();

    if !goal_places.is_empty() {
        stages.push(StrategicStage {
            kind: StrategicStageKind::Goal,
            places: goal_places.to_vec(),
        });
    }

    StageBuildResult {
        stages,
        skeleton_source: None,
        method_trace: Some(fallback_method_trace(
            method_selection.selected,
            &goal.motive_sources,
            rejected_methods,
        )),
    }
}

fn skeleton_source_for_method(method: &MethodSchema) -> Option<PartialPlanSkeletonSource> {
    let expected_pre = method
        .preconditions
        .iter()
        .filter_map(|precondition| match precondition {
            MethodPrecondition::BeliefHolds(predicate) => Some(predicate.clone()),
            MethodPrecondition::MotiveSourcePresent(_) | MethodPrecondition::LocationKnown(_) => {
                None
            }
        })
        .collect::<Vec<_>>();
    let remaining_skeleton = method
        .subgoals
        .iter()
        .filter_map(|subgoal| match &subgoal.template {
            SubgoalTemplate::PerformAction(op, payload)
                if skeleton_op_is_preservable(*op) && payload_template_is_preservable(payload) =>
            {
                Some(PlannedSkeletonStep {
                    op: *op,
                    target_template: payload.clone(),
                    expected_pre: expected_pre.clone(),
                })
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    (!remaining_skeleton.is_empty()).then_some(PartialPlanSkeletonSource { remaining_skeleton })
}

fn skeleton_op_is_preservable(op: PlannerOpKind) -> bool {
    !matches!(op, PlannerOpKind::Attack | PlannerOpKind::Defend)
}

fn payload_template_is_preservable(payload: &PayloadTemplate) -> bool {
    match payload {
        PayloadTemplate::FromContext => true,
        PayloadTemplate::Explicit(payload) => !payload_value_template_has_fixed_entity(payload),
    }
}

fn payload_value_template_has_fixed_entity(payload: &PayloadValueTemplate) -> bool {
    match payload {
        PayloadValueTemplate::Trade { .. } | PayloadValueTemplate::Craft { .. } => false,
        PayloadValueTemplate::Attack { target }
        | PayloadValueTemplate::ClaimBounty { bounty: target } => {
            entity_template_has_fixed_entity(*target)
        }
        PayloadValueTemplate::EscortToSafety {
            escortee,
            destination,
        } => {
            entity_template_has_fixed_entity(*escortee)
                || location_template_has_fixed_entity(destination)
        }
    }
}

fn location_template_has_fixed_entity(location: &LocationTemplate) -> bool {
    match location {
        LocationTemplate::LastKnownTargetPlace { target }
        | LocationTemplate::StagingPlaceForConfrontation { target }
        | LocationTemplate::BountyIssuerPlace { bounty: target }
        | LocationTemplate::OfficePlace {
            institution: target,
        }
        | LocationTemplate::EscorteeHome { escortee: target } => {
            entity_template_has_fixed_entity(*target)
        }
        LocationTemplate::NearestSellerOf { .. }
        | LocationTemplate::KnownWorkstationFor { .. }
        | LocationTemplate::AgentHome => false,
    }
}

fn entity_template_has_fixed_entity(template: EntityTemplate) -> bool {
    matches!(template, EntityTemplate::Fixed(_))
}

fn rejected_method_traces(method_selection: &MethodSelection<'_>) -> Vec<RejectedMethodTrace> {
    method_selection
        .rejected
        .iter()
        .map(|rejected| RejectedMethodTrace {
            method_id: rejected.method_id,
            failed_precondition: rejected.failed_precondition.clone(),
        })
        .collect()
}

fn method_trace(
    method: &MethodSchema,
    motives: &[MotiveSourceRef],
    rejected_methods: Vec<RejectedMethodTrace>,
) -> MethodPlanAttemptTrace {
    MethodPlanAttemptTrace {
        method_id: Some(method.id),
        rejected_methods,
        fallback_reason: None,
        subgoals_attempted: method
            .subgoals
            .iter()
            .enumerate()
            .map(|(template_index, subgoal)| SubgoalAttemptResult {
                template_index,
                authority: subgoal.authority,
                kind: (&subgoal.template).into(),
                outcome: SubgoalAttemptOutcome::Pending,
            })
            .collect(),
        failure_mode: None,
        motive_score: method_motive_score(method, motives),
    }
}

fn fallback_method_trace(
    selected_method: Option<&MethodSchema>,
    motives: &[MotiveSourceRef],
    rejected_methods: Vec<RejectedMethodTrace>,
) -> MethodPlanAttemptTrace {
    MethodPlanAttemptTrace {
        method_id: selected_method.map(|method| method.id),
        rejected_methods,
        fallback_reason: Some(if selected_method.is_some() {
            StrategicFallbackReason::MethodProducedNoStages
        } else {
            StrategicFallbackReason::NoViableMethod
        }),
        subgoals_attempted: selected_method.map_or_else(Vec::new, |method| {
            method
                .subgoals
                .iter()
                .enumerate()
                .map(|(template_index, subgoal)| SubgoalAttemptResult {
                    template_index,
                    authority: subgoal.authority,
                    kind: (&subgoal.template).into(),
                    outcome: SubgoalAttemptOutcome::Pending,
                })
                .collect()
        }),
        failure_mode: None,
        motive_score: selected_method.map_or(0, |method| method_motive_score(method, motives)),
    }
}

fn method_motive_score(method: &MethodSchema, motives: &[MotiveSourceRef]) -> u32 {
    method
        .motive_bias
        .iter()
        .filter(|bias| {
            motives
                .iter()
                .any(|source| MotiveSourceDiscriminant::from(&source.source) == bias.motive_variant)
        })
        .map(|bias| u32::from(bias.weight.value()))
        .sum()
}

fn collapse_repeated_stages(stages: Vec<StrategicStage>) -> Vec<StrategicStage> {
    stages.into_iter().fold(Vec::new(), |mut collapsed, stage| {
        if collapsed.last() != Some(&stage) {
            collapsed.push(stage);
        }
        collapsed
    })
}

#[allow(clippy::too_many_arguments)]
fn template_to_stages(
    template: &SubgoalTemplate,
    state: &PlanningState<'_>,
    snapshot: &PlanningSnapshot,
    goal: &GoalOffer,
    recipes: &RecipeRegistry,
    actor: EntityId,
    actor_place: EntityId,
    goal_places: &[EntityId],
    per_stage_limit: u8,
) -> Vec<StrategicStage> {
    match template {
        SubgoalTemplate::AcquireCommodity { commodity, .. } => {
            let Some(commodity) = resolve_commodity_template(*commodity, goal, recipes) else {
                return Vec::new();
            };
            if matches!(
                goal.key.kind,
                GoalKind::RestockCommodity {
                    commodity: goal_commodity
                } if goal_commodity == commodity
            ) {
                return stage_for_places(goal_places.to_vec());
            }
            let places = acquisition_places_for_commodity(
                state,
                snapshot,
                actor,
                actor_place,
                commodity,
                per_stage_limit,
            );
            (!places.is_empty())
                .then_some(StrategicStage {
                    kind: StrategicStageKind::Acquire(commodity),
                    places,
                })
                .into_iter()
                .collect()
        }
        SubgoalTemplate::TravelTo(location) | SubgoalTemplate::ReturnTo(location) => {
            stage_for_places(resolve_location_template(
                location, state, goal, recipes, actor,
            ))
        }
        SubgoalTemplate::ObserveTarget(criterion) => stage_for_places(
            resolve_entity_criterion_places(criterion, state, goal, actor),
        ),
        SubgoalTemplate::InspectArtifact(artifact) => stage_for_places(
            resolve_artifact_template_places(artifact, state, goal, actor),
        ),
        SubgoalTemplate::ResolveCoordination(claim) => {
            stage_for_places(resolve_claim_requirement_places(claim, state, goal, actor))
        }
        SubgoalTemplate::PerformAction(_, payload) => stage_for_places(
            resolve_payload_template_places(payload, state, goal, recipes, actor)
                .unwrap_or_else(|| goal_places.to_vec()),
        ),
        SubgoalTemplate::AskWitness(_) => stage_for_places(goal_places.to_vec()),
    }
}

fn stage_for_places(mut places: Vec<EntityId>) -> Vec<StrategicStage> {
    places.sort_unstable();
    places.dedup();
    (!places.is_empty())
        .then_some(StrategicStage {
            kind: StrategicStageKind::Goal,
            places,
        })
        .into_iter()
        .collect()
}

fn resolve_commodity_template(
    template: CommodityTemplate,
    goal: &GoalOffer,
    recipes: &RecipeRegistry,
) -> Option<CommodityKind> {
    match template {
        CommodityTemplate::GoalCommodity => match goal.key.kind {
            GoalKind::ConsumeOwnedCommodity { commodity }
            | GoalKind::AcquireCommodity { commodity, .. }
            | GoalKind::SellCommodity { commodity }
            | GoalKind::RestockCommodity { commodity }
            | GoalKind::MoveCargo { commodity, .. } => Some(commodity),
            _ => None,
        },
        CommodityTemplate::RecipeInput { recipe, ordinal } => {
            let recipe_id = resolve_recipe_template(recipe, goal)?;
            recipes
                .get(recipe_id)?
                .inputs
                .get(usize::from(ordinal))
                .map(|(commodity, _)| *commodity)
        }
        CommodityTemplate::Fixed(commodity) => Some(commodity),
    }
}

fn resolve_recipe_template(template: RecipeTemplate, goal: &GoalOffer) -> Option<RecipeId> {
    match template {
        RecipeTemplate::GoalRecipe => {
            if let GoalKind::ProduceCommodity { recipe_id } = goal.key.kind {
                Some(recipe_id)
            } else {
                None
            }
        }
        RecipeTemplate::Fixed(recipe) => Some(RecipeId(recipe)),
    }
}

fn resolve_location_template(
    template: &LocationTemplate,
    state: &PlanningState<'_>,
    goal: &GoalOffer,
    recipes: &RecipeRegistry,
    actor: EntityId,
) -> Vec<EntityId> {
    match template {
        LocationTemplate::LastKnownTargetPlace { target }
        | LocationTemplate::StagingPlaceForConfrontation { target }
        | LocationTemplate::EscorteeHome { escortee: target } => {
            resolve_entity_template(*target, state, goal, actor)
                .and_then(|entity| state.effective_place(entity))
                .into_iter()
                .collect()
        }
        LocationTemplate::NearestSellerOf { commodity } => {
            resolve_commodity_template(*commodity, goal, recipes)
                .map(|commodity| seller_places(state, commodity))
                .unwrap_or_default()
        }
        LocationTemplate::AgentHome => state.effective_place(actor).into_iter().collect(),
        LocationTemplate::BountyIssuerPlace { bounty } => {
            resolve_entity_template(*bounty, state, goal, actor)
                .and_then(|entity| state.effective_place(entity))
                .into_iter()
                .collect()
        }
        LocationTemplate::OfficePlace { institution } => {
            resolve_entity_template(*institution, state, goal, actor)
                .and_then(|entity| {
                    state
                        .effective_place(entity)
                        .or_else(|| state.snapshot().seat(entity))
                })
                .into_iter()
                .collect()
        }
        LocationTemplate::KnownWorkstationFor { recipe } => {
            let Some(recipe_id) = resolve_recipe_template(*recipe, goal) else {
                return Vec::new();
            };
            let Some(recipe) = recipes.get(recipe_id) else {
                return Vec::new();
            };
            recipe
                .required_workstation_tag
                .map_or_else(Vec::new, |tag| workstation_places(state, tag))
        }
    }
}

fn resolve_entity_template(
    template: EntityTemplate,
    state: &PlanningState<'_>,
    goal: &GoalOffer,
    actor: EntityId,
) -> Option<EntityId> {
    match template {
        EntityTemplate::GoalPrimaryEntity | EntityTemplate::BountyTarget => {
            primary_goal_entity(goal)
        }
        EntityTemplate::GoalSecondaryEntity => secondary_goal_entity(goal),
        EntityTemplate::GoalPlace => match goal.anchor {
            OpportunityAnchor::Place(place) => Some(place),
            OpportunityAnchor::Entity(entity) => state.effective_place(entity),
            OpportunityAnchor::None => None,
        },
        EntityTemplate::Violation => goal.evidence_entities.iter().next().copied(),
        EntityTemplate::Institution => goal.obligation_source,
        EntityTemplate::Escortee => {
            if let GoalKind::EscortToSafety { subject, .. } = goal.key.kind {
                Some(subject)
            } else {
                None
            }
        }
        EntityTemplate::Fixed(entity) => Some(entity),
    }
    .or_else(|| (template == EntityTemplate::GoalPrimaryEntity).then_some(actor))
}

fn primary_goal_entity(goal: &GoalOffer) -> Option<EntityId> {
    match goal.key.kind {
        GoalKind::EngageHostile { target }
        | GoalKind::RaidTarget { target }
        | GoalKind::TreatWounds { patient: target }
        | GoalKind::SearchForMissing {
            subject: target, ..
        }
        | GoalKind::ReportMissing {
            subject: target, ..
        }
        | GoalKind::ReportFound {
            subject: target, ..
        }
        | GoalKind::EscortToSafety {
            subject: target, ..
        }
        | GoalKind::FulfillBounty { bounty: target }
        | GoalKind::ClaimOffice { office: target }
        | GoalKind::SupportCandidateForOffice { office: target, .. }
        | GoalKind::InvestigateViolation { place: target, .. }
        | GoalKind::Patrol { place: target }
        | GoalKind::ExploreLocation {
            target_place: target,
            ..
        }
        | GoalKind::StealItem {
            target_item: target,
        }
        | GoalKind::Accuse {
            crime_register: target,
            ..
        }
        | GoalKind::PunishAccused { office: target, .. } => Some(target),
        _ => match goal.anchor {
            OpportunityAnchor::Entity(entity) | OpportunityAnchor::Place(entity) => Some(entity),
            OpportunityAnchor::None => goal.evidence_entities.iter().next().copied(),
        },
    }
}

fn secondary_goal_entity(goal: &GoalOffer) -> Option<EntityId> {
    match goal.key.kind {
        GoalKind::SupportCandidateForOffice { candidate, .. }
        | GoalKind::Accuse {
            accused: candidate, ..
        }
        | GoalKind::PunishAccused {
            accused: candidate, ..
        } => Some(candidate),
        GoalKind::MoveCargo { destination, .. } | GoalKind::EscortToSafety { destination, .. } => {
            Some(destination)
        }
        _ => None,
    }
}

fn resolve_entity_criterion_places(
    criterion: &EntityCriterion,
    state: &PlanningState<'_>,
    goal: &GoalOffer,
    actor: EntityId,
) -> Vec<EntityId> {
    match criterion {
        EntityCriterion::Target(template) => resolve_entity_template(*template, state, goal, actor)
            .and_then(|entity| state.effective_place(entity))
            .into_iter()
            .collect(),
        EntityCriterion::Workstation(tag) => workstation_places(state, *tag),
        EntityCriterion::ResourceSource(commodity) => {
            resolve_commodity_template(*commodity, goal, &RecipeRegistry::new())
                .map(|commodity| resource_source_places(state, commodity))
                .unwrap_or_default()
        }
        EntityCriterion::Seller(commodity) => {
            resolve_commodity_template(*commodity, goal, &RecipeRegistry::new())
                .map(|commodity| seller_places(state, commodity))
                .unwrap_or_default()
        }
    }
}

fn resolve_artifact_template_places(
    artifact: &ArtifactTemplate,
    state: &PlanningState<'_>,
    goal: &GoalOffer,
    actor: EntityId,
) -> Vec<EntityId> {
    match artifact {
        ArtifactTemplate::ViolationEvidence { violation } => {
            resolve_entity_template(*violation, state, goal, actor)
        }
        ArtifactTemplate::Ledger { institution } => {
            resolve_entity_template(*institution, state, goal, actor)
        }
        ArtifactTemplate::BountyProof { bounty, .. } => {
            resolve_entity_template(*bounty, state, goal, actor)
        }
    }
    .and_then(|entity| state.effective_place(entity))
    .into_iter()
    .chain(goal.evidence_places.iter().copied())
    .collect()
}

fn resolve_claim_requirement_places(
    claim: &ClaimRequirement,
    state: &PlanningState<'_>,
    goal: &GoalOffer,
    actor: EntityId,
) -> Vec<EntityId> {
    match claim {
        ClaimRequirement::OfficeAuthority { office } => {
            resolve_entity_template(*office, state, goal, actor)
        }
        ClaimRequirement::ResourceSourceAccess { place, .. }
        | ClaimRequirement::FacilityQueueSlot { facility: place } => {
            resolve_entity_template(*place, state, goal, actor)
        }
        ClaimRequirement::BountyIssuance { bounty } => {
            resolve_entity_template(*bounty, state, goal, actor)
        }
    }
    .and_then(|entity| state.effective_place(entity).or(Some(entity)))
    .into_iter()
    .collect()
}

fn resolve_payload_template_places(
    payload: &PayloadTemplate,
    state: &PlanningState<'_>,
    goal: &GoalOffer,
    recipes: &RecipeRegistry,
    actor: EntityId,
) -> Option<Vec<EntityId>> {
    let PayloadTemplate::Explicit(payload) = payload else {
        return None;
    };
    match payload {
        PayloadValueTemplate::Trade { commodity, .. } => {
            resolve_commodity_template(*commodity, goal, recipes).map(|commodity| {
                let mut places = seller_places(state, commodity);
                places.extend(resource_source_places(state, commodity));
                places
            })
        }
        PayloadValueTemplate::Craft { recipe } => {
            let recipe_id = resolve_recipe_template(*recipe, goal)?;
            let tag = recipes.get(recipe_id)?.required_workstation_tag?;
            Some(workstation_places(state, tag))
        }
        PayloadValueTemplate::Attack { target } => {
            resolve_entity_template(*target, state, goal, actor)
                .and_then(|entity| state.effective_place(entity))
                .map(|place| vec![place])
        }
        PayloadValueTemplate::ClaimBounty { bounty } => {
            resolve_entity_template(*bounty, state, goal, actor)
                .and_then(|entity| state.effective_place(entity))
                .map(|place| vec![place])
        }
        PayloadValueTemplate::EscortToSafety { destination, .. } => Some(
            resolve_location_template(destination, state, goal, recipes, actor),
        ),
    }
}

fn workstation_places(state: &PlanningState<'_>, tag: WorkstationTag) -> Vec<EntityId> {
    places_for_entities(
        state,
        entities_admitted_for_physical_fields(state)
            .filter(|entity| state.workstation_tag(*entity) == Some(tag)),
    )
}

fn seller_places(state: &PlanningState<'_>, commodity: CommodityKind) -> Vec<EntityId> {
    places_for_entities(
        state,
        entities_admitted_for_physical_fields(state).filter(|entity| {
            state.item_lot_commodity(*entity) == Some(commodity) && state.has_sale_listing(*entity)
        }),
    )
}

fn resource_source_places(state: &PlanningState<'_>, commodity: CommodityKind) -> Vec<EntityId> {
    places_for_entities(
        state,
        entities_admitted_for_physical_fields(state).filter(|entity| {
            state
                .resource_source(*entity)
                .is_some_and(|source| source.commodity == commodity)
        }),
    )
}

fn entities_admitted_for_physical_fields<'state>(
    state: &'state PlanningState<'_>,
) -> impl Iterator<Item = EntityId> + 'state {
    state
        .snapshot()
        .entities
        .iter()
        .filter_map(|(entity, snapshot)| {
            admission_exposes_physical_fields(snapshot.admission).then_some(*entity)
        })
}

fn admission_exposes_physical_fields(source: AdmissionSource) -> bool {
    matches!(
        source,
        AdmissionSource::SelfAuthoritative
            | AdmissionSource::LocalSameTickPhysical
            | AdmissionSource::GroundedEvidence
            | AdmissionSource::BeliefLastSeen
    )
}

fn places_for_entities(
    state: &PlanningState<'_>,
    entities: impl IntoIterator<Item = EntityId>,
) -> Vec<EntityId> {
    let mut places = entities
        .into_iter()
        .filter_map(|entity| state.effective_place(entity))
        .collect::<Vec<_>>();
    places.sort_unstable();
    places.dedup();
    places
}

fn acquisition_places_for_commodity(
    state: &PlanningState<'_>,
    snapshot: &PlanningSnapshot,
    actor: EntityId,
    actor_place: EntityId,
    commodity: CommodityKind,
    per_stage_limit: u8,
) -> Vec<EntityId> {
    let mut places = BTreeSet::new();
    for entity in entities_admitted_for_physical_fields(state) {
        let Some(place) = state.effective_place(entity) else {
            continue;
        };
        if !place_supports_commodity(state, place, commodity) {
            continue;
        }
        places.insert(place);
    }

    let mut ranked = places.into_iter().collect::<Vec<_>>();
    ranked.sort_by_key(|place| {
        (
            snapshot
                .min_perceived_travel_cost_to_any(actor_place, &[*place])
                .unwrap_or(u32::MAX),
            *place,
        )
    });
    ranked.truncate(usize::from(per_stage_limit.max(1)));

    if ranked.is_empty() && state.commodity_quantity(actor, commodity) > Quantity(0) {
        ranked.push(actor_place);
    }

    ranked
}

fn place_supports_commodity(
    state: &PlanningState<'_>,
    place: EntityId,
    commodity: CommodityKind,
) -> bool {
    entities_admitted_for_physical_fields(state).any(|entity| {
        state.effective_place(entity) == Some(place)
            && (commodity_entity_supports_commodity(state, entity, commodity))
    })
}

fn commodity_entity_supports_commodity(
    state: &PlanningState<'_>,
    entity: EntityId,
    commodity: CommodityKind,
) -> bool {
    state.resource_source(entity).is_some_and(|source| {
        source.commodity == commodity && source.available_quantity > Quantity(0)
    }) || state
        .merchandise_profile(entity)
        .is_some_and(|profile| profile.sale_kinds.contains(&commodity))
        || (state.item_lot_commodity(entity) == Some(commodity)
            && state.commodity_quantity(entity, commodity) > Quantity(0)
            && state.direct_possessor(entity).is_none()
            && state.direct_container(entity).is_none())
}

fn root_goal_satisfaction_allowed(goal: &GoalOffer, state: &PlanningState<'_>) -> bool {
    match &goal.key.kind {
        GoalKind::AcquireCommodity {
            purpose: worldwake_core::CommodityPurpose::SelfConsume,
            ..
        } => false,
        GoalKind::SellCommodity { commodity } => {
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

fn consume_current_place_prerequisites(
    current_place: EntityId,
    stages: &mut Vec<StrategicStage>,
    local_steps: &mut Vec<StrategicStep>,
) {
    loop {
        let Some(stage) = stages.first() else {
            return;
        };
        let StrategicStageKind::Acquire(commodity) = stage.kind else {
            return;
        };
        if !stage.places.contains(&current_place) {
            return;
        }
        local_steps.push(StrategicStep {
            destination: current_place,
            sub_goal: TacticalSubGoal::AcquirePrerequisite(commodity),
            estimated_travel_ticks: 0,
        });
        stages.remove(0);
    }
}

fn matches_local_goal_stage(stages: &[StrategicStage], current_place: EntityId) -> bool {
    stages.first().is_some_and(|stage| {
        matches!(stage.kind, StrategicStageKind::Goal) && stage.places.contains(&current_place)
    })
}

fn exploration_plan(
    snapshot: &PlanningSnapshot,
    actor_place: EntityId,
    goal_kind: &GoalKind,
) -> Option<StrategicPlan> {
    let sub_goal = explore_sub_goal_for(goal_kind);
    let step = snapshot
        .places
        .get(&actor_place)?
        .adjacent_places_with_travel_ticks
        .iter()
        .map(|(destination, ticks)| {
            let estimated_travel_ticks = snapshot
                .direct_perceived_travel_cost(actor_place, *destination)
                .unwrap_or_else(|| ticks.get());
            StrategicStep {
                destination: *destination,
                sub_goal,
                estimated_travel_ticks,
            }
        })
        .min_by_key(|step| (step.estimated_travel_ticks, step.destination))?;
    Some(StrategicPlan { steps: vec![step] })
}

fn explore_sub_goal_for(goal_kind: &GoalKind) -> TacticalSubGoal {
    match goal_kind {
        GoalKind::AcquireCommodity { .. } | GoalKind::SearchForMissing { .. } => {
            TacticalSubGoal::ExploreWithBarrier
        }
        _ => TacticalSubGoal::ExploreFallback,
    }
}

fn social_query_plan(
    snapshot: &PlanningSnapshot,
    actor_place: EntityId,
    commodity: Option<CommodityKind>,
) -> Option<StrategicPlan> {
    let commodity = commodity?;
    let actor = snapshot.actor();
    let has_colocated_agent = snapshot
        .places
        .get(&actor_place)?
        .entities
        .iter()
        .copied()
        .filter(|entity| *entity != actor)
        .any(|entity| {
            snapshot
                .entities
                .get(&entity)
                .and_then(|entity| entity.entity.kind)
                == Some(EntityKind::Agent)
        });

    has_colocated_agent.then_some(StrategicPlan {
        steps: vec![StrategicStep {
            destination: actor_place,
            sub_goal: TacticalSubGoal::SocialQuery(commodity),
            estimated_travel_ticks: 0,
        }],
    })
}

fn sub_goal_for_stage(stage: &StrategicStage) -> TacticalSubGoal {
    match stage.kind {
        StrategicStageKind::Goal => TacticalSubGoal::SatisfyGoal,
        StrategicStageKind::Acquire(commodity) => TacticalSubGoal::AcquirePrerequisite(commodity),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        TacticalSubGoal, acquisition_places_for_commodity, plan, seller_places, workstation_places,
    };
    use crate::build_planning_snapshot;
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        AcquisitionQuantity, AgentBeliefStore, BeliefConfidencePolicy, BelievedEntityState,
        BodyCostPerTick, BodyPart, CommodityKind, DeprivationKind, EntityId, EntityKind, GoalKind,
        InTransitOnEdge, InstitutionalBeliefRead, LoadUnits, MerchandiseProfile, OpportunityAnchor,
        PatrolRoute, Permille, Quantity, RecipeId, ResourceSource, Tick, TickRange,
        ToldBeliefMemory, WorkstationTag, Wound, WoundCause, WoundId,
    };
    use worldwake_sim::{
        ActionDuration, ActionPayload, CombatBeliefView, ControlBeliefView, EconomicBeliefView,
        EntityBeliefView, FacilityBeliefView, InventoryBeliefView, PoliticalBeliefView,
        ProfileBeliefView, RecipeDefinition, RecipeRegistry, RuntimeBeliefView, SocialBeliefView,
        SpatialBeliefView, TemporalBeliefView,
    };

    struct StubBeliefView {
        current_tick: Tick,
        alive: BTreeMap<EntityId, bool>,
        kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        entities_at: BTreeMap<EntityId, Vec<EntityId>>,
        adjacent: BTreeMap<EntityId, Vec<(EntityId, NonZeroU32)>>,
        known_entity_beliefs: BTreeMap<EntityId, Vec<(EntityId, BelievedEntityState)>>,
        belief_stores: BTreeMap<EntityId, AgentBeliefStore>,
        direct_possessions: BTreeMap<EntityId, Vec<EntityId>>,
        direct_containers: BTreeMap<EntityId, EntityId>,
        direct_possessors: BTreeMap<EntityId, EntityId>,
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        item_lot_commodities: BTreeMap<EntityId, CommodityKind>,
        resource_sources: BTreeMap<EntityId, ResourceSource>,
        workstation_tags: BTreeMap<EntityId, WorkstationTag>,
        merchandise_profiles: BTreeMap<EntityId, MerchandiseProfile>,
        wounds: BTreeMap<EntityId, Vec<Wound>>,
    }

    impl Default for StubBeliefView {
        fn default() -> Self {
            Self {
                current_tick: Tick(0),
                alive: BTreeMap::new(),
                kinds: BTreeMap::new(),
                effective_places: BTreeMap::new(),
                entities_at: BTreeMap::new(),
                adjacent: BTreeMap::new(),
                known_entity_beliefs: BTreeMap::new(),
                belief_stores: BTreeMap::new(),
                direct_possessions: BTreeMap::new(),
                direct_containers: BTreeMap::new(),
                direct_possessors: BTreeMap::new(),
                commodity_quantities: BTreeMap::new(),
                item_lot_commodities: BTreeMap::new(),
                resource_sources: BTreeMap::new(),
                workstation_tags: BTreeMap::new(),
                merchandise_profiles: BTreeMap::new(),
                wounds: BTreeMap::new(),
            }
        }
    }

    impl ControlBeliefView for StubBeliefView {
        fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
            actor == entity
        }

        fn has_control(&self, entity: EntityId) -> bool {
            self.kinds.get(&entity) == Some(&EntityKind::Agent)
        }
    }

    impl worldwake_sim::BelievedAuthorityView for StubBeliefView {}

    impl EntityBeliefView for StubBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.get(&entity).copied().unwrap_or(false)
        }

        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.kinds.get(&entity).copied()
        }

        fn is_dead(&self, entity: EntityId) -> bool {
            !self.is_alive(entity)
        }

        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
    }

    impl ProfileBeliefView for StubBeliefView {
        fn homeostatic_needs(&self, _agent: EntityId) -> Option<worldwake_core::HomeostaticNeeds> {
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

        fn disposal_profile(&self, _agent: EntityId) -> Option<worldwake_core::DisposalProfile> {
            None
        }
    }

    impl SpatialBeliefView for StubBeliefView {
        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.effective_places.get(&entity).copied()
        }

        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }

        fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.entities_at.get(&place).cloned().unwrap_or_default()
        }

        fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
            self.adjacent_places_with_travel_ticks(place)
                .into_iter()
                .map(|(adjacent, _)| adjacent)
                .collect()
        }

        fn patrol_route(&self, _agent: EntityId) -> Option<PatrolRoute> {
            None
        }

        fn route_exists(&self, _from: EntityId, _to: EntityId) -> bool {
            false
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
    }

    impl TemporalBeliefView for StubBeliefView {
        fn current_tick(&self) -> Tick {
            self.current_tick
        }

        fn has_contention_policy(&self, _entity: EntityId) -> bool {
            false
        }

        fn facility_queue_position(&self, _facility: EntityId, _actor: EntityId) -> Option<u32> {
            None
        }

        fn facility_grant(&self, _facility: EntityId) -> Option<&worldwake_core::ContentionGrant> {
            None
        }

        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }

        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
            Vec::new()
        }

        fn estimate_duration(
            &self,
            _actor: EntityId,
            _duration: &worldwake_sim::DurationExpr,
            _targets: &[EntityId],
            _payload: &ActionPayload,
        ) -> Option<ActionDuration> {
            None
        }
    }

    impl SocialBeliefView for StubBeliefView {
        fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
            self.known_entity_beliefs
                .get(&agent)
                .cloned()
                .unwrap_or_default()
        }

        fn agent_belief_store(&self, agent: EntityId) -> Option<&AgentBeliefStore> {
            self.belief_stores.get(&agent)
        }

        fn belief_confidence_policy(&self, _agent: EntityId) -> BeliefConfidencePolicy {
            BeliefConfidencePolicy::default()
        }

        fn intention_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::IntentionDispositionProfile> {
            None
        }

        fn told_belief_memories(
            &self,
            _agent: EntityId,
        ) -> Vec<(worldwake_core::TellMemoryKey, ToldBeliefMemory)> {
            Vec::new()
        }
    }

    impl PoliticalBeliefView for StubBeliefView {
        fn office_data(&self, _office: EntityId) -> Option<worldwake_core::OfficeData> {
            None
        }

        fn believed_support_declarations_for_office(
            &self,
            _office: EntityId,
        ) -> Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)> {
            Vec::new()
        }
    }

    impl CombatBeliefView for StubBeliefView {
        fn combat_profile(&self, _agent: EntityId) -> Option<worldwake_core::CombatProfile> {
            None
        }

        fn wounds(&self, agent: EntityId) -> Vec<worldwake_core::Wound> {
            self.wounds.get(&agent).cloned().unwrap_or_default()
        }

        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn patrol_profile(&self, _agent: EntityId) -> Option<worldwake_core::PatrolProfile> {
            None
        }

        fn has_wounds(&self, entity: EntityId) -> bool {
            self.wounds
                .get(&entity)
                .is_some_and(|wounds| !wounds.is_empty())
        }
    }

    impl EconomicBeliefView for StubBeliefView {
        fn trade_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::TradeDispositionProfile> {
            None
        }

        fn controlled_commodity_quantity_at_place(
            &self,
            _actor: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Quantity {
            Quantity(0)
        }

        fn local_controlled_lots_for(
            &self,
            _actor: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn listed_sale_lots_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn seller_for_sale_lot(&self, _lot: EntityId) -> Option<EntityId> {
            None
        }

        fn demand_memory(&self, _agent: EntityId) -> Vec<worldwake_core::DemandObservation> {
            Vec::new()
        }

        fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile> {
            self.merchandise_profiles.get(&agent).cloned()
        }
    }

    impl InventoryBeliefView for StubBeliefView {
        fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId> {
            self.direct_possessions
                .get(&holder)
                .cloned()
                .unwrap_or_default()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
            false
        }

        fn unique_item_count(
            &self,
            _holder: EntityId,
            _kind: worldwake_core::UniqueItemKind,
        ) -> u32 {
            0
        }

        fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
            self.commodity_quantities
                .get(&(holder, kind))
                .copied()
                .unwrap_or(Quantity(0))
        }

        fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> {
            self.item_lot_commodities.get(&entity).copied()
        }

        fn item_lot_consumable_profile(
            &self,
            _entity: EntityId,
        ) -> Option<worldwake_core::CommodityConsumableProfile> {
            None
        }

        fn direct_container(&self, entity: EntityId) -> Option<EntityId> {
            self.direct_containers.get(&entity).copied()
        }

        fn direct_possessor(&self, entity: EntityId) -> Option<EntityId> {
            self.direct_possessors.get(&entity).copied()
        }

        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            Vec::new()
        }
    }

    impl FacilityBeliefView for StubBeliefView {
        fn workstation_tag(&self, entity: EntityId) -> Option<worldwake_core::WorkstationTag> {
            self.workstation_tags.get(&entity).copied()
        }

        fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
            self.resource_sources.get(&entity).cloned()
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn matching_workstations_at(
            &self,
            place: EntityId,
            tag: worldwake_core::WorkstationTag,
        ) -> Vec<EntityId> {
            self.entities_at(place)
                .into_iter()
                .filter(|entity| self.workstation_tags.get(entity) == Some(&tag))
                .collect()
        }

        fn resource_sources_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.entities_at(place)
                .into_iter()
                .filter(|entity| {
                    self.resource_sources
                        .get(entity)
                        .is_some_and(|source| source.commodity == commodity)
                })
                .collect()
        }
    }

    impl RuntimeBeliefView for StubBeliefView {}
    impl worldwake_sim::LocalPhysicalObservationView for StubBeliefView {}

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn belief(kind: EntityKind, place: EntityId) -> BelievedEntityState {
        BelievedEntityState {
            believed_kind: Some(kind),
            last_known_place: Some(place),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            ..BelievedEntityState::single_observation_defaults(
                Tick(1),
                worldwake_core::PerceptionSource::DirectObservation,
            )
        }
    }

    fn remember(
        view: &mut StubBeliefView,
        actor: EntityId,
        entity: EntityId,
        belief: BelievedEntityState,
    ) {
        view.known_entity_beliefs
            .entry(actor)
            .or_default()
            .push((entity, belief));
    }

    fn connect(view: &mut StubBeliefView, from: EntityId, to: EntityId, ticks: u32) {
        let ticks = NonZeroU32::new(ticks).unwrap();
        view.adjacent.entry(from).or_default().push((to, ticks));
        view.adjacent.entry(to).or_default().push((from, ticks));
    }

    fn register_agent(view: &mut StubBeliefView, agent: EntityId, place: EntityId) {
        view.alive.insert(agent, true);
        view.kinds.insert(agent, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.entities_at.entry(place).or_default().push(agent);
    }

    fn register_facility(
        view: &mut StubBeliefView,
        facility: EntityId,
        place: EntityId,
        source: ResourceSource,
    ) {
        view.alive.insert(facility, true);
        view.kinds.insert(facility, EntityKind::Facility);
        view.effective_places.insert(facility, place);
        view.entities_at.entry(place).or_default().push(facility);
        view.resource_sources.insert(facility, source);
    }

    fn register_workstation(
        view: &mut StubBeliefView,
        workstation: EntityId,
        place: EntityId,
        tag: WorkstationTag,
    ) {
        view.alive.insert(workstation, true);
        view.kinds.insert(workstation, EntityKind::Facility);
        view.effective_places.insert(workstation, place);
        view.entities_at.entry(place).or_default().push(workstation);
        view.workstation_tags.insert(workstation, tag);
    }

    fn register_patient(view: &mut StubBeliefView, patient: EntityId, place: EntityId) {
        register_agent(view, patient, place);
    }

    fn add_wound(view: &mut StubBeliefView, patient: EntityId) {
        view.wounds.insert(
            patient,
            vec![Wound {
                id: WoundId(1),
                body_part: BodyPart::Torso,
                cause: WoundCause::Deprivation(DeprivationKind::Dehydration),
                severity: Permille::new(50).unwrap(),
                inflicted_at: Tick(0),
                bleed_rate_per_tick: Permille::new(0).unwrap(),
            }],
        );
    }

    fn snapshot(view: &StubBeliefView, actor: EntityId, horizon: u8) -> crate::PlanningSnapshot {
        build_planning_snapshot(view, actor, &BTreeSet::new(), &BTreeSet::new(), horizon)
    }

    fn set_admission(
        snapshot: &mut crate::PlanningSnapshot,
        entity: EntityId,
        admission: crate::planning_snapshot::AdmissionSource,
    ) {
        snapshot
            .entities
            .get_mut(&entity)
            .expect("test entity should be present in planning snapshot")
            .admission = admission;
    }

    fn base_budget() -> worldwake_core::ExecutionBudget {
        worldwake_core::ExecutionBudget::new(
            worldwake_core::ExecutionBudget::default().beam_width(),
            3,
            worldwake_core::ExecutionBudget::default().preferred_operator_boost(),
        )
    }

    #[test]
    fn seller_places_ignores_public_topology_sale_lot() {
        let actor = entity(1);
        let place = entity(10);
        let sale_lot = entity(20);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place);
        view.alive.insert(sale_lot, true);
        view.kinds.insert(sale_lot, EntityKind::ItemLot);
        view.effective_places.insert(sale_lot, place);
        view.entities_at.entry(place).or_default().push(sale_lot);
        view.item_lot_commodities
            .insert(sale_lot, CommodityKind::Bread);

        let mut snapshot = snapshot(&view, actor, 0);
        set_admission(
            &mut snapshot,
            sale_lot,
            crate::planning_snapshot::AdmissionSource::PublicTopology,
        );
        snapshot
            .entities
            .get_mut(&sale_lot)
            .expect("sale lot should be present")
            .economic
            .has_sale_listing = true;
        let state = crate::PlanningState::new(&snapshot);

        assert!(seller_places(&state, CommodityKind::Bread).is_empty());
    }

    #[test]
    fn workstation_places_ignores_public_topology_entity() {
        let actor = entity(1);
        let place = entity(10);
        let workstation = entity(20);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place);
        register_workstation(&mut view, workstation, place, WorkstationTag::Mill);

        let mut snapshot = snapshot(&view, actor, 0);
        set_admission(
            &mut snapshot,
            workstation,
            crate::planning_snapshot::AdmissionSource::PublicTopology,
        );
        let state = crate::PlanningState::new(&snapshot);

        assert!(workstation_places(&state, WorkstationTag::Mill).is_empty());
    }

    #[test]
    fn acquisition_places_ignore_public_topology_resource_fields() {
        let actor = entity(1);
        let place = entity(10);
        let resource = entity(20);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place);
        register_facility(
            &mut view,
            resource,
            place,
            ResourceSource {
                commodity: CommodityKind::Water,
                available_quantity: Quantity(2),
                max_quantity: Quantity(2),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        );

        let mut snapshot = snapshot(&view, actor, 0);
        set_admission(
            &mut snapshot,
            resource,
            crate::planning_snapshot::AdmissionSource::PublicTopology,
        );
        let state = crate::PlanningState::new(&snapshot);

        assert!(
            acquisition_places_for_commodity(
                &state,
                &snapshot,
                actor,
                place,
                CommodityKind::Water,
                3,
            )
            .is_empty()
        );
    }

    #[test]
    fn test_single_location_goal_no_travel() {
        let actor = entity(1);
        let place = entity(10);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place);

        let snapshot = snapshot(&view, actor, 0);
        let goal = crate::GoalOffer {
            key: worldwake_core::GoalKey::from(GoalKind::Patrol { place }),
            anchor: OpportunityAnchor::Place(place),
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

        let plan = plan(&snapshot, &goal, &base_budget(), &RecipeRegistry::new()).unwrap();

        assert!(plan.steps.is_empty());
    }

    #[test]
    fn test_multi_location_prerequisite_then_goal() {
        let actor = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let place_c = entity(12);
        let patient = entity(2);
        let medicine_source = entity(20);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place_a);
        register_patient(&mut view, patient, place_c);
        add_wound(&mut view, patient);
        register_facility(
            &mut view,
            medicine_source,
            place_b,
            ResourceSource {
                commodity: CommodityKind::Medicine,
                available_quantity: Quantity(3),
                max_quantity: Quantity(3),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        );
        connect(&mut view, place_a, place_b, 3);
        connect(&mut view, place_b, place_c, 5);
        let mut source_belief = belief(EntityKind::Facility, place_b);
        source_belief.resource_source = view.resource_sources.get(&medicine_source).cloned();
        remember(&mut view, actor, medicine_source, source_belief);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([patient]),
            &BTreeSet::from([place_c]),
            2,
        );
        let goal = crate::GoalOffer {
            key: worldwake_core::GoalKey::from(GoalKind::TreatWounds { patient }),
            anchor: OpportunityAnchor::Entity(patient),
            evidence_entities: BTreeSet::from([patient]),
            evidence_places: BTreeSet::from([place_c]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };

        let plan = plan(&snapshot, &goal, &base_budget(), &RecipeRegistry::new()).unwrap();

        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].destination, place_b);
        assert_eq!(
            plan.steps[0].sub_goal,
            TacticalSubGoal::AcquirePrerequisite(CommodityKind::Medicine)
        );
        assert_eq!(plan.steps[0].estimated_travel_ticks, 3);
        assert_eq!(plan.steps[1].destination, place_c);
        assert_eq!(plan.steps[1].sub_goal, TacticalSubGoal::SatisfyGoal);
        assert_eq!(plan.steps[1].estimated_travel_ticks, 5);
    }

    #[test]
    fn test_belief_only_excludes_unknown_locations() {
        let actor = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let unknown_place_c = entity(12);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place_a);
        connect(&mut view, place_a, place_b, 2);

        let snapshot = snapshot(&view, actor, 1);
        let goal = crate::GoalOffer {
            key: worldwake_core::GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: worldwake_core::CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
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
        };

        let plan = plan(&snapshot, &goal, &base_budget(), &RecipeRegistry::new()).unwrap();

        assert!(
            plan.steps
                .iter()
                .all(|step| step.destination != unknown_place_c)
        );
    }

    #[test]
    fn test_empty_beliefs_exploration_fallback_uses_barrier_required_variant_for_supported_goal() {
        let actor = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let place_c = entity(12);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place_a);
        connect(&mut view, place_a, place_b, 2);
        connect(&mut view, place_a, place_c, 5);

        let snapshot = snapshot(&view, actor, 1);
        let goal = crate::GoalOffer {
            key: worldwake_core::GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: worldwake_core::CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
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
        };

        let plan = plan(&snapshot, &goal, &base_budget(), &RecipeRegistry::new()).unwrap();

        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].destination, place_b);
        assert_eq!(plan.steps[0].sub_goal, TacticalSubGoal::ExploreWithBarrier);
        assert_eq!(plan.steps[0].estimated_travel_ticks, 2);
    }

    #[test]
    fn test_empty_beliefs_exploration_fallback_uses_generic_variant_for_unsupported_goal() {
        let actor = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let place_c = entity(12);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place_a);
        connect(&mut view, place_a, place_b, 2);
        connect(&mut view, place_a, place_c, 5);

        let snapshot = snapshot(&view, actor, 1);
        let goal = crate::GoalOffer {
            key: worldwake_core::GoalKey::from(GoalKind::Sleep),
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
        };

        let plan = plan(&snapshot, &goal, &base_budget(), &RecipeRegistry::new()).unwrap();

        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].destination, place_b);
        assert_eq!(plan.steps[0].sub_goal, TacticalSubGoal::ExploreFallback);
        assert_eq!(plan.steps[0].estimated_travel_ticks, 2);
    }

    #[test]
    fn test_social_query_when_colocated_agents() {
        let actor = entity(1);
        let listener = entity(2);
        let place = entity(10);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place);
        register_agent(&mut view, listener, place);

        let snapshot = snapshot(&view, actor, 0);
        let goal = crate::GoalOffer {
            key: worldwake_core::GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: worldwake_core::CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
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
        };

        let plan = plan(&snapshot, &goal, &base_budget(), &RecipeRegistry::new()).unwrap();

        assert_eq!(
            plan.steps,
            vec![super::StrategicStep {
                destination: place,
                sub_goal: TacticalSubGoal::SocialQuery(CommodityKind::Water),
                estimated_travel_ticks: 0,
            }]
        );
    }

    #[test]
    fn test_no_fallback_returns_none() {
        let actor = entity(1);
        let place = entity(10);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place);

        let snapshot = snapshot(&view, actor, 0);
        let goal = crate::GoalOffer {
            key: worldwake_core::GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: worldwake_core::CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
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
        };

        assert!(plan(&snapshot, &goal, &base_budget(), &RecipeRegistry::new()).is_none());
    }

    #[test]
    fn strategic_budget_trace_records_successful_stage_search() {
        let actor = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place_a);
        connect(&mut view, place_a, place_b, 4);

        let snapshot = snapshot(&view, actor, 1);
        let goal = crate::GoalOffer {
            key: worldwake_core::GoalKey::from(GoalKind::Patrol { place: place_b }),
            anchor: OpportunityAnchor::Place(place_b),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::from([place_b]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };

        let result = super::plan_with_budget_trace(
            &snapshot,
            &goal,
            &base_budget(),
            u16::MAX,
            &RecipeRegistry::new(),
        );

        assert_eq!(result.plan.expect("strategic plan").steps.len(), 1);
        assert_eq!(
            result.budget_trace,
            Some(crate::decision_trace::StrategicBudgetTrace {
                stages_count: 1,
                budget_total: 6,
                budget_used: 1,
                exhausted: false,
            })
        );
    }

    #[test]
    fn method_selection_substitutes_method_subgoals_into_stage_list() {
        let actor = entity(1);
        let place_a = entity(10);
        let grain_field = entity(11);
        let mill_place = entity(12);
        let grain_source = entity(20);
        let mill = entity(21);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place_a);
        register_facility(
            &mut view,
            grain_source,
            grain_field,
            ResourceSource {
                commodity: CommodityKind::Grain,
                available_quantity: Quantity(3),
                max_quantity: Quantity(3),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        );
        register_workstation(&mut view, mill, mill_place, WorkstationTag::Mill);
        connect(&mut view, place_a, grain_field, 2);
        connect(&mut view, grain_field, mill_place, 3);

        let mut recipes = RecipeRegistry::new();
        let recipe_id = recipes.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(1))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(4).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::zero(),
        });
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([grain_source, mill]),
            &BTreeSet::from([grain_field, mill_place]),
            2,
        );
        let state = crate::PlanningState::new(&snapshot);
        let goal = crate::GoalOffer {
            key: worldwake_core::GoalKey::from(GoalKind::ProduceCommodity { recipe_id }),
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::from([grain_source, mill]),
            evidence_places: BTreeSet::from([grain_field, mill_place]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };

        let mut registry = crate::htn::MethodRegistry::default();
        registry.insert(crate::htn::MethodSchema {
            id: worldwake_core::MethodSchemaId(99),
            goal_kind: worldwake_core::GoalKindDiscriminant::ProduceCommodity,
            preconditions: Vec::new(),
            subgoals: vec![
                crate::htn::MethodSubgoal::stage_hint(
                    crate::htn::SubgoalTemplate::AcquireCommodity {
                        commodity: crate::htn::CommodityTemplate::Fixed(CommodityKind::Grain),
                        min_quantity: Quantity(1),
                    },
                ),
                crate::htn::MethodSubgoal {
                    template: crate::htn::SubgoalTemplate::TravelTo(
                        crate::htn::LocationTemplate::KnownWorkstationFor {
                            recipe: crate::htn::RecipeTemplate::GoalRecipe,
                        },
                    ),
                    authority: crate::htn::MethodSubgoalAuthority::RequiredActionLeaf,
                },
            ],
            explanation_template: crate::htn::ExplanationTemplateId(99),
            motive_bias: Vec::new(),
            planning_budget_hint: None,
        });

        let stage_build = super::build_stages(
            &state,
            &snapshot,
            &goal,
            &recipes,
            &crate::htn::MethodSelection {
                selected: registry.get(worldwake_core::MethodSchemaId(99)),
                rejected: Vec::new(),
            },
            actor,
            place_a,
            &[],
            &[CommodityKind::Grain],
            base_budget().max_prerequisite_locations(),
        );

        assert_eq!(
            stage_build.stages,
            vec![
                super::StrategicStage {
                    kind: super::StrategicStageKind::Acquire(CommodityKind::Grain),
                    places: vec![grain_field],
                },
                super::StrategicStage {
                    kind: super::StrategicStageKind::Goal,
                    places: vec![mill_place],
                },
            ],
            "produce_with_gather should replace the flat missing-commodity-only stage list with method subgoals"
        );
        assert_eq!(
            stage_build
                .method_trace
                .as_ref()
                .map(|trace| trace.method_id),
            Some(Some(worldwake_core::MethodSchemaId(99)))
        );
        assert_eq!(
            stage_build.method_trace.as_ref().map(|trace| {
                trace
                    .subgoals_attempted
                    .iter()
                    .map(|subgoal| subgoal.authority)
                    .collect::<Vec<_>>()
            }),
            Some(vec![
                crate::htn::MethodSubgoalAuthority::StageHint,
                crate::htn::MethodSubgoalAuthority::RequiredActionLeaf,
            ]),
            "method trace should preserve the selected method's per-subgoal authority labels"
        );
    }

    #[test]
    fn skeleton_source_for_method_preserves_template_only_action_steps() {
        let expected = crate::htn::BeliefPredicate::SellerKnown {
            commodity: crate::htn::CommodityTemplate::GoalCommodity,
        };
        let payload =
            crate::htn::PayloadTemplate::Explicit(crate::htn::PayloadValueTemplate::Trade {
                commodity: crate::htn::CommodityTemplate::GoalCommodity,
                quantity: Quantity(3),
            });
        let method = crate::htn::MethodSchema {
            id: worldwake_core::MethodSchemaId(101),
            goal_kind: worldwake_core::GoalKindDiscriminant::RestockCommodity,
            preconditions: vec![crate::htn::MethodPrecondition::BeliefHolds(
                expected.clone(),
            )],
            subgoals: vec![
                crate::htn::MethodSubgoal::stage_hint(crate::htn::SubgoalTemplate::TravelTo(
                    crate::htn::LocationTemplate::NearestSellerOf {
                        commodity: crate::htn::CommodityTemplate::GoalCommodity,
                    },
                )),
                crate::htn::MethodSubgoal::stage_hint(crate::htn::SubgoalTemplate::PerformAction(
                    crate::PlannerOpKind::Trade,
                    payload.clone(),
                )),
            ],
            explanation_template: crate::htn::ExplanationTemplateId(101),
            motive_bias: Vec::new(),
            planning_budget_hint: None,
        };

        let source = super::skeleton_source_for_method(&method)
            .expect("template-only trade action should be preservable");

        assert_eq!(
            source.remaining_skeleton,
            vec![crate::PlannedSkeletonStep {
                op: crate::PlannerOpKind::Trade,
                target_template: payload,
                expected_pre: vec![expected],
            }]
        );
    }

    #[test]
    fn skeleton_source_for_method_excludes_combat_and_fixed_target_steps() {
        let fixed_target = entity(44);
        let method = crate::htn::MethodSchema {
            id: worldwake_core::MethodSchemaId(102),
            goal_kind: worldwake_core::GoalKindDiscriminant::FulfillBounty,
            preconditions: Vec::new(),
            subgoals: vec![
                crate::htn::MethodSubgoal::stage_hint(crate::htn::SubgoalTemplate::PerformAction(
                    crate::PlannerOpKind::Attack,
                    crate::htn::PayloadTemplate::Explicit(
                        crate::htn::PayloadValueTemplate::Attack {
                            target: crate::htn::EntityTemplate::BountyTarget,
                        },
                    ),
                )),
                crate::htn::MethodSubgoal::stage_hint(crate::htn::SubgoalTemplate::PerformAction(
                    crate::PlannerOpKind::ClaimBounty,
                    crate::htn::PayloadTemplate::Explicit(
                        crate::htn::PayloadValueTemplate::ClaimBounty {
                            bounty: crate::htn::EntityTemplate::Fixed(fixed_target),
                        },
                    ),
                )),
            ],
            explanation_template: crate::htn::ExplanationTemplateId(102),
            motive_bias: Vec::new(),
            planning_budget_hint: None,
        };

        assert_eq!(super::skeleton_source_for_method(&method), None);
    }

    #[test]
    fn strategic_budget_trace_marks_budget_exhaustion() {
        let trace = super::strategic_budget_trace(5, 30, 30, true);

        assert_eq!(
            trace,
            crate::decision_trace::StrategicBudgetTrace {
                stages_count: 5,
                budget_total: 30,
                budget_used: 30,
                exhausted: true,
            }
        );
    }

    #[test]
    fn test_estimated_travel_ticks_from_beliefs() {
        let actor = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let source = entity(20);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place_a);
        register_facility(
            &mut view,
            source,
            place_b,
            ResourceSource {
                commodity: CommodityKind::Water,
                available_quantity: Quantity(2),
                max_quantity: Quantity(2),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        );
        connect(&mut view, place_a, place_b, 7);
        let mut source_belief = belief(EntityKind::Facility, place_b);
        source_belief.resource_source = view.resource_sources.get(&source).cloned();
        remember(&mut view, actor, source, source_belief);

        let snapshot = snapshot(&view, actor, 1);
        let goal = crate::GoalOffer {
            key: worldwake_core::GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: worldwake_core::CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
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
        };

        let plan = plan(&snapshot, &goal, &base_budget(), &RecipeRegistry::new()).unwrap();

        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].destination, place_b);
        assert_eq!(plan.steps[0].estimated_travel_ticks, 7);
        assert_eq!(
            plan.steps[0].sub_goal,
            TacticalSubGoal::AcquirePrerequisite(CommodityKind::Water)
        );
        assert_eq!(plan.steps[1].destination, place_b);
        assert_eq!(plan.steps[1].estimated_travel_ticks, 0);
        assert_eq!(plan.steps[1].sub_goal, TacticalSubGoal::SatisfyGoal);
    }

    #[test]
    fn test_local_acquire_goal_does_not_force_remote_prerequisite_guidance() {
        let actor = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let local_seller = entity(20);
        let remote_source = entity(21);
        let local_lot = entity(22);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place_a);
        register_agent(&mut view, local_seller, place_a);
        register_facility(
            &mut view,
            remote_source,
            place_b,
            ResourceSource {
                commodity: CommodityKind::Bread,
                available_quantity: Quantity(2),
                max_quantity: Quantity(2),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        );
        view.kinds.insert(local_lot, EntityKind::ItemLot);
        view.alive.insert(local_lot, true);
        view.effective_places.insert(local_lot, place_a);
        view.entities_at.entry(place_a).or_default().push(local_lot);
        view.item_lot_commodities
            .insert(local_lot, CommodityKind::Bread);
        view.commodity_quantities
            .insert((local_lot, CommodityKind::Bread), Quantity(1));
        view.merchandise_profiles.insert(
            local_seller,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_facility: Some(place_a),
            },
        );
        connect(&mut view, place_a, place_b, 7);
        let mut source_belief = belief(EntityKind::Facility, place_b);
        source_belief.resource_source = view.resource_sources.get(&remote_source).cloned();
        remember(&mut view, actor, remote_source, source_belief);

        let snapshot = snapshot(&view, actor, 1);
        let goal = crate::GoalOffer {
            key: worldwake_core::GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: worldwake_core::CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::from([local_seller, remote_source]),
            evidence_places: BTreeSet::from([place_a, place_b]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };

        let plan = plan(&snapshot, &goal, &base_budget(), &RecipeRegistry::new()).unwrap();

        assert!(
            plan.steps.is_empty(),
            "local acquire opportunities should remain a direct tactical problem instead of forcing prerequisite travel staging"
        );
    }

    #[test]
    fn place_anchored_acquire_opportunity_scopes_strategic_goal_place() {
        let actor = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let local_source = entity(20);
        let remote_source = entity(21);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place_a);
        register_facility(
            &mut view,
            local_source,
            place_a,
            ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(2),
                max_quantity: Quantity(2),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        );
        register_facility(
            &mut view,
            remote_source,
            place_b,
            ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(2),
                max_quantity: Quantity(2),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        );
        connect(&mut view, place_a, place_b, 5);

        let snapshot = snapshot(&view, actor, 1);
        let goal = crate::GoalOffer {
            key: worldwake_core::GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: worldwake_core::CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            anchor: OpportunityAnchor::Place(place_b),
            evidence_entities: BTreeSet::from([local_source, remote_source]),
            evidence_places: BTreeSet::from([place_a, place_b]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        };

        let plan = plan(&snapshot, &goal, &base_budget(), &RecipeRegistry::new()).unwrap();

        assert_eq!(
            plan.steps,
            vec![super::StrategicStep {
                destination: place_b,
                sub_goal: TacticalSubGoal::SatisfyGoal,
                estimated_travel_ticks: 5,
            }],
            "selected place-anchored opportunities must remain scoped to their chosen place"
        );
    }

    #[test]
    fn test_recipe_input_social_query_uses_missing_commodity() {
        let actor = entity(1);
        let listener = entity(2);
        let place = entity(10);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place);
        register_agent(&mut view, listener, place);

        let mut recipes = RecipeRegistry::new();
        let recipe_id = recipes.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(4).unwrap(),
            required_workstation_tag: None,
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::zero(),
        });
        let snapshot = snapshot(&view, actor, 0);
        let goal = crate::GoalOffer {
            key: worldwake_core::GoalKey::from(GoalKind::ProduceCommodity { recipe_id }),
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
        };

        let plan = plan(&snapshot, &goal, &base_budget(), &recipes).unwrap();

        assert_eq!(plan.steps.len(), 1);
        assert_eq!(
            plan.steps[0].sub_goal,
            TacticalSubGoal::SocialQuery(CommodityKind::Grain)
        );
    }

    #[test]
    fn test_sell_goal_does_not_gain_social_query_fallback_from_target_commodity() {
        let actor = entity(1);
        let listener = entity(2);
        let place = entity(10);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place);
        register_agent(&mut view, listener, place);

        let snapshot = snapshot(&view, actor, 0);
        let goal = crate::GoalOffer {
            key: worldwake_core::GoalKey::from(GoalKind::SellCommodity {
                commodity: CommodityKind::Water,
            }),
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
        };

        assert!(
            plan(&snapshot, &goal, &base_budget(), &RecipeRegistry::new()).is_none(),
            "SellCommodity should keep the pre-existing strategic contract and avoid social-query fallback"
        );
    }
}
