use crate::planner_ops::planner_only_candidates;
use crate::{
    apply_hypothetical_transition, GoalKindPlannerExt, GroundedGoal, PlanTerminalKind, PlannedPlan,
    PlannedStep, PlannerOpKind, PlannerOpSemantics, PlanningBudget, PlanningEntityRef,
    PlanningSnapshot, PlanningState,
};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap};
use worldwake_core::{ActionDefId, EntityId, GoalKind};
use worldwake_sim::{
    get_affordances, ActionDefRegistry, ActionDuration, ActionHandlerRegistry, ActionPayload,
    Affordance, BeliefView, QueueForFacilityUsePayload,
};

#[derive(Clone)]
struct SearchNode<'snapshot> {
    state: PlanningState<'snapshot>,
    steps: Vec<PlannedStep>,
    total_estimated_ticks: u32,
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

pub fn search_plan(
    snapshot: &PlanningSnapshot,
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    budget: &PlanningBudget,
) -> Option<PlannedPlan> {
    if unsupported_goal(&goal.key.kind) {
        return None;
    }

    let mut frontier = BinaryHeap::new();
    frontier.push(FrontierEntry::new(root_node(snapshot)));
    let mut expansions = 0u16;

    while let Some(node) = frontier.pop().map(FrontierEntry::into_node) {
        if goal.key.kind.is_satisfied(&node.state) {
            return Some(PlannedPlan::new(
                goal.key,
                node.steps,
                PlanTerminalKind::GoalSatisfied,
            ));
        }
        if node.steps.len() >= usize::from(budget.max_plan_depth) {
            continue;
        }
        if expansions >= budget.max_node_expansions {
            return None;
        }
        expansions = expansions.saturating_add(1);

        let mut terminal_successors = Vec::new();
        let mut successors = Vec::new();
        for candidate in search_candidates(goal, &node, semantics_table, registry, handlers) {
            let Some((terminal, successor)) =
                build_successor(goal, semantics_table, registry, &node, &candidate)
            else {
                continue;
            };
            if let Some(terminal_kind) = terminal {
                terminal_successors.push((terminal_kind, successor));
            } else {
                successors.push((terminal, successor));
            }
        }
        if !terminal_successors.is_empty() {
            terminal_successors.sort_by(|left, right| compare_search_nodes(&left.1, &right.1));
            let (terminal_kind, successor) = terminal_successors.remove(0);
            return Some(PlannedPlan::new(goal.key, successor.steps, terminal_kind));
        }
        successors.sort_by(|left, right| compare_search_nodes(&left.1, &right.1));
        successors.truncate(usize::from(budget.beam_width));

        for (terminal, successor) in successors {
            if let Some(terminal_kind) = terminal {
                return Some(PlannedPlan::new(goal.key, successor.steps, terminal_kind));
            }
            frontier.push(FrontierEntry::new(successor));
        }
    }

    None
}

fn root_node(snapshot: &PlanningSnapshot) -> SearchNode<'_> {
    SearchNode {
        state: PlanningState::new(snapshot),
        steps: Vec::new(),
        total_estimated_ticks: 0,
    }
}

fn build_successor<'snapshot>(
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    node: &SearchNode<'snapshot>,
    candidate: &SearchCandidate,
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
    let estimated_ticks = match duration {
        ActionDuration::Finite(ticks) => ticks,
        ActionDuration::Indefinite if semantics.may_appear_mid_plan => return None,
        ActionDuration::Indefinite => 0,
    };

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
    let mut steps = node.steps.clone();
    steps.push(step);

    Some((
        terminal,
        SearchNode {
            state: transition.state,
            steps,
            total_estimated_ticks,
        },
    ))
}

fn search_candidates(
    goal: &GroundedGoal,
    node: &SearchNode<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
) -> Vec<SearchCandidate> {
    let mut candidates = get_affordances(
        &node.state,
        node.state.snapshot().actor(),
        registry,
        handlers,
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
    matches!(
        goal,
        GoalKind::SellCommodity { .. } | GoalKind::BuryCorpse { .. }
    )
}

fn compare_search_nodes(left: &SearchNode<'_>, right: &SearchNode<'_>) -> Ordering {
    left.total_estimated_ticks
        .cmp(&right.total_estimated_ticks)
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
        build_successor, root_node, search_candidate_from_planner, search_candidates,
        search_candidates_from_affordance, search_plan, FrontierEntry, SearchCandidate, SearchNode,
    };
    use crate::planner_ops::planner_only_candidates;
    use crate::{
        build_planning_snapshot, build_semantics_table, CommodityPurpose, GoalKey, GoalKind,
        GroundedGoal, PlanTerminalKind, PlannedStep, PlannerOpKind, PlannerOpSemantics,
        PlanningBudget, PlanningEntityRef, PlanningSnapshot, PlanningState,
    };
    use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, prototype_place_entity,
        test_utils::sample_trade_disposition_profile, ActionDefId, BodyCostPerTick, CarryCapacity,
        CauseRef, CombatProfile, CommodityConsumableProfile, CommodityKind, ControlSource,
        DemandMemory, DemandObservation, DemandObservationReason, DeprivationExposure,
        DriveThresholds, EntityId, EntityKind, EventLog, ExclusiveFacilityPolicy, FacilityUseQueue,
        GrantedFacilityUse, HomeostaticNeeds, InTransitOnEdge, KnownRecipes, LoadUnits,
        MerchandiseProfile, MetabolismProfile, Permille, Place, PrototypePlace, Quantity, RecipeId,
        ResourceSource, Tick, TickRange, Topology, TradeDispositionProfile, TravelEdge,
        TravelEdgeId, UniqueItemKind, VisibilitySpec, WitnessData, WorkstationMarker,
        WorkstationTag, World, WorldTxn, Wound,
    };
    use worldwake_sim::{
        estimate_duration_from_beliefs, ActionDefRegistry, ActionPayload, Affordance, BeliefView,
        DurationExpr, OmniscientBeliefView, QueueForFacilityUsePayload, RecipeDefinition,
        RecipeRegistry, TransportActionPayload,
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

    impl BeliefView for TestBeliefView {
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
        )
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
        )
        .unwrap();

        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Travel);
        assert_eq!(plan.steps[1].op_kind, PlannerOpKind::Consume);
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
        );

        assert_eq!(plan, None);
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
        )
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
        )
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
        );

        assert_eq!(plan, None);
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
        );

        assert_eq!(plan, None);
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
        )
        .unwrap();

        assert_eq!(narrow_beam_plan, None);
        assert_eq!(
            wide_beam_plan.terminal_kind,
            PlanTerminalKind::GoalSatisfied
        );
        assert_eq!(wide_beam_plan.steps.len(), 2);
        assert_eq!(wide_beam_plan.steps[0].op_kind, PlannerOpKind::Travel);
        assert_eq!(wide_beam_plan.steps[1].op_kind, PlannerOpKind::Consume);
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
        )
        .unwrap();

        assert_eq!(beam_two_plan, None);
        assert_eq!(
            beam_three_plan.terminal_kind,
            PlanTerminalKind::GoalSatisfied
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
        );
        let sufficient_budget_plan = search_plan(
            &snapshot,
            &consume_goal(CommodityKind::Bread),
            &build_semantics_table(&registry),
            &registry,
            &handlers,
            &PlanningBudget {
                beam_width: 3,
                max_node_expansions: 4,
                ..PlanningBudget::default()
            },
        )
        .unwrap();

        assert_eq!(exhausted_plan, None);
        assert_eq!(
            sufficient_budget_plan.terminal_kind,
            PlanTerminalKind::GoalSatisfied
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
        );

        assert_eq!(plan, None);
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
        );

        assert_eq!(plan, None);
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
        )
        .unwrap();

        assert_eq!(plan.terminal_kind, PlanTerminalKind::GoalSatisfied);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].op_kind, PlannerOpKind::MoveCargo);
    }

    #[test]
    fn search_returns_pick_up_goal_satisfaction_for_local_treatment_lot() {
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
            key: acquire_goal_with_purpose(CommodityKind::Medicine, CommodityPurpose::Treatment)
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
        )
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
        )
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
        )
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
        )
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

        let view = OmniscientBeliefView::new(&world);
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
        };

        let initial_candidates = search_candidates(&goal, &node, &semantics, &registry, &handlers);
        let pick_up = initial_candidates
            .iter()
            .find(|candidate| {
                registry
                    .get(candidate.def_id)
                    .is_some_and(|def| def.name == "pick_up")
            })
            .expect("authoritative snapshot should expose cargo pick_up");
        let (terminal, after_pick_up) =
            build_successor(&goal, &semantics, &registry, &node, pick_up).unwrap();
        assert_eq!(terminal, None);
        assert_eq!(
            after_pick_up.steps[0].targets,
            vec![PlanningEntityRef::Authoritative(bread)]
        );
        assert!(!after_pick_up.steps[0].expected_materializations.is_empty());

        let follow_up_candidates =
            search_candidates(&goal, &after_pick_up, &semantics, &registry, &handlers);
        let travel = follow_up_candidates
            .iter()
            .find(|candidate| {
                registry
                    .get(candidate.def_id)
                    .is_some_and(|def| def.name == "travel")
                    && candidate.authoritative_targets == vec![destination]
            })
            .expect("partial cargo successor should expose travel to destination");
        let (terminal, _) =
            build_successor(&goal, &semantics, &registry, &after_pick_up, travel).unwrap();

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
        )
        .unwrap();

        assert_eq!(plan.steps.len(), 1);
        assert!(matches!(
            (plan.steps[0].op_kind, plan.terminal_kind),
            (PlannerOpKind::Travel, PlanTerminalKind::GoalSatisfied)
                | (
                    PlannerOpKind::Attack | PlannerOpKind::Defend,
                    PlanTerminalKind::CombatCommitment
                )
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
        )
        .unwrap();

        assert!(matches!(
            plan.steps[0].op_kind,
            PlannerOpKind::Attack | PlannerOpKind::Defend
        ));
        assert_eq!(plan.terminal_kind, PlanTerminalKind::CombatCommitment);
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
        let (_, successor) =
            build_successor(&goal, &semantics_table, &registry, &node, &candidate).unwrap();

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
        let (_, successor) =
            build_successor(&goal, &semantics_table, &registry, &node, &candidate).unwrap();

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
        let (registry, handlers) = build_registry_with_recipes(&recipes);
        let semantics = build_semantics_table(&registry);
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::RestockCommodity {
                commodity: CommodityKind::Apple,
            }),
            evidence_entities: BTreeSet::from([orchard_row]),
            evidence_places: BTreeSet::from([village_square, orchard_farm]),
        };
        let view = OmniscientBeliefView::new(&world);
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
        )
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
        let view = OmniscientBeliefView::new(&fixture.world);
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
        )
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
        let view = OmniscientBeliefView::new(&fixture.world);
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
        )
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
        let view = OmniscientBeliefView::new(&fixture.world);
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
            &root_node(&snapshot),
            &fixture.semantics,
            &fixture.registry,
            &fixture.handlers,
        );

        assert!(!candidates.iter().any(|candidate| {
            candidate.def_id == queue_def
                && candidate.authoritative_targets == vec![fixture.orchard_row]
        }));
    }

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
        let view = OmniscientBeliefView::new(&world);
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
}
