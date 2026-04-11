#![allow(dead_code)]

use crate::{GoalKindPlannerExt, GroundedGoal, PlanningSnapshot, PlanningState};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use worldwake_core::{CommodityKind, EntityId, EntityKind, ExecutionBudget, GoalKind, Quantity};
use worldwake_sim::{
    EconomicBeliefView, FacilityBeliefView, InventoryBeliefView, RecipeRegistry, SpatialBeliefView,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StrategicPlan {
    pub steps: Vec<StrategicStep>,
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
    goal: &GroundedGoal,
    execution_budget: &ExecutionBudget,
    recipes: &RecipeRegistry,
) -> Option<StrategicPlan> {
    let state = PlanningState::new(snapshot);
    if goal.key.kind.is_satisfied(&state) {
        return Some(StrategicPlan { steps: Vec::new() });
    }

    let actor = snapshot.actor();
    let actor_place = state.effective_place(actor)?;
    let goal_places = goal_places(goal, &state, recipes);
    let missing_commodities = missing_commodities(
        goal,
        &state,
        recipes,
        actor_place,
        &goal_places,
    );
    let query_commodity = social_query_commodity(goal, &missing_commodities);
    let mut stages = build_stages(
        &state,
        snapshot,
        actor,
        actor_place,
        &goal_places,
        &missing_commodities,
        execution_budget.max_prerequisite_locations,
    );

    if stages.is_empty() {
        if goal_places.contains(&actor_place) {
            return Some(StrategicPlan { steps: Vec::new() });
        }
        return exploration_plan(snapshot, actor_place, &goal.key.kind)
            .or_else(|| social_query_plan(snapshot, actor_place, query_commodity));
    }

    let mut local_steps = Vec::new();
    consume_current_place_prerequisites(actor_place, &mut stages, &mut local_steps);

    if stages.is_empty() || matches_local_goal_stage(&stages, actor_place) {
        return Some(StrategicPlan { steps: local_steps });
    }

    let search_budget = usize::max(
        1,
        usize::from(execution_budget.max_prerequisite_locations) * 2,
    );
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
            return Some(StrategicPlan { steps: node.steps });
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

    None
}

fn goal_places(
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
) -> Vec<EntityId> {
    let mut places = goal.key.kind.goal_relevant_places(state, recipes);
    if places.is_empty() && matches!(goal.key.kind, GoalKind::SearchForMissing { .. }) {
        places.extend(goal.evidence_places.iter().copied());
    }
    places.sort_unstable();
    places.dedup();
    places
}

fn missing_commodities(
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
    actor_place: EntityId,
    goal_places: &[EntityId],
) -> Vec<CommodityKind> {
    let actor = state.snapshot().actor();
    let mut commodities = match goal.key.kind {
        GoalKind::AcquireCommodity { commodity, .. } => (state.commodity_quantity(actor, commodity)
            == Quantity(0)
            && !goal_places.contains(&actor_place))
        .then_some(commodity)
        .into_iter()
        .collect(),
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
    goal: &GroundedGoal,
    missing_commodities: &[CommodityKind],
) -> Option<CommodityKind> {
    match goal.key.kind {
        GoalKind::ConsumeOwnedCommodity { commodity }
        | GoalKind::AcquireCommodity { commodity, .. }
        | GoalKind::RestockCommodity { commodity } => Some(commodity),
        GoalKind::TreatWounds { .. } => Some(CommodityKind::Medicine),
        GoalKind::ProduceCommodity { .. } => missing_commodities.first().copied(),
        _ => None,
    }
}

fn build_stages(
    state: &PlanningState<'_>,
    snapshot: &PlanningSnapshot,
    actor: EntityId,
    actor_place: EntityId,
    goal_places: &[EntityId],
    missing_commodities: &[CommodityKind],
    per_stage_limit: u8,
) -> Vec<StrategicStage> {
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

    stages
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
    for entity in state.snapshot().entities.keys().copied() {
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
    state.entities_at(place).into_iter().any(|entity| {
        state.resource_source(entity).is_some_and(|source| {
            source.commodity == commodity && source.available_quantity > Quantity(0)
        }) || state
            .merchandise_profile(entity)
            .is_some_and(|profile| profile.sale_kinds.contains(&commodity))
            || (state.item_lot_commodity(entity) == Some(commodity)
                && state.commodity_quantity(entity, commodity) > Quantity(0)
                && state.direct_possessor(entity).is_none()
                && state.direct_container(entity).is_none())
    })
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
    use super::{TacticalSubGoal, plan};
    use crate::build_planning_snapshot;
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        AgentBeliefStore, BeliefConfidencePolicy, BelievedEntityState, BodyCostPerTick, BodyPart,
        CommodityKind, DeprivationKind, EntityId, EntityKind, GoalKind, InTransitOnEdge,
        InstitutionalBeliefRead, LoadUnits, MerchandiseProfile, OpportunityAnchor, PatrolRoute,
        Permille, Quantity, RecipeId, ResourceSource, Tick, TickRange, ToldBeliefMemory, Wound,
        WoundCause, WoundId,
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
                merchandise_profiles: BTreeMap::new(),
                wounds: BTreeMap::new(),
            }
        }
    }

    impl ControlBeliefView for StubBeliefView {
        fn believed_owner_of(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
            actor == entity
        }

        fn has_control(&self, entity: EntityId) -> bool {
            self.kinds.get(&entity) == Some(&EntityKind::Agent)
        }
    }

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

        fn believed_office_holder(
            &self,
            _office: EntityId,
        ) -> InstitutionalBeliefRead<Option<EntityId>> {
            InstitutionalBeliefRead::Unknown
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
        fn workstation_tag(&self, _entity: EntityId) -> Option<worldwake_core::WorkstationTag> {
            None
        }

        fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
            self.resource_sources.get(&entity).cloned()
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn matching_workstations_at(
            &self,
            _place: EntityId,
            _tag: worldwake_core::WorkstationTag,
        ) -> Vec<EntityId> {
            Vec::new()
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
            observed_tick: Tick(1),
            source: worldwake_core::PerceptionSource::DirectObservation,
        }
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

    fn base_budget() -> worldwake_core::ExecutionBudget {
        worldwake_core::ExecutionBudget {
            max_prerequisite_locations: 3,
            ..worldwake_core::ExecutionBudget::default()
        }
    }

    #[test]
    fn test_single_location_goal_no_travel() {
        let actor = entity(1);
        let place = entity(10);
        let mut view = StubBeliefView::default();
        register_agent(&mut view, actor, place);

        let snapshot = snapshot(&view, actor, 0);
        let goal = crate::GroundedGoal {
            key: worldwake_core::GoalKey::from(GoalKind::Patrol { place }),
            anchor: OpportunityAnchor::Place(place),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
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
            },
        );
        connect(&mut view, place_a, place_b, 3);
        connect(&mut view, place_b, place_c, 5);
        view.known_entity_beliefs
            .insert(actor, vec![(patient, belief(EntityKind::Agent, place_c))]);

        let snapshot = snapshot(&view, actor, 2);
        let goal = crate::GroundedGoal {
            key: worldwake_core::GoalKey::from(GoalKind::TreatWounds { patient }),
            anchor: OpportunityAnchor::Entity(patient),
            evidence_entities: BTreeSet::from([patient]),
            evidence_places: BTreeSet::from([place_c]),
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
        let goal = crate::GroundedGoal {
            key: worldwake_core::GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: worldwake_core::CommodityPurpose::SelfConsume,
            }),
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
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
        let goal = crate::GroundedGoal {
            key: worldwake_core::GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: worldwake_core::CommodityPurpose::SelfConsume,
            }),
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
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
        let goal = crate::GroundedGoal {
            key: worldwake_core::GoalKey::from(GoalKind::Sleep),
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
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
        let goal = crate::GroundedGoal {
            key: worldwake_core::GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: worldwake_core::CommodityPurpose::SelfConsume,
            }),
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
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
        let goal = crate::GroundedGoal {
            key: worldwake_core::GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: worldwake_core::CommodityPurpose::SelfConsume,
            }),
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
        };

        assert!(plan(&snapshot, &goal, &base_budget(), &RecipeRegistry::new()).is_none());
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
            },
        );
        connect(&mut view, place_a, place_b, 7);

        let snapshot = snapshot(&view, actor, 1);
        let goal = crate::GroundedGoal {
            key: worldwake_core::GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: worldwake_core::CommodityPurpose::SelfConsume,
            }),
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
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

        let snapshot = snapshot(&view, actor, 1);
        let goal = crate::GroundedGoal {
            key: worldwake_core::GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: worldwake_core::CommodityPurpose::SelfConsume,
            }),
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::from([local_seller, remote_source]),
            evidence_places: BTreeSet::from([place_a, place_b]),
        };

        let plan = plan(&snapshot, &goal, &base_budget(), &RecipeRegistry::new()).unwrap();

        assert!(
            plan.steps.is_empty(),
            "local acquire opportunities should remain a direct tactical problem instead of forcing prerequisite travel staging"
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
        let goal = crate::GroundedGoal {
            key: worldwake_core::GoalKey::from(GoalKind::ProduceCommodity { recipe_id }),
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
        };

        let plan = plan(&snapshot, &goal, &base_budget(), &recipes).unwrap();

        assert_eq!(plan.steps.len(), 1);
        assert_eq!(
            plan.steps[0].sub_goal,
            TacticalSubGoal::SocialQuery(CommodityKind::Grain)
        );
    }
}
