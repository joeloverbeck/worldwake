use super::candidates::{
    CandidateSearchContext, CommodityFilterContext, apply_commodity_relevance_filter,
    candidate_blocked_by_place, relevant_action_defs, root_candidate_trace_from_candidate,
};
use super::{
    FrontierEntry, SearchCandidate, SearchNode, TacticalGoal, compare_search_nodes,
    compute_heuristic, prune_travel_away_from_goal, search_candidate_from_planner,
    search_candidates, search_candidates_from_affordance,
};
use crate::goal_model::GoalKindPlannerExt;
use crate::planner_ops::planner_only_candidates;
use crate::shared_collections::SharedVec;
use crate::{
    CommodityPurpose, GoalKey, GoalKind, GoalOffer, PlanSearchResult, PlanTerminalKind,
    PlannedStep, PlannerOpKind, PlannerOpSemantics, PlannerSyntheticCargo, PlanningEntityRef,
    PlanningSnapshot, PlanningState, ProfileFixture, RequiredFact, build_planning_snapshot,
    build_planning_snapshot_with_blocked_facility_uses, build_semantics_table,
};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};
use std::num::NonZeroU32;
use worldwake_core::{
    AcquisitionQuantity, ActionDefId, AgentBeliefStore, ArtifactActionability, ArtifactCredibility,
    ArtifactExistence, ArtifactKind, ArtifactLegalEffect, ArtifactPostingContext,
    ArtifactVisibility, BelievedArtifactState, BelievedBountyTerms, BelievedEntityState, Blocker,
    BlockerKey, BlockerMemory, BlockingFact, BodyCostPerTick, BodyPart, BountyTarget, BountyTerms,
    CarryCapacity, CauseRef, CognitiveProfile, CombatProfile, CommodityConsumableProfile,
    CommodityKind, ContentionGrant, ContentionPolicy, ContentionQueue, ControlSource, DeadAt,
    DemandMemory, DemandObservation, DemandObservationReason, DeprivationExposure, DeprivationKind,
    DriveThresholds, EntityId, EntityKind, EpistemicDispositionProfile, EventLog, ExecutionBudget,
    HomeostaticNeedId, HomeostaticNeeds, InTransitOnEdge, KnownRecipes, LoadUnits,
    MerchandiseProfile, MetabolismProfile, NoticeTopic, ObservationOmission,
    ObservationOmissionLog, OmissionReason, OpportunityAnchor, OpportunityKey, PatrolProfile,
    PatrolRoute, PerceptionSource, Permille, Place, PlaceTag, PortfolioSlotWeights,
    ProofRequirement, PrototypePlace, Quantity, RecipeId, RecordedViolation, ResourceSource,
    RewardSource, TellTopic, TheftDispositionProfile, Tick, TickRange, Topology,
    TradeDispositionProfile, TravelEdge, TravelEdgeId, UniqueItemKind, ViolationDispositionProfile,
    ViolationId, ViolationKind, VisibilitySpec, WashBasinState, WitnessData, WorkstationMarker,
    WorkstationTag, World, WorldTxn, Wound, WoundCause, WoundId, build_believed_entity_state,
    build_prototype_world, prototype_place_entity, test_utils::sample_trade_disposition_profile,
};
use worldwake_sim::{
    ActionDefRegistry, ActionPayload, Affordance, CombatBeliefView, ControlBeliefView,
    DurationExpr, EconomicBeliefView, EntityBeliefView, PerAgentBeliefView, ProfileBeliefView,
    QueueForFacilityUsePayload, RecipeDefinition, RecipeRegistry, RuntimeBeliefView,
    SpatialBeliefView, TemporalBeliefView, TradeActionPayload, TransportActionPayload,
    estimate_duration_from_beliefs,
};
use worldwake_systems::build_full_action_registries;

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
        structural_block_ticks: reasoning.structural_block_ticks,
        stale_belief_backoff_ticks: CognitiveProfile::default().stale_belief_backoff_ticks,
        contradicted_belief_backoff_ticks: CognitiveProfile::default()
            .contradicted_belief_backoff_ticks,
        improper_state_backoff_ticks: CognitiveProfile::default().improper_state_backoff_ticks,
        missing_observation_backoff_ticks: CognitiveProfile::default()
            .missing_observation_backoff_ticks,
        no_legal_binding_backoff_ticks: CognitiveProfile::default().no_legal_binding_backoff_ticks,
        counterparty_refusal_backoff_ticks: CognitiveProfile::default()
            .counterparty_refusal_backoff_ticks,
        route_unknown_backoff_ticks: CognitiveProfile::default().route_unknown_backoff_ticks,
        search_exhaustion_backoff_ticks: CognitiveProfile::default()
            .search_exhaustion_backoff_ticks,
        partial_drift_backoff_ticks: CognitiveProfile::default().partial_drift_backoff_ticks,
        expectation_tolerance_ticks: CognitiveProfile::default().expectation_tolerance_ticks,
        guard_min_confidence_ceiling: CognitiveProfile::default().guard_min_confidence_ceiling,
        repair_memory_ticks: CognitiveProfile::default().repair_memory_ticks,
        learned_opportunity_memory_ticks: CognitiveProfile::default()
            .learned_opportunity_memory_ticks,
        survey_memory_capacity: CognitiveProfile::default().survey_memory_capacity,
        survey_memory_retention_ticks: CognitiveProfile::default().survey_memory_retention_ticks,
        initial_cooldown_ticks: reasoning.initial_cooldown_ticks,
        max_cooldown_ticks: reasoning.max_cooldown_ticks,
        landmark_extraction_depth: CognitiveProfile::default().landmark_extraction_depth,
        use_ff_heuristic: CognitiveProfile::default().use_ff_heuristic,
        decision_history_alternatives: CognitiveProfile::default().decision_history_alternatives,
        detour_budget_permille: CognitiveProfile::default().detour_budget_permille,
        compile_opportunity_cap: CognitiveProfile::default().compile_opportunity_cap,
        slot_weights: PortfolioSlotWeights::default(),
    }
}

fn execution_budget(reasoning: &ProfileFixture) -> ExecutionBudget {
    ExecutionBudget::new(
        reasoning.beam_width,
        reasoning.max_prerequisite_locations,
        ExecutionBudget::default().preferred_operator_boost(),
    )
}

fn candidate_search_context<'a>(
    semantics_table: &'a BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &'a ActionDefRegistry,
    handlers: &'a worldwake_sim::ActionHandlerRegistry,
    blocked: &'a BlockerMemory,
    current_tick: Tick,
    relevant_defs: &'a BTreeSet<ActionDefId>,
) -> CandidateSearchContext<'a> {
    CandidateSearchContext {
        semantics_table,
        registry,
        handlers,
        blocked,
        current_tick,
        relevant_defs,
        candidate_source: crate::decision_trace::CandidateSource::Emitter,
    }
}

fn commodity_filter_context<'a>(
    tactical_goal: Option<&'a TacticalGoal>,
    semantics_table: &'a BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &'a ActionDefRegistry,
    recipes: &'a RecipeRegistry,
) -> CommodityFilterContext<'a> {
    CommodityFilterContext {
        tactical_goal,
        semantics_table,
        registry,
        recipes,
    }
}

#[allow(clippy::too_many_arguments)]
fn search_plan(
    snapshot: &PlanningSnapshot,
    goal: &GoalOffer,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &worldwake_sim::ActionHandlerRegistry,
    reasoning: &ProfileFixture,
    recipes: &RecipeRegistry,
    blocked: &BlockerMemory,
    current_tick: Tick,
    binding_rejections: Option<&mut Vec<crate::decision_trace::BindingRejection>>,
    expansion_summaries: Option<&mut Vec<crate::decision_trace::SearchExpansionSummary>>,
) -> PlanSearchResult {
    super::search_plan(
        snapshot,
        goal,
        semantics_table,
        registry,
        handlers,
        &cognitive(reasoning),
        &execution_budget(reasoning),
        recipes,
        blocked,
        current_tick,
        binding_rejections,
        expansion_summaries,
    )
}

#[allow(clippy::too_many_arguments)]
fn search_plan_with_trace_metadata(
    snapshot: &PlanningSnapshot,
    goal: &GoalOffer,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &worldwake_sim::ActionHandlerRegistry,
    reasoning: &ProfileFixture,
    recipes: &RecipeRegistry,
    blocked: &BlockerMemory,
    current_tick: Tick,
    binding_rejections: Option<&mut Vec<crate::decision_trace::BindingRejection>>,
    expansion_summaries: Option<&mut Vec<crate::decision_trace::SearchExpansionSummary>>,
    trace_metadata: Option<&mut super::SearchTraceMetadata>,
) -> PlanSearchResult {
    super::search_plan_with_trace_metadata(
        snapshot,
        goal,
        semantics_table,
        registry,
        handlers,
        &cognitive(reasoning),
        &execution_budget(reasoning),
        recipes,
        blocked,
        current_tick,
        binding_rejections,
        expansion_summaries,
        trace_metadata,
    )
}

fn build_successor<'snapshot>(
    goal: &GoalOffer,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    node: &SearchNode<'snapshot>,
    candidate: &SearchCandidate,
    recipes: &RecipeRegistry,
    reasoning: &ProfileFixture,
) -> Option<(Option<PlanTerminalKind>, SearchNode<'snapshot>)> {
    super::build_successor(
        goal,
        semantics_table,
        registry,
        node,
        candidate,
        recipes,
        &execution_budget(reasoning),
        None,
        &super::landmarks::LandmarkSet::empty(),
    )
}

fn combined_relevant_places(
    goal: &GoalOffer,
    state: &PlanningState<'_>,
    recipes: &RecipeRegistry,
    reasoning: &ProfileFixture,
) -> super::heuristic::CombinedRelevantPlaces {
    super::combined_relevant_places(goal, state, recipes, &execution_budget(reasoning))
}

fn root_node<'snapshot>(
    snapshot: &'snapshot PlanningSnapshot,
    goal: &GoalOffer,
    recipes: &RecipeRegistry,
    reasoning: &ProfileFixture,
) -> SearchNode<'snapshot> {
    super::root_node(snapshot, goal, recipes, &execution_budget(reasoning))
}

type TacticalRootSuccessor<'snapshot> =
    (usize, Option<PlanTerminalKind>, SearchNode<'snapshot>, bool);
type TacticalRootSuccessorSet<'snapshot> = (
    SearchNode<'snapshot>,
    Vec<TacticalRootSuccessor<'snapshot>>,
    Vec<SearchCandidate>,
    Vec<super::landmarks::PlanningOperator>,
    BTreeMap<ActionDefId, PlannerOpSemantics>,
);

fn collect_root_non_terminal_successors_for_tactical_goal<'snapshot>(
    snapshot: &'snapshot PlanningSnapshot,
    goal: &GoalOffer,
    registry: &ActionDefRegistry,
    handlers: &worldwake_sim::ActionHandlerRegistry,
    reasoning: &ProfileFixture,
    recipes: &RecipeRegistry,
    tactical_goal: &super::TacticalGoal,
) -> TacticalRootSuccessorSet<'snapshot> {
    let semantics_table = build_semantics_table(registry);
    let budget = execution_budget(reasoning);
    let node = super::heuristic::root_node_for_tactical(
        snapshot,
        goal,
        recipes,
        &budget,
        Some(tactical_goal),
    );
    let relevant_defs = relevant_action_defs(goal, &semantics_table);
    let mut candidates = search_candidates(
        goal,
        &node,
        candidate_search_context(
            &semantics_table,
            registry,
            handlers,
            &BlockerMemory::default(),
            Tick(0),
            &relevant_defs,
        ),
        None,
        None,
        None,
    );
    apply_commodity_relevance_filter(
        &mut candidates,
        goal,
        &node.state,
        commodity_filter_context(Some(tactical_goal), &semantics_table, registry, recipes),
        None,
    );
    super::apply_tactical_candidate_filter(
        &mut candidates,
        goal,
        &node,
        &semantics_table,
        Some(tactical_goal),
    );
    let combined_places = super::heuristic::combined_relevant_places_for_tactical(
        goal,
        &node.state,
        recipes,
        &budget,
        Some(tactical_goal),
    );
    if let Some(current_place) = node
        .state
        .effective_place_ref(PlanningEntityRef::Authoritative(snapshot.actor()))
    {
        prune_travel_away_from_goal(
            &mut candidates,
            current_place,
            &combined_places.places,
            snapshot,
            &semantics_table,
        );
    }

    let successor_candidates = candidates.clone();
    let mut successors = Vec::new();
    let mut operators = Vec::new();
    for candidate in candidates {
        let Ok((terminal, successor, _)) = super::build_successor_detailed(
            goal,
            &semantics_table,
            registry,
            &node,
            &candidate,
            recipes,
            &budget,
            Some(tactical_goal),
            &super::landmarks::LandmarkSet::empty(),
        ) else {
            continue;
        };
        if terminal.is_none() {
            operators.push(super::landmarks::planning_operator_from_transition(
                &node.state,
                &successor.state,
            ));
            successors.push((successors.len(), terminal, successor, false));
        }
    }
    (
        node,
        successors,
        successor_candidates,
        operators,
        semantics_table,
    )
}

struct TestBeliefView {
    current_tick: Tick,
    alive: BTreeSet<EntityId>,
    kinds: BTreeMap<EntityId, EntityKind>,
    effective_places: BTreeMap<EntityId, EntityId>,
    entities_at: BTreeMap<EntityId, Vec<EntityId>>,
    direct_possessions: BTreeMap<EntityId, Vec<EntityId>>,
    direct_possessors: BTreeMap<EntityId, EntityId>,
    direct_containers: BTreeMap<EntityId, EntityId>,
    owners: BTreeMap<EntityId, EntityId>,
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
    theft_profiles: BTreeMap<EntityId, TheftDispositionProfile>,
    merchandise_profiles: BTreeMap<EntityId, MerchandiseProfile>,
    listed_lots: BTreeMap<(EntityId, CommodityKind), Vec<EntityId>>,
    lot_sellers: BTreeMap<EntityId, EntityId>,
    demand_memory: BTreeMap<EntityId, Vec<DemandObservation>>,
    hostiles: BTreeMap<EntityId, Vec<EntityId>>,
    attackers: BTreeMap<EntityId, Vec<EntityId>>,
    wounds: BTreeMap<EntityId, Vec<Wound>>,
    office_data: BTreeMap<EntityId, worldwake_core::OfficeData>,
    office_holder_beliefs:
        BTreeMap<EntityId, worldwake_core::InstitutionalBeliefRead<Option<EntityId>>>,
    consultation_speed_factors: BTreeMap<EntityId, Permille>,
    record_data: BTreeMap<EntityId, worldwake_core::RecordData>,
    patrol_profiles: BTreeMap<EntityId, PatrolProfile>,
    patrol_routes: BTreeMap<EntityId, PatrolRoute>,
    violation_profiles: BTreeMap<EntityId, ViolationDispositionProfile>,
    active_violation_records: BTreeMap<EntityId, Vec<RecordedViolation>>,
    known_entity_beliefs: BTreeMap<EntityId, Vec<(EntityId, BelievedEntityState)>>,
    belief_stores: BTreeMap<EntityId, AgentBeliefStore>,
    epistemic_profiles: BTreeMap<EntityId, EpistemicDispositionProfile>,
    stock_storage_policies: BTreeMap<EntityId, worldwake_core::StockStoragePolicy>,
}

impl Default for TestBeliefView {
    fn default() -> Self {
        Self {
            current_tick: Tick(0),
            alive: BTreeSet::new(),
            kinds: BTreeMap::new(),
            effective_places: BTreeMap::new(),
            entities_at: BTreeMap::new(),
            direct_possessions: BTreeMap::new(),
            direct_possessors: BTreeMap::new(),
            direct_containers: BTreeMap::new(),
            owners: BTreeMap::new(),
            controllable: BTreeSet::new(),
            adjacent: BTreeMap::new(),
            lot_commodities: BTreeMap::new(),
            consumable_profiles: BTreeMap::new(),
            commodity_quantities: BTreeMap::new(),
            carry_capacities: BTreeMap::new(),
            entity_loads: BTreeMap::new(),
            needs: BTreeMap::new(),
            thresholds: BTreeMap::new(),
            trade_profiles: BTreeMap::new(),
            theft_profiles: BTreeMap::new(),
            merchandise_profiles: BTreeMap::new(),
            listed_lots: BTreeMap::new(),
            lot_sellers: BTreeMap::new(),
            demand_memory: BTreeMap::new(),
            hostiles: BTreeMap::new(),
            attackers: BTreeMap::new(),
            wounds: BTreeMap::new(),
            office_data: BTreeMap::new(),
            office_holder_beliefs: BTreeMap::new(),
            consultation_speed_factors: BTreeMap::new(),
            record_data: BTreeMap::new(),
            patrol_profiles: BTreeMap::new(),
            patrol_routes: BTreeMap::new(),
            violation_profiles: BTreeMap::new(),
            active_violation_records: BTreeMap::new(),
            known_entity_beliefs: BTreeMap::new(),
            belief_stores: BTreeMap::new(),
            epistemic_profiles: BTreeMap::new(),
            stock_storage_policies: BTreeMap::new(),
        }
    }
}

impl ControlBeliefView for TestBeliefView {
    fn believed_owner_of(&self, entity: EntityId) -> Option<EntityId> {
        self.owners.get(&entity).copied()
    }

    fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
        self.controllable.contains(&(actor, entity))
    }

    fn has_control(&self, entity: EntityId) -> bool {
        self.kinds.get(&entity) == Some(&EntityKind::Agent)
    }
}

impl EntityBeliefView for TestBeliefView {
    fn is_alive(&self, entity: EntityId) -> bool {
        self.alive.contains(&entity)
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

impl ProfileBeliefView for TestBeliefView {
    fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
        self.needs.get(&agent).copied()
    }
    fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
        self.thresholds.get(&agent).copied()
    }
    fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
        Some(MetabolismProfile::default())
    }
}

impl SpatialBeliefView for TestBeliefView {
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
            .map(|(place, _)| place)
            .collect()
    }
    fn route_exists(&self, _from: EntityId, _to: EntityId) -> bool {
        false
    }
    fn in_transit_state(&self, _entity: EntityId) -> Option<InTransitOnEdge> {
        None
    }
    fn adjacent_places_with_travel_ticks(&self, place: EntityId) -> Vec<(EntityId, NonZeroU32)> {
        self.adjacent.get(&place).cloned().unwrap_or_default()
    }
    fn patrol_route(&self, agent: EntityId) -> Option<PatrolRoute> {
        self.patrol_routes.get(&agent).cloned()
    }
}

impl TemporalBeliefView for TestBeliefView {
    fn current_tick(&self) -> Tick {
        self.current_tick
    }
    fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
        false
    }
    fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
        Vec::new()
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

impl RuntimeBeliefView for TestBeliefView {}

impl worldwake_sim::SocialBeliefView for TestBeliefView {
    fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
        self.known_entity_beliefs
            .get(&agent)
            .cloned()
            .unwrap_or_default()
    }
    fn agent_belief_store(&self, agent: EntityId) -> Option<&AgentBeliefStore> {
        self.belief_stores.get(&agent)
    }
    fn belief_confidence_policy(&self, _agent: EntityId) -> worldwake_core::BeliefConfidencePolicy {
        worldwake_core::BeliefConfidencePolicy::default()
    }
    fn epistemic_disposition_profile(
        &self,
        agent: EntityId,
    ) -> Option<EpistemicDispositionProfile> {
        self.epistemic_profiles.get(&agent).cloned()
    }
    fn theft_disposition_profile(&self, agent: EntityId) -> Option<TheftDispositionProfile> {
        self.theft_profiles.get(&agent).cloned()
    }
    fn intention_disposition_profile(
        &self,
        _agent: EntityId,
    ) -> Option<worldwake_core::IntentionDispositionProfile> {
        None
    }
}

impl worldwake_sim::PoliticalBeliefView for TestBeliefView {
    fn record_data(&self, record: EntityId) -> Option<worldwake_core::RecordData> {
        self.record_data.get(&record).cloned()
    }
    fn office_data(&self, office: EntityId) -> Option<worldwake_core::OfficeData> {
        self.office_data.get(&office).cloned()
    }
    fn believed_office_holder(
        &self,
        office: EntityId,
    ) -> worldwake_core::InstitutionalBeliefRead<Option<EntityId>> {
        self.office_holder_beliefs
            .get(&office)
            .cloned()
            .unwrap_or(worldwake_core::InstitutionalBeliefRead::Unknown)
    }
    fn violation_disposition_profile(
        &self,
        agent: EntityId,
    ) -> Option<ViolationDispositionProfile> {
        self.violation_profiles.get(&agent).cloned()
    }
    fn active_violation_records(&self, agent: EntityId) -> Vec<RecordedViolation> {
        self.active_violation_records
            .get(&agent)
            .cloned()
            .unwrap_or_default()
    }
}

impl CombatBeliefView for TestBeliefView {
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
    fn consultation_speed_factor(&self, agent: EntityId) -> Option<Permille> {
        self.consultation_speed_factors.get(&agent).copied()
    }
    fn wounds(&self, agent: EntityId) -> Vec<Wound> {
        self.wounds.get(&agent).cloned().unwrap_or_default()
    }
    fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId> {
        self.hostiles.get(&agent).cloned().unwrap_or_default()
    }
    fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId> {
        self.attackers.get(&agent).cloned().unwrap_or_default()
    }
    fn has_wounds(&self, entity: EntityId) -> bool {
        self.wounds
            .get(&entity)
            .is_some_and(|wounds| !wounds.is_empty())
    }
    fn patrol_profile(&self, agent: EntityId) -> Option<PatrolProfile> {
        self.patrol_profiles.get(&agent).cloned()
    }
}

impl EconomicBeliefView for TestBeliefView {
    fn trade_disposition_profile(&self, agent: EntityId) -> Option<TradeDispositionProfile> {
        self.trade_profiles.get(&agent).cloned()
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
        entities
            .extend(<Self as worldwake_sim::InventoryBeliefView>::direct_possessions(self, actor));
        entities.sort();
        entities.dedup();
        entities
            .into_iter()
            .filter(|entity| {
                <Self as worldwake_sim::InventoryBeliefView>::item_lot_commodity(self, *entity)
                    == Some(commodity)
            })
            .filter(|entity| self.can_control(actor, *entity))
            .collect()
    }
    fn listed_sale_lots_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
        self.listed_lots
            .get(&(place, commodity))
            .cloned()
            .unwrap_or_default()
    }
    fn seller_for_sale_lot(&self, lot: EntityId) -> Option<EntityId> {
        self.lot_sellers.get(&lot).copied()
    }
    fn has_sale_listing(&self, lot: EntityId) -> bool {
        self.lot_sellers.contains_key(&lot)
    }
    fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation> {
        self.demand_memory.get(&agent).cloned().unwrap_or_default()
    }
    fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile> {
        self.merchandise_profiles.get(&agent).cloned()
    }
}

impl worldwake_sim::InventoryBeliefView for TestBeliefView {
    fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId> {
        self.direct_possessions
            .get(&holder)
            .cloned()
            .unwrap_or_default()
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
    fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> {
        self.lot_commodities.get(&entity).copied()
    }
    fn item_lot_consumable_profile(&self, entity: EntityId) -> Option<CommodityConsumableProfile> {
        self.consumable_profiles.get(&entity).copied()
    }
    fn direct_container(&self, entity: EntityId) -> Option<EntityId> {
        self.direct_containers.get(&entity).copied()
    }
    fn direct_possessor(&self, entity: EntityId) -> Option<EntityId> {
        self.direct_possessors.get(&entity).copied()
    }
    fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits> {
        self.carry_capacities.get(&entity).copied()
    }
    fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits> {
        self.entity_loads.get(&entity).copied()
    }
    fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
        Vec::new()
    }
}

impl worldwake_sim::FacilityBeliefView for TestBeliefView {
    fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
        None
    }
    fn stock_storage_policy(
        &self,
        facility: EntityId,
    ) -> Option<worldwake_core::StockStoragePolicy> {
        self.stock_storage_policies.get(&facility).cloned()
    }
    fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
        None
    }
    fn has_production_job(&self, _entity: EntityId) -> bool {
        false
    }
    fn matching_workstations_at(&self, _place: EntityId, _tag: WorkstationTag) -> Vec<EntityId> {
        Vec::new()
    }
    fn resource_sources_at(&self, _place: EntityId, _commodity: CommodityKind) -> Vec<EntityId> {
        Vec::new()
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

fn active_believed_bounty_artifact(
    issuer: EntityId,
    claim_place: EntityId,
    target: BountyTarget,
    reward_quantity: Quantity,
    observed_tick: Tick,
) -> BelievedArtifactState {
    BelievedArtifactState {
        kind: ArtifactKind::Bounty,
        issuer,
        expires_at: None,
        existence: ArtifactExistence::Exists,
        visibility: ArtifactVisibility::Posted { place: claim_place },
        legal_effect: ArtifactLegalEffect::Active { expires_at: None },
        credibility: ArtifactCredibility::Credible,
        actionability: ArtifactActionability::Actionable,
        bounty_terms: Some(BelievedBountyTerms {
            target,
            reward_commodity: CommodityKind::Coin,
            reward_quantity,
            claim_place,
        }),
        notice_topic: None,
        observed_tick,
    }
}

fn wound(severity: u16) -> Wound {
    Wound {
        id: WoundId(u64::from(severity)),
        body_part: BodyPart::Torso,
        cause: WoundCause::Deprivation(DeprivationKind::Starvation),
        severity: pm(severity),
        inflicted_at: Tick(1),
        bleed_rate_per_tick: pm(0),
    }
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

fn patch_believed_entity_state<F>(
    world: &mut World,
    observer: EntityId,
    entity: EntityId,
    observed_tick: Tick,
    patch: F,
) where
    F: FnOnce(&mut BelievedEntityState),
{
    let mut store = world
        .get_component_agent_belief_store(observer)
        .cloned()
        .expect("observer must have AgentBeliefStore");
    let state = store
        .known_entities
        .get_mut(&entity)
        .expect("entity belief should exist before patching");
    patch(state);
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

fn build_two_place_travel_view() -> (TestBeliefView, EntityId, EntityId, EntityId) {
    let actor = entity(1);
    let origin = entity(10);
    let remote = entity(11);

    let mut view = TestBeliefView::default();
    view.alive.extend([actor, origin, remote]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(origin, EntityKind::Place);
    view.kinds.insert(remote, EntityKind::Place);
    view.effective_places.insert(actor, origin);
    view.entities_at.insert(origin, vec![actor]);
    view.entities_at.insert(remote, Vec::new());
    view.thresholds.insert(actor, DriveThresholds::default());
    view.adjacent
        .insert(origin, vec![(remote, NonZeroU32::new(2).unwrap())]);
    view.adjacent
        .insert(remote, vec![(origin, NonZeroU32::new(2).unwrap())]);

    (view, actor, origin, remote)
}

fn build_registry_with_recipes(
    recipes: &RecipeRegistry,
) -> (ActionDefRegistry, worldwake_sim::ActionHandlerRegistry) {
    let registries = build_full_action_registries(recipes).unwrap();
    (registries.defs, registries.handlers)
}

fn epistemic_profile() -> EpistemicDispositionProfile {
    EpistemicDispositionProfile {
        stale_evidence_barrier_threshold: Permille::new(400).unwrap(),
        witness_query_duration_ticks: NonZeroU32::new(3).unwrap(),
        ask_memory_retention_ticks: 10,
    }
}

fn believed_entity_state_at(
    place: EntityId,
    observed_tick: Tick,
    resource_source: Option<ResourceSource>,
) -> BelievedEntityState {
    BelievedEntityState {
        believed_kind: None,
        last_known_place: Some(place),
        last_known_inventory: BTreeMap::new(),
        workstation_tag: None,
        resource_source,
        alive: true,
        wounds: Vec::new(),
        last_known_courage: None,
        believed_activity: None,
        believed_artifact: None,
        believed_contention: None,
        believed_evidence: None,
        ..BelievedEntityState::single_observation_defaults(
            observed_tick,
            PerceptionSource::DirectObservation,
        )
    }
}

fn remember_entity(
    view: &mut TestBeliefView,
    actor: EntityId,
    entity: EntityId,
    belief: BelievedEntityState,
) {
    view.known_entity_beliefs
        .entry(actor)
        .or_default()
        .push((entity, belief));
}

fn combat_belief_at(place: EntityId, observed_tick: Tick) -> BelievedEntityState {
    let mut state = believed_entity_state_at(place, observed_tick, None);
    state.believed_activity = Some(worldwake_core::BelievedActivity {
        action_domain: worldwake_core::ActionDomain::Combat,
        target: None,
        observed_tick,
    });
    state
}

fn harvest_apple_recipe() -> RecipeDefinition {
    RecipeDefinition {
        name: "Harvest Apples".to_string(),
        inputs: vec![],
        outputs: vec![(CommodityKind::Apple, Quantity(2))],
        work_ticks: NonZeroU32::new(3).unwrap(),
        required_workstation_tag: Some(WorkstationTag::OrchardRow),
        required_tool_kinds: vec![],
        body_cost_per_tick: BodyCostPerTick::new(pm(3), pm(2), pm(5), pm(0), pm(1)),
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
        body_cost_per_tick: BodyCostPerTick::new(pm(3), pm(2), pm(5), pm(0), pm(1)),
    }
}

fn bake_bread_recipe() -> RecipeDefinition {
    RecipeDefinition {
        name: "Bake Bread".to_string(),
        inputs: vec![(CommodityKind::Firewood, Quantity(1))],
        outputs: vec![(CommodityKind::Bread, Quantity(1))],
        work_ticks: NonZeroU32::new(4).unwrap(),
        required_workstation_tag: Some(WorkstationTag::Mill),
        required_tool_kinds: vec![],
        body_cost_per_tick: BodyCostPerTick::new(pm(3), pm(2), pm(4), pm(0), pm(1)),
    }
}

fn named_place(name: &str, tags: &[PlaceTag]) -> Place {
    Place {
        name: name.to_string(),
        capacity: None,
        tags: tags.iter().copied().collect(),
    }
}

fn connect_bidirectional(
    topology: &mut Topology,
    base_id: u32,
    from: EntityId,
    to: EntityId,
    ticks: u32,
) {
    topology
        .add_edge(TravelEdge::new(TravelEdgeId(base_id), from, to, ticks, None).unwrap())
        .unwrap();
    topology
        .add_edge(TravelEdge::new(TravelEdgeId(base_id + 1), to, from, ticks, None).unwrap())
        .unwrap();
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
    let mut belief = BelievedEntityState {
        believed_kind: Some(EntityKind::ItemLot),
        last_known_place: Some(place),
        last_known_inventory: BTreeMap::from([(commodity, Quantity(1))]),
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
            PerceptionSource::DirectObservation,
        )
    };
    if view.effective_places.get(&actor) == Some(&place) {
        belief.last_known_place = Some(place);
    }
    view.known_entity_beliefs
        .entry(actor)
        .or_default()
        .push((lot, belief));
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

fn consume_goal(commodity: CommodityKind) -> GoalOffer {
    GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(worldwake_core::GoalKind::ConsumeOwnedCommodity { commodity }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::new(),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    }
}

fn acquire_goal_with_purpose(commodity: CommodityKind, purpose: CommodityPurpose) -> GoalOffer {
    GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity,
            purpose,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::new(),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    }
}

fn acquire_goal(commodity: CommodityKind) -> GoalOffer {
    acquire_goal_with_purpose(commodity, CommodityPurpose::SelfConsume)
}

fn sample_step(
    def_id: u32,
    op_kind: PlannerOpKind,
    estimated_ticks: u32,
    targets: Vec<EntityId>,
) -> PlannedStep {
    let target_place = targets.first().copied();
    PlannedStep {
        def_id: ActionDefId(def_id),
        targets: targets
            .into_iter()
            .map(PlanningEntityRef::Authoritative)
            .collect(),
        target_place,
        payload_override: None,
        op_kind,
        estimated_ticks,
        is_materialization_barrier: false,
        expected_materializations: Vec::new(),
        guard: None,
        expectations: Vec::new(),
    }
}

fn frontier_test_node(
    snapshot: &PlanningSnapshot,
    total_estimated_ticks: u32,
    steps: Vec<PlannedStep>,
) -> SearchNode<'_> {
    SearchNode {
        state: PlanningState::new(snapshot),
        steps: shared_steps(steps),
        total_estimated_ticks,
        search_cost: total_estimated_ticks,
        tactical_barrier_reached: false,
        heuristic_ticks: 0,
    }
}

fn shared_steps(steps: Vec<PlannedStep>) -> SharedVec<PlannedStep> {
    let mut shared = SharedVec::new();
    for step in steps {
        shared.push(step);
    }
    shared
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
            steps: SharedVec::new(),
            total_estimated_ticks: 0,
            search_cost: 0,
            tactical_barrier_reached: false,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
        .map(|node| node.steps.into_vec())
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
    let mut bread_belief = believed_entity_state_at(field, Tick(1), None);
    bread_belief.believed_kind = Some(EntityKind::ItemLot);
    bread_belief
        .last_known_inventory
        .insert(CommodityKind::Bread, Quantity(1));
    remember_entity(&mut view, actor, bread, bread_belief);
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
    // Protects the search_plan -> effect-schema hypothetical seam for consume targets.
    let plan = search_plan(
        &snapshot,
        &consume_goal(CommodityKind::Bread),
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
    let seller_lot = entity(100);
    let mut view = TestBeliefView::default();
    view.alive.extend([actor, seller, town, market, seller_lot]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(seller, EntityKind::Agent);
    view.kinds.insert(town, EntityKind::Place);
    view.kinds.insert(market, EntityKind::Place);
    view.kinds.insert(seller_lot, EntityKind::ItemLot);
    view.effective_places.insert(actor, town);
    view.effective_places.insert(seller, market);
    view.effective_places.insert(seller_lot, market);
    view.entities_at.insert(town, vec![actor]);
    view.entities_at.insert(market, vec![seller, seller_lot]);
    view.adjacent
        .insert(town, vec![(market, NonZeroU32::new(4).unwrap())]);
    view.adjacent
        .insert(market, vec![(town, NonZeroU32::new(4).unwrap())]);
    view.lot_commodities
        .insert(seller_lot, CommodityKind::Bread);
    view.lot_sellers.insert(seller_lot, seller);
    view.direct_possessors.insert(seller_lot, seller);
    view.direct_possessions
        .entry(seller)
        .or_default()
        .push(seller_lot);
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
            home_facility: Some(market),
        },
    );
    view.commodity_quantities
        .insert((actor, CommodityKind::Coin), Quantity(3));
    view.commodity_quantities
        .insert((seller, CommodityKind::Bread), Quantity(2));
    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::from([seller]),
        evidence_places: BTreeSet::from([market]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
    let trade_guard = plan.steps[1]
        .guard
        .as_ref()
        .expect("trade step should carry the authored guard template");
    assert_eq!(
        trade_guard.required_facts,
        vec![RequiredFact::TargetPresent {
            target: seller,
            at_place: market,
        }]
    );
    assert_eq!(trade_guard.invalidators.len(), 2);
    assert!(plan.steps[1].expectations.is_empty());
}

#[test]
fn search_returns_travel_then_trade_barrier_for_remote_listed_sale_lot_without_custody_detail() {
    let actor = entity(1);
    let town = entity(10);
    let market = entity(11);
    let seller = entity(2);
    let sale_lot = entity(100);
    let mut view = TestBeliefView::default();
    view.alive.extend([actor, seller, town, market, sale_lot]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(seller, EntityKind::Agent);
    view.kinds.insert(town, EntityKind::Place);
    view.kinds.insert(market, EntityKind::Place);
    view.kinds.insert(sale_lot, EntityKind::ItemLot);
    view.effective_places.insert(actor, town);
    view.effective_places.insert(seller, market);
    view.effective_places.insert(sale_lot, market);
    view.entities_at.insert(town, vec![actor]);
    view.entities_at.insert(market, vec![seller, sale_lot]);
    view.adjacent
        .insert(town, vec![(market, NonZeroU32::new(4).unwrap())]);
    view.adjacent
        .insert(market, vec![(town, NonZeroU32::new(4).unwrap())]);
    view.lot_commodities.insert(sale_lot, CommodityKind::Bread);
    view.listed_lots
        .insert((market, CommodityKind::Bread), vec![sale_lot]);
    view.lot_sellers.insert(sale_lot, seller);
    remember_entity(
        &mut view,
        actor,
        seller,
        BelievedEntityState {
            believed_kind: Some(EntityKind::Agent),
            ..believed_entity_state_at(market, Tick(0), None)
        },
    );
    remember_entity(
        &mut view,
        actor,
        sale_lot,
        BelievedEntityState {
            believed_kind: Some(EntityKind::ItemLot),
            ..believed_entity_state_at(market, Tick(0), None)
        },
    );
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
            home_facility: Some(market),
        },
    );
    view.commodity_quantities
        .insert((actor, CommodityKind::Coin), Quantity(3));
    view.commodity_quantities
        .insert((seller, CommodityKind::Bread), Quantity(2));

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(market),
        key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::from([seller]),
        evidence_places: BTreeSet::from([market]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
fn search_returns_travel_then_trade_barrier_for_remote_displayed_sale_lot_with_container_detail() {
    let actor = entity(1);
    let town = entity(10);
    let market = entity(11);
    let seller = entity(2);
    let sale_lot = entity(100);
    let display_container = entity(101);
    let mut view = TestBeliefView::default();
    view.alive
        .extend([actor, seller, town, market, sale_lot, display_container]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(seller, EntityKind::Agent);
    view.kinds.insert(town, EntityKind::Place);
    view.kinds.insert(market, EntityKind::Place);
    view.kinds.insert(sale_lot, EntityKind::ItemLot);
    view.kinds.insert(display_container, EntityKind::Container);
    view.effective_places.insert(actor, town);
    view.effective_places.insert(seller, market);
    view.effective_places.insert(sale_lot, market);
    view.effective_places.insert(display_container, market);
    view.entities_at.insert(town, vec![actor]);
    view.entities_at
        .insert(market, vec![seller, sale_lot, display_container]);
    view.adjacent
        .insert(town, vec![(market, NonZeroU32::new(4).unwrap())]);
    view.adjacent
        .insert(market, vec![(town, NonZeroU32::new(4).unwrap())]);
    view.lot_commodities.insert(sale_lot, CommodityKind::Bread);
    view.listed_lots
        .insert((market, CommodityKind::Bread), vec![sale_lot]);
    view.lot_sellers.insert(sale_lot, seller);
    view.direct_containers.insert(sale_lot, display_container);
    remember_entity(
        &mut view,
        actor,
        seller,
        BelievedEntityState {
            believed_kind: Some(EntityKind::Agent),
            ..believed_entity_state_at(market, Tick(0), None)
        },
    );
    remember_entity(
        &mut view,
        actor,
        sale_lot,
        BelievedEntityState {
            believed_kind: Some(EntityKind::ItemLot),
            ..believed_entity_state_at(market, Tick(0), None)
        },
    );
    remember_entity(
        &mut view,
        actor,
        display_container,
        BelievedEntityState {
            believed_kind: Some(EntityKind::Container),
            ..believed_entity_state_at(market, Tick(0), None)
        },
    );
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
            home_facility: Some(market),
        },
    );
    view.commodity_quantities
        .insert((actor, CommodityKind::Coin), Quantity(3));
    view.commodity_quantities
        .insert((seller, CommodityKind::Bread), Quantity(2));

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(market),
        key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::from([seller]),
        evidence_places: BTreeSet::from([market]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
    let seller_lot = entity(100);
    let mut view = TestBeliefView::default();
    view.alive.extend([actor, seller, town, seller_lot]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(seller, EntityKind::Agent);
    view.kinds.insert(town, EntityKind::Place);
    view.kinds.insert(seller_lot, EntityKind::ItemLot);
    view.effective_places.insert(actor, town);
    view.effective_places.insert(seller, town);
    view.effective_places.insert(seller_lot, town);
    view.entities_at
        .insert(town, vec![actor, seller, seller_lot]);
    view.lot_commodities
        .insert(seller_lot, CommodityKind::Bread);
    view.lot_sellers.insert(seller_lot, seller);
    view.direct_possessors.insert(seller_lot, seller);
    view.direct_possessions
        .entry(seller)
        .or_default()
        .push(seller_lot);
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
            home_facility: Some(town),
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::from([seller]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
            home_facility: Some(town),
        },
    );
    view.commodity_quantities
        .insert((actor, CommodityKind::Coin), Quantity(3));
    view.commodity_quantities
        .insert((seller, CommodityKind::Firewood), Quantity(1));
    let sale_lot = entity(50);
    view.alive.insert(sale_lot);
    view.kinds.insert(sale_lot, EntityKind::ItemLot);
    view.effective_places.insert(sale_lot, town);
    view.entities_at.get_mut(&town).unwrap().push(sale_lot);
    view.lot_commodities
        .insert(sale_lot, CommodityKind::Firewood);
    view.direct_possessors.insert(sale_lot, seller);
    view.direct_possessions
        .entry(seller)
        .or_default()
        .push(sale_lot);
    view.listed_lots
        .insert((town, CommodityKind::Firewood), vec![sale_lot]);
    view.lot_sellers.insert(sale_lot, seller);

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Firewood,
            purpose: CommodityPurpose::RecipeInput(RecipeId(0)),
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::from([seller]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
    let budget = ProfileFixture {
        max_plan_depth: 1,
        ..ProfileFixture::default()
    };
    let plan = search_plan(
        &snapshot,
        &consume_goal(CommodityKind::Bread),
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &budget,
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
    let budget = ProfileFixture {
        max_node_expansions: 0,
        ..ProfileFixture::default()
    };
    let plan = search_plan(
        &snapshot,
        &consume_goal(CommodityKind::Bread),
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &budget,
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    );

    assert!(!plan.is_found());
}

#[test]
fn search_candidate_safety_valve_triggers_at_threshold() {
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
    let mut cognitive = cognitive(&ProfileFixture::default());
    cognitive.max_candidates_per_expansion = 0;

    let result = super::search_plan(
        &snapshot,
        &consume_goal(CommodityKind::Bread),
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &cognitive,
        &execution_budget(&ProfileFixture::default()),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    );

    match result {
        PlanSearchResult::BudgetExhausted { expansions_used } => {
            assert_eq!(expansions_used, 1);
        }
        other => panic!("expected BudgetExhausted from candidate safety valve, got {other:?}"),
    }
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
        &ProfileFixture {
            beam_width: 1,
            ..ProfileFixture::default()
        },
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    );
    let wide_beam_plan = search_plan(
        &snapshot,
        &consume_goal(CommodityKind::Bread),
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture {
            beam_width: 2,
            ..ProfileFixture::default()
        },
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
        &ProfileFixture {
            beam_width: 2,
            ..ProfileFixture::default()
        },
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    );
    let beam_three_plan = search_plan(
        &snapshot,
        &consume_goal(CommodityKind::Bread),
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture {
            beam_width: 3,
            ..ProfileFixture::default()
        },
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
        &ProfileFixture {
            beam_width: 3,
            max_node_expansions: 2,
            ..ProfileFixture::default()
        },
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    );
    let sufficient_budget_plan = search_plan(
        &snapshot,
        &consume_goal(CommodityKind::Bread),
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture {
            beam_width: 3,
            max_node_expansions: 6,
            ..ProfileFixture::default()
        },
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
        &ProfileFixture {
            max_plan_depth: 0,
            ..ProfileFixture::default()
        },
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
            home_facility: Some(market),
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::from([seller]),
        evidence_places: BTreeSet::from([market]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: acquire_goal(CommodityKind::Bread).key,
        evidence_entities: BTreeSet::from([bread]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
fn search_blocks_remote_stale_move_cargo_by_target_place() {
    let actor = entity(1);
    let home = entity(10);
    let orchard = entity(11);
    let bread = entity(20);
    let mut view = TestBeliefView::default();
    view.alive.extend([actor, home, orchard, bread]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(home, EntityKind::Place);
    view.kinds.insert(orchard, EntityKind::Place);
    view.kinds.insert(bread, EntityKind::ItemLot);
    view.effective_places.insert(actor, home);
    view.effective_places.insert(bread, orchard);
    view.entities_at.insert(home, vec![actor]);
    view.entities_at.insert(orchard, vec![bread]);
    view.lot_commodities.insert(bread, CommodityKind::Bread);
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
    view.adjacent
        .insert(home, vec![(orchard, NonZeroU32::new(1).unwrap())]);
    view.adjacent
        .insert(orchard, vec![(home, NonZeroU32::new(1).unwrap())]);

    let (registry, _handlers) = build_registry();
    let semantics_table = build_semantics_table(&registry);
    let pick_up_id = registry
        .iter()
        .find(|def| def.name == "pick_up")
        .map(|def| def.id)
        .expect("pick_up action should be registered");
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(orchard),
        key: acquire_goal(CommodityKind::Bread).key,
        evidence_entities: BTreeSet::from([bread]),
        evidence_places: BTreeSet::from([orchard]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let node = SearchNode {
        state: PlanningState::new(&snapshot),
        steps: SharedVec::new(),
        total_estimated_ticks: 0,
        search_cost: 0,
        tactical_barrier_reached: false,
        heuristic_ticks: 0,
    };
    let candidate = SearchCandidate {
        def_id: pick_up_id,
        authoritative_targets: vec![bread],
        planning_targets: vec![PlanningEntityRef::Authoritative(bread)],
        payload_override: Some(ActionPayload::Transport(TransportActionPayload {
            quantity: Quantity(1),
        })),
        planner_only: false,
        trace_index: None,
        expansion_trace_index: None,
    };
    let mut blocked = BlockerMemory::default();
    blocked.record(Blocker {
        blocker_key: BlockerKey {
            goal_key: goal.key,
            place: Some(orchard),
            target: Some(bread),
            action_def: Some(pick_up_id),
        },
        blocking_fact: BlockingFact::TargetGone,
        diagnostic_context: None,
        observed_tick: Tick(1),
        expires_tick: Tick(20),
        clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
        baseline_snapshot: None,
    });

    assert_eq!(
        candidate_blocked_by_place(
            &candidate,
            &goal,
            &node,
            &semantics_table,
            &blocked,
            Tick(5)
        ),
        Some((Some(orchard), BlockingFact::TargetGone)),
    );
}

#[test]
fn place_anchored_acquire_search_does_not_retarget_sibling_place_lot() {
    let actor = entity(1);
    let home = entity(10);
    let orchard = entity(11);
    let home_apple = entity(20);
    let orchard_apple = entity(21);
    let mut view = TestBeliefView::default();
    view.alive
        .extend([actor, home, orchard, home_apple, orchard_apple]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(home, EntityKind::Place);
    view.kinds.insert(orchard, EntityKind::Place);
    view.kinds.insert(home_apple, EntityKind::ItemLot);
    view.kinds.insert(orchard_apple, EntityKind::ItemLot);
    view.effective_places.insert(actor, orchard);
    view.effective_places.insert(home_apple, home);
    view.effective_places.insert(orchard_apple, orchard);
    view.entities_at.insert(home, vec![home_apple]);
    view.entities_at.insert(orchard, vec![actor, orchard_apple]);
    view.lot_commodities
        .insert(home_apple, CommodityKind::Apple);
    view.lot_commodities
        .insert(orchard_apple, CommodityKind::Apple);
    view.consumable_profiles.insert(
        home_apple,
        CommodityKind::Apple.spec().consumable_profile.unwrap(),
    );
    view.consumable_profiles.insert(
        orchard_apple,
        CommodityKind::Apple.spec().consumable_profile.unwrap(),
    );
    view.commodity_quantities
        .insert((home_apple, CommodityKind::Apple), Quantity(1));
    view.commodity_quantities
        .insert((orchard_apple, CommodityKind::Apple), Quantity(1));
    view.carry_capacities.insert(actor, LoadUnits(4));
    view.entity_loads.insert(actor, LoadUnits(0));
    view.entity_loads.insert(home_apple, LoadUnits(1));
    view.entity_loads.insert(orchard_apple, LoadUnits(1));
    view.needs.insert(
        actor,
        HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
    );
    view.thresholds.insert(actor, DriveThresholds::default());
    view.adjacent
        .insert(home, vec![(orchard, NonZeroU32::new(3).unwrap())]);
    view.adjacent
        .insert(orchard, vec![(home, NonZeroU32::new(3).unwrap())]);

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(orchard),
        key: GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::from([home_apple, orchard_apple]),
        evidence_places: BTreeSet::from([home, orchard]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        3,
    );

    let plan = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    )
    .into_plan()
    .expect("anchored local apple should remain satisfiable without sibling travel");

    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::MoveCargo);
    assert_eq!(
        plan.steps[0].targets,
        vec![PlanningEntityRef::Authoritative(orchard_apple)]
    );
}

#[test]
fn place_anchored_acquire_search_does_not_escape_blocked_local_lot_to_sibling_place() {
    let actor = entity(1);
    let home = entity(10);
    let orchard = entity(11);
    let home_apple = entity(20);
    let orchard_apple = entity(21);
    let mut view = TestBeliefView::default();
    view.alive
        .extend([actor, home, orchard, home_apple, orchard_apple]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(home, EntityKind::Place);
    view.kinds.insert(orchard, EntityKind::Place);
    view.kinds.insert(home_apple, EntityKind::ItemLot);
    view.kinds.insert(orchard_apple, EntityKind::ItemLot);
    view.effective_places.insert(actor, orchard);
    view.effective_places.insert(home_apple, home);
    view.effective_places.insert(orchard_apple, orchard);
    view.entities_at.insert(home, vec![home_apple]);
    view.entities_at.insert(orchard, vec![actor, orchard_apple]);
    view.lot_commodities
        .insert(home_apple, CommodityKind::Apple);
    view.lot_commodities
        .insert(orchard_apple, CommodityKind::Apple);
    view.consumable_profiles.insert(
        home_apple,
        CommodityKind::Apple.spec().consumable_profile.unwrap(),
    );
    view.consumable_profiles.insert(
        orchard_apple,
        CommodityKind::Apple.spec().consumable_profile.unwrap(),
    );
    view.commodity_quantities
        .insert((home_apple, CommodityKind::Apple), Quantity(1));
    view.commodity_quantities
        .insert((orchard_apple, CommodityKind::Apple), Quantity(1));
    view.carry_capacities.insert(actor, LoadUnits(4));
    view.entity_loads.insert(actor, LoadUnits(0));
    view.entity_loads.insert(home_apple, LoadUnits(1));
    view.entity_loads.insert(orchard_apple, LoadUnits(1));
    view.needs.insert(
        actor,
        HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
    );
    view.thresholds.insert(actor, DriveThresholds::default());
    view.adjacent
        .insert(home, vec![(orchard, NonZeroU32::new(3).unwrap())]);
    view.adjacent
        .insert(orchard, vec![(home, NonZeroU32::new(3).unwrap())]);

    let (registry, handlers) = build_registry();
    let pick_up_id = registry
        .iter()
        .find(|def| def.name == "pick_up")
        .map(|def| def.id)
        .expect("pick_up action should be registered");
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(orchard),
        key: GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::from([home_apple, orchard_apple]),
        evidence_places: BTreeSet::from([home, orchard]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        3,
    );
    let mut blocked = BlockerMemory::default();
    blocked.record(Blocker {
        blocker_key: BlockerKey {
            goal_key: goal.key,
            place: Some(orchard),
            target: Some(orchard_apple),
            action_def: Some(pick_up_id),
        },
        blocking_fact: BlockingFact::TargetGone,
        diagnostic_context: None,
        observed_tick: Tick(1),
        expires_tick: Tick(20),
        clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
        baseline_snapshot: None,
    });

    let result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &blocked,
        Tick(5),
        None,
        None,
    );

    assert!(
        !result.is_found(),
        "anchored local commodity goals must not reroute to sibling places when the local lot is blocked"
    );
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: acquire_goal_with_purpose(CommodityKind::Medicine, CommodityPurpose::SelfConsume).key,
        evidence_entities: BTreeSet::from([medicine]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: acquire_goal(CommodityKind::Apple).key,
        evidence_entities: BTreeSet::from([apples]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
    let facility = entity(12);
    view.alive
        .extend([actor, origin, destination, facility, bread]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(origin, EntityKind::Place);
    view.kinds.insert(destination, EntityKind::Place);
    view.kinds.insert(facility, EntityKind::Facility);
    view.kinds.insert(bread, EntityKind::ItemLot);
    view.effective_places.insert(actor, origin);
    view.effective_places.insert(facility, destination);
    view.effective_places.insert(bread, origin);
    view.entities_at.insert(origin, vec![actor, bread]);
    view.entities_at.insert(destination, vec![facility]);
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
            home_facility: Some(facility),
        },
    );
    let mut facility_belief = believed_entity_state_at(destination, Tick(1), None);
    facility_belief.believed_kind = Some(EntityKind::Facility);
    remember_entity(&mut view, actor, facility, facility_belief);
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination: facility,
        }),
        evidence_entities: BTreeSet::from([bread]),
        evidence_places: BTreeSet::from([origin, destination]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
    let facility = entity(12);
    view.alive
        .extend([actor, origin, destination, facility, bread]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(origin, EntityKind::Place);
    view.kinds.insert(destination, EntityKind::Place);
    view.kinds.insert(facility, EntityKind::Facility);
    view.kinds.insert(bread, EntityKind::ItemLot);
    view.effective_places.insert(actor, origin);
    view.effective_places.insert(facility, destination);
    view.effective_places.insert(bread, origin);
    view.entities_at.insert(origin, vec![actor, bread]);
    view.entities_at.insert(destination, vec![facility]);
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
            home_facility: Some(facility),
        },
    );
    let mut facility_belief = believed_entity_state_at(destination, Tick(1), None);
    facility_belief.believed_kind = Some(EntityKind::Facility);
    remember_entity(&mut view, actor, facility, facility_belief);
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination: facility,
        }),
        evidence_entities: BTreeSet::from([bread]),
        evidence_places: BTreeSet::from([origin, destination]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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

#[test]
fn cargo_search_for_facility_destination_requires_store_stock_after_travel() {
    let actor = entity(1);
    let origin = entity(10);
    let destination = entity(11);
    let facility = entity(12);
    let stock_container = entity(13);
    let bread = entity(20);
    let mut view = TestBeliefView::default();
    view.alive
        .extend([actor, origin, destination, facility, stock_container, bread]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(origin, EntityKind::Place);
    view.kinds.insert(destination, EntityKind::Place);
    view.kinds.insert(facility, EntityKind::Facility);
    view.kinds.insert(stock_container, EntityKind::Container);
    view.kinds.insert(bread, EntityKind::ItemLot);
    view.effective_places.insert(actor, origin);
    view.effective_places.insert(facility, destination);
    view.effective_places.insert(stock_container, destination);
    view.effective_places.insert(bread, origin);
    view.entities_at.insert(origin, vec![actor, bread]);
    view.entities_at
        .insert(destination, vec![facility, stock_container]);
    view.adjacent
        .insert(origin, vec![(destination, NonZeroU32::new(2).unwrap())]);
    view.adjacent
        .insert(destination, vec![(origin, NonZeroU32::new(2).unwrap())]);
    view.lot_commodities.insert(bread, CommodityKind::Bread);
    view.commodity_quantities
        .insert((bread, CommodityKind::Bread), Quantity(2));
    view.controllable
        .extend([(actor, bread), (actor, facility), (actor, stock_container)]);
    view.carry_capacities.insert(actor, LoadUnits(4));
    view.entity_loads.insert(actor, LoadUnits(0));
    view.entity_loads.insert(bread, LoadUnits(2));
    view.thresholds.insert(actor, DriveThresholds::default());
    view.merchandise_profiles.insert(
        actor,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([CommodityKind::Bread]),
            home_facility: Some(facility),
        },
    );
    let mut facility_belief = believed_entity_state_at(destination, Tick(1), None);
    facility_belief.believed_kind = Some(EntityKind::Facility);
    remember_entity(&mut view, actor, facility, facility_belief);
    let mut container_belief = believed_entity_state_at(destination, Tick(1), None);
    container_belief.believed_kind = Some(EntityKind::Container);
    remember_entity(&mut view, actor, stock_container, container_belief);
    view.stock_storage_policies.insert(
        facility,
        worldwake_core::StockStoragePolicy {
            stock_container,
            display_container: None,
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination: facility,
        }),
        evidence_entities: BTreeSet::from([bread]),
        evidence_places: BTreeSet::from([origin, destination]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    )
    .into_plan()
    .unwrap();

    assert_eq!(plan.terminal_kind, PlanTerminalKind::GoalSatisfied);
    assert_eq!(plan.steps.len(), 3);
    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::MoveCargo);
    assert_eq!(plan.steps[1].op_kind, PlannerOpKind::Travel);
    assert_eq!(plan.steps[2].op_kind, PlannerOpKind::StockManagement);
    assert_eq!(
        registry
            .get(plan.steps[2].def_id)
            .map(|def| def.name.as_str()),
        Some("store_stock")
    );
}

#[test]
fn sell_search_for_stored_home_stock_requires_stage_before_goal_satisfaction() {
    let actor = entity(1);
    let market = entity(10);
    let facility = entity(11);
    let stock_container = entity(12);
    let display_container = entity(13);
    let bread = entity(20);
    let mut view = TestBeliefView::default();
    view.alive.extend([
        actor,
        market,
        facility,
        stock_container,
        display_container,
        bread,
    ]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(market, EntityKind::Place);
    view.kinds.insert(facility, EntityKind::Facility);
    view.kinds.insert(stock_container, EntityKind::Container);
    view.kinds.insert(display_container, EntityKind::Container);
    view.kinds.insert(bread, EntityKind::ItemLot);
    view.effective_places.insert(actor, market);
    view.effective_places.insert(facility, market);
    view.effective_places.insert(stock_container, market);
    view.effective_places.insert(display_container, market);
    view.effective_places.insert(bread, market);
    view.entities_at.insert(
        market,
        vec![actor, facility, stock_container, display_container, bread],
    );
    view.direct_containers.insert(bread, stock_container);
    view.lot_commodities.insert(bread, CommodityKind::Bread);
    view.commodity_quantities
        .insert((bread, CommodityKind::Bread), Quantity(3));
    view.controllable.extend([
        (actor, facility),
        (actor, stock_container),
        (actor, display_container),
        (actor, bread),
    ]);
    view.merchandise_profiles.insert(
        actor,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([CommodityKind::Bread]),
            home_facility: Some(facility),
        },
    );
    view.stock_storage_policies.insert(
        facility,
        worldwake_core::StockStoragePolicy {
            stock_container,
            display_container: Some(display_container),
        },
    );

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(market),
        key: GoalKey::from(GoalKind::SellCommodity {
            commodity: CommodityKind::Bread,
        }),
        evidence_entities: BTreeSet::from([bread, facility]),
        evidence_places: BTreeSet::from([market]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    )
    .into_plan()
    .unwrap();

    assert_eq!(plan.terminal_kind, PlanTerminalKind::GoalSatisfied);
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::StockManagement);
    assert_eq!(
        registry
            .get(plan.steps[0].def_id)
            .map(|def| def.name.as_str()),
        Some("stage_stock_for_sale")
    );
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
                home_facility: Some(destination),
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination,
        }),
        evidence_entities: BTreeSet::from([bread]),
        evidence_places: BTreeSet::from([origin, destination]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        steps: SharedVec::new(),
        total_estimated_ticks: 0,
        search_cost: 0,
        tactical_barrier_reached: false,
        heuristic_ticks: 0,
    };

    let rel_defs = relevant_action_defs(&goal, &semantics);
    let initial_candidates = search_candidates(
        &goal,
        &node,
        candidate_search_context(
            &semantics,
            &registry,
            &handlers,
            &BlockerMemory::default(),
            Tick(0),
            &rel_defs,
        ),
        None,
        None,
        None,
    );
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
        &ProfileFixture::default(),
    )
    .unwrap();
    assert_eq!(terminal, None);
    assert_eq!(
        after_pick_up.steps.as_slice()[0].targets,
        vec![PlanningEntityRef::Authoritative(bread)]
    );
    assert!(
        !after_pick_up.steps.as_slice()[0]
            .expected_materializations
            .is_empty()
    );

    let follow_up_candidates = search_candidates(
        &goal,
        &after_pick_up,
        candidate_search_context(
            &semantics,
            &registry,
            &handlers,
            &BlockerMemory::default(),
            Tick(0),
            &rel_defs,
        ),
        None,
        None,
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
        &ProfileFixture::default(),
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(worldwake_core::GoalKind::ReduceDanger),
        evidence_entities: BTreeSet::from([attacker]),
        evidence_places: BTreeSet::from([town, refuge]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(worldwake_core::GoalKind::ReduceDanger),
        evidence_entities: BTreeSet::from([attacker]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(worldwake_core::GoalKind::ReduceDanger),
        evidence_entities: BTreeSet::from([attacker]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        steps: SharedVec::new(),
        total_estimated_ticks: 0,
        search_cost: 0,
        tactical_barrier_reached: false,
        heuristic_ticks: 0,
    };
    let candidate = SearchCandidate {
        def_id: defend.id,
        authoritative_targets: Vec::new(),
        planning_targets: Vec::new(),
        payload_override: None,
        planner_only: true,
        trace_index: None,
        expansion_trace_index: None,
    };

    let (_, successor) = build_successor(
        &goal,
        &semantics_table,
        &registry,
        &node,
        &candidate,
        &RecipeRegistry::new(),
        &ProfileFixture::default(),
    )
    .unwrap();

    assert_eq!(successor.steps.len(), 1);
    assert_eq!(successor.steps.as_slice()[0].op_kind, PlannerOpKind::Defend);
    assert_eq!(successor.steps.as_slice()[0].estimated_ticks, 10);
    assert_eq!(successor.total_estimated_ticks, 10);
}

#[test]
fn build_successor_preserves_parent_steps_when_appending_child_step() {
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

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(worldwake_core::GoalKind::ReduceDanger),
        evidence_entities: BTreeSet::from([attacker]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        0,
    );
    let (registry, _handlers) = build_registry();
    let semantics_table = build_semantics_table(&registry);
    let defend = registry.iter().find(|def| def.name == "defend").unwrap();
    let parent_step = sample_step(99, PlannerOpKind::Travel, 2, vec![town]);
    let node = SearchNode {
        state: PlanningState::new(&snapshot),
        steps: shared_steps(vec![parent_step.clone()]),
        total_estimated_ticks: 2,
        search_cost: 2,
        tactical_barrier_reached: false,
        heuristic_ticks: 0,
    };
    let candidate = SearchCandidate {
        def_id: defend.id,
        authoritative_targets: Vec::new(),
        planning_targets: Vec::new(),
        payload_override: None,
        planner_only: true,
        trace_index: None,
        expansion_trace_index: None,
    };

    let (terminal, successor) = build_successor(
        &goal,
        &semantics_table,
        &registry,
        &node,
        &candidate,
        &RecipeRegistry::new(),
        &ProfileFixture::default(),
    )
    .expect("defend successor should append a child step without mutating the parent");

    assert_eq!(terminal, Some(PlanTerminalKind::CombatCommitment));
    assert_eq!(node.steps.as_slice(), &[parent_step]);
    assert_eq!(successor.steps.len(), 2);
    assert_eq!(successor.steps.as_slice()[0].op_kind, PlannerOpKind::Travel);
    assert_eq!(successor.steps.as_slice()[1].op_kind, PlannerOpKind::Defend);
}

#[test]
fn build_successor_estimates_steal_ticks_from_theft_profile() {
    let actor = entity(1);
    let owner = entity(2);
    let target_item = entity(3);
    let town = entity(10);

    let mut view = TestBeliefView::default();
    view.alive.extend([actor, owner, town]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(owner, EntityKind::Agent);
    view.kinds.insert(target_item, EntityKind::ItemLot);
    view.kinds.insert(town, EntityKind::Place);
    view.effective_places.insert(actor, town);
    view.effective_places.insert(owner, town);
    view.effective_places.insert(target_item, town);
    view.entities_at
        .insert(town, vec![actor, owner, target_item]);
    view.owners.insert(target_item, owner);
    view.lot_commodities
        .insert(target_item, CommodityKind::Bread);
    view.commodity_quantities
        .insert((target_item, CommodityKind::Bread), Quantity(1));
    view.carry_capacities.insert(actor, LoadUnits(10));
    view.entity_loads.insert(actor, LoadUnits(0));
    view.entity_loads.insert(target_item, LoadUnits(1));
    view.theft_profiles.insert(
        actor,
        TheftDispositionProfile {
            steal_duration_ticks: NonZeroU32::new(2).unwrap(),
            theft_motive_weight: pm(500),
            witness_risk_penalty: pm(100),
        },
    );

    let (registry, _handlers) = build_registry();
    let semantics_table = build_semantics_table(&registry);
    let steal = registry.iter().find(|def| def.name == "steal").unwrap();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::StealItem { target_item }),
        evidence_entities: BTreeSet::from([target_item]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let node = SearchNode {
        state: PlanningState::new(&snapshot),
        steps: SharedVec::new(),
        total_estimated_ticks: 0,
        search_cost: 0,
        tactical_barrier_reached: false,
        heuristic_ticks: 0,
    };
    let candidate = SearchCandidate {
        def_id: steal.id,
        authoritative_targets: vec![target_item],
        planning_targets: vec![PlanningEntityRef::Authoritative(target_item)],
        payload_override: None,
        planner_only: false,
        trace_index: None,
        expansion_trace_index: None,
    };

    let (_, successor) = build_successor(
        &goal,
        &semantics_table,
        &registry,
        &node,
        &candidate,
        &RecipeRegistry::new(),
        &ProfileFixture::default(),
    )
    .expect("steal successor should estimate duration from the preserved theft profile");

    assert_eq!(successor.steps.len(), 1);
    assert_eq!(
        successor.steps.as_slice()[0].op_kind,
        PlannerOpKind::MoveCargo
    );
    assert_eq!(successor.steps.as_slice()[0].estimated_ticks, 2);
    assert_eq!(successor.total_estimated_ticks, 2);
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
        planner_only: false,
        trace_index: None,
        expansion_trace_index: None,
    };
    let (_, successor) = build_successor(
        &goal,
        &semantics_table,
        &registry,
        &node,
        &candidate,
        &RecipeRegistry::new(),
        &ProfileFixture::default(),
    )
    .unwrap();

    let step = &successor.steps.as_slice()[0];
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
        planner_only: false,
        trace_index: None,
        expansion_trace_index: None,
    };
    let (_, successor) = build_successor(
        &goal,
        &semantics_table,
        &registry,
        &node,
        &candidate,
        &RecipeRegistry::new(),
        &ProfileFixture::default(),
    )
    .unwrap();

    let candidates = planner_only_candidates(&successor.state, &semantics_table)
        .into_iter()
        .map(search_candidate_from_planner)
        .collect::<Vec<_>>();
    assert_eq!(candidates.len(), 2);
    for candidate in &candidates {
        assert!(candidate.authoritative_targets.is_empty());
        assert_eq!(candidate.payload_override, None);
        assert!(matches!(
            candidate.planning_targets.as_slice(),
            [PlanningEntityRef::Hypothetical(_)]
        ));
    }
    let put_down = registry.iter().find(|def| def.name == "put_down").unwrap();
    let drop_item = registry.iter().find(|def| def.name == "drop_item").unwrap();
    let candidate_ids = candidates
        .iter()
        .map(|candidate| candidate.def_id)
        .collect::<Vec<_>>();
    assert!(candidate_ids.contains(&put_down.id));
    assert!(candidate_ids.contains(&drop_item.id));
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
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        }),
        evidence_entities: BTreeSet::from([orchard_row]),
        evidence_places: BTreeSet::from([village_square, orchard_farm]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(actor, &world);
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );

    let plan = search_plan(
        &snapshot,
        &goal,
        &semantics,
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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

#[test]
fn search_wash_finds_direct_plan_at_current_clean_basin() {
    let orchard_farm = prototype_place_entity(PrototypePlace::OrchardFarm);
    let mut world = World::new(build_prototype_world()).unwrap();
    let (actor, wash_basin) = {
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        let actor = txn.create_agent("Washer", ControlSource::Ai).unwrap();
        let wash_basin = txn.create_entity(EntityKind::Facility);
        txn.set_ground_location(actor, orchard_farm).unwrap();
        txn.set_ground_location(wash_basin, orchard_farm).unwrap();
        txn.set_component_homeostatic_needs(
            actor,
            HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(700)),
        )
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
            wash_basin,
            WorkstationMarker(WorkstationTag::WashBasin),
        )
        .unwrap();
        txn.set_component_wash_basin_state(wash_basin, WashBasinState::default())
            .unwrap();
        let mut event_log = EventLog::new();
        let _ = txn.commit(&mut event_log);
        (actor, wash_basin)
    };

    let recipes = RecipeRegistry::new();
    sync_all_beliefs(&mut world, actor, Tick(1));

    let (registry, handlers) = build_registry_with_recipes(&recipes);
    let semantics = build_semantics_table(&registry);
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Entity(wash_basin),
        key: GoalKey::from(GoalKind::Wash),
        evidence_entities: BTreeSet::from([wash_basin]),
        evidence_places: BTreeSet::from([orchard_farm]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(actor, &world);
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );

    let mut expansions = Vec::new();
    let result = search_plan(
        &snapshot,
        &goal,
        &semantics,
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &recipes,
        &BlockerMemory::default(),
        Tick(0),
        None,
        Some(&mut expansions),
    );
    let plan = match result {
        PlanSearchResult::Found(plan) => *plan,
        other => {
            let root = expansions.iter().find(|summary| summary.depth == 0);
            let saw_wash = expansions.iter().any(|summary| {
                summary
                    .expansion_candidates
                    .iter()
                    .any(|candidate| candidate.op_kind == Some(PlannerOpKind::Wash))
            });
            panic!(
                "wash should find a lawful direct wash plan at the current clean basin; result={other:?}; root={root:?}; saw_wash={saw_wash}"
            );
        }
    };

    assert_eq!(
        plan.steps.last().map(|step| step.op_kind),
        Some(PlannerOpKind::Wash)
    );
    assert!(
        plan.steps
            .iter()
            .all(|step| step.op_kind != PlannerOpKind::Harvest),
        "wash should no longer require harvest under the facility-mediated contract: {plan:?}"
    );
    assert!(
        plan.steps
            .iter()
            .all(|step| step.op_kind != PlannerOpKind::MoveCargo),
        "wash should no longer require picking up water under the facility-mediated contract: {plan:?}"
    );
}

#[test]
fn search_local_wash_candidates_require_clean_basin() {
    let orchard_farm = prototype_place_entity(PrototypePlace::OrchardFarm);
    let mut world = World::new(build_prototype_world()).unwrap();
    let (actor, wash_basin) = {
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        let actor = txn.create_agent("Washer", ControlSource::Ai).unwrap();
        let wash_basin = txn.create_entity(EntityKind::Facility);
        txn.set_ground_location(actor, orchard_farm).unwrap();
        txn.set_ground_location(wash_basin, orchard_farm).unwrap();
        txn.set_component_homeostatic_needs(
            actor,
            HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(700)),
        )
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
            wash_basin,
            WorkstationMarker(WorkstationTag::WashBasin),
        )
        .unwrap();
        txn.set_component_wash_basin_state(wash_basin, WashBasinState::default())
            .unwrap();
        let mut event_log = EventLog::new();
        let _ = txn.commit(&mut event_log);
        (actor, wash_basin)
    };

    let recipes = RecipeRegistry::new();
    sync_all_beliefs(&mut world, actor, Tick(1));

    let (registry, handlers) = build_registry_with_recipes(&recipes);
    let semantics_table = build_semantics_table(&registry);
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Entity(wash_basin),
        key: GoalKey::from(GoalKind::Wash),
        evidence_entities: BTreeSet::from([wash_basin]),
        evidence_places: BTreeSet::from([orchard_farm]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(actor, &world);
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );
    let reasoning = ProfileFixture::default();
    let root = root_node(&snapshot, &goal, &recipes, &reasoning);
    let relevant_defs = relevant_action_defs(&goal, &semantics_table);
    let local_node = SearchNode {
        state: root.state.clone().move_actor_to(orchard_farm),
        steps: root.steps.clone(),
        total_estimated_ticks: root.total_estimated_ticks,
        search_cost: root.search_cost,
        tactical_barrier_reached: root.tactical_barrier_reached,
        heuristic_ticks: root.heuristic_ticks,
    };
    let local_candidates = search_candidates(
        &goal,
        &local_node,
        candidate_search_context(
            &semantics_table,
            &registry,
            &handlers,
            &BlockerMemory::default(),
            Tick(0),
            &relevant_defs,
        ),
        None,
        None,
        None,
    );
    assert!(
        local_candidates.iter().any(|candidate| {
            semantics_table
                .get(&candidate.def_id)
                .is_some_and(|semantics| semantics.op_kind == PlannerOpKind::Wash)
                && matches!(
                    candidate.planning_targets.as_slice(),
                    [PlanningEntityRef::Authoritative(target0)] if *target0 == wash_basin
                )
        }),
        "local wash candidates should directly target the clean basin"
    );
}

struct RestockThreatFixture {
    world: World,
    actor: EntityId,
    orchard_row: EntityId,
    market: EntityId,
    dangerous_road: EntityId,
    safe_route: EntityId,
    remote_farm: EntityId,
}

fn build_restock_threat_fixture(with_combat_belief: bool) -> RestockThreatFixture {
    let market = entity(600);
    let dangerous_road = entity(601);
    let bandit_camp = entity(602);
    let safe_route = entity(603);
    let remote_farm = entity(604);
    let mut topology = Topology::new();
    topology
        .add_place(
            market,
            named_place("Market", &[PlaceTag::Village, PlaceTag::Store]),
        )
        .unwrap();
    topology
        .add_place(
            dangerous_road,
            named_place("Dangerous Road", &[PlaceTag::Road, PlaceTag::Forest]),
        )
        .unwrap();
    topology
        .add_place(
            bandit_camp,
            named_place("Bandit Camp", &[PlaceTag::Camp, PlaceTag::Forest]),
        )
        .unwrap();
    topology
        .add_place(
            safe_route,
            named_place("Safe Route", &[PlaceTag::Road, PlaceTag::Field]),
        )
        .unwrap();
    topology
        .add_place(
            remote_farm,
            named_place("Remote Farm", &[PlaceTag::Farm, PlaceTag::Field]),
        )
        .unwrap();
    connect_bidirectional(&mut topology, 700, market, dangerous_road, 2);
    connect_bidirectional(&mut topology, 710, dangerous_road, bandit_camp, 1);
    connect_bidirectional(&mut topology, 720, dangerous_road, remote_farm, 1);
    connect_bidirectional(&mut topology, 730, market, safe_route, 2);
    connect_bidirectional(&mut topology, 740, safe_route, remote_farm, 2);

    let mut world = World::new(topology).unwrap();
    let (actor, bandit, orchard_row) = {
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
        let bandit = txn.create_agent("Bandit", ControlSource::Ai).unwrap();
        let orchard_row = txn.create_entity(EntityKind::Facility);
        txn.set_ground_location(actor, market).unwrap();
        txn.set_ground_location(bandit, dangerous_road).unwrap();
        txn.set_ground_location(orchard_row, remote_farm).unwrap();
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
        txn.set_component_merchandise_profile(
            actor,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Apple]),
                home_facility: Some(market),
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(actor, sample_trade_disposition_profile())
            .unwrap();
        txn.set_component_demand_memory(
            actor,
            DemandMemory {
                observations: vec![DemandObservation {
                    commodity: CommodityKind::Apple,
                    quantity: Quantity(2),
                    place: market,
                    tick: Tick(1),
                    counterparty: None,
                    reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
                }],
            },
        )
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
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        )
        .unwrap();
        let mut event_log = EventLog::new();
        let _ = txn.commit(&mut event_log);
        (actor, bandit, orchard_row)
    };

    sync_all_beliefs(&mut world, actor, Tick(1));
    if with_combat_belief {
        let mut store = world
            .get_component_agent_belief_store(actor)
            .cloned()
            .expect("restock threat fixture actor should have a belief store");
        let mut bandit_belief = build_believed_entity_state(
            &world,
            bandit,
            Tick(1),
            PerceptionSource::DirectObservation,
        )
        .expect("bandit belief should build");
        bandit_belief.believed_activity = Some(worldwake_core::BelievedActivity {
            action_domain: worldwake_core::ActionDomain::Combat,
            target: None,
            observed_tick: Tick(1),
        });
        store.update_entity(bandit, bandit_belief);
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        txn.set_component_agent_belief_store(actor, store).unwrap();
        let mut event_log = EventLog::new();
        let _ = txn.commit(&mut event_log);
    }

    RestockThreatFixture {
        world,
        actor,
        orchard_row,
        market,
        dangerous_road,
        safe_route,
        remote_farm,
    }
}

fn first_travel_destination(plan: &crate::PlannedPlan) -> Option<EntityId> {
    plan.steps.iter().find_map(|step| {
        (step.op_kind == PlannerOpKind::Travel)
            .then(|| step.targets.first().copied())
            .flatten()
            .and_then(|target| match target {
                PlanningEntityRef::Authoritative(entity) => Some(entity),
                PlanningEntityRef::Hypothetical(_) => None,
            })
    })
}

struct RemoteProduceCommodityFixture {
    world: World,
    actor: EntityId,
    mill: EntityId,
    firewood: EntityId,
    recipe_id: RecipeId,
    recipes: RecipeRegistry,
    registry: ActionDefRegistry,
    handlers: worldwake_sim::ActionHandlerRegistry,
    semantics: BTreeMap<ActionDefId, PlannerOpSemantics>,
}

fn build_remote_produce_commodity_fixture() -> RemoteProduceCommodityFixture {
    let village_square = prototype_place_entity(PrototypePlace::VillageSquare);
    let orchard_farm = prototype_place_entity(PrototypePlace::OrchardFarm);
    let mut recipes = RecipeRegistry::new();
    let recipe_id = recipes.register(bake_bread_recipe());
    let (registry, handlers) = build_registry_with_recipes(&recipes);
    let semantics = build_semantics_table(&registry);
    let mut world = World::new(build_prototype_world()).unwrap();
    let (actor, mill, firewood) = {
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        let actor = txn.create_agent("Baker", ControlSource::Ai).unwrap();
        let mill = txn.create_entity(EntityKind::Facility);
        let firewood = txn
            .create_item_lot(CommodityKind::Firewood, Quantity(1))
            .expect("firewood lot should be creatable");
        txn.set_ground_location(actor, village_square).unwrap();
        txn.set_ground_location(mill, village_square).unwrap();
        txn.set_ground_location(firewood, orchard_farm).unwrap();
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
        txn.set_component_known_recipes(actor, KnownRecipes::with([recipe_id]))
            .unwrap();
        txn.set_component_workstation_marker(mill, WorkstationMarker(WorkstationTag::Mill))
            .unwrap();
        let mut event_log = EventLog::new();
        let _ = txn.commit(&mut event_log);
        (actor, mill, firewood)
    };
    sync_all_beliefs(&mut world, actor, Tick(1));

    RemoteProduceCommodityFixture {
        world,
        actor,
        mill,
        firewood,
        recipe_id,
        recipes,
        registry,
        handlers,
        semantics,
    }
}

#[test]
fn search_restock_route_preference_follows_believed_combat_threat() {
    let mut recipes = RecipeRegistry::new();
    recipes.register(harvest_apple_recipe());
    let (registry, handlers) = build_registry_with_recipes(&recipes);
    let semantics = build_semantics_table(&registry);

    let short_route_fixture = build_restock_threat_fixture(false);
    let short_route_goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(short_route_fixture.remote_farm),
        key: GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        }),
        evidence_entities: BTreeSet::from([short_route_fixture.orchard_row]),
        evidence_places: BTreeSet::from([
            short_route_fixture.market,
            short_route_fixture.remote_farm,
        ]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let short_route_view =
        PerAgentBeliefView::from_world(short_route_fixture.actor, &short_route_fixture.world);
    let short_route_snapshot = build_planning_snapshot(
        &short_route_view,
        short_route_fixture.actor,
        &short_route_goal.evidence_entities,
        &short_route_goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );
    let short_route_plan = search_plan(
        &short_route_snapshot,
        &short_route_goal,
        &semantics,
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(1),
        None,
        None,
    )
    .into_plan()
    .expect("restock search should find a short dangerous route without a combat belief");

    assert_eq!(
        first_travel_destination(&short_route_plan),
        Some(short_route_fixture.dangerous_road),
        "without a combat belief, the merchant should prefer the shorter dangerous road"
    );

    let safe_route_fixture = build_restock_threat_fixture(true);
    let safe_route_goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(safe_route_fixture.remote_farm),
        key: GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        }),
        evidence_entities: BTreeSet::from([safe_route_fixture.orchard_row]),
        evidence_places: BTreeSet::from([
            safe_route_fixture.market,
            safe_route_fixture.remote_farm,
        ]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let safe_route_view =
        PerAgentBeliefView::from_world(safe_route_fixture.actor, &safe_route_fixture.world);
    let safe_route_snapshot = build_planning_snapshot(
        &safe_route_view,
        safe_route_fixture.actor,
        &safe_route_goal.evidence_entities,
        &safe_route_goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );
    let safe_route_plan = search_plan(
        &safe_route_snapshot,
        &safe_route_goal,
        &semantics,
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(1),
        None,
        None,
    )
    .into_plan()
    .expect("restock search should still find a route after acquiring a combat belief");

    assert_eq!(
        first_travel_destination(&safe_route_plan),
        Some(safe_route_fixture.safe_route),
        "with a combat belief on the dangerous road, the merchant should prefer the safe route"
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
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        )
        .unwrap();
        txn.set_component_contention_policy(
            orchard_row,
            ContentionPolicy {
                grant_hold_ticks: NonZeroU32::new(3).unwrap(),
                auto_promote: true,
                max_waiters: None,
            },
        )
        .unwrap();
        let granted = granted.then_some(ContentionGrant {
            actor,
            intended_action: harvest_action,
            granted_at: Tick(2),
            expires_at: Tick(5),
        });
        txn.set_component_contention_queue(
            orchard_row,
            ContentionQueue {
                granted,
                ..ContentionQueue::default()
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
        .get_component_contention_queue(fixture.orchard_row)
        .cloned()
        .expect("exclusive fixture should include queue state");
    queue
        .enqueue(fixture.actor, fixture.harvest_action, queued_at, None)
        .expect("fixture actor should be queueable");
    txn.set_component_contention_queue(fixture.orchard_row, queue)
        .unwrap();
    let mut event_log = EventLog::new();
    let _ = txn.commit(&mut event_log);
    sync_all_beliefs(&mut fixture.world, fixture.actor, queued_at);
}

struct ContentionCorpseFixture {
    world: World,
    actor: EntityId,
    town: EntityId,
    corpse: EntityId,
    grave_plot: EntityId,
    loot_action: ActionDefId,
    bury_action: ActionDefId,
    registry: ActionDefRegistry,
    handlers: worldwake_sim::ActionHandlerRegistry,
}

fn build_contention_corpse_fixture() -> ContentionCorpseFixture {
    let town = prototype_place_entity(PrototypePlace::VillageSquare);
    let (registry, handlers) = build_registry();
    let loot_action = registry
        .iter()
        .find(|def| def.name == "loot")
        .map(|def| def.id)
        .expect("loot action should be registered");
    let bury_action = registry
        .iter()
        .find(|def| def.name == "bury")
        .map(|def| def.id)
        .expect("bury action should be registered");
    let mut world = World::new(build_prototype_world()).unwrap();
    let (actor, corpse, grave_plot) = {
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        let actor = txn.create_agent("Gravedigger", ControlSource::Ai).unwrap();
        let corpse = txn.create_agent("Corpse", ControlSource::Ai).unwrap();
        let grave_plot = txn.create_entity(EntityKind::Facility);
        let coins = txn
            .create_item_lot(CommodityKind::Coin, Quantity(3))
            .unwrap();
        txn.set_ground_location(actor, town).unwrap();
        txn.set_ground_location(corpse, town).unwrap();
        txn.set_ground_location(grave_plot, town).unwrap();
        txn.set_ground_location(coins, town).unwrap();
        txn.set_possessor(coins, corpse).unwrap();
        txn.set_component_dead_at(
            corpse,
            DeadAt {
                tick: Tick(1),
                cause: worldwake_core::DeathCause::CombatWounds,
            },
        )
        .unwrap();
        txn.set_component_workstation_marker(
            grave_plot,
            WorkstationMarker(WorkstationTag::GravePlot),
        )
        .unwrap();
        txn.set_component_contention_policy(
            corpse,
            ContentionPolicy {
                grant_hold_ticks: NonZeroU32::new(4).unwrap(),
                auto_promote: true,
                max_waiters: None,
            },
        )
        .unwrap();
        txn.set_component_contention_queue(corpse, ContentionQueue::default())
            .unwrap();
        let mut event_log = EventLog::new();
        let _ = txn.commit(&mut event_log);
        (actor, corpse, grave_plot)
    };
    sync_all_beliefs(&mut world, actor, Tick(1));
    patch_believed_entity_state(&mut world, actor, corpse, Tick(1), |state| {
        state
            .last_known_inventory
            .insert(CommodityKind::Coin, Quantity(3));
        state.alive = false;
    });

    ContentionCorpseFixture {
        world,
        actor,
        town,
        corpse,
        grave_plot,
        loot_action,
        bury_action,
        registry,
        handlers,
    }
}

struct ContentionCareFixture {
    world: World,
    actor: EntityId,
    town: EntityId,
    patient: EntityId,
    heal_action: ActionDefId,
    registry: ActionDefRegistry,
    handlers: worldwake_sim::ActionHandlerRegistry,
}

fn build_contention_care_fixture() -> ContentionCareFixture {
    let town = prototype_place_entity(PrototypePlace::VillageSquare);
    let (registry, handlers) = build_registry();
    let heal_action = registry
        .iter()
        .find(|def| def.name == "heal")
        .map(|def| def.id)
        .expect("heal action should be registered");
    let mut world = World::new(build_prototype_world()).unwrap();
    let (actor, patient) = {
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        let actor = txn.create_agent("Healer", ControlSource::Ai).unwrap();
        let patient = txn.create_agent("Patient", ControlSource::Ai).unwrap();
        let medicine = txn
            .create_item_lot(CommodityKind::Medicine, Quantity(1))
            .unwrap();
        txn.set_ground_location(actor, town).unwrap();
        txn.set_ground_location(patient, town).unwrap();
        txn.set_ground_location(medicine, town).unwrap();
        txn.set_possessor(medicine, actor).unwrap();
        txn.set_component_wound_list(
            patient,
            worldwake_core::WoundList {
                wounds: vec![wound(400)],
            },
        )
        .unwrap();
        txn.set_component_contention_policy(
            patient,
            ContentionPolicy {
                grant_hold_ticks: NonZeroU32::new(4).unwrap(),
                auto_promote: true,
                max_waiters: None,
            },
        )
        .unwrap();
        txn.set_component_contention_queue(patient, ContentionQueue::default())
            .unwrap();
        let mut event_log = EventLog::new();
        let _ = txn.commit(&mut event_log);
        (actor, patient)
    };
    sync_all_beliefs(&mut world, actor, Tick(1));
    patch_believed_entity_state(&mut world, actor, patient, Tick(1), |state| {
        state.wounds = vec![wound(400)];
        state.alive = true;
    });

    ContentionCareFixture {
        world,
        actor,
        town,
        patient,
        heal_action,
        registry,
        handlers,
    }
}

#[test]
fn search_queues_before_harvest_at_exclusive_facility_without_grant() {
    let fixture = build_exclusive_orchard_fixture(false);

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        }),
        evidence_entities: BTreeSet::from([fixture.orchard_row]),
        evidence_places: BTreeSet::from([fixture.orchard_farm]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );

    let plan = search_plan(
        &snapshot,
        &goal,
        &fixture.semantics,
        &fixture.registry,
        &fixture.handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
fn search_acquire_self_consume_queues_before_harvest_at_exclusive_facility_without_grant() {
    let fixture = build_exclusive_orchard_fixture(false);

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::from([fixture.orchard_row]),
        evidence_places: BTreeSet::from([fixture.orchard_farm]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );

    let plan = search_plan(
        &snapshot,
        &goal,
        &fixture.semantics,
        &fixture.registry,
        &fixture.handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    )
    .into_plan()
    .expect("exclusive orchard self-consume should still queue before harvest");

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

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        }),
        evidence_entities: BTreeSet::from([fixture.orchard_row]),
        evidence_places: BTreeSet::from([fixture.orchard_farm]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );

    let plan = search_plan(
        &snapshot,
        &goal,
        &fixture.semantics,
        &fixture.registry,
        &fixture.handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
            requested_quantity: Quantity(2),
            required_tool_kinds: Vec::new(),
        })
    );
    assert_ne!(plan.steps[0].op_kind, PlannerOpKind::QueueForFacilityUse);
}

#[test]
fn search_acquire_self_consume_skips_queue_when_matching_grant_is_already_active() {
    let fixture = build_exclusive_orchard_fixture(true);

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::from([fixture.orchard_row]),
        evidence_places: BTreeSet::from([fixture.orchard_farm]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );

    let plan = search_plan(
        &snapshot,
        &goal,
        &fixture.semantics,
        &fixture.registry,
        &fixture.handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    )
    .into_plan()
    .expect("matching grant should still allow direct self-consume harvest");

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
            requested_quantity: Quantity(2),
            required_tool_kinds: Vec::new(),
        })
    );
    assert_ne!(plan.steps[0].op_kind, PlannerOpKind::QueueForFacilityUse);
}

#[test]
fn search_does_not_offer_duplicate_queue_candidate_when_actor_is_already_queued() {
    let mut fixture = build_exclusive_orchard_fixture(false);
    enqueue_actor_for_exclusive_fixture(&mut fixture, Tick(2));

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        }),
        evidence_entities: BTreeSet::from([fixture.orchard_row]),
        evidence_places: BTreeSet::from([fixture.orchard_farm]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );
    let queue_def = fixture
        .registry
        .iter()
        .find(|def| def.name == "queue_for_facility_use")
        .map(|def| def.id)
        .expect("queue action should be registered");

    let rel_defs = relevant_action_defs(&goal, &fixture.semantics);
    let candidates = search_candidates(
        &goal,
        &root_node(
            &snapshot,
            &goal,
            &RecipeRegistry::new(),
            &ProfileFixture::default(),
        ),
        candidate_search_context(
            &fixture.semantics,
            &fixture.registry,
            &fixture.handlers,
            &BlockerMemory::default(),
            Tick(0),
            &rel_defs,
        ),
        None,
        None,
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        }),
        evidence_entities: BTreeSet::from([fixture.orchard_row]),
        evidence_places: BTreeSet::from([fixture.orchard_farm]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let mut blocked = BlockerMemory::default();
    blocked.record(Blocker {
        blocker_key: BlockerKey {
            goal_key: goal.key,
            place: Some(fixture.orchard_farm),
            target: Some(fixture.orchard_row),
            action_def: Some(fixture.harvest_action),
        },
        blocking_fact: BlockingFact::ExclusiveFacilityUnavailable,
        diagnostic_context: None,
        observed_tick: Tick(2),
        expires_tick: Tick(20),
        clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
        baseline_snapshot: None,
    });
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot_with_blocked_facility_uses(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
        &blocked,
        Tick(3),
        goal.key.kind.relevant_op_kinds(),
    );
    let queue_def = fixture
        .registry
        .iter()
        .find(|def| def.name == "queue_for_facility_use")
        .map(|def| def.id)
        .expect("queue action should be registered");

    let rel_defs = relevant_action_defs(&goal, &fixture.semantics);
    let candidates = search_candidates(
        &goal,
        &root_node(
            &snapshot,
            &goal,
            &RecipeRegistry::new(),
            &ProfileFixture::default(),
        ),
        candidate_search_context(
            &fixture.semantics,
            &fixture.registry,
            &fixture.handlers,
            &BlockerMemory::default(),
            Tick(0),
            &rel_defs,
        ),
        None,
        None,
        None,
    );

    assert!(!candidates.iter().any(|candidate| {
        candidate.def_id == queue_def
            && candidate.authoritative_targets == vec![fixture.orchard_row]
    }));
}

#[test]
fn search_trace_records_blocked_facility_use_root_filter() {
    let fixture = build_exclusive_orchard_fixture(false);
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        }),
        evidence_entities: BTreeSet::from([fixture.orchard_row]),
        evidence_places: BTreeSet::from([fixture.orchard_farm]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let mut blocked = BlockerMemory::default();
    blocked.record(Blocker {
        blocker_key: BlockerKey {
            goal_key: goal.key,
            place: Some(fixture.orchard_farm),
            target: Some(fixture.orchard_row),
            action_def: Some(fixture.harvest_action),
        },
        blocking_fact: BlockingFact::ExclusiveFacilityUnavailable,
        diagnostic_context: None,
        observed_tick: Tick(2),
        expires_tick: Tick(20),
        clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
        baseline_snapshot: None,
    });
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot_with_blocked_facility_uses(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
        &blocked,
        Tick(3),
        goal.key.kind.relevant_op_kinds(),
    );
    let queue_def = fixture
        .registry
        .iter()
        .find(|def| def.name == "queue_for_facility_use")
        .map(|def| def.id)
        .expect("queue action should be registered");
    let mut expansions = Vec::new();

    let _result = search_plan(
        &snapshot,
        &goal,
        &fixture.semantics,
        &fixture.registry,
        &fixture.handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        Some(&mut expansions),
    );

    let root = expansions
        .iter()
        .find(|summary| summary.depth == 0)
        .expect("root expansion summary should be recorded");
    let blocked_queue = root
        .root_candidates
        .iter()
        .find(|candidate| {
            candidate.def_id == queue_def
                && candidate.authoritative_targets == vec![fixture.orchard_row]
        })
        .expect("blocked queue candidate should still appear in root provenance");
    assert_eq!(
        blocked_queue.outcome,
        crate::decision_trace::RootCandidateOutcome::Filtered(
            crate::decision_trace::RootCandidateFilterReason::BlockedFacilityUse {
                facility: fixture.orchard_row,
                intended_action: fixture.harvest_action,
            },
        )
    );
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
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        )
        .unwrap();
        txn.set_component_contention_policy(
            orchard_row,
            ContentionPolicy {
                grant_hold_ticks: NonZeroU32::new(3).unwrap(),
                auto_promote: true,
                max_waiters: None,
            },
        )
        .unwrap();
        txn.set_component_contention_queue(orchard_row, ContentionQueue::default())
            .unwrap();
        let mut event_log = EventLog::new();
        let _ = txn.commit(&mut event_log);
        orchard_row
    };
    sync_all_beliefs(&mut fixture.world, fixture.actor, Tick(2));
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        }),
        evidence_entities: BTreeSet::from([fixture.orchard_row, second_orchard]),
        evidence_places: BTreeSet::from([fixture.orchard_farm]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let mut blocked = BlockerMemory::default();
    blocked.record(Blocker {
        blocker_key: BlockerKey {
            goal_key: goal.key,
            place: Some(fixture.orchard_farm),
            target: Some(fixture.orchard_row),
            action_def: Some(fixture.harvest_action),
        },
        blocking_fact: BlockingFact::ExclusiveFacilityUnavailable,
        diagnostic_context: None,
        observed_tick: Tick(2),
        expires_tick: Tick(20),
        clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
        baseline_snapshot: None,
    });
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot_with_blocked_facility_uses(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
        &blocked,
        Tick(3),
        goal.key.kind.relevant_op_kinds(),
    );

    let plan = search_plan(
        &snapshot,
        &goal,
        &fixture.semantics,
        &fixture.registry,
        &fixture.handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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

#[test]
fn corpse_queue_affordance_expands_to_loot_and_filters_direct_loot_without_grant() {
    let fixture = build_contention_corpse_fixture();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::LootCorpse {
            corpse: fixture.corpse,
        }),
        evidence_entities: BTreeSet::from([fixture.corpse]),
        evidence_places: BTreeSet::from([fixture.town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );
    let state = PlanningState::new(&snapshot);
    let queue_affordance = Affordance {
        actor: fixture.actor,
        def_id: fixture
            .registry
            .iter()
            .find(|def| def.name == "queue_for_corpse_use")
            .map(|def| def.id)
            .expect("queue_for_corpse_use should be registered"),
        bound_targets: vec![fixture.corpse],
        payload_override: None,
        explanation: None,
        contention_status: worldwake_core::ContentionStatus::Available,
    };
    let queue_candidates = search_candidates_from_affordance(
        &goal,
        &state,
        &fixture.registry,
        &fixture.handlers,
        &queue_affordance,
    );

    assert_eq!(queue_candidates.len(), 1);
    assert_eq!(
        queue_candidates[0].payload_override,
        Some(ActionPayload::QueueForFacilityUse(
            QueueForFacilityUsePayload {
                intended_action: fixture.loot_action,
            },
        ))
    );
    assert_eq!(
        queue_candidates[0].authoritative_targets,
        vec![fixture.corpse]
    );

    let direct_loot_affordance = Affordance {
        actor: fixture.actor,
        def_id: fixture.loot_action,
        bound_targets: vec![fixture.corpse],
        payload_override: None,
        explanation: None,
        contention_status: worldwake_core::ContentionStatus::Available,
    };

    assert!(
        search_candidates_from_affordance(
            &goal,
            &state,
            &fixture.registry,
            &fixture.handlers,
            &direct_loot_affordance
        )
        .is_empty(),
        "direct loot should not search as available until the actor holds the grant"
    );
}

#[test]
fn corpse_loot_goal_searches_queue_step_before_loot_when_corpse_is_contention_managed() {
    let fixture = build_contention_corpse_fixture();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::LootCorpse {
            corpse: fixture.corpse,
        }),
        evidence_entities: BTreeSet::from([fixture.corpse]),
        evidence_places: BTreeSet::from([fixture.town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );

    let plan = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&fixture.registry),
        &fixture.registry,
        &fixture.handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    )
    .into_plan()
    .expect("contention-managed corpse loot should search a queue-backed plan");

    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::QueueForFacilityUse);
}

#[test]
fn corpse_queue_affordance_expands_to_bury_and_filters_direct_bury_without_grant() {
    let fixture = build_contention_corpse_fixture();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::BuryCorpse {
            corpse: fixture.corpse,
            burial_site: fixture.grave_plot,
        }),
        evidence_entities: BTreeSet::from([fixture.corpse, fixture.grave_plot]),
        evidence_places: BTreeSet::from([fixture.town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );
    let state = PlanningState::new(&snapshot);
    let queue_affordance = Affordance {
        actor: fixture.actor,
        def_id: fixture
            .registry
            .iter()
            .find(|def| def.name == "queue_for_corpse_use")
            .map(|def| def.id)
            .expect("queue_for_corpse_use should be registered"),
        bound_targets: vec![fixture.corpse],
        payload_override: None,
        explanation: None,
        contention_status: worldwake_core::ContentionStatus::Available,
    };
    let queue_candidates = search_candidates_from_affordance(
        &goal,
        &state,
        &fixture.registry,
        &fixture.handlers,
        &queue_affordance,
    );

    assert_eq!(queue_candidates.len(), 1);
    assert_eq!(
        queue_candidates[0].payload_override,
        Some(ActionPayload::QueueForFacilityUse(
            QueueForFacilityUsePayload {
                intended_action: fixture.bury_action,
            },
        ))
    );
    assert_eq!(
        queue_candidates[0].authoritative_targets,
        vec![fixture.corpse]
    );

    let direct_bury_affordance = Affordance {
        actor: fixture.actor,
        def_id: fixture.bury_action,
        bound_targets: vec![fixture.corpse, fixture.grave_plot],
        payload_override: None,
        explanation: None,
        contention_status: worldwake_core::ContentionStatus::Available,
    };

    assert!(
        search_candidates_from_affordance(
            &goal,
            &state,
            &fixture.registry,
            &fixture.handlers,
            &direct_bury_affordance
        )
        .is_empty(),
        "direct bury should not search as available until the actor holds the grant"
    );
}

#[test]
fn corpse_bury_goal_searches_queue_step_before_bury_when_corpse_is_contention_managed() {
    let fixture = build_contention_corpse_fixture();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::BuryCorpse {
            corpse: fixture.corpse,
            burial_site: fixture.grave_plot,
        }),
        evidence_entities: BTreeSet::from([fixture.corpse, fixture.grave_plot]),
        evidence_places: BTreeSet::from([fixture.town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );

    let plan = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&fixture.registry),
        &fixture.registry,
        &fixture.handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    )
    .into_plan()
    .expect("contention-managed corpse burial should search a queue-backed plan");

    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::QueueForFacilityUse);
}

#[test]
fn care_queue_affordance_expands_to_heal_and_filters_direct_heal_without_grant() {
    let fixture = build_contention_care_fixture();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::TreatWounds {
            patient: fixture.patient,
        }),
        evidence_entities: BTreeSet::from([fixture.patient]),
        evidence_places: BTreeSet::from([fixture.town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );
    let state = PlanningState::new(&snapshot);
    let queue_affordance = Affordance {
        actor: fixture.actor,
        def_id: fixture
            .registry
            .iter()
            .find(|def| def.name == "queue_for_care_target")
            .map(|def| def.id)
            .expect("queue_for_care_target should be registered"),
        bound_targets: vec![fixture.patient],
        payload_override: None,
        explanation: None,
        contention_status: worldwake_core::ContentionStatus::Available,
    };
    let queue_candidates = search_candidates_from_affordance(
        &goal,
        &state,
        &fixture.registry,
        &fixture.handlers,
        &queue_affordance,
    );

    assert_eq!(queue_candidates.len(), 1);
    assert_eq!(
        queue_candidates[0].payload_override,
        Some(ActionPayload::QueueForFacilityUse(
            QueueForFacilityUsePayload {
                intended_action: fixture.heal_action,
            },
        ))
    );
    assert_eq!(
        queue_candidates[0].authoritative_targets,
        vec![fixture.patient]
    );

    let direct_heal_affordance = Affordance {
        actor: fixture.actor,
        def_id: fixture.heal_action,
        bound_targets: vec![fixture.patient],
        payload_override: None,
        explanation: None,
        contention_status: worldwake_core::ContentionStatus::Available,
    };

    assert!(
        search_candidates_from_affordance(
            &goal,
            &state,
            &fixture.registry,
            &fixture.handlers,
            &direct_heal_affordance
        )
        .is_empty(),
        "direct heal should not search as available until the actor holds the grant"
    );
}

#[test]
fn care_queue_affordance_does_not_expand_when_actor_cannot_currently_heal() {
    let mut fixture = build_contention_care_fixture();
    let medicine = fixture
        .world
        .entities()
        .find(|entity| {
            fixture
                .world
                .get_component_item_lot(*entity)
                .is_some_and(|lot| lot.commodity == CommodityKind::Medicine)
        })
        .expect("fixture should seed medicine");
    let mut txn = WorldTxn::new(
        &mut fixture.world,
        Tick(2),
        CauseRef::Bootstrap,
        None,
        None,
        VisibilitySpec::SamePlace,
        WitnessData::default(),
    );
    txn.clear_possessor(medicine).unwrap();
    txn.set_ground_location(medicine, fixture.town).unwrap();
    let mut event_log = EventLog::new();
    let _ = txn.commit(&mut event_log);

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::TreatWounds {
            patient: fixture.patient,
        }),
        evidence_entities: BTreeSet::from([fixture.patient]),
        evidence_places: BTreeSet::from([fixture.town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );
    let state = PlanningState::new(&snapshot);
    let queue_affordance = Affordance {
        actor: fixture.actor,
        def_id: fixture
            .registry
            .iter()
            .find(|def| def.name == "queue_for_care_target")
            .map(|def| def.id)
            .expect("queue_for_care_target should be registered"),
        bound_targets: vec![fixture.patient],
        payload_override: None,
        explanation: None,
        contention_status: worldwake_core::ContentionStatus::Available,
    };

    assert!(
        search_candidates_from_affordance(
            &goal,
            &state,
            &fixture.registry,
            &fixture.handlers,
            &queue_affordance
        )
        .is_empty(),
        "queue_for_care_target should not expand when the actor cannot currently perform heal"
    );
}

#[allow(clippy::too_many_lines)]
#[test]
fn queue_affordance_expands_to_one_candidate_per_matching_intended_action() {
    let orchard_farm = prototype_place_entity(PrototypePlace::OrchardFarm);
    let mut recipes = RecipeRegistry::new();
    recipes.register(harvest_apple_recipe_variant("Harvest Apples Alpha", 2));
    recipes.register(harvest_apple_recipe_variant("Harvest Apples Beta", 1));
    let (registry, handlers) = build_registry_with_recipes(&recipes);
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
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        )
        .unwrap();
        txn.set_component_contention_policy(
            orchard_row,
            ContentionPolicy {
                grant_hold_ticks: NonZeroU32::new(3).unwrap(),
                auto_promote: true,
                max_waiters: None,
            },
        )
        .unwrap();
        txn.set_component_contention_queue(orchard_row, ContentionQueue::default())
            .unwrap();
        let mut event_log = EventLog::new();
        let _ = txn.commit(&mut event_log);
        (actor, orchard_row)
    };
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        }),
        evidence_entities: BTreeSet::from([orchard_row]),
        evidence_places: BTreeSet::from([orchard_farm]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        ProfileFixture::default().snapshot_travel_horizon,
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
        contention_status: worldwake_core::ContentionStatus::Unmanaged,
    };

    let queue_candidates =
        search_candidates_from_affordance(&goal, &state, &registry, &handlers, &affordance);

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

#[test]
fn search_candidates_from_affordance_rejects_trade_for_wrong_seller_opportunity() {
    let actor = entity(1);
    let seller_a = entity(2);
    let seller_b = entity(3);
    let market = entity(10);
    let lot_a = entity(20);
    let lot_b = entity(21);

    let mut view = TestBeliefView::default();
    view.alive
        .extend([actor, seller_a, seller_b, market, lot_a, lot_b]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(seller_a, EntityKind::Agent);
    view.kinds.insert(seller_b, EntityKind::Agent);
    view.kinds.insert(market, EntityKind::Place);
    view.kinds.insert(lot_a, EntityKind::ItemLot);
    view.kinds.insert(lot_b, EntityKind::ItemLot);
    view.effective_places.insert(actor, market);
    view.effective_places.insert(seller_a, market);
    view.effective_places.insert(seller_b, market);
    view.effective_places.insert(lot_a, market);
    view.effective_places.insert(lot_b, market);
    view.entities_at
        .insert(market, vec![actor, seller_a, seller_b, lot_a, lot_b]);
    view.trade_profiles
        .insert(actor, sample_trade_disposition_profile());
    view.merchandise_profiles.insert(
        seller_a,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([CommodityKind::Bread]),
            home_facility: Some(market),
        },
    );
    view.merchandise_profiles.insert(
        seller_b,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([CommodityKind::Bread]),
            home_facility: Some(market),
        },
    );
    view.commodity_quantities
        .insert((actor, CommodityKind::Coin), Quantity(3));
    view.commodity_quantities
        .insert((seller_a, CommodityKind::Bread), Quantity(1));
    view.commodity_quantities
        .insert((seller_b, CommodityKind::Bread), Quantity(1));
    view.lot_commodities.insert(lot_a, CommodityKind::Bread);
    view.lot_commodities.insert(lot_b, CommodityKind::Bread);
    view.direct_possessors.insert(lot_a, seller_a);
    view.direct_possessors.insert(lot_b, seller_b);
    view.direct_possessions
        .entry(seller_a)
        .or_default()
        .push(lot_a);
    view.direct_possessions
        .entry(seller_b)
        .or_default()
        .push(lot_b);
    view.listed_lots
        .insert((market, CommodityKind::Bread), vec![lot_a, lot_b]);
    view.lot_sellers.insert(lot_a, seller_a);
    view.lot_sellers.insert(lot_b, seller_b);

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::from([seller_b]),
        evidence_places: BTreeSet::from([market]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let state = PlanningState::new(&snapshot);
    let (registry, handlers) = build_registry();
    let trade_def_id = registry
        .iter()
        .find(|def| def.name == "trade")
        .map(|def| def.id)
        .expect("trade action should be registered");

    let local_seller_affordance = Affordance {
        def_id: trade_def_id,
        actor,
        bound_targets: vec![seller_a],
        payload_override: Some(ActionPayload::Trade(TradeActionPayload {
            counterparty: seller_a,
            sale_lot: lot_a,
            offered_commodity: CommodityKind::Coin,
            offered_quantity: Quantity(1),
            requested_quantity: Quantity(1),
        })),
        explanation: None,
        contention_status: worldwake_core::ContentionStatus::Unmanaged,
    };
    let remote_seller_affordance = Affordance {
        def_id: trade_def_id,
        actor,
        bound_targets: vec![seller_b],
        payload_override: Some(ActionPayload::Trade(TradeActionPayload {
            counterparty: seller_b,
            sale_lot: lot_b,
            offered_commodity: CommodityKind::Coin,
            offered_quantity: Quantity(1),
            requested_quantity: Quantity(1),
        })),
        explanation: None,
        contention_status: worldwake_core::ContentionStatus::Unmanaged,
    };

    let wrong_candidates = search_candidates_from_affordance(
        &goal,
        &state,
        &registry,
        &handlers,
        &local_seller_affordance,
    );
    let correct_candidates = search_candidates_from_affordance(
        &goal,
        &state,
        &registry,
        &handlers,
        &remote_seller_affordance,
    );

    assert!(wrong_candidates.is_empty());
    assert_eq!(correct_candidates.len(), 1);
    assert_eq!(correct_candidates[0].authoritative_targets, vec![seller_b]);
    assert_eq!(
        correct_candidates[0]
            .payload_override
            .as_ref()
            .and_then(ActionPayload::as_trade)
            .map(|trade| trade.counterparty),
        Some(seller_b)
    );
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
    let mut medicine_belief = believed_entity_state_at(medicine_place, Tick(1), None);
    medicine_belief.believed_kind = Some(EntityKind::ItemLot);
    medicine_belief
        .last_known_inventory
        .insert(CommodityKind::Medicine, Quantity(1));
    remember_entity(&mut view, actor, medicine, medicine_belief);

    (
        view,
        actor,
        patient,
        current_place,
        patient_place,
        medicine_place,
    )
}

fn build_local_medicine_remote_patient_view()
-> (TestBeliefView, EntityId, EntityId, EntityId, EntityId) {
    let (mut view, actor, patient, current_place, patient_place, medicine_place) =
        build_branching_care_view();
    let medicine = entity(20);
    view.effective_places.insert(medicine, current_place);
    view.entities_at
        .insert(current_place, vec![actor, medicine]);
    view.entities_at.insert(medicine_place, Vec::new());
    if let Some((_, belief)) = view
        .known_entity_beliefs
        .entry(actor)
        .or_default()
        .iter_mut()
        .find(|(entity, _)| *entity == medicine)
    {
        belief.last_known_place = Some(current_place);
    }
    (view, actor, patient, current_place, patient_place)
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
fn search_prefers_longer_low_threat_route_over_shorter_dangerous_route() {
    let actor = entity(1);
    let origin = entity(10);
    let dangerous_waypoint = entity(11);
    let safe_waypoint = entity(12);
    let market = entity(13);
    let bread = entity(20);
    let hostile = entity(30);

    let mut view = TestBeliefView {
        current_tick: Tick(10),
        ..TestBeliefView::default()
    };
    view.alive.extend([
        actor,
        origin,
        dangerous_waypoint,
        safe_waypoint,
        market,
        bread,
    ]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(origin, EntityKind::Place);
    view.kinds.insert(dangerous_waypoint, EntityKind::Place);
    view.kinds.insert(safe_waypoint, EntityKind::Place);
    view.kinds.insert(market, EntityKind::Place);
    view.kinds.insert(bread, EntityKind::ItemLot);
    view.effective_places.insert(actor, origin);
    view.effective_places.insert(bread, market);
    view.entities_at.insert(origin, vec![actor]);
    view.entities_at.insert(market, vec![bread]);
    view.controllable.insert((actor, bread));
    view.adjacent.insert(
        origin,
        vec![
            (dangerous_waypoint, NonZeroU32::new(1).unwrap()),
            (safe_waypoint, NonZeroU32::new(1).unwrap()),
        ],
    );
    view.adjacent.insert(
        dangerous_waypoint,
        vec![
            (origin, NonZeroU32::new(1).unwrap()),
            (market, NonZeroU32::new(1).unwrap()),
        ],
    );
    view.adjacent.insert(
        safe_waypoint,
        vec![
            (origin, NonZeroU32::new(1).unwrap()),
            (market, NonZeroU32::new(2).unwrap()),
        ],
    );
    view.adjacent.insert(
        market,
        vec![
            (dangerous_waypoint, NonZeroU32::new(1).unwrap()),
            (safe_waypoint, NonZeroU32::new(2).unwrap()),
        ],
    );
    view.lot_commodities.insert(bread, CommodityKind::Bread);
    view.commodity_quantities
        .insert((bread, CommodityKind::Bread), Quantity(1));
    view.carry_capacities.insert(actor, LoadUnits(10));
    view.entity_loads.insert(actor, LoadUnits(0));
    view.entity_loads.insert(bread, LoadUnits(1));
    view.known_entity_beliefs.insert(
        actor,
        vec![(hostile, combat_belief_at(dangerous_waypoint, Tick(10)))],
    );

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::from([bread]),
        evidence_places: BTreeSet::from([market]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        2,
    );
    let (registry, handlers) = build_registry();
    let plan = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(10),
        None,
        None,
    )
    .into_plan()
    .expect("planner should still find the safe detour plan");

    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Travel);
    assert_eq!(
        plan.steps[0].targets,
        vec![PlanningEntityRef::Authoritative(safe_waypoint)]
    );
    assert_eq!(plan.steps[1].op_kind, PlannerOpKind::Travel);
    assert_eq!(
        plan.steps[1].targets,
        vec![PlanningEntityRef::Authoritative(market)]
    );
}

#[test]
fn prune_travel_trace_records_perceived_cost_components_for_retained_rivals() {
    let actor = entity(1);
    let origin = entity(10);
    let dangerous_waypoint = entity(11);
    let safe_waypoint = entity(12);
    let market = entity(13);
    let hostile = entity(30);

    let mut view = TestBeliefView {
        current_tick: Tick(10),
        ..TestBeliefView::default()
    };
    view.alive
        .extend([actor, origin, dangerous_waypoint, safe_waypoint, market]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(origin, EntityKind::Place);
    view.kinds.insert(dangerous_waypoint, EntityKind::Place);
    view.kinds.insert(safe_waypoint, EntityKind::Place);
    view.kinds.insert(market, EntityKind::Place);
    view.effective_places.insert(actor, origin);
    view.entities_at.insert(origin, vec![actor]);
    view.adjacent.insert(
        origin,
        vec![
            (dangerous_waypoint, NonZeroU32::new(1).unwrap()),
            (safe_waypoint, NonZeroU32::new(1).unwrap()),
        ],
    );
    view.adjacent.insert(
        dangerous_waypoint,
        vec![
            (origin, NonZeroU32::new(1).unwrap()),
            (market, NonZeroU32::new(1).unwrap()),
        ],
    );
    view.adjacent.insert(
        safe_waypoint,
        vec![
            (origin, NonZeroU32::new(1).unwrap()),
            (market, NonZeroU32::new(2).unwrap()),
        ],
    );
    view.adjacent.insert(
        market,
        vec![
            (dangerous_waypoint, NonZeroU32::new(1).unwrap()),
            (safe_waypoint, NonZeroU32::new(2).unwrap()),
        ],
    );
    view.known_entity_beliefs.insert(
        actor,
        vec![(hostile, combat_belief_at(dangerous_waypoint, Tick(10)))],
    );

    let snapshot =
        build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::from([market]), 2);

    let travel_dangerous_id = ActionDefId(100);
    let travel_safe_id = ActionDefId(101);
    let mut semantics_table = BTreeMap::new();
    semantics_table.insert(travel_dangerous_id, travel_semantics());
    semantics_table.insert(travel_safe_id, travel_semantics());

    let mut candidates = vec![
        make_travel_candidate(travel_dangerous_id, dangerous_waypoint),
        make_travel_candidate(travel_safe_id, safe_waypoint),
    ];

    let pruning = prune_travel_away_from_goal(
        &mut candidates,
        origin,
        &[market],
        &snapshot,
        &semantics_table,
    )
    .expect("retained rival travel branches should still record a comparative pruning trace");

    assert_eq!(pruning.current_place, origin);
    assert_eq!(pruning.current_remaining_travel_ticks, 3);
    assert_eq!(pruning.pruned, Vec::new());
    assert_eq!(
        pruning.retained,
        vec![
            crate::decision_trace::TravelSuccessorTrace {
                destination: dangerous_waypoint,
                base_ticks: 1,
                threat_permille: Permille::new(950).unwrap(),
                penalty_ticks: 1,
                direct_perceived_cost: 2,
                remaining_travel_ticks: 2,
                projected_total_cost: 4,
                attribution: crate::decision_trace::TravelPruningAttribution::WithinBestCost,
            },
            crate::decision_trace::TravelSuccessorTrace {
                destination: safe_waypoint,
                base_ticks: 1,
                threat_permille: Permille::new(0).unwrap(),
                penalty_ticks: 0,
                direct_perceived_cost: 1,
                remaining_travel_ticks: 2,
                projected_total_cost: 3,
                attribution: crate::decision_trace::TravelPruningAttribution::WithinBestCost,
            },
        ],
    );
}

#[test]
fn search_uses_shorter_route_when_no_danger_beliefs_exist() {
    let actor = entity(1);
    let origin = entity(10);
    let dangerous_waypoint = entity(11);
    let safe_waypoint = entity(12);
    let market = entity(13);
    let bread = entity(20);

    let mut view = TestBeliefView {
        current_tick: Tick(10),
        ..TestBeliefView::default()
    };
    view.alive.extend([
        actor,
        origin,
        dangerous_waypoint,
        safe_waypoint,
        market,
        bread,
    ]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(origin, EntityKind::Place);
    view.kinds.insert(dangerous_waypoint, EntityKind::Place);
    view.kinds.insert(safe_waypoint, EntityKind::Place);
    view.kinds.insert(market, EntityKind::Place);
    view.kinds.insert(bread, EntityKind::ItemLot);
    view.effective_places.insert(actor, origin);
    view.effective_places.insert(bread, market);
    view.entities_at.insert(origin, vec![actor]);
    view.entities_at.insert(market, vec![bread]);
    view.controllable.insert((actor, bread));
    view.adjacent.insert(
        origin,
        vec![
            (dangerous_waypoint, NonZeroU32::new(1).unwrap()),
            (safe_waypoint, NonZeroU32::new(1).unwrap()),
        ],
    );
    view.adjacent.insert(
        dangerous_waypoint,
        vec![
            (origin, NonZeroU32::new(1).unwrap()),
            (market, NonZeroU32::new(1).unwrap()),
        ],
    );
    view.adjacent.insert(
        safe_waypoint,
        vec![
            (origin, NonZeroU32::new(1).unwrap()),
            (market, NonZeroU32::new(2).unwrap()),
        ],
    );
    view.adjacent.insert(
        market,
        vec![
            (dangerous_waypoint, NonZeroU32::new(1).unwrap()),
            (safe_waypoint, NonZeroU32::new(2).unwrap()),
        ],
    );
    view.lot_commodities.insert(bread, CommodityKind::Bread);
    view.commodity_quantities
        .insert((bread, CommodityKind::Bread), Quantity(1));
    view.carry_capacities.insert(actor, LoadUnits(10));
    view.entity_loads.insert(actor, LoadUnits(0));
    view.entity_loads.insert(bread, LoadUnits(1));

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::from([bread]),
        evidence_places: BTreeSet::from([market]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        2,
    );
    let (registry, handlers) = build_registry();
    let plan = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(10),
        None,
        None,
    )
    .into_plan()
    .expect("planner should find the shorter default route");

    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Travel);
    assert_eq!(
        plan.steps[0].targets,
        vec![PlanningEntityRef::Authoritative(dangerous_waypoint)]
    );
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
        steps: SharedVec::new(),
        total_estimated_ticks: 2,
        search_cost: 2,
        tactical_barrier_reached: false,
        heuristic_ticks: 1, // f = 3
    };
    let high_f = SearchNode {
        state: PlanningState::new(&snapshot),
        steps: SharedVec::new(),
        total_estimated_ticks: 3,
        search_cost: 3,
        tactical_barrier_reached: false,
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
        steps: SharedVec::new(),
        total_estimated_ticks: 2,
        search_cost: 2,
        tactical_barrier_reached: false,
        heuristic_ticks: 3, // f = 5, g = 2
    };
    let high_g = SearchNode {
        state: PlanningState::new(&snapshot),
        steps: SharedVec::new(),
        total_estimated_ticks: 3,
        search_cost: 3,
        tactical_barrier_reached: false,
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
        steps: SharedVec::new(),
        total_estimated_ticks: 5,
        search_cost: 5,
        tactical_barrier_reached: false,
        heuristic_ticks: 0,
    };
    let node_b = SearchNode {
        state: PlanningState::new(&snapshot),
        steps: SharedVec::new(),
        total_estimated_ticks: 3,
        search_cost: 3,
        tactical_barrier_reached: false,
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
        planner_only: false,
        trace_index: None,
        expansion_trace_index: None,
    }
}

fn make_non_travel_candidate(def_id: ActionDefId, target: EntityId) -> SearchCandidate {
    SearchCandidate {
        def_id,
        authoritative_targets: vec![target],
        planning_targets: vec![PlanningEntityRef::Authoritative(target)],
        payload_override: None,
        planner_only: false,
        trace_index: None,
        expansion_trace_index: None,
    }
}

fn opportunity_index_for_place(
    place: EntityId,
    saliences: impl IntoIterator<Item = u16>,
) -> crate::opportunity_compiler::PerceivedOpportunityIndex {
    let all = saliences
        .into_iter()
        .enumerate()
        .map(
            |(index, salience)| crate::opportunity_compiler::Opportunity {
                key: OpportunityKey {
                    goal_key: GoalKey::from(GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Bread,
                        purpose: CommodityPurpose::SelfConsume,
                        quantity: AcquisitionQuantity::single(),
                    }),
                    anchor: OpportunityAnchor::Place(place),
                },
                perceived_at: Tick(u64::try_from(index).unwrap_or(u64::MAX)),
                source_belief: worldwake_core::BeliefRef {
                    claim_key: worldwake_core::BeliefClaimKey {
                        subject: place,
                        aspect: worldwake_core::EntityBeliefAspect::Location,
                    },
                    claim_held_at_tick: Tick(0),
                    status: worldwake_core::BeliefStatusTag::Probable,
                },
                possible_effects: vec![
                    crate::opportunity_compiler::EffectFactKey::CommodityTransfer,
                ],
                possible_information: Vec::new(),
                required_actions: vec![PlannerOpKind::Trade],
                legal_status: crate::opportunity_compiler::BelievedLegalStatus::BelievedUnclaimed,
                social_exposure: crate::opportunity_compiler::SocialExposureBand::Private,
                risks: Vec::new(),
                salience: Permille::new(salience).unwrap(),
            },
        )
        .collect::<Vec<_>>();
    let mut by_place = BTreeMap::new();
    by_place.insert(
        place,
        (0..all.len())
            .map(|index| {
                crate::opportunity_compiler::OpportunityHandle(
                    u32::try_from(index).unwrap_or(u32::MAX),
                )
            })
            .collect(),
    );
    crate::opportunity_compiler::PerceivedOpportunityIndex {
        by_place,
        by_anchor: BTreeMap::new(),
        all,
    }
}

fn travel_semantics() -> PlannerOpSemantics {
    PlannerOpSemantics {
        op_kind: PlannerOpKind::Travel,
        may_appear_mid_plan: true,
        is_materialization_barrier: false,
        synthetic_cargo: PlannerSyntheticCargo::None,
    }
}

fn harvest_semantics() -> PlannerOpSemantics {
    PlannerOpSemantics {
        op_kind: PlannerOpKind::Harvest,
        may_appear_mid_plan: true,
        is_materialization_barrier: true,
        synthetic_cargo: PlannerSyntheticCargo::None,
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
            base_ticks: 5,
            threat_permille: Permille::new(0).unwrap(),
            penalty_ticks: 0,
            direct_perceived_cost: 5,
            remaining_travel_ticks: 2,
            projected_total_cost: 7,
            attribution: crate::decision_trace::TravelPruningAttribution::WithinBestCost,
        }]
    );
    assert_eq!(
        pruning.pruned,
        vec![
            crate::decision_trace::TravelSuccessorTrace {
                destination: south,
                base_ticks: 4,
                threat_permille: Permille::new(0).unwrap(),
                penalty_ticks: 0,
                direct_perceived_cost: 4,
                remaining_travel_ticks: 11,
                projected_total_cost: 15,
                attribution: crate::decision_trace::TravelPruningAttribution::PrunedAsAwayFromGoal,
            },
            crate::decision_trace::TravelSuccessorTrace {
                destination: north,
                base_ticks: 3,
                threat_permille: Permille::new(0).unwrap(),
                penalty_ticks: 0,
                direct_perceived_cost: 3,
                remaining_travel_ticks: 10,
                projected_total_cost: 13,
                attribution: crate::decision_trace::TravelPruningAttribution::PrunedAsAwayFromGoal,
            },
        ]
    );
}

#[test]
fn prune_travel_with_empty_opportunity_index_matches_default_pruning() {
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

    let original_candidates = vec![
        make_travel_candidate(travel_east_id, east),
        make_travel_candidate(travel_south_id, south),
        make_travel_candidate(travel_north_id, north),
    ];
    let mut default_candidates = original_candidates.clone();
    let mut opportunity_candidates = original_candidates;

    let default_pruning = prune_travel_away_from_goal(
        &mut default_candidates,
        hub,
        &[goal_store],
        &snapshot,
        &semantics_table,
    );
    let empty_index = crate::opportunity_compiler::PerceivedOpportunityIndex::default();
    let opportunity_pruning = super::prune_travel_away_from_goal_with_expansion_trace(
        &mut opportunity_candidates,
        None,
        hub,
        &[goal_store],
        &snapshot,
        &semantics_table,
        Permille::new(150).unwrap(),
        &empty_index,
    );

    assert_eq!(
        default_candidates
            .iter()
            .map(|candidate| candidate.def_id)
            .collect::<Vec<_>>(),
        opportunity_candidates
            .iter()
            .map(|candidate| candidate.def_id)
            .collect::<Vec<_>>()
    );
    assert_eq!(default_pruning, opportunity_pruning);
}

#[test]
fn prune_travel_allows_high_salience_opportunity_detour() {
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
    let mut semantics_table = BTreeMap::new();
    semantics_table.insert(travel_east_id, travel_semantics());
    semantics_table.insert(travel_south_id, travel_semantics());
    let mut candidates = vec![
        make_travel_candidate(travel_east_id, east),
        make_travel_candidate(travel_south_id, south),
    ];

    let opportunity_index = opportunity_index_for_place(south, [540]);
    let pruning = super::prune_travel_away_from_goal_with_expansion_trace(
        &mut candidates,
        None,
        hub,
        &[goal_store],
        &snapshot,
        &semantics_table,
        Permille::new(150).unwrap(),
        &opportunity_index,
    )
    .expect("travel pruning trace should be emitted");

    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.def_id)
            .collect::<Vec<_>>(),
        vec![travel_east_id, travel_south_id],
        "south detour cost increase is 4, and 540 * 150 >= 4 * 1000"
    );
    assert_eq!(pruning.retained.len(), 2);
    assert!(pruning.pruned.is_empty());
    let south_successor = pruning
        .retained
        .iter()
        .find(|entry| entry.destination == south)
        .expect("south detour should be retained");
    assert_eq!(
        south_successor.attribution,
        crate::decision_trace::TravelPruningAttribution::OpportunityDetour {
            salience_permille: 540,
            detour_budget_permille: Permille::new(150).unwrap(),
            cost_increase: 4,
            cost_threshold: 4000,
        }
    );
    assert!(
        pruning
            .retained
            .iter()
            .any(|entry| entry.destination == south)
    );
}

#[test]
fn prune_travel_prunes_low_salience_opportunity_detour() {
    let (view, actor, hub, east, south, _north, goal_store) = build_hub_pruning_view();
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::new(),
        &BTreeSet::from([hub, east, south, goal_store]),
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

    let opportunity_index = opportunity_index_for_place(south, [20]);
    let pruning = super::prune_travel_away_from_goal_with_expansion_trace(
        &mut candidates,
        None,
        hub,
        &[goal_store],
        &snapshot,
        &semantics_table,
        Permille::new(150).unwrap(),
        &opportunity_index,
    )
    .expect("travel pruning trace should be emitted");

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].def_id, travel_east_id);
    assert_eq!(pruning.pruned.len(), 1);
    assert_eq!(pruning.pruned[0].destination, south);
    assert_eq!(
        pruning.pruned[0].attribution,
        crate::decision_trace::TravelPruningAttribution::PrunedAsAwayFromGoal
    );
}

#[test]
fn prune_travel_uses_per_agent_detour_budget() {
    let (view, actor, hub, east, south, _north, goal_store) = build_hub_pruning_view();
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::new(),
        &BTreeSet::from([hub, east, south, goal_store]),
        5,
    );
    let travel_east_id = ActionDefId(100);
    let travel_south_id = ActionDefId(101);
    let mut semantics_table = BTreeMap::new();
    semantics_table.insert(travel_east_id, travel_semantics());
    semantics_table.insert(travel_south_id, travel_semantics());
    let opportunity_index = opportunity_index_for_place(south, [20]);

    let mut conservative_candidates = vec![
        make_travel_candidate(travel_east_id, east),
        make_travel_candidate(travel_south_id, south),
    ];
    super::prune_travel_away_from_goal_with_expansion_trace(
        &mut conservative_candidates,
        None,
        hub,
        &[goal_store],
        &snapshot,
        &semantics_table,
        Permille::new(150).unwrap(),
        &opportunity_index,
    );

    let mut permissive_candidates = vec![
        make_travel_candidate(travel_east_id, east),
        make_travel_candidate(travel_south_id, south),
    ];
    super::prune_travel_away_from_goal_with_expansion_trace(
        &mut permissive_candidates,
        None,
        hub,
        &[goal_store],
        &snapshot,
        &semantics_table,
        Permille::new(300).unwrap(),
        &opportunity_index,
    );

    assert_eq!(conservative_candidates.len(), 1);
    assert_eq!(permissive_candidates.len(), 2);
}

#[test]
fn prune_travel_opportunity_salience_sum_is_deterministic() {
    let (view, actor, hub, east, south, _north, goal_store) = build_hub_pruning_view();
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::new(),
        &BTreeSet::from([hub, east, south, goal_store]),
        5,
    );
    let travel_east_id = ActionDefId(100);
    let travel_south_id = ActionDefId(101);
    let mut semantics_table = BTreeMap::new();
    semantics_table.insert(travel_east_id, travel_semantics());
    semantics_table.insert(travel_south_id, travel_semantics());

    let mut forward_candidates = vec![
        make_travel_candidate(travel_east_id, east),
        make_travel_candidate(travel_south_id, south),
    ];
    let forward_index = opportunity_index_for_place(south, [10, 11]);
    let forward_pruning = super::prune_travel_away_from_goal_with_expansion_trace(
        &mut forward_candidates,
        None,
        hub,
        &[goal_store],
        &snapshot,
        &semantics_table,
        Permille::new(200).unwrap(),
        &forward_index,
    );

    let mut reverse_candidates = vec![
        make_travel_candidate(travel_east_id, east),
        make_travel_candidate(travel_south_id, south),
    ];
    let reverse_index = opportunity_index_for_place(south, [11, 10]);
    let reverse_pruning = super::prune_travel_away_from_goal_with_expansion_trace(
        &mut reverse_candidates,
        None,
        hub,
        &[goal_store],
        &snapshot,
        &semantics_table,
        Permille::new(200).unwrap(),
        &reverse_index,
    );

    assert_eq!(
        forward_candidates
            .iter()
            .map(|candidate| candidate.def_id)
            .collect::<Vec<_>>(),
        reverse_candidates
            .iter()
            .map(|candidate| candidate.def_id)
            .collect::<Vec<_>>()
    );
    assert_eq!(forward_pruning, reverse_pruning);
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
fn travel_candidate_cap_keeps_lowest_cost_local_travel_when_goal_places_unknown() {
    let (view, actor, hub, east, south, north, _goal_store) = build_hub_pruning_view();
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::new(),
        &BTreeSet::from([hub, east, south, north]),
        5,
    );
    let state = PlanningState::new(&snapshot);

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

    let mut expansion_candidates = None;
    let mut root_candidates = None;
    super::cap_travel_candidates(
        &mut candidates,
        &snapshot,
        &state,
        &[],
        &semantics_table,
        Some(2),
        &mut expansion_candidates,
        &mut root_candidates,
    );

    assert_eq!(
        candidates.len(),
        2,
        "travel cap should reduce retained travel branches"
    );
    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.authoritative_targets[0])
            .collect::<Vec<_>>(),
        vec![south, north],
        "with no goal-place guidance, the cap should keep the cheapest direct local travel branches"
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
            synthetic_cargo: PlannerSyntheticCargo::None,
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::TreatWounds { patient }),
        evidence_entities: BTreeSet::from([patient]),
        evidence_places: BTreeSet::from([patient_place, medicine_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };

    let places = combined_relevant_places(
        &goal,
        &state,
        &RecipeRegistry::new(),
        &ProfileFixture::default(),
    );

    assert!(places.places.contains(&patient_place));
    assert!(places.places.contains(&medicine_place));
    assert_eq!(places.places.len(), 2);
    assert_eq!(places.prerequisite_places_count, 1);
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::TreatWounds { patient }),
        evidence_entities: BTreeSet::from([patient]),
        evidence_places: BTreeSet::from([patient_place, medicine_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let node = SearchNode {
        state: PlanningState::new(&snapshot).move_actor_to(medicine_place),
        steps: SharedVec::new(),
        total_estimated_ticks: 0,
        search_cost: 0,
        tactical_barrier_reached: false,
        heuristic_ticks: 0,
    };

    let rel_defs = relevant_action_defs(&goal, &semantics);
    let pick_up = search_candidates(
        &goal,
        &node,
        candidate_search_context(
            &semantics,
            &registry,
            &handlers,
            &BlockerMemory::default(),
            Tick(0),
            &rel_defs,
        ),
        None,
        None,
        None,
    )
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
        &ProfileFixture::default(),
    )
    .expect("hypothetical pick_up should build a successor");

    let places = combined_relevant_places(
        &goal,
        &successor.state,
        &RecipeRegistry::new(),
        &ProfileFixture::default(),
    );

    assert_eq!(places.places, vec![patient_place]);
    assert_eq!(places.prerequisite_places_count, 0);
}

#[test]
fn combined_places_include_grounded_evidence_place_when_goal_has_no_intrinsic_place() {
    let actor = entity(1);
    let subject = entity(2);
    let village_square = entity(10);
    let orchard_farm = entity(11);
    let mut view = TestBeliefView::default();
    view.alive
        .extend([actor, subject, village_square, orchard_farm]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(subject, EntityKind::Agent);
    view.kinds.insert(village_square, EntityKind::Place);
    view.kinds.insert(orchard_farm, EntityKind::Place);
    view.effective_places.insert(actor, village_square);
    view.entities_at.insert(village_square, vec![actor]);
    view.entities_at.insert(orchard_farm, vec![subject]);
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::SearchForMissing {
            subject,
            last_seen: None,
        }),
        evidence_entities: BTreeSet::from([subject]),
        evidence_places: BTreeSet::from([orchard_farm]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );
    let state = PlanningState::new(&snapshot);

    let places = combined_relevant_places(
        &goal,
        &state,
        &RecipeRegistry::new(),
        &ProfileFixture::default(),
    );

    assert_eq!(places.places, vec![orchard_farm]);
    assert_eq!(places.prerequisite_places_count, 0);
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::TreatWounds { patient }),
        evidence_entities: BTreeSet::from([patient]),
        evidence_places: BTreeSet::from([patient_place, medicine_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let goal_places = combined_relevant_places(
        &goal,
        &state,
        &RecipeRegistry::new(),
        &ProfileFixture::default(),
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
        &goal_places.places,
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::TreatWounds { patient }),
        evidence_entities: BTreeSet::from([patient]),
        evidence_places: BTreeSet::from([patient_place, medicine_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let node = root_node(
        &snapshot,
        &goal,
        &RecipeRegistry::new(),
        &ProfileFixture::default(),
    );

    let rel_defs = relevant_action_defs(&goal, &semantics);
    let candidates = search_candidates(
        &goal,
        &node,
        candidate_search_context(
            &semantics,
            &registry,
            &handlers,
            &BlockerMemory::default(),
            Tick(0),
            &rel_defs,
        ),
        None,
        None,
        None,
    );

    assert!(
        candidates.iter().any(|candidate| {
            registry
                .get(candidate.def_id)
                .is_some_and(|def| def.name == "pick_up")
        }),
        "TreatWounds should consider pick_up when remote medicine is co-located"
    );
}

#[test]
fn steal_goal_surfaces_search_candidates_after_action_lands() {
    let actor = entity(1);
    let owner = entity(2);
    let target_item = entity(3);
    let town = entity(10);

    let mut view = TestBeliefView::default();
    view.alive.extend([actor, owner, town]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(owner, EntityKind::Agent);
    view.kinds.insert(target_item, EntityKind::ItemLot);
    view.kinds.insert(town, EntityKind::Place);
    view.effective_places.insert(actor, town);
    view.effective_places.insert(owner, town);
    view.effective_places.insert(target_item, town);
    view.entities_at
        .insert(town, vec![actor, owner, target_item]);
    view.owners.insert(target_item, owner);
    view.controllable.insert((actor, actor));
    view.lot_commodities
        .insert(target_item, CommodityKind::Bread);
    view.commodity_quantities
        .insert((target_item, CommodityKind::Bread), Quantity(1));
    view.carry_capacities.insert(actor, LoadUnits(10));
    view.entity_loads.insert(actor, LoadUnits(0));
    view.entity_loads.insert(target_item, LoadUnits(1));
    view.theft_profiles.insert(
        actor,
        TheftDispositionProfile {
            steal_duration_ticks: NonZeroU32::new(2).unwrap(),
            theft_motive_weight: pm(500),
            witness_risk_penalty: pm(100),
        },
    );

    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([owner, target_item]),
        &BTreeSet::from([town]),
        1,
    );
    let (registry, handlers) = build_registry();
    let semantics = build_semantics_table(&registry);
    let recipes = RecipeRegistry::new();
    let budget = ProfileFixture::default();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::StealItem { target_item }),
        evidence_entities: BTreeSet::from([target_item]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let node = root_node(&snapshot, &goal, &recipes, &budget);
    let rel_defs = relevant_action_defs(&goal, &semantics);
    assert!(
        rel_defs
            .iter()
            .any(|def_id| registry.get(*def_id).is_some_and(|def| def.name == "steal")),
        "StealItem should recognize the landed steal action as relevant"
    );

    let candidates = search_candidates(
        &goal,
        &node,
        candidate_search_context(
            &semantics,
            &registry,
            &handlers,
            &BlockerMemory::default(),
            Tick(0),
            &rel_defs,
        ),
        None,
        None,
        None,
    );
    assert!(
        candidates.iter().any(|candidate| {
            registry
                .get(candidate.def_id)
                .is_some_and(|def| def.name == "steal")
                && candidate.authoritative_targets == vec![target_item]
        }),
        "StealItem should surface a steal candidate for the exact bound target"
    );

    let plan = search_plan(
        &snapshot,
        &goal,
        &semantics,
        &registry,
        &handlers,
        &budget,
        &recipes,
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    );
    assert!(
        matches!(plan, PlanSearchResult::Found(_)),
        "StealItem should produce a concrete plan once search candidates exist, got {plan:?}"
    );
}

#[test]
fn steal_goal_plans_for_contained_displayed_sale_lot_without_owner_belief() {
    let actor = entity(1);
    let seller = entity(2);
    let target_item = entity(3);
    let display_container = entity(4);
    let town = entity(10);

    let mut view = TestBeliefView::default();
    view.alive.extend([actor, seller, town]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(seller, EntityKind::Agent);
    view.kinds.insert(target_item, EntityKind::ItemLot);
    view.kinds.insert(display_container, EntityKind::Container);
    view.kinds.insert(town, EntityKind::Place);
    view.effective_places.insert(actor, town);
    view.effective_places.insert(seller, town);
    view.effective_places.insert(target_item, town);
    view.effective_places.insert(display_container, town);
    view.entities_at
        .insert(town, vec![actor, seller, target_item, display_container]);
    view.direct_containers
        .insert(target_item, display_container);
    view.lot_sellers.insert(target_item, seller);
    view.listed_lots
        .insert((town, CommodityKind::Bread), vec![target_item]);
    view.controllable.insert((actor, actor));
    view.lot_commodities
        .insert(target_item, CommodityKind::Bread);
    view.commodity_quantities
        .insert((target_item, CommodityKind::Bread), Quantity(1));
    view.carry_capacities.insert(actor, LoadUnits(10));
    view.entity_loads.insert(actor, LoadUnits(0));
    view.entity_loads.insert(target_item, LoadUnits(1));
    view.theft_profiles.insert(
        actor,
        TheftDispositionProfile {
            steal_duration_ticks: NonZeroU32::new(2).unwrap(),
            theft_motive_weight: pm(500),
            witness_risk_penalty: pm(100),
        },
    );

    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([target_item]),
        &BTreeSet::from([town]),
        1,
    );
    let (registry, handlers) = build_registry();
    let semantics = build_semantics_table(&registry);
    let recipes = RecipeRegistry::new();
    let budget = ProfileFixture::default();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::StealItem { target_item }),
        evidence_entities: BTreeSet::from([target_item]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };

    let plan = search_plan(
        &snapshot,
        &goal,
        &semantics,
        &registry,
        &handlers,
        &budget,
        &recipes,
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    );
    assert!(
        matches!(plan, PlanSearchResult::Found(_)),
        "seller-backed displayed lots should remain steal-plannable even when the lot is container-backed, got {plan:?}"
    );
}

#[test]
fn accuse_goal_exposes_accuse_action_while_punish_remains_deferred() {
    let actor = entity(1);
    let accused = entity(2);
    let punish_accused = entity(3);
    let town = entity(10);
    let outpost = entity(11);
    let crime_register = entity(12);
    let faction = entity(20);
    let accusation_entry = worldwake_core::RecordEntryId(1);
    let claim = worldwake_core::InstitutionalClaim::Accusation {
        accuser: actor,
        accused,
        violation_id: worldwake_core::ViolationId(1),
        theft: worldwake_core::TheftFacts {
            missing_entity: entity(30),
            expected_place: town,
            commodity: CommodityKind::Bread,
            quantity: Quantity(1),
        },
        effective_tick: Tick(1),
    };

    let mut view = TestBeliefView::default();
    view.alive.extend([
        actor,
        accused,
        punish_accused,
        town,
        outpost,
        crime_register,
        faction,
    ]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(accused, EntityKind::Agent);
    view.kinds.insert(punish_accused, EntityKind::Agent);
    view.kinds.insert(town, EntityKind::Place);
    view.kinds.insert(outpost, EntityKind::Place);
    view.kinds.insert(crime_register, EntityKind::Record);
    view.kinds.insert(faction, EntityKind::Faction);
    view.effective_places.insert(actor, town);
    view.effective_places.insert(accused, outpost);
    view.effective_places.insert(punish_accused, town);
    view.effective_places.insert(crime_register, town);
    view.entities_at
        .insert(town, vec![actor, punish_accused, crime_register]);
    view.entities_at.insert(outpost, vec![accused]);
    view.record_data.insert(
        crime_register,
        worldwake_core::RecordData {
            record_kind: worldwake_core::RecordKind::CrimeRegister,
            home_place: town,
            issuer: actor,
            consultation_ticks: 1,
            max_entries_per_consult: 4,
            entries: vec![worldwake_core::InstitutionalRecordEntry {
                entry_id: accusation_entry,
                claim,
                recorded_tick: Tick(1),
                supersedes: None,
            }],
            next_entry_id: 2,
        },
    );

    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([accused, punish_accused, crime_register, faction]),
        &BTreeSet::from([town, outpost]),
        1,
    );
    let (registry, handlers) = build_registry();
    let semantics = build_semantics_table(&registry);
    let recipes = RecipeRegistry::new();
    let budget = ProfileFixture::default();
    let accuse_goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::Accuse {
            crime_register,
            accused,
            violation_id: worldwake_core::ViolationId(1),
        }),
        evidence_entities: BTreeSet::from([accused, crime_register]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let punish_goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::PunishAccused {
            office: faction,
            accused: punish_accused,
            accusation_entry: worldwake_core::RecordEntryId(1),
            punishment: worldwake_core::PunishmentKind::Exile {
                from_faction: faction,
            },
        }),
        evidence_entities: BTreeSet::from([punish_accused]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };

    let accuse_defs = relevant_action_defs(&accuse_goal, &semantics);
    assert!(
        accuse_defs.iter().any(|def_id| {
            registry
                .get(*def_id)
                .is_some_and(|def| def.name == "accuse")
        }),
        "Accuse goals should expose the accuse operator once the action exists"
    );

    let punish_defs = relevant_action_defs(&punish_goal, &semantics);
    assert!(
        punish_defs
            .iter()
            .any(|def_id| { registry.get(*def_id).is_some_and(|def| def.name == "exile") }),
        "PunishAccused goals should expose the exile operator once the action exists"
    );

    let node = root_node(&snapshot, &accuse_goal, &recipes, &budget);
    let candidates = search_candidates(
        &accuse_goal,
        &node,
        candidate_search_context(
            &semantics,
            &registry,
            &handlers,
            &BlockerMemory::default(),
            Tick(0),
            &accuse_defs,
        ),
        None,
        None,
        None,
    );
    assert!(
        candidates.iter().any(|candidate| {
            registry
                .get(candidate.def_id)
                .is_some_and(|def| def.name == "accuse")
                && candidate.authoritative_targets == vec![accused]
                && candidate.planning_targets == vec![PlanningEntityRef::Authoritative(town)]
        }),
        "Accuse goals should bind payloads to the accused while planning against the crime register home place"
    );

    let punish_node = root_node(&snapshot, &punish_goal, &recipes, &budget);
    let punish_candidates = search_candidates(
        &punish_goal,
        &punish_node,
        candidate_search_context(
            &semantics,
            &registry,
            &handlers,
            &BlockerMemory::default(),
            Tick(0),
            &punish_defs,
        ),
        None,
        None,
        None,
    );
    assert!(
        punish_candidates.iter().any(|candidate| {
            registry
                .get(candidate.def_id)
                .is_some_and(|def| def.name == "exile")
                && candidate.authoritative_targets == vec![punish_accused]
        }),
        "PunishAccused goals should surface the exact bound punishment candidate from goal identity once the action exists"
    );
}

#[test]
fn build_successor_keeps_accuse_step_target_bound_to_accused() {
    let actor = entity(1);
    let accused = entity(2);
    let town = entity(10);
    let outpost = entity(11);
    let crime_register = entity(12);

    let mut view = TestBeliefView::default();
    view.alive
        .extend([actor, accused, town, outpost, crime_register]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(accused, EntityKind::Agent);
    view.kinds.insert(town, EntityKind::Place);
    view.kinds.insert(outpost, EntityKind::Place);
    view.kinds.insert(crime_register, EntityKind::Record);
    view.effective_places.insert(actor, town);
    view.effective_places.insert(accused, outpost);
    view.effective_places.insert(crime_register, town);
    view.entities_at.insert(town, vec![actor, crime_register]);
    view.entities_at.insert(outpost, vec![accused]);
    view.record_data.insert(
        crime_register,
        worldwake_core::RecordData {
            record_kind: worldwake_core::RecordKind::CrimeRegister,
            home_place: town,
            issuer: actor,
            consultation_ticks: 1,
            max_entries_per_consult: 4,
            entries: Vec::new(),
            next_entry_id: 1,
        },
    );

    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([accused, crime_register]),
        &BTreeSet::from([town, outpost]),
        1,
    );
    let (registry, _handlers) = build_registry();
    let semantics = build_semantics_table(&registry);
    let recipes = RecipeRegistry::new();
    let reasoning = ProfileFixture::default();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Entity(accused),
        key: GoalKey::from(GoalKind::Accuse {
            crime_register,
            accused,
            violation_id: worldwake_core::ViolationId(1),
        }),
        evidence_entities: BTreeSet::from([accused, crime_register]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let node = root_node(&snapshot, &goal, &recipes, &reasoning);
    let accuse_def = registry
        .iter()
        .find(|def| def.name == "accuse")
        .expect("accuse action should be registered");
    let candidate = SearchCandidate {
        def_id: accuse_def.id,
        authoritative_targets: vec![accused],
        planning_targets: vec![PlanningEntityRef::Authoritative(town)],
        payload_override: None,
        planner_only: false,
        trace_index: None,
        expansion_trace_index: None,
    };

    let (terminal, successor) = build_successor(
        &goal, &semantics, &registry, &node, &candidate, &recipes, &reasoning,
    )
    .expect("accuse successor should build");

    assert_eq!(terminal, Some(PlanTerminalKind::ProgressBarrier));
    assert_eq!(
        successor.steps.as_slice()[0].targets,
        vec![PlanningEntityRef::Authoritative(accused)],
        "Accuse should preserve the accused as the executable step target even when search routes via the register place"
    );
}

#[test]
fn fulfill_bounty_goal_surfaces_exact_bound_claim_candidate() {
    let actor = entity(1);
    let bounty = entity(2);
    let issuer = entity(3);
    let target = entity(4);
    let claim_place = entity(10);

    let mut view = TestBeliefView::default();
    view.alive.insert(actor);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(bounty, EntityKind::SocialArtifact);
    view.kinds.insert(target, EntityKind::Agent);
    view.effective_places.insert(actor, claim_place);
    view.effective_places.insert(bounty, claim_place);
    view.entities_at.insert(claim_place, vec![actor, bounty]);
    view.known_entity_beliefs.insert(
        actor,
        vec![(
            bounty,
            BelievedEntityState {
                believed_kind: None,
                last_known_place: Some(claim_place),
                last_known_inventory: BTreeMap::new(),
                workstation_tag: None,
                resource_source: None,
                alive: true,
                wounds: Vec::new(),
                last_known_courage: None,
                believed_activity: None,
                believed_artifact: Some(active_believed_bounty_artifact(
                    issuer,
                    claim_place,
                    BountyTarget::EliminateEntity { target },
                    Quantity(25),
                    Tick(1),
                )),
                believed_contention: None,
                believed_evidence: None,
                ..BelievedEntityState::single_observation_defaults(
                    Tick(1),
                    PerceptionSource::DirectObservation,
                )
            },
        )],
    );

    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([bounty, target]),
        &BTreeSet::from([claim_place]),
        1,
    );
    let (registry, handlers) = build_registry();
    let semantics = build_semantics_table(&registry);
    let recipes = RecipeRegistry::new();
    let budget = ProfileFixture::default();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Entity(bounty),
        key: GoalKey::from(GoalKind::FulfillBounty { bounty }),
        evidence_entities: BTreeSet::from([bounty]),
        evidence_places: BTreeSet::from([claim_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };

    let rel_defs = relevant_action_defs(&goal, &semantics);
    assert!(
        rel_defs.iter().any(|def_id| {
            registry
                .get(*def_id)
                .is_some_and(|def| def.name == "claim_bounty")
        }),
        "FulfillBounty goals should expose claim_bounty once the action is classified"
    );

    let node = root_node(&snapshot, &goal, &recipes, &budget);
    let candidates = search_candidates(
        &goal,
        &node,
        candidate_search_context(
            &semantics,
            &registry,
            &handlers,
            &BlockerMemory::default(),
            Tick(0),
            &rel_defs,
        ),
        None,
        None,
        None,
    );
    assert!(
        candidates.iter().any(|candidate| {
            registry
                .get(candidate.def_id)
                .is_some_and(|def| def.name == "claim_bounty")
                && candidate.authoritative_targets == vec![bounty]
        }),
        "FulfillBounty should synthesize the exact bound claim_bounty root candidate"
    );
}

#[test]
fn fulfill_bounty_delivery_search_finds_delivery_then_claim_plan() {
    let actor = entity(1);
    let bounty = entity(2);
    let issuer = entity(3);
    let origin = entity(10);
    let destination = entity(11);
    let claim_place = entity(12);
    let bread = entity(20);

    let mut view = TestBeliefView::default();
    view.alive
        .extend([actor, bounty, origin, destination, claim_place, bread]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(bounty, EntityKind::SocialArtifact);
    view.kinds.insert(origin, EntityKind::Place);
    view.kinds.insert(destination, EntityKind::Place);
    view.kinds.insert(claim_place, EntityKind::Place);
    view.kinds.insert(bread, EntityKind::ItemLot);
    view.effective_places.insert(actor, origin);
    view.effective_places.insert(bounty, origin);
    view.effective_places.insert(bread, origin);
    view.entities_at.insert(origin, vec![actor, bread]);
    view.entities_at.insert(destination, vec![bounty]);
    view.entities_at.insert(claim_place, Vec::new());
    view.adjacent
        .insert(origin, vec![(destination, NonZeroU32::new(2).unwrap())]);
    view.adjacent.insert(
        destination,
        vec![
            (origin, NonZeroU32::new(2).unwrap()),
            (claim_place, NonZeroU32::new(1).unwrap()),
        ],
    );
    view.adjacent.insert(
        claim_place,
        vec![(destination, NonZeroU32::new(1).unwrap())],
    );
    view.lot_commodities.insert(bread, CommodityKind::Bread);
    view.commodity_quantities
        .insert((bread, CommodityKind::Bread), Quantity(3));
    view.controllable.insert((actor, bread));
    view.carry_capacities.insert(actor, LoadUnits(4));
    view.entity_loads.insert(actor, LoadUnits(0));
    view.entity_loads.insert(bread, LoadUnits(3));
    view.known_entity_beliefs.insert(
        actor,
        vec![
            (
                bounty,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(destination),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: Some(active_believed_bounty_artifact(
                        issuer,
                        claim_place,
                        BountyTarget::DeliverCommodity {
                            commodity: CommodityKind::Bread,
                            quantity: Quantity(3),
                            destination,
                        },
                        Quantity(25),
                        Tick(1),
                    )),
                    believed_contention: None,
                    believed_evidence: None,
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(1),
                        PerceptionSource::DirectObservation,
                    )
                },
            ),
            (
                bread,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(origin),
                    last_known_inventory: BTreeMap::from([(CommodityKind::Bread, Quantity(3))]),
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
                        PerceptionSource::DirectObservation,
                    )
                },
            ),
        ],
    );

    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([bounty, bread]),
        &BTreeSet::from([origin, destination, claim_place]),
        1,
    );
    let (registry, handlers) = build_registry();
    let plan = search_plan(
        &snapshot,
        &GoalOffer {
            anchor: worldwake_core::OpportunityAnchor::Entity(bounty),
            key: GoalKey::from(GoalKind::FulfillBounty { bounty }),
            evidence_entities: BTreeSet::from([bounty, bread]),
            evidence_places: BTreeSet::from([origin, destination, claim_place]),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        },
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    )
    .into_plan()
    .unwrap();

    assert!(
        plan.steps
            .iter()
            .any(|step| step.op_kind == PlannerOpKind::MoveCargo),
        "delivery bounty plan should use cargo movement"
    );
    assert!(
        plan.steps
            .iter()
            .any(|step| step.op_kind == PlannerOpKind::Travel),
        "delivery bounty plan should use travel"
    );
    assert!(
        plan.steps
            .iter()
            .filter(|step| step.op_kind == PlannerOpKind::Travel)
            .count()
            >= 2,
        "delivery bounty plan should travel once to deliver and again to reach the claim place"
    );
    assert!(
        plan.steps.iter().any(|step| {
            step.op_kind == PlannerOpKind::Travel
                && step.targets
                    == vec![crate::planning_state::PlanningEntityRef::Authoritative(
                        claim_place,
                    )]
        }),
        "delivery bounty plan should include travel to the distinct claim place before claim"
    );
    assert_eq!(
        plan.steps.last().map(|step| step.op_kind),
        Some(PlannerOpKind::ClaimBounty)
    );
}

#[test]
fn fulfill_bounty_elimination_does_not_surface_claim_candidate_before_target_death() {
    let actor = entity(1);
    let bounty = entity(2);
    let issuer = entity(3);
    let claim_place = entity(10);
    let target = entity(11);

    let mut view = TestBeliefView::default();
    view.alive.extend([actor, issuer, target, claim_place]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(issuer, EntityKind::Agent);
    view.kinds.insert(target, EntityKind::Agent);
    view.kinds.insert(bounty, EntityKind::SocialArtifact);
    view.kinds.insert(claim_place, EntityKind::Place);
    view.effective_places.insert(actor, claim_place);
    view.effective_places.insert(issuer, claim_place);
    view.effective_places.insert(target, claim_place);
    view.effective_places.insert(bounty, claim_place);
    view.entities_at
        .insert(claim_place, vec![actor, issuer, target, bounty]);
    view.known_entity_beliefs.insert(
        actor,
        vec![(
            bounty,
            BelievedEntityState {
                believed_kind: None,
                last_known_place: Some(claim_place),
                last_known_inventory: BTreeMap::new(),
                workstation_tag: None,
                resource_source: None,
                alive: true,
                wounds: Vec::new(),
                last_known_courage: None,
                believed_activity: None,
                believed_artifact: Some(active_believed_bounty_artifact(
                    issuer,
                    claim_place,
                    BountyTarget::EliminateEntity { target },
                    Quantity(25),
                    Tick(1),
                )),
                believed_contention: None,
                believed_evidence: None,
                ..BelievedEntityState::single_observation_defaults(
                    Tick(1),
                    PerceptionSource::DirectObservation,
                )
            },
        )],
    );

    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([bounty, target, issuer]),
        &BTreeSet::from([claim_place]),
        1,
    );
    let (registry, handlers) = build_registry();
    let semantics = build_semantics_table(&registry);
    let recipes = RecipeRegistry::new();
    let budget = ProfileFixture::default();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Entity(bounty),
        key: GoalKey::from(GoalKind::FulfillBounty { bounty }),
        evidence_entities: BTreeSet::from([bounty, target]),
        evidence_places: BTreeSet::from([claim_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };

    let node = root_node(&snapshot, &goal, &recipes, &budget);
    let rel_defs = relevant_action_defs(&goal, &semantics);
    let candidates = search_candidates(
        &goal,
        &node,
        candidate_search_context(
            &semantics,
            &registry,
            &handlers,
            &BlockerMemory::default(),
            Tick(0),
            &rel_defs,
        ),
        None,
        None,
        None,
    );

    assert!(
        !candidates.iter().any(|candidate| {
            registry
                .get(candidate.def_id)
                .is_some_and(|def| def.name == "claim_bounty")
        }),
        "elimination bounty should not surface claim_bounty before the target is dead"
    );
}

#[test]
fn fulfill_bounty_delivery_does_not_surface_claim_candidate_before_delivery_gap_closes() {
    let actor = entity(1);
    let bounty = entity(2);
    let issuer = entity(3);
    let origin = entity(10);
    let destination = entity(11);
    let bread = entity(20);

    let mut view = TestBeliefView::default();
    view.alive
        .extend([actor, bounty, issuer, origin, destination, bread]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(bounty, EntityKind::SocialArtifact);
    view.kinds.insert(issuer, EntityKind::Agent);
    view.kinds.insert(origin, EntityKind::Place);
    view.kinds.insert(destination, EntityKind::Place);
    view.kinds.insert(bread, EntityKind::ItemLot);
    view.effective_places.insert(actor, origin);
    view.effective_places.insert(bounty, destination);
    view.effective_places.insert(issuer, destination);
    view.effective_places.insert(bread, origin);
    view.entities_at.insert(origin, vec![actor, bread]);
    view.entities_at.insert(destination, vec![bounty, issuer]);
    view.carry_capacities.insert(actor, LoadUnits(6));
    view.entity_loads.insert(actor, LoadUnits(0));
    view.lot_commodities.insert(bread, CommodityKind::Bread);
    view.commodity_quantities
        .insert((bread, CommodityKind::Bread), Quantity(3));
    view.known_entity_beliefs.insert(
        actor,
        vec![
            (
                bounty,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(destination),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: Some(active_believed_bounty_artifact(
                        issuer,
                        destination,
                        BountyTarget::DeliverCommodity {
                            commodity: CommodityKind::Bread,
                            quantity: Quantity(3),
                            destination,
                        },
                        Quantity(25),
                        Tick(1),
                    )),
                    believed_contention: None,
                    believed_evidence: None,
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(1),
                        PerceptionSource::DirectObservation,
                    )
                },
            ),
            (
                bread,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(origin),
                    last_known_inventory: BTreeMap::from([(CommodityKind::Bread, Quantity(3))]),
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
                        PerceptionSource::DirectObservation,
                    )
                },
            ),
        ],
    );

    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([bounty, bread]),
        &BTreeSet::from([origin, destination]),
        1,
    );
    let (registry, handlers) = build_registry();
    let semantics = build_semantics_table(&registry);
    let recipes = RecipeRegistry::new();
    let budget = ProfileFixture::default();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Entity(bounty),
        key: GoalKey::from(GoalKind::FulfillBounty { bounty }),
        evidence_entities: BTreeSet::from([bounty, bread]),
        evidence_places: BTreeSet::from([origin, destination]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };

    let node = root_node(&snapshot, &goal, &recipes, &budget);
    let rel_defs = relevant_action_defs(&goal, &semantics);
    let candidates = search_candidates(
        &goal,
        &node,
        candidate_search_context(
            &semantics,
            &registry,
            &handlers,
            &BlockerMemory::default(),
            Tick(0),
            &rel_defs,
        ),
        None,
        None,
        None,
    );

    assert!(
        !candidates.iter().any(|candidate| {
            registry
                .get(candidate.def_id)
                .is_some_and(|def| def.name == "claim_bounty")
        }),
        "delivery bounty should not surface claim_bounty before the delivery gap is closed"
    );
}

#[test]
fn fulfill_bounty_delivery_does_not_surface_claim_candidate_before_reaching_claim_place() {
    let actor = entity(1);
    let bounty = entity(2);
    let issuer = entity(3);
    let destination = entity(11);
    let claim_place = entity(12);
    let bread = entity(20);

    let mut view = TestBeliefView::default();
    view.alive
        .extend([actor, bounty, issuer, destination, claim_place, bread]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(bounty, EntityKind::SocialArtifact);
    view.kinds.insert(issuer, EntityKind::Agent);
    view.kinds.insert(destination, EntityKind::Place);
    view.kinds.insert(claim_place, EntityKind::Place);
    view.kinds.insert(bread, EntityKind::ItemLot);
    view.effective_places.insert(actor, destination);
    view.effective_places.insert(bounty, claim_place);
    view.effective_places.insert(issuer, claim_place);
    view.effective_places.insert(bread, destination);
    view.entities_at.insert(destination, vec![actor, bread]);
    view.entities_at.insert(claim_place, vec![bounty, issuer]);
    view.controllable.insert((actor, bread));
    view.carry_capacities.insert(actor, LoadUnits(6));
    view.entity_loads.insert(actor, LoadUnits(0));
    view.lot_commodities.insert(bread, CommodityKind::Bread);
    view.commodity_quantities
        .insert((bread, CommodityKind::Bread), Quantity(3));
    view.known_entity_beliefs.insert(
        actor,
        vec![
            (
                bounty,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(claim_place),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: Some(active_believed_bounty_artifact(
                        issuer,
                        claim_place,
                        BountyTarget::DeliverCommodity {
                            commodity: CommodityKind::Bread,
                            quantity: Quantity(3),
                            destination,
                        },
                        Quantity(25),
                        Tick(1),
                    )),
                    believed_contention: None,
                    believed_evidence: None,
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(1),
                        PerceptionSource::DirectObservation,
                    )
                },
            ),
            (
                bread,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(destination),
                    last_known_inventory: BTreeMap::from([(CommodityKind::Bread, Quantity(3))]),
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
                        PerceptionSource::DirectObservation,
                    )
                },
            ),
        ],
    );

    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([bounty, bread]),
        &BTreeSet::from([destination, claim_place]),
        1,
    );
    let (registry, handlers) = build_registry();
    let semantics = build_semantics_table(&registry);
    let recipes = RecipeRegistry::new();
    let budget = ProfileFixture::default();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Entity(bounty),
        key: GoalKey::from(GoalKind::FulfillBounty { bounty }),
        evidence_entities: BTreeSet::from([bounty, bread]),
        evidence_places: BTreeSet::from([destination, claim_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };

    let node = root_node(&snapshot, &goal, &recipes, &budget);
    let rel_defs = relevant_action_defs(&goal, &semantics);
    let candidates = search_candidates(
        &goal,
        &node,
        candidate_search_context(
            &semantics,
            &registry,
            &handlers,
            &BlockerMemory::default(),
            Tick(0),
            &rel_defs,
        ),
        None,
        None,
        None,
    );

    assert!(
        !candidates.iter().any(|candidate| {
            registry
                .get(candidate.def_id)
                .is_some_and(|def| def.name == "claim_bounty")
        }),
        "delivery bounty should not surface claim_bounty until the actor reaches claim_place"
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::LootCorpse { corpse: corpse_x }),
        evidence_entities: BTreeSet::from([corpse_x, corpse_y]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::EngageHostile { target: hostile_a }),
        evidence_entities: BTreeSet::from([hostile_a, hostile_b]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
        HomeostaticNeeds::new(pm(0), pm(0), pm(300), pm(0), pm(0)),
    );
    view.thresholds.insert(actor, DriveThresholds::default());

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::Sleep),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        Some(&mut rejections),
        None,
    );

    let plan = result
        .into_plan()
        .expect("search should find a low-band sleep plan");
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::LootCorpse { corpse: corpse_x }),
        evidence_entities: BTreeSet::from([corpse_x, corpse_y]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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

/// For `AcquireCommodity`, if a current-place acquire opportunity already
/// exists, search must not be trapped behind a manufactured local prerequisite
/// stage. The tactical layer should remain free to explore a farther
/// goal-satisfied branch.
#[test]
fn search_local_acquire_goal_remains_direct_without_prerequisite_stage() {
    let actor = entity(1);
    let seller = entity(2);
    let town = entity(10);
    let market = entity(11);
    let bread = entity(20);
    let seller_lot = entity(21);

    let mut view = TestBeliefView::default();
    view.alive
        .extend([actor, seller, town, market, bread, seller_lot]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(seller, EntityKind::Agent);
    view.kinds.insert(town, EntityKind::Place);
    view.kinds.insert(market, EntityKind::Place);
    view.kinds.insert(bread, EntityKind::ItemLot);
    view.kinds.insert(seller_lot, EntityKind::ItemLot);
    view.effective_places.insert(actor, town);
    view.effective_places.insert(seller, town);
    view.effective_places.insert(bread, market);
    view.effective_places.insert(seller_lot, town);
    view.entities_at
        .insert(town, vec![actor, seller, seller_lot]);
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
            home_facility: Some(town),
        },
    );
    view.lot_commodities
        .insert(seller_lot, CommodityKind::Bread);
    view.lot_sellers.insert(seller_lot, seller);
    view.direct_possessors.insert(seller_lot, seller);
    view.direct_possessions
        .entry(seller)
        .or_default()
        .push(seller_lot);
    view.commodity_quantities
        .insert((seller_lot, CommodityKind::Bread), Quantity(1));
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::from([seller, bread]),
        evidence_places: BTreeSet::from([town, market]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    )
    .into_plan()
    .expect("should find a plan");

    // The search is free to chase the remote market lot because the local
    // seller did not force an `AcquirePrerequisite` stage at the current
    // place.
    assert_eq!(
        plan.terminal_kind,
        PlanTerminalKind::GoalSatisfied,
        "a local acquire opportunity must not force prerequisite staging"
    );
    assert_eq!(plan.steps.len(), 2);
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
    let seller_lot = entity(100);

    let mut view = TestBeliefView::default();
    view.alive.extend([actor, seller, town, seller_lot]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(seller, EntityKind::Agent);
    view.kinds.insert(town, EntityKind::Place);
    view.kinds.insert(seller_lot, EntityKind::ItemLot);
    view.effective_places.insert(actor, town);
    view.effective_places.insert(seller, town);
    view.effective_places.insert(seller_lot, town);
    view.entities_at
        .insert(town, vec![actor, seller, seller_lot]);
    view.lot_commodities
        .insert(seller_lot, CommodityKind::Bread);
    view.lot_sellers.insert(seller_lot, seller);
    view.direct_possessors.insert(seller_lot, seller);
    view.direct_possessions
        .entry(seller)
        .or_default()
        .push(seller_lot);
    view.commodity_quantities
        .insert((actor, CommodityKind::Coin), Quantity(3));
    view.merchandise_profiles.insert(
        seller,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([CommodityKind::Bread]),
            home_facility: Some(town),
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::from([seller]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
    let seller_lot = entity(100);

    let mut view = TestBeliefView::default();
    view.alive.extend([actor, seller, town, market, seller_lot]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(seller, EntityKind::Agent);
    view.kinds.insert(town, EntityKind::Place);
    view.kinds.insert(market, EntityKind::Place);
    view.kinds.insert(seller_lot, EntityKind::ItemLot);
    view.effective_places.insert(actor, town);
    view.effective_places.insert(seller, town);
    view.effective_places.insert(seller_lot, town);
    view.entities_at
        .insert(town, vec![actor, seller, seller_lot]);
    view.entities_at.insert(market, vec![]);
    view.lot_commodities
        .insert(seller_lot, CommodityKind::Bread);
    view.lot_sellers.insert(seller_lot, seller);
    view.direct_possessors.insert(seller_lot, seller);
    view.direct_possessions
        .entry(seller)
        .or_default()
        .push(seller_lot);
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
            home_facility: Some(town),
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
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::from([seller]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
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
    let tight_budget = ProfileFixture {
        max_node_expansions: 2,
        ..ProfileFixture::default()
    };
    let result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &tight_budget,
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        Some(&mut summaries),
    );

    assert!(result.is_found(), "plan should be found");
    assert!(
        !summaries.is_empty(),
        "expansion summaries should be non-empty when tracing is enabled"
    );
    let first = &summaries[0];
    // Depth should start at 0.
    assert_eq!(first.depth, 0);
    assert_eq!(first.combined_places_count, 0);
    assert_eq!(first.prerequisite_places_count, 0);
    // At least one candidate was generated.
    assert!(first.candidates_generated > 0);
}

#[test]
fn search_expansion_summary_counts_prerequisite_places_for_remote_treat_wounds() {
    let (mut view, actor, patient, _current_place, patient_place, medicine_place) =
        build_branching_care_view();
    view.wounds.insert(patient, vec![wound(400)]);
    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::TreatWounds { patient }),
        evidence_entities: BTreeSet::from([patient]),
        evidence_places: BTreeSet::from([patient_place, medicine_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        2,
    );

    let mut summaries = Vec::new();
    let result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        Some(&mut summaries),
    );

    assert!(
        result.is_found(),
        "planner should find a remote medicine plan"
    );
    let first = summaries
        .first()
        .expect("tracing should record at least one expansion summary");
    assert_eq!(first.depth, 0);
    assert_eq!(first.combined_places_count, 2);
    assert_eq!(first.prerequisite_places_count, 1);
    let guidance = first
        .prerequisite_guidance
        .as_ref()
        .expect("root expansion should preserve prerequisite guidance members");
    assert_eq!(guidance.goal_relevant_places, vec![patient_place]);
    assert_eq!(guidance.prerequisite_places, vec![medicine_place]);
    assert!(guidance.exclusions.is_empty());
    assert!(
        first.travel_pruning.is_some(),
        "root expansion should record travel pruning context"
    );
}

#[test]
fn search_trace_metadata_records_two_phase_strategic_and_landmark_details() {
    let fixture = build_remote_produce_commodity_fixture();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::ProduceCommodity {
            recipe_id: fixture.recipe_id,
        }),
        evidence_entities: BTreeSet::from([fixture.mill, fixture.firewood]),
        evidence_places: BTreeSet::from([
            prototype_place_entity(PrototypePlace::VillageSquare),
            prototype_place_entity(PrototypePlace::OrchardFarm),
        ]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );

    let mut summaries = Vec::new();
    let mut trace_metadata = super::SearchTraceMetadata::default();
    let result = super::search_plan_with_trace_metadata(
        &snapshot,
        &goal,
        &fixture.semantics,
        &fixture.registry,
        &fixture.handlers,
        &cognitive(&ProfileFixture::default()),
        &execution_budget(&ProfileFixture::default()),
        &fixture.recipes,
        &BlockerMemory::default(),
        Tick(1),
        None,
        Some(&mut summaries),
        Some(&mut trace_metadata),
    );

    assert!(
        result.is_found(),
        "two-phase production plan should be found"
    );
    let strategic_plan = trace_metadata
        .strategic_plan
        .as_ref()
        .expect("remote production should record a strategic plan");
    assert_eq!(strategic_plan.steps.len(), 2);
    assert_eq!(
        strategic_plan.steps[0].destination,
        prototype_place_entity(PrototypePlace::OrchardFarm)
    );
    assert_eq!(
        trace_metadata.tactical_goal.as_deref(),
        Some(
            format!(
                "AcquirePrerequisite {{ commodity: Firewood, destination: {:?} }}",
                prototype_place_entity(PrototypePlace::OrchardFarm)
            )
            .as_str()
        )
    );
    assert!(
        trace_metadata.landmarks_extracted > 0,
        "two-phase production should extract landmarks"
    );
    assert_eq!(
        trace_metadata.landmark_orderings, 0,
        "this production fixture currently yields an unordered landmark set"
    );
    assert!(
        summaries
            .iter()
            .any(|summary| summary.preferred_candidates > 0),
        "at least one expansion should report preferred candidates"
    );
    assert!(
        summaries
            .iter()
            .any(|summary| summary.landmark_heuristic > 0),
        "at least one expansion should report a non-zero landmark heuristic"
    );
}

#[test]
fn search_trace_metadata_zero_landmarks_reports_zero_counts() {
    let fixture = build_remote_produce_commodity_fixture();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::ProduceCommodity {
            recipe_id: fixture.recipe_id,
        }),
        evidence_entities: BTreeSet::from([fixture.mill, fixture.firewood]),
        evidence_places: BTreeSet::from([
            prototype_place_entity(PrototypePlace::VillageSquare),
            prototype_place_entity(PrototypePlace::OrchardFarm),
        ]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );
    let mut no_landmarks = cognitive(&ProfileFixture::default());
    no_landmarks.landmark_extraction_depth = 0;

    let mut summaries = Vec::new();
    let mut trace_metadata = super::SearchTraceMetadata::default();
    let result = super::search_plan_with_trace_metadata(
        &snapshot,
        &goal,
        &fixture.semantics,
        &fixture.registry,
        &fixture.handlers,
        &no_landmarks,
        &execution_budget(&ProfileFixture::default()),
        &fixture.recipes,
        &BlockerMemory::default(),
        Tick(1),
        None,
        Some(&mut summaries),
        Some(&mut trace_metadata),
    );

    assert!(
        result.is_found(),
        "zero-landmark mode should still find a plan"
    );
    assert_eq!(
        trace_metadata.tactical_goal.as_deref(),
        Some(
            format!(
                "AcquirePrerequisite {{ commodity: Firewood, destination: {:?} }}",
                prototype_place_entity(PrototypePlace::OrchardFarm)
            )
            .as_str()
        )
    );
    assert_eq!(trace_metadata.landmarks_extracted, 0);
    assert_eq!(trace_metadata.landmark_orderings, 0);
    assert!(
        summaries
            .iter()
            .all(|summary| summary.preferred_candidates == 0 && summary.landmark_heuristic == 0),
        "zero-landmark mode should keep preferred-candidate and landmark-heuristic trace fields at zero"
    );
}

#[test]
fn ff_successor_rewrite_uses_relaxed_plan_when_it_exceeds_spatial_heuristic() {
    let (mut view, actor, patient, current_place, _patient_place) =
        build_local_medicine_remote_patient_view();
    view.wounds.insert(patient, vec![wound(404)]);
    let (registry, handlers) = build_registry();
    let recipes = RecipeRegistry::new();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::TreatWounds { patient }),
        evidence_entities: BTreeSet::from([patient]),
        evidence_places: BTreeSet::from([current_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        2,
    );
    let tactical_goal = TacticalGoal::AcquirePrerequisite {
        commodity: CommodityKind::Medicine,
        destination: current_place,
    };
    let (node, mut successors, successor_candidates, successor_operators, semantics_table) =
        collect_root_non_terminal_successors_for_tactical_goal(
            &snapshot,
            &goal,
            &registry,
            &handlers,
            &ProfileFixture::default(),
            &recipes,
            &tactical_goal,
        );
    let move_cargo_index = successors
        .iter()
        .position(|(_, _, successor, _)| {
            successor
                .steps
                .as_slice()
                .last()
                .is_some_and(|step| step.op_kind == PlannerOpKind::MoveCargo)
        })
        .expect("local prerequisite fixture should expose a root MoveCargo successor");
    let combined_places = super::heuristic::combined_relevant_places_for_tactical(
        &goal,
        &successors[move_cargo_index].2.state,
        &recipes,
        &execution_budget(&ProfileFixture::default()),
        Some(&tactical_goal),
    );
    let spatial_heuristic = compute_heuristic(
        &snapshot,
        &successors[move_cargo_index].2.state,
        &combined_places.places,
    );

    assert_eq!(spatial_heuristic, 0);

    let ff_result = super::apply_ff_heuristic_to_successors(
        &snapshot,
        &goal,
        &node,
        &cognitive(&ProfileFixture::default()),
        &recipes,
        &execution_budget(&ProfileFixture::default()),
        Some(&tactical_goal),
        &semantics_table,
        &successor_candidates,
        &successor_operators,
        &mut successors,
    )
    .expect("reachable local prerequisite should yield an FF relaxed plan");

    assert!(
        ff_result.h_ff > spatial_heuristic,
        "local pickup should raise the heuristic floor above the zero spatial baseline"
    );
    assert!(
        ff_result.helpful_action_indices.contains(&move_cargo_index),
        "root MoveCargo should be selected as a helpful action"
    );
    assert_eq!(
        successors[move_cargo_index].2.heuristic_ticks, ff_result.h_ff,
        "successor heuristic should take the max of spatial and FF guidance"
    );
    assert!(successors[move_cargo_index].3);
}

#[test]
fn ff_helpful_non_travel_candidates_always_stay_preferred() {
    let (view, actor, hub, east, _south, _north, goal_store) = build_hub_pruning_view();
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::new(),
        &BTreeSet::from([hub, east, goal_store]),
        5,
    );
    let node = SearchNode {
        state: PlanningState::new(&snapshot),
        steps: SharedVec::new(),
        total_estimated_ticks: 0,
        search_cost: 0,
        tactical_barrier_reached: false,
        heuristic_ticks: 0,
    };
    let candidate = make_non_travel_candidate(ActionDefId(200), goal_store);
    let mut semantics_table = BTreeMap::new();
    semantics_table.insert(ActionDefId(200), harvest_semantics());

    assert!(
        super::ff_helpful_candidate_is_preferred(
            &snapshot,
            &node,
            &node.state,
            &candidate,
            true,
            &[],
            &semantics_table,
        ),
        "helpful non-travel candidates should always enter the preferred queue even without travel guidance"
    );
}

#[test]
fn ff_successor_rewrite_preserves_spatial_heuristic_when_it_exceeds_relaxed_plan() {
    let (mut view, actor, patient, current_place, patient_place, medicine_place) =
        build_branching_care_view();
    view.wounds.insert(patient, vec![wound(403)]);
    view.effective_places.insert(actor, medicine_place);
    view.entities_at
        .insert(medicine_place, vec![actor, entity(20)]);
    view.entities_at.insert(current_place, Vec::new());
    let (registry, handlers) = build_registry();
    let semantics = build_semantics_table(&registry);
    let recipes = RecipeRegistry::new();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::TreatWounds { patient }),
        evidence_entities: BTreeSet::from([patient]),
        evidence_places: BTreeSet::from([patient_place, medicine_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        2,
    );
    let tactical_goal = TacticalGoal::AcquirePrerequisite {
        commodity: CommodityKind::Medicine,
        destination: patient_place,
    };
    let budget = execution_budget(&ProfileFixture::default());
    let node = super::heuristic::root_node_for_tactical(
        &snapshot,
        &goal,
        &recipes,
        &budget,
        Some(&tactical_goal),
    );
    let relevant_defs = relevant_action_defs(&goal, &semantics);
    let candidates = search_candidates(
        &goal,
        &node,
        candidate_search_context(
            &semantics,
            &registry,
            &handlers,
            &BlockerMemory::default(),
            Tick(0),
            &relevant_defs,
        ),
        None,
        None,
        None,
    );
    let move_cargo = candidates
        .iter()
        .find(|candidate| {
            registry
                .get(candidate.def_id)
                .is_some_and(|def| def.name == "pick_up")
        })
        .expect("local medicine fixture should expose a root MoveCargo candidate");
    let (terminal, successor, _) = super::build_successor_detailed(
        &goal,
        &semantics,
        &registry,
        &node,
        move_cargo,
        &recipes,
        &budget,
        Some(&tactical_goal),
        &super::landmarks::LandmarkSet::empty(),
    )
    .expect("local medicine pickup should build a tactical successor");
    let mut successors = vec![(0usize, terminal, successor, false)];
    let successor_candidates = vec![move_cargo.clone()];
    let successor_operators = vec![super::landmarks::planning_operator_from_transition(
        &node.state,
        &successors[0].2.state,
    )];
    let move_cargo_index = successors
        .iter()
        .position(|(_, _, successor, _)| {
            successor
                .steps
                .as_slice()
                .last()
                .is_some_and(|step| step.op_kind == PlannerOpKind::MoveCargo)
        })
        .expect("local medicine fixture should expose a root MoveCargo successor");
    let combined_places = super::heuristic::combined_relevant_places_for_tactical(
        &goal,
        &successors[move_cargo_index].2.state,
        &recipes,
        &execution_budget(&ProfileFixture::default()),
        Some(&tactical_goal),
    );
    let spatial_heuristic = compute_heuristic(
        &snapshot,
        &successors[move_cargo_index].2.state,
        &combined_places.places,
    );

    let ff_result = super::apply_ff_heuristic_to_successors(
        &snapshot,
        &goal,
        &node,
        &cognitive(&ProfileFixture::default()),
        &recipes,
        &execution_budget(&ProfileFixture::default()),
        Some(&tactical_goal),
        &semantics,
        &successor_candidates,
        &successor_operators,
        &mut successors,
    )
    .expect("reachable local medicine pickup should yield an FF relaxed plan");

    assert!(
        spatial_heuristic > ff_result.h_ff,
        "remote patient travel should remain the dominant heuristic after local pickup"
    );
    assert_eq!(
        successors[move_cargo_index].2.heuristic_ticks, spatial_heuristic,
        "successor heuristic should preserve the larger spatial signal"
    );
    assert!(
        ff_result.helpful_action_indices.contains(&move_cargo_index),
        "local medicine MoveCargo should still be marked helpful"
    );
    assert!(successors[move_cargo_index].3);
}

#[test]
fn search_trace_metadata_records_ff_heuristic_and_helpful_actions_when_enabled() {
    let (mut view, actor, patient, _current_place, patient_place, medicine_place) =
        build_branching_care_view();
    view.wounds.insert(patient, vec![wound(405)]);
    let (registry, handlers) = build_registry();
    let recipes = RecipeRegistry::new();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::TreatWounds { patient }),
        evidence_entities: BTreeSet::from([patient]),
        evidence_places: BTreeSet::from([patient_place, medicine_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        2,
    );
    let mut summaries = Vec::new();
    let mut trace_metadata = super::SearchTraceMetadata::default();

    let result = super::search_plan_with_trace_metadata(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &cognitive(&ProfileFixture::default()),
        &execution_budget(&ProfileFixture::default()),
        &recipes,
        &BlockerMemory::default(),
        Tick(0),
        None,
        Some(&mut summaries),
        Some(&mut trace_metadata),
    );

    assert!(
        result.is_found(),
        "FF-enabled two-phase production should find a plan"
    );
    assert!(
        summaries
            .iter()
            .any(|summary| summary.ff_heuristic.is_some()),
        "at least one expansion summary should report h_ff when FF guidance is enabled"
    );
    assert!(
        summaries
            .iter()
            .any(|summary| summary.helpful_action_count > 0),
        "at least one expansion summary should report helpful actions when FF guidance is enabled"
    );
}

#[test]
fn search_trace_metadata_disables_ff_fields_when_profile_toggle_is_off() {
    let fixture = build_remote_produce_commodity_fixture();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::ProduceCommodity {
            recipe_id: fixture.recipe_id,
        }),
        evidence_entities: BTreeSet::from([fixture.mill, fixture.firewood]),
        evidence_places: BTreeSet::from([
            prototype_place_entity(PrototypePlace::VillageSquare),
            prototype_place_entity(PrototypePlace::OrchardFarm),
        ]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );
    let mut no_ff = cognitive(&ProfileFixture::default());
    no_ff.use_ff_heuristic = false;
    let mut summaries = Vec::new();
    let mut trace_metadata = super::SearchTraceMetadata::default();

    let result = super::search_plan_with_trace_metadata(
        &snapshot,
        &goal,
        &fixture.semantics,
        &fixture.registry,
        &fixture.handlers,
        &no_ff,
        &execution_budget(&ProfileFixture::default()),
        &fixture.recipes,
        &BlockerMemory::default(),
        Tick(1),
        None,
        Some(&mut summaries),
        Some(&mut trace_metadata),
    );

    assert!(
        result.is_found(),
        "FF-disabled mode should still find the remote production plan"
    );
    assert!(
        summaries
            .iter()
            .all(|summary| summary.ff_heuristic.is_none() && summary.helpful_action_count == 0),
        "FF-disabled mode should keep the FF trace fields inert"
    );
    assert!(
        summaries
            .iter()
            .any(|summary| summary.landmark_heuristic > 0),
        "disabling FF should preserve the landmark guidance path"
    );
}

#[test]
fn ff_dead_end_falls_back_to_landmark_guidance_at_root() {
    let fixture = build_remote_produce_commodity_fixture();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::ProduceCommodity {
            recipe_id: fixture.recipe_id,
        }),
        evidence_entities: BTreeSet::from([fixture.mill, fixture.firewood]),
        evidence_places: BTreeSet::from([
            prototype_place_entity(PrototypePlace::VillageSquare),
            prototype_place_entity(PrototypePlace::OrchardFarm),
        ]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );
    let mut ff_summaries = Vec::new();
    let ff_result = super::search_plan_with_trace_metadata(
        &snapshot,
        &goal,
        &fixture.semantics,
        &fixture.registry,
        &fixture.handlers,
        &cognitive(&ProfileFixture::default()),
        &execution_budget(&ProfileFixture::default()),
        &fixture.recipes,
        &BlockerMemory::default(),
        Tick(1),
        None,
        Some(&mut ff_summaries),
        None,
    );
    let mut no_ff = cognitive(&ProfileFixture::default());
    no_ff.use_ff_heuristic = false;
    let mut no_ff_summaries = Vec::new();
    let no_ff_result = super::search_plan_with_trace_metadata(
        &snapshot,
        &goal,
        &fixture.semantics,
        &fixture.registry,
        &fixture.handlers,
        &no_ff,
        &execution_budget(&ProfileFixture::default()),
        &fixture.recipes,
        &BlockerMemory::default(),
        Tick(1),
        None,
        Some(&mut no_ff_summaries),
        None,
    );

    assert!(
        ff_result.is_found(),
        "FF-enabled baseline should still find a plan"
    );
    assert!(
        no_ff_result.is_found(),
        "landmark-only baseline should still find a plan"
    );

    let ff_root = ff_summaries
        .first()
        .expect("FF-enabled run should record a root expansion summary");
    let no_ff_root = no_ff_summaries
        .first()
        .expect("landmark-only run should record a root expansion summary");

    assert_eq!(ff_root.ff_heuristic, None);
    assert_eq!(ff_root.helpful_action_count, 0);
    assert_eq!(
        ff_root.preferred_candidates,
        no_ff_root.preferred_candidates
    );
    assert_eq!(ff_root.landmark_heuristic, no_ff_root.landmark_heuristic);
}

#[test]
fn search_trace_metadata_records_acquire_prerequisite_for_known_remote_acquire_self_consume() {
    let village_square = prototype_place_entity(PrototypePlace::VillageSquare);
    let orchard_farm = prototype_place_entity(PrototypePlace::OrchardFarm);
    let mut recipes = RecipeRegistry::new();
    recipes.register(harvest_apple_recipe());
    let (registry, handlers) = build_registry_with_recipes(&recipes);
    let semantics = build_semantics_table(&registry);
    let mut world = World::new(build_prototype_world()).unwrap();
    let (actor, _orchard_row) = {
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        let actor = txn.create_agent("Forager", ControlSource::Ai).unwrap();
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
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        )
        .unwrap();
        let mut event_log = EventLog::new();
        let _ = txn.commit(&mut event_log);
        (actor, orchard_row)
    };
    sync_all_beliefs(&mut world, actor, Tick(1));

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([orchard_farm]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(actor, &world);
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );
    let mut trace_metadata = super::SearchTraceMetadata::default();

    let result = super::search_plan_with_trace_metadata(
        &snapshot,
        &goal,
        &semantics,
        &registry,
        &handlers,
        &cognitive(&ProfileFixture::default()),
        &execution_budget(&ProfileFixture::default()),
        &recipes,
        &BlockerMemory::default(),
        Tick(1),
        None,
        None,
        Some(&mut trace_metadata),
    );

    assert!(
        result.is_found(),
        "known remote self-consume acquisition should still produce a plan"
    );
    assert_eq!(
        trace_metadata.tactical_goal.as_deref(),
        Some(
            format!("AcquirePrerequisite {{ commodity: Apple, destination: {orchard_farm:?} }}")
                .as_str()
        )
    );
    let strategic_plan = trace_metadata
        .strategic_plan
        .as_ref()
        .expect("known remote acquire should record a strategic plan");
    assert_eq!(strategic_plan.steps.len(), 2);
    assert_eq!(strategic_plan.steps[0].destination, orchard_farm);
    assert_eq!(
        strategic_plan.steps[0].sub_goal,
        super::strategic::TacticalSubGoal::AcquirePrerequisite(CommodityKind::Apple)
    );
}

#[test]
fn search_trace_metadata_records_no_tactical_goal_for_local_sleep() {
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
        HomeostaticNeeds::new(pm(0), pm(0), pm(300), pm(0), pm(0)),
    );
    view.thresholds.insert(actor, DriveThresholds::default());

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::Sleep),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        0,
    );
    let mut trace_metadata = super::SearchTraceMetadata::default();

    let result = super::search_plan_with_trace_metadata(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &cognitive(&ProfileFixture::default()),
        &execution_budget(&ProfileFixture::default()),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
        Some(&mut trace_metadata),
    );

    assert!(
        result.is_found(),
        "local sleep planning should still succeed"
    );
    assert_eq!(trace_metadata.tactical_goal, None);
}

#[test]
fn local_critical_sleep_returns_progress_barrier_after_one_step() {
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
        HomeostaticNeeds::new(pm(0), pm(0), pm(1000), pm(0), pm(0)),
    );
    view.thresholds.insert(actor, DriveThresholds::default());

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::Sleep),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        0,
    );

    let result = super::search_plan_with_trace_metadata(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &cognitive(&ProfileFixture::default()),
        &execution_budget(&ProfileFixture::default()),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
        None,
    );

    let plan = result
        .into_plan()
        .expect("critical local sleep should plan");
    assert_eq!(plan.terminal_kind, PlanTerminalKind::ProgressBarrier);
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Sleep);
}

#[test]
fn search_fail_fast_when_barrier_required_explore_has_no_tactical_goal() {
    let step = super::strategic::StrategicStep {
        destination: entity(11),
        sub_goal: super::strategic::TacticalSubGoal::ExploreWithBarrier,
        estimated_travel_ticks: 2,
    };

    assert!(super::should_fail_fast_for_missing_tactical_goal(
        Some(&step),
        None
    ));
    assert!(!super::should_fail_fast_for_missing_tactical_goal(
        Some(&step),
        Some(&super::TacticalGoal::Explore {
            destination: entity(11),
        })
    ));
}

#[test]
fn search_generic_explore_without_tactical_goal_still_finds_plan() {
    let (mut view, actor, _origin, destination) = build_two_place_travel_view();
    view.needs.insert(
        actor,
        HomeostaticNeeds::new(pm(0), pm(0), pm(300), pm(0), pm(0)),
    );
    view.thresholds.insert(actor, DriveThresholds::default());

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::Sleep),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([destination]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let mut trace_metadata = super::SearchTraceMetadata::default();

    let result = super::search_plan_with_trace_metadata(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &cognitive(&ProfileFixture::default()),
        &execution_budget(&ProfileFixture::default()),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
        Some(&mut trace_metadata),
    );

    assert!(
        result.is_found(),
        "generic exploration fallback should still search without a tactical barrier"
    );
    assert_eq!(trace_metadata.tactical_goal, None);
    assert!(
        trace_metadata
            .strategic_plan
            .as_ref()
            .and_then(|plan| plan.steps.first())
            .is_some_and(|step| {
                step.destination == destination
                    && matches!(
                        step.sub_goal,
                        super::strategic::TacticalSubGoal::ExploreFallback
                    )
            }),
        "Sleep should now use the generic exploration fallback variant"
    );
}

#[test]
fn search_explore_tactical_goal_produced_despite_nonempty_evidence() {
    let (view, actor, origin, destination) = build_two_place_travel_view();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([destination]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let step = super::strategic::StrategicStep {
        destination: origin,
        sub_goal: super::strategic::TacticalSubGoal::ExploreWithBarrier,
        estimated_travel_ticks: 1,
    };

    let tactical_goal = super::TacticalGoal::from_strategic_step(&goal, Some(&step), &snapshot);

    assert_eq!(
        tactical_goal,
        Some(super::TacticalGoal::Explore { destination }),
        "non-empty evidence places must no longer suppress AcquireCommodity exploration"
    );
}

#[test]
fn search_evidence_directed_exploration_prefers_evidence_place() {
    let actor = entity(1);
    let origin = entity(10);
    let evidence_place = entity(11);
    let fallback_place = entity(12);

    let mut view = TestBeliefView::default();
    view.alive
        .extend([actor, origin, evidence_place, fallback_place]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(origin, EntityKind::Place);
    view.kinds.insert(evidence_place, EntityKind::Place);
    view.kinds.insert(fallback_place, EntityKind::Place);
    view.effective_places.insert(actor, origin);
    view.entities_at.insert(origin, vec![actor]);
    view.entities_at.insert(evidence_place, Vec::new());
    view.entities_at.insert(fallback_place, Vec::new());
    view.adjacent.insert(
        origin,
        vec![
            (evidence_place, NonZeroU32::new(1).unwrap()),
            (fallback_place, NonZeroU32::new(3).unwrap()),
        ],
    );
    view.adjacent
        .insert(evidence_place, vec![(origin, NonZeroU32::new(1).unwrap())]);
    view.adjacent
        .insert(fallback_place, vec![(origin, NonZeroU32::new(3).unwrap())]);

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([evidence_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        2,
    );
    let step = super::strategic::StrategicStep {
        destination: fallback_place,
        sub_goal: super::strategic::TacticalSubGoal::ExploreWithBarrier,
        estimated_travel_ticks: 3,
    };

    let tactical_goal = super::TacticalGoal::from_strategic_step(&goal, Some(&step), &snapshot);

    assert_eq!(
        tactical_goal,
        Some(super::TacticalGoal::Explore {
            destination: evidence_place,
        }),
        "exploration should target the nearest evidence place instead of the strategic fallback destination"
    );
}

#[test]
fn search_accuse_satisfy_goal_does_not_install_travel_barrier() {
    let (view, actor, origin, destination) = build_two_place_travel_view();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Entity(entity(99)),
        key: GoalKey::from(GoalKind::Accuse {
            crime_register: entity(98),
            accused: entity(99),
            violation_id: worldwake_core::ViolationId(7),
        }),
        evidence_entities: BTreeSet::from([entity(98), entity(99)]),
        evidence_places: BTreeSet::from([destination]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let step = super::strategic::StrategicStep {
        destination: origin,
        sub_goal: super::strategic::TacticalSubGoal::SatisfyGoal,
        estimated_travel_ticks: 1,
    };

    let tactical_goal = super::TacticalGoal::from_strategic_step(&goal, Some(&step), &snapshot);

    assert_eq!(
        tactical_goal, None,
        "Accuse should not treat a strategic satisfy step as a travel barrier because the action is register-bound, not destination-bound"
    );
}

#[test]
fn search_accuse_search_without_tactical_barrier_still_finds_plan() {
    let (mut view, actor, origin, town) = build_two_place_travel_view();
    let accused = entity(2);
    let crime_register = entity(12);

    view.alive.extend([accused, crime_register]);
    view.kinds.insert(accused, EntityKind::Agent);
    view.kinds.insert(crime_register, EntityKind::Record);
    view.effective_places.insert(accused, origin);
    view.effective_places.insert(crime_register, town);
    view.entities_at.entry(origin).or_default().push(accused);
    view.entities_at
        .entry(town)
        .or_default()
        .push(crime_register);
    view.record_data.insert(
        crime_register,
        worldwake_core::RecordData {
            record_kind: worldwake_core::RecordKind::CrimeRegister,
            home_place: town,
            issuer: actor,
            consultation_ticks: 1,
            max_entries_per_consult: 4,
            entries: Vec::new(),
            next_entry_id: 1,
        },
    );

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Entity(accused),
        key: GoalKey::from(GoalKind::Accuse {
            crime_register,
            accused,
            violation_id: worldwake_core::ViolationId(1),
        }),
        evidence_entities: BTreeSet::from([accused, crime_register]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let (registry, handlers) = build_registry();
    let mut trace_metadata = super::SearchTraceMetadata::default();

    let plan = search_plan_with_trace_metadata(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
        Some(&mut trace_metadata),
    )
    .into_plan()
    .expect("Accuse should still plan without a tactical travel barrier");

    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Travel);
    assert_eq!(
        plan.steps[0].targets,
        vec![PlanningEntityRef::Authoritative(town)]
    );
    assert_eq!(
        plan.steps.last().map(|step| step.op_kind),
        Some(PlannerOpKind::Accuse)
    );
    assert_eq!(trace_metadata.tactical_goal, None);
    assert!(
        trace_metadata
            .strategic_plan
            .as_ref()
            .and_then(|plan| plan.steps.first())
            .is_some_and(|step| {
                step.destination == town
                    && matches!(
                        step.sub_goal,
                        super::strategic::TacticalSubGoal::SatisfyGoal
                    )
            }),
        "Accuse should keep its lawful register-bound strategic step"
    );
}

#[test]
fn search_acquire_commodity_uses_travel_to_goal() {
    let (mut view, actor, _origin, market) = build_two_place_travel_view();
    let bread = entity(20);
    view.alive.insert(bread);
    view.kinds.insert(bread, EntityKind::ItemLot);
    view.effective_places.insert(bread, market);
    view.entities_at.entry(market).or_default().push(bread);
    view.controllable.insert((actor, bread));
    view.lot_commodities.insert(bread, CommodityKind::Bread);
    view.commodity_quantities
        .insert((bread, CommodityKind::Bread), Quantity(1));
    view.carry_capacities.insert(actor, LoadUnits(10));
    view.entity_loads.insert(actor, LoadUnits(0));
    view.entity_loads.insert(
        bread,
        LoadUnits(worldwake_core::load_per_unit(CommodityKind::Bread).0),
    );

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([market]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let (registry, handlers) = build_registry();
    let mut trace_metadata = super::SearchTraceMetadata::default();
    let plan = search_plan_with_trace_metadata(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
        Some(&mut trace_metadata),
    )
    .into_plan()
    .expect("remote acquire goal should plan travel to the known commodity place");

    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Travel);
    assert_eq!(
        plan.steps[0].targets,
        vec![PlanningEntityRef::Authoritative(market)]
    );
}

#[test]
fn search_patrol_uses_travel_to_goal_for_remote_place() {
    let (mut view, actor, _origin, patrol_place) = build_two_place_travel_view();
    view.patrol_profiles.insert(
        actor,
        PatrolProfile {
            base_dwell_ticks: 8,
            dwell_vigilance_scale_ticks: 8,
            vigilance: pm(625),
            route_adaptation_sensitivity: pm(400),
            patrol_motive_weight: pm(550),
        },
    );
    view.patrol_routes.insert(
        actor,
        PatrolRoute {
            assigned_places: vec![patrol_place],
            current_index: 0,
        },
    );
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(patrol_place),
        key: GoalKey::from(GoalKind::Patrol {
            place: patrol_place,
        }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([patrol_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let (registry, handlers) = build_registry();
    let mut trace_metadata = super::SearchTraceMetadata::default();
    let plan = search_plan_with_trace_metadata(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
        Some(&mut trace_metadata),
    )
    .into_plan()
    .expect("remote patrol goal should plan travel to the patrol place");

    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Travel);
    assert_eq!(
        plan.steps[0].targets,
        vec![PlanningEntityRef::Authoritative(patrol_place)]
    );
    assert_eq!(
        plan.steps.last().map(|step| step.op_kind),
        Some(PlannerOpKind::Patrol)
    );
    assert_eq!(
        trace_metadata.tactical_goal.as_deref(),
        Some(format!("TravelToGoal {{ destination: {patrol_place:?} }}").as_str())
    );
}

#[test]
fn search_investigate_uses_travel_to_goal() {
    let (mut view, actor, _origin, violation_place) = build_two_place_travel_view();
    view.violation_profiles.insert(
        actor,
        ViolationDispositionProfile {
            investigation_duration_ticks: NonZeroU32::new(3).unwrap(),
            violation_memory_retention_ticks: 50,
            investigation_motive_weight: pm(500),
            ownership_motive_bonus: pm(200),
        },
    );
    view.active_violation_records.insert(
        actor,
        vec![RecordedViolation {
            id: ViolationId(1),
            kind: ViolationKind::SupplyDepleted {
                commodity: CommodityKind::Bread,
                source: violation_place,
                place: violation_place,
            },
            observed_tick: Tick(0),
            resolved_tick: None,
            expires_tick: Tick(50),
        }],
    );
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(violation_place),
        key: GoalKey::from(GoalKind::InvestigateViolation {
            violation_id: ViolationId(1),
            place: violation_place,
        }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([violation_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let (registry, handlers) = build_registry();
    let mut trace_metadata = super::SearchTraceMetadata::default();
    let plan = search_plan_with_trace_metadata(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
        Some(&mut trace_metadata),
    )
    .into_plan()
    .expect("remote investigation goal should plan travel to the violation place");

    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Travel);
    assert_eq!(
        plan.steps[0].targets,
        vec![PlanningEntityRef::Authoritative(violation_place)]
    );
    assert_eq!(
        plan.steps.last().map(|step| step.op_kind),
        Some(PlannerOpKind::Investigate)
    );
    assert_eq!(
        trace_metadata.tactical_goal.as_deref(),
        Some(format!("TravelToGoal {{ destination: {violation_place:?} }}").as_str())
    );
}

#[test]
fn search_investigate_allows_local_scene_investigation_despite_stale_remote_evidence() {
    let actor = entity(1);
    let stale_subject = entity(2);
    let violation_place = entity(10);
    let remote_place = entity(11);

    let mut view = TestBeliefView::default();
    view.alive
        .extend([actor, stale_subject, violation_place, remote_place]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(stale_subject, EntityKind::ItemLot);
    view.kinds.insert(violation_place, EntityKind::Place);
    view.kinds.insert(remote_place, EntityKind::Place);
    view.current_tick = Tick(50);
    view.effective_places.insert(actor, violation_place);
    view.effective_places.insert(stale_subject, remote_place);
    view.entities_at.insert(violation_place, vec![actor]);
    view.entities_at.insert(remote_place, vec![stale_subject]);
    view.epistemic_profiles.insert(actor, epistemic_profile());
    view.violation_profiles.insert(
        actor,
        ViolationDispositionProfile {
            investigation_duration_ticks: NonZeroU32::new(3).unwrap(),
            violation_memory_retention_ticks: 50,
            investigation_motive_weight: pm(500),
            ownership_motive_bonus: pm(200),
        },
    );
    view.active_violation_records.insert(
        actor,
        vec![RecordedViolation {
            id: ViolationId(1),
            kind: ViolationKind::SupplyDepleted {
                commodity: CommodityKind::Bread,
                source: stale_subject,
                place: violation_place,
            },
            observed_tick: Tick(0),
            resolved_tick: None,
            expires_tick: Tick(50),
        }],
    );
    view.known_entity_beliefs.insert(
        actor,
        vec![(
            stale_subject,
            believed_entity_state_at(remote_place, Tick(0), None),
        )],
    );

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(violation_place),
        key: GoalKey::from(GoalKind::InvestigateViolation {
            violation_id: ViolationId(1),
            place: violation_place,
        }),
        evidence_entities: BTreeSet::from([stale_subject]),
        evidence_places: BTreeSet::from([violation_place, remote_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        2,
    );
    let (registry, handlers) = build_registry();
    let plan = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    )
    .into_plan()
    .expect("local investigation should proceed instead of bouncing toward stale remote evidence");

    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Investigate);
    assert_eq!(
        plan.steps[0].targets,
        vec![PlanningEntityRef::Authoritative(violation_place)]
    );
}

#[test]
fn search_travel_to_goal_barrier_satisfied_at_destination() {
    let (view, actor, _origin, destination) = build_two_place_travel_view();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(destination),
        key: GoalKey::from(GoalKind::Patrol { place: destination }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([destination]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let reached_state = PlanningState::new(&snapshot).move_actor_to(destination);
    let not_reached_state = PlanningState::new(&snapshot);
    let tactical_goal = super::TacticalGoal::TravelToGoal { destination };

    assert!(tactical_goal.progress_barrier_satisfied(&reached_state));
    assert!(!tactical_goal.progress_barrier_satisfied(&not_reached_state));
}

#[test]
fn search_travel_to_goal_candidate_filter() {
    let (view, actor, origin, destination) = build_two_place_travel_view();
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::new(),
        &BTreeSet::from([destination]),
        1,
    );
    let node = SearchNode {
        state: PlanningState::new(&snapshot),
        steps: SharedVec::new(),
        total_estimated_ticks: 0,
        search_cost: 0,
        tactical_barrier_reached: false,
        heuristic_ticks: 0,
    };
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(destination),
        key: GoalKey::from(GoalKind::Patrol { place: destination }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([destination]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let travel_id = ActionDefId(100);
    let patrol_id = ActionDefId(101);
    let harvest_id = ActionDefId(102);

    let mut semantics_table = BTreeMap::new();
    semantics_table.insert(travel_id, travel_semantics());
    semantics_table.insert(
        patrol_id,
        PlannerOpSemantics {
            op_kind: PlannerOpKind::Patrol,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            synthetic_cargo: PlannerSyntheticCargo::None,
        },
    );
    semantics_table.insert(harvest_id, harvest_semantics());

    let mut candidates = vec![
        make_travel_candidate(travel_id, destination),
        make_non_travel_candidate(patrol_id, origin),
        make_non_travel_candidate(harvest_id, origin),
    ];

    super::apply_tactical_candidate_filter(
        &mut candidates,
        &goal,
        &node,
        &semantics_table,
        Some(&super::TacticalGoal::TravelToGoal { destination }),
    );

    assert_eq!(candidates.len(), 2);
    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.def_id == travel_id)
    );
    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.def_id == patrol_id)
    );
    assert!(
        candidates
            .iter()
            .all(|candidate| candidate.def_id != harvest_id),
        "irrelevant non-travel setup should be removed before departure"
    );
}

#[test]
fn commodity_relevance_filter_prunes_mismatched_trade_movecargo_and_craft_candidates() {
    let actor = entity(1);
    let place = entity(10);
    let bread_lot = entity(20);
    let waste_lot = entity(21);
    let workshop = entity(30);
    let seller = entity(40);

    let mut view = TestBeliefView::default();
    view.alive
        .extend([actor, place, bread_lot, waste_lot, workshop, seller]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(place, EntityKind::Place);
    view.kinds.insert(bread_lot, EntityKind::ItemLot);
    view.kinds.insert(waste_lot, EntityKind::ItemLot);
    view.kinds.insert(workshop, EntityKind::Place);
    view.kinds.insert(seller, EntityKind::Agent);
    view.effective_places.insert(actor, place);
    view.effective_places.insert(bread_lot, place);
    view.effective_places.insert(waste_lot, place);
    view.effective_places.insert(workshop, place);
    view.effective_places.insert(seller, place);
    view.entities_at
        .insert(place, vec![actor, bread_lot, waste_lot, workshop, seller]);
    view.lot_commodities.insert(bread_lot, CommodityKind::Bread);
    view.lot_commodities.insert(waste_lot, CommodityKind::Waste);
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let mut recipes = RecipeRegistry::new();
    recipes.register(RecipeDefinition {
        name: "Press Apples".to_string(),
        inputs: vec![(CommodityKind::Grain, Quantity(1))],
        outputs: vec![(CommodityKind::Apple, Quantity(1))],
        work_ticks: NonZeroU32::new(3).unwrap(),
        required_workstation_tag: Some(WorkstationTag::Mill),
        required_tool_kinds: Vec::new(),
        body_cost_per_tick: BodyCostPerTick::zero(),
    });
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let state = PlanningState::new(&snapshot);
    let (registry, _handlers) = build_registry_with_recipes(&recipes);
    let semantics_table = build_semantics_table(&registry);

    let move_cargo_id = registry
        .iter()
        .find(|def| def.name == "pick_up")
        .map(|def| def.id)
        .expect("pick_up action should be registered");
    let trade_id = registry
        .iter()
        .find(|def| def.name == "trade")
        .map(|def| def.id)
        .expect("trade action should be registered");
    let craft_id = registry
        .iter()
        .find_map(|def| {
            def.payload
                .as_craft()
                .is_some_and(|payload| {
                    payload
                        .outputs
                        .iter()
                        .any(|(commodity, _)| *commodity == CommodityKind::Apple)
                })
                .then_some(def.id)
        })
        .expect("apple craft action should be registered");

    let mut candidates = vec![
        SearchCandidate {
            def_id: move_cargo_id,
            authoritative_targets: vec![bread_lot],
            planning_targets: vec![PlanningEntityRef::Authoritative(bread_lot)],
            payload_override: Some(ActionPayload::Transport(TransportActionPayload {
                quantity: Quantity(1),
            })),
            planner_only: false,
            trace_index: Some(0),
            expansion_trace_index: None,
        },
        SearchCandidate {
            def_id: move_cargo_id,
            authoritative_targets: vec![waste_lot],
            planning_targets: vec![PlanningEntityRef::Authoritative(waste_lot)],
            payload_override: Some(ActionPayload::Transport(TransportActionPayload {
                quantity: Quantity(1),
            })),
            planner_only: false,
            trace_index: Some(1),
            expansion_trace_index: None,
        },
        SearchCandidate {
            def_id: trade_id,
            authoritative_targets: vec![seller],
            planning_targets: vec![PlanningEntityRef::Authoritative(seller)],
            payload_override: Some(ActionPayload::Trade(TradeActionPayload {
                counterparty: seller,
                sale_lot: bread_lot,
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(1),
                requested_quantity: Quantity(1),
            })),
            planner_only: false,
            trace_index: Some(2),
            expansion_trace_index: None,
        },
        SearchCandidate {
            def_id: trade_id,
            authoritative_targets: vec![seller],
            planning_targets: vec![PlanningEntityRef::Authoritative(seller)],
            payload_override: Some(ActionPayload::Trade(TradeActionPayload {
                counterparty: seller,
                sale_lot: waste_lot,
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(1),
                requested_quantity: Quantity(1),
            })),
            planner_only: false,
            trace_index: Some(3),
            expansion_trace_index: None,
        },
        SearchCandidate {
            def_id: craft_id,
            authoritative_targets: vec![workshop],
            planning_targets: vec![PlanningEntityRef::Authoritative(workshop)],
            payload_override: None,
            planner_only: false,
            trace_index: Some(4),
            expansion_trace_index: None,
        },
    ];
    let mut root_candidates = candidates
        .iter()
        .map(|candidate| {
            root_candidate_trace_from_candidate(
                candidate,
                &registry,
                &semantics_table,
                None,
                crate::decision_trace::CandidateSource::Emitter,
            )
        })
        .collect::<Vec<_>>();

    apply_commodity_relevance_filter(
        &mut candidates,
        &goal,
        &state,
        commodity_filter_context(None, &semantics_table, &registry, &recipes),
        Some(&mut root_candidates),
    );

    assert_eq!(candidates.len(), 2);
    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.trace_index)
            .collect::<Vec<_>>(),
        vec![Some(0), Some(2)]
    );
    assert_eq!(
        root_candidates[1].outcome,
        crate::decision_trace::RootCandidateOutcome::Filtered(
            crate::decision_trace::RootCandidateFilterReason::CommodityIrrelevant {
                candidate_commodity: Some(CommodityKind::Waste),
                goal_commodity: CommodityKind::Bread,
            },
        )
    );
    assert_eq!(
        root_candidates[3].outcome,
        crate::decision_trace::RootCandidateOutcome::Filtered(
            crate::decision_trace::RootCandidateFilterReason::CommodityIrrelevant {
                candidate_commodity: Some(CommodityKind::Waste),
                goal_commodity: CommodityKind::Bread,
            },
        )
    );
    assert_eq!(
        root_candidates[4].outcome,
        crate::decision_trace::RootCandidateOutcome::Filtered(
            crate::decision_trace::RootCandidateFilterReason::CommodityIrrelevant {
                candidate_commodity: Some(CommodityKind::Apple),
                goal_commodity: CommodityKind::Bread,
            },
        )
    );
}

#[test]
fn commodity_relevance_filter_keeps_travel_unknown_and_queue_for_matching_craft() {
    let actor = entity(1);
    let place = entity(10);
    let unknown_lot = entity(20);
    let mill = entity(30);

    let mut view = TestBeliefView::default();
    view.alive.extend([actor, place, unknown_lot, mill]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(place, EntityKind::Place);
    view.kinds.insert(unknown_lot, EntityKind::ItemLot);
    view.kinds.insert(mill, EntityKind::Place);
    view.effective_places.insert(actor, place);
    view.effective_places.insert(unknown_lot, place);
    view.effective_places.insert(mill, place);
    view.entities_at
        .insert(place, vec![actor, unknown_lot, mill]);

    let mut recipes = RecipeRegistry::new();
    let recipe_id = recipes.register(RecipeDefinition {
        name: "Bake Bread".to_string(),
        inputs: vec![(CommodityKind::Grain, Quantity(2))],
        outputs: vec![(CommodityKind::Bread, Quantity(1))],
        work_ticks: NonZeroU32::new(3).unwrap(),
        required_workstation_tag: Some(WorkstationTag::Mill),
        required_tool_kinds: Vec::new(),
        body_cost_per_tick: BodyCostPerTick::zero(),
    });
    let (registry, _handlers) = build_registry_with_recipes(&recipes);
    let semantics_table = build_semantics_table(&registry);
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::ProduceCommodity { recipe_id }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let state = PlanningState::new(&snapshot);

    let travel_id = registry
        .iter()
        .find(|def| def.name == "travel")
        .map(|def| def.id)
        .expect("travel action should be registered");
    let queue_id = registry
        .iter()
        .find(|def| def.name == "queue_for_facility_use")
        .map(|def| def.id)
        .expect("queue_for_facility_use action should be registered");
    let pick_up_id = registry
        .iter()
        .find(|def| def.name == "pick_up")
        .map(|def| def.id)
        .expect("pick_up action should be registered");
    let matching_craft_id = registry
        .iter()
        .find_map(|def| {
            def.payload.as_craft().and_then(|payload| {
                (payload
                    .outputs
                    .iter()
                    .any(|(commodity, _)| *commodity == CommodityKind::Bread))
                .then_some(def.id)
            })
        })
        .expect("bread craft action should be registered");

    let mut candidates = vec![
        SearchCandidate {
            def_id: travel_id,
            authoritative_targets: vec![mill],
            planning_targets: vec![PlanningEntityRef::Authoritative(mill)],
            payload_override: None,
            planner_only: false,
            trace_index: None,
            expansion_trace_index: None,
        },
        SearchCandidate {
            def_id: queue_id,
            authoritative_targets: vec![mill],
            planning_targets: vec![PlanningEntityRef::Authoritative(mill)],
            payload_override: Some(ActionPayload::QueueForFacilityUse(
                QueueForFacilityUsePayload {
                    intended_action: matching_craft_id,
                },
            )),
            planner_only: false,
            trace_index: None,
            expansion_trace_index: None,
        },
        SearchCandidate {
            def_id: pick_up_id,
            authoritative_targets: vec![unknown_lot],
            planning_targets: vec![PlanningEntityRef::Authoritative(unknown_lot)],
            payload_override: Some(ActionPayload::Transport(TransportActionPayload {
                quantity: Quantity(1),
            })),
            planner_only: false,
            trace_index: None,
            expansion_trace_index: None,
        },
    ];

    apply_commodity_relevance_filter(
        &mut candidates,
        &goal,
        &state,
        commodity_filter_context(None, &semantics_table, &registry, &recipes),
        None,
    );

    assert_eq!(candidates.len(), 3);
}

#[test]
fn commodity_relevance_filter_bypasses_non_commodity_goals() {
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
    view.lot_commodities.insert(lot, CommodityKind::Waste);

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::Sleep),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let state = PlanningState::new(&snapshot);
    let (registry, _handlers) = build_registry();
    let semantics_table = build_semantics_table(&registry);
    let pick_up_id = registry
        .iter()
        .find(|def| def.name == "pick_up")
        .map(|def| def.id)
        .expect("pick_up action should be registered");
    let original = SearchCandidate {
        def_id: pick_up_id,
        authoritative_targets: vec![lot],
        planning_targets: vec![PlanningEntityRef::Authoritative(lot)],
        payload_override: Some(ActionPayload::Transport(TransportActionPayload {
            quantity: Quantity(1),
        })),
        planner_only: false,
        trace_index: None,
        expansion_trace_index: None,
    };
    let mut candidates = vec![original.clone()];

    apply_commodity_relevance_filter(
        &mut candidates,
        &goal,
        &state,
        commodity_filter_context(None, &semantics_table, &registry, &RecipeRegistry::new()),
        None,
    );

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].def_id, original.def_id);
    assert_eq!(
        candidates[0].authoritative_targets,
        original.authoritative_targets
    );
}

#[test]
fn commodity_relevance_filter_uses_active_prerequisite_commodity_for_produce_goal() {
    let actor = entity(1);
    let place = entity(10);
    let firewood_lot = entity(20);
    let waste_lot = entity(21);

    let mut view = TestBeliefView::default();
    view.alive.extend([actor, place, firewood_lot, waste_lot]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(place, EntityKind::Place);
    view.kinds.insert(firewood_lot, EntityKind::ItemLot);
    view.kinds.insert(waste_lot, EntityKind::ItemLot);
    view.effective_places.insert(actor, place);
    view.effective_places.insert(firewood_lot, place);
    view.effective_places.insert(waste_lot, place);
    view.entities_at
        .insert(place, vec![actor, firewood_lot, waste_lot]);
    view.lot_commodities
        .insert(firewood_lot, CommodityKind::Firewood);
    view.lot_commodities.insert(waste_lot, CommodityKind::Waste);

    let mut recipes = RecipeRegistry::new();
    let recipe_id = recipes.register(RecipeDefinition {
        name: "Bake Bread".to_string(),
        inputs: vec![(CommodityKind::Firewood, Quantity(1))],
        outputs: vec![(CommodityKind::Bread, Quantity(1))],
        work_ticks: NonZeroU32::new(3).unwrap(),
        required_workstation_tag: Some(WorkstationTag::Mill),
        required_tool_kinds: Vec::new(),
        body_cost_per_tick: BodyCostPerTick::zero(),
    });
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::ProduceCommodity { recipe_id }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let state = PlanningState::new(&snapshot);
    let (registry, _handlers) = build_registry();
    let semantics_table = build_semantics_table(&registry);
    let pick_up_id = registry
        .iter()
        .find(|def| def.name == "pick_up")
        .map(|def| def.id)
        .expect("pick_up action should be registered");
    let mut candidates = vec![
        SearchCandidate {
            def_id: pick_up_id,
            authoritative_targets: vec![firewood_lot],
            planning_targets: vec![PlanningEntityRef::Authoritative(firewood_lot)],
            payload_override: Some(ActionPayload::Transport(TransportActionPayload {
                quantity: Quantity(1),
            })),
            planner_only: false,
            trace_index: None,
            expansion_trace_index: None,
        },
        SearchCandidate {
            def_id: pick_up_id,
            authoritative_targets: vec![waste_lot],
            planning_targets: vec![PlanningEntityRef::Authoritative(waste_lot)],
            payload_override: Some(ActionPayload::Transport(TransportActionPayload {
                quantity: Quantity(1),
            })),
            planner_only: false,
            trace_index: None,
            expansion_trace_index: None,
        },
    ];
    let tactical_goal = TacticalGoal::AcquirePrerequisite {
        commodity: CommodityKind::Firewood,
        destination: place,
    };

    apply_commodity_relevance_filter(
        &mut candidates,
        &goal,
        &state,
        commodity_filter_context(Some(&tactical_goal), &semantics_table, &registry, &recipes),
        None,
    );

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].authoritative_targets, vec![firewood_lot]);
}

#[test]
fn search_pipeline_records_commodity_irrelevant_root_candidate_filtering() {
    let actor = entity(1);
    let seller = entity(2);
    let market = entity(10);
    let bread_lot = entity(20);
    let waste_lot = entity(21);

    let mut view = TestBeliefView::default();
    view.alive
        .extend([actor, seller, market, bread_lot, waste_lot]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(seller, EntityKind::Agent);
    view.kinds.insert(market, EntityKind::Place);
    view.kinds.insert(bread_lot, EntityKind::ItemLot);
    view.kinds.insert(waste_lot, EntityKind::ItemLot);
    view.effective_places.insert(actor, market);
    view.effective_places.insert(seller, market);
    view.effective_places.insert(bread_lot, market);
    view.effective_places.insert(waste_lot, market);
    view.entities_at
        .insert(market, vec![actor, seller, bread_lot, waste_lot]);
    view.trade_profiles
        .insert(actor, sample_trade_disposition_profile());
    view.merchandise_profiles.insert(
        seller,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([CommodityKind::Bread, CommodityKind::Waste]),
            home_facility: Some(market),
        },
    );
    view.commodity_quantities
        .insert((actor, CommodityKind::Coin), Quantity(3));
    view.commodity_quantities
        .insert((seller, CommodityKind::Bread), Quantity(1));
    view.commodity_quantities
        .insert((seller, CommodityKind::Waste), Quantity(1));
    view.lot_commodities.insert(bread_lot, CommodityKind::Bread);
    view.lot_commodities.insert(waste_lot, CommodityKind::Waste);
    view.direct_possessors.insert(bread_lot, seller);
    view.direct_possessors.insert(waste_lot, seller);
    view.direct_possessions
        .entry(seller)
        .or_default()
        .extend([bread_lot, waste_lot]);
    view.listed_lots
        .insert((market, CommodityKind::Bread), vec![bread_lot]);
    view.listed_lots
        .insert((market, CommodityKind::Waste), vec![waste_lot]);
    view.lot_sellers.insert(bread_lot, seller);
    view.lot_sellers.insert(waste_lot, seller);

    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([market]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let (registry, handlers) = build_registry();
    let mut expansions = Vec::new();

    let _result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        Some(&mut expansions),
    );

    let root = expansions
        .iter()
        .find(|summary| summary.depth == 0)
        .expect("root expansion summary should be recorded");
    assert!(
        root.root_candidates.iter().any(|candidate| {
            matches!(
                candidate.outcome,
                crate::decision_trace::RootCandidateOutcome::Filtered(
                    crate::decision_trace::RootCandidateFilterReason::CommodityIrrelevant {
                        candidate_commodity: Some(CommodityKind::Waste),
                        goal_commodity: CommodityKind::Bread,
                    },
                )
            )
        }),
        "root trace should record commodity-irrelevant candidate pruning"
    );
}

#[test]
fn search_treat_wounds_uses_two_phase_pick_up_before_heal() {
    let (mut view, actor, patient, _current_place, patient_place, medicine_place) =
        build_branching_care_view();
    view.wounds.insert(patient, vec![wound(401)]);
    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::TreatWounds { patient }),
        evidence_entities: BTreeSet::from([patient]),
        evidence_places: BTreeSet::from([patient_place, medicine_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        2,
    );

    let plan = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    )
    .into_plan()
    .expect("two-phase search should find a full care plan");

    let ops = plan
        .steps
        .iter()
        .map(|step| step.op_kind)
        .collect::<Vec<_>>();
    assert_eq!(plan.terminal_kind, PlanTerminalKind::GoalSatisfied);
    assert!(
        ops.starts_with(&[PlannerOpKind::Travel, PlannerOpKind::MoveCargo]),
        "two-phase search should first route to medicine, then pick it up: {ops:?}"
    );
    assert!(
        ops.contains(&PlannerOpKind::Heal),
        "care plan should still complete healing after the prerequisite stage: {ops:?}"
    );
}

#[test]
fn search_treat_wounds_with_zero_landmarks_preserves_two_phase_plan_shape() {
    let (mut view, actor, patient, _current_place, patient_place, medicine_place) =
        build_branching_care_view();
    view.wounds.insert(patient, vec![wound(402)]);
    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::TreatWounds { patient }),
        evidence_entities: BTreeSet::from([patient]),
        evidence_places: BTreeSet::from([patient_place, medicine_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        2,
    );
    let mut no_landmarks = cognitive(&ProfileFixture::default());
    no_landmarks.landmark_extraction_depth = 0;

    let result = super::search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &no_landmarks,
        &execution_budget(&ProfileFixture::default()),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    )
    .into_plan()
    .expect("two-phase strategic search should still find a full care plan");

    let ops = result
        .steps
        .iter()
        .map(|step| step.op_kind)
        .collect::<Vec<_>>();
    assert_eq!(result.terminal_kind, PlanTerminalKind::GoalSatisfied);
    assert!(
        ops.starts_with(&[PlannerOpKind::Travel, PlannerOpKind::MoveCargo]),
        "zero-landmark mode should preserve the prerequisite-first shape: {ops:?}"
    );
    assert!(
        ops.contains(&PlannerOpKind::Heal),
        "zero-landmark mode should still finish healing: {ops:?}"
    );
}

#[test]
fn search_produce_commodity_uses_two_phase_pick_up_before_craft() {
    let fixture = build_remote_produce_commodity_fixture();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::ProduceCommodity {
            recipe_id: fixture.recipe_id,
        }),
        evidence_entities: BTreeSet::from([fixture.mill, fixture.firewood]),
        evidence_places: BTreeSet::from([
            prototype_place_entity(PrototypePlace::VillageSquare),
            prototype_place_entity(PrototypePlace::OrchardFarm),
        ]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );

    let plan = search_plan(
        &snapshot,
        &goal,
        &fixture.semantics,
        &fixture.registry,
        &fixture.handlers,
        &ProfileFixture::default(),
        &fixture.recipes,
        &BlockerMemory::default(),
        Tick(1),
        None,
        None,
    )
    .into_plan()
    .expect("two-phase search should find a remote production plan");

    let ops = plan
        .steps
        .iter()
        .map(|step| step.op_kind)
        .collect::<Vec<_>>();
    let pick_up_index = ops
        .iter()
        .position(|op| *op == PlannerOpKind::MoveCargo)
        .expect("plan should include remote firewood pickup");
    let craft_index = ops
        .iter()
        .position(|op| *op == PlannerOpKind::Craft)
        .expect("plan should include craft after prerequisite acquisition");

    assert!(
        ops[..pick_up_index]
            .iter()
            .all(|op| *op == PlannerOpKind::Travel),
        "two-phase production should reach the remote prerequisite via travel before pickup: {ops:?}"
    );
    assert_eq!(
        plan.steps
            .iter()
            .filter(|step| step.op_kind == PlannerOpKind::Travel)
            .map(|step| step.targets.clone())
            .find(|targets| {
                *targets
                    == vec![PlanningEntityRef::Authoritative(prototype_place_entity(
                        PrototypePlace::OrchardFarm,
                    ))]
            }),
        Some(vec![PlanningEntityRef::Authoritative(
            prototype_place_entity(PrototypePlace::OrchardFarm),
        )]),
        "plan should include travel to the remote prerequisite location"
    );
    assert!(
        craft_index > pick_up_index,
        "craft should remain downstream of remote pickup: {ops:?}"
    );
    assert!(
        ops[pick_up_index + 1..craft_index]
            .iter()
            .all(|op| *op == PlannerOpKind::Travel),
        "production should return after pickup before crafting at the mill: {ops:?}"
    );
}

#[test]
fn search_produce_commodity_with_zero_landmarks_preserves_two_phase_plan_shape() {
    let fixture = build_remote_produce_commodity_fixture();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::ProduceCommodity {
            recipe_id: fixture.recipe_id,
        }),
        evidence_entities: BTreeSet::from([fixture.mill, fixture.firewood]),
        evidence_places: BTreeSet::from([
            prototype_place_entity(PrototypePlace::VillageSquare),
            prototype_place_entity(PrototypePlace::OrchardFarm),
        ]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let view = PerAgentBeliefView::from_world(fixture.actor, &fixture.world);
    let snapshot = build_planning_snapshot(
        &view,
        fixture.actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        ProfileFixture::default().snapshot_travel_horizon,
    );
    let mut no_landmarks = cognitive(&ProfileFixture::default());
    no_landmarks.landmark_extraction_depth = 0;

    let plan = super::search_plan(
        &snapshot,
        &goal,
        &fixture.semantics,
        &fixture.registry,
        &fixture.handlers,
        &no_landmarks,
        &execution_budget(&ProfileFixture::default()),
        &fixture.recipes,
        &BlockerMemory::default(),
        Tick(1),
        None,
        None,
    )
    .into_plan()
    .expect("zero-landmark mode should still find a remote production plan");

    let ops = plan
        .steps
        .iter()
        .map(|step| step.op_kind)
        .collect::<Vec<_>>();
    let pick_up_index = ops
        .iter()
        .position(|op| *op == PlannerOpKind::MoveCargo)
        .expect("zero-landmark mode should still pick up remote firewood");
    let craft_index = ops
        .iter()
        .position(|op| *op == PlannerOpKind::Craft)
        .expect("zero-landmark mode should still reach craft");

    assert!(
        ops[..pick_up_index]
            .iter()
            .all(|op| *op == PlannerOpKind::Travel),
        "zero-landmark mode should still travel to the prerequisite before pickup: {ops:?}"
    );
    assert!(
        craft_index > pick_up_index,
        "zero-landmark mode should preserve pickup-before-craft ordering: {ops:?}"
    );
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
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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
        &ProfileFixture {
            beam_width: 1,
            ..ProfileFixture::default()
        },
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
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

#[test]
#[allow(clippy::too_many_lines)]
fn search_political_goal_uses_consult_record_as_mid_plan_prerequisite_when_belief_unknown() {
    let actor = entity(1);
    let candidate = entity(2);
    let office = entity(3);
    let town = entity(10);
    let archive = entity(11);
    let hall = entity(12);
    let record = entity(20);
    let mut view = TestBeliefView::default();
    view.alive
        .extend([actor, candidate, office, record, town, archive, hall]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(candidate, EntityKind::Agent);
    view.kinds.insert(office, EntityKind::Office);
    view.kinds.insert(record, EntityKind::Record);
    view.kinds.insert(town, EntityKind::Place);
    view.kinds.insert(archive, EntityKind::Place);
    view.kinds.insert(hall, EntityKind::Place);
    view.effective_places.insert(actor, town);
    view.effective_places.insert(candidate, hall);
    view.effective_places.insert(office, hall);
    view.effective_places.insert(record, archive);
    view.entities_at.insert(town, vec![actor]);
    view.entities_at.insert(archive, vec![record]);
    view.entities_at.insert(hall, vec![candidate, office]);
    view.carry_capacities.insert(actor, LoadUnits(10));
    view.entity_loads.insert(actor, LoadUnits(0));
    view.consultation_speed_factors
        .insert(actor, Permille::new(500).unwrap());
    view.office_data.insert(
        office,
        worldwake_core::OfficeData {
            title: "Steward".to_string(),
            seat: hall,
            jurisdiction: BTreeSet::from([hall]),
            succession_law: worldwake_core::SuccessionLaw::Support,
            eligibility_rules: Vec::new(),
            succession_period_ticks: 10,
            vacancy_since: Some(Tick(2)),
        },
    );
    view.record_data.insert(
        record,
        worldwake_core::RecordData {
            record_kind: worldwake_core::RecordKind::OfficeRegister,
            home_place: archive,
            issuer: actor,
            consultation_ticks: 4,
            max_entries_per_consult: 2,
            entries: vec![worldwake_core::InstitutionalRecordEntry {
                entry_id: worldwake_core::RecordEntryId(0),
                claim: worldwake_core::InstitutionalClaim::OfficeHolder {
                    office,
                    holder: None,
                    effective_tick: Tick(2),
                },
                recorded_tick: Tick(2),
                supersedes: None,
            }],
            next_entry_id: 1,
        },
    );
    view.adjacent
        .insert(town, vec![(archive, NonZeroU32::new(1).unwrap())]);
    view.adjacent.insert(
        archive,
        vec![
            (town, NonZeroU32::new(1).unwrap()),
            (hall, NonZeroU32::new(1).unwrap()),
        ],
    );
    view.adjacent
        .insert(hall, vec![(archive, NonZeroU32::new(1).unwrap())]);

    let (registry, handlers) = build_registry();
    let semantics = build_semantics_table(&registry);
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([candidate, office, record]),
        &BTreeSet::from([town, archive, hall]),
        2,
    );
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::SupportCandidateForOffice { office, candidate }),
        evidence_entities: BTreeSet::from([candidate, office, record]),
        evidence_places: BTreeSet::from([archive, hall]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };

    let result = search_plan(
        &snapshot,
        &goal,
        &semantics,
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    );

    let plan = match result {
        PlanSearchResult::Found(plan) => plan,
        other => panic!("expected plan, got {other:?}"),
    };
    let op_kinds = plan
        .steps
        .iter()
        .map(|step| step.op_kind)
        .collect::<Vec<_>>();
    assert_eq!(
        op_kinds,
        vec![
            PlannerOpKind::Travel,
            PlannerOpKind::ConsultRecord,
            PlannerOpKind::Travel,
            PlannerOpKind::DeclareSupport,
        ]
    );
}

#[test]
fn search_political_goal_skips_consult_record_when_vacancy_belief_is_already_certain() {
    let actor = entity(1);
    let candidate = entity(2);
    let office = entity(3);
    let town = entity(10);
    let hall = entity(12);
    let mut view = TestBeliefView::default();
    view.alive.extend([actor, candidate, office, town, hall]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(candidate, EntityKind::Agent);
    view.kinds.insert(office, EntityKind::Office);
    view.kinds.insert(town, EntityKind::Place);
    view.kinds.insert(hall, EntityKind::Place);
    view.effective_places.insert(actor, town);
    view.effective_places.insert(candidate, hall);
    view.effective_places.insert(office, hall);
    view.entities_at.insert(town, vec![actor]);
    view.entities_at.insert(hall, vec![candidate, office]);
    view.carry_capacities.insert(actor, LoadUnits(10));
    view.entity_loads.insert(actor, LoadUnits(0));
    view.office_holder_beliefs.insert(
        office,
        worldwake_core::InstitutionalBeliefRead::Certain(None),
    );
    view.office_data.insert(
        office,
        worldwake_core::OfficeData {
            title: "Steward".to_string(),
            seat: hall,
            jurisdiction: BTreeSet::from([hall]),
            succession_law: worldwake_core::SuccessionLaw::Support,
            eligibility_rules: Vec::new(),
            succession_period_ticks: 10,
            vacancy_since: Some(Tick(2)),
        },
    );
    view.adjacent
        .insert(town, vec![(hall, NonZeroU32::new(1).unwrap())]);
    view.adjacent
        .insert(hall, vec![(town, NonZeroU32::new(1).unwrap())]);

    let (registry, handlers) = build_registry();
    let semantics = build_semantics_table(&registry);
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([candidate, office]),
        &BTreeSet::from([town, hall]),
        1,
    );
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::SupportCandidateForOffice { office, candidate }),
        evidence_entities: BTreeSet::from([candidate, office]),
        evidence_places: BTreeSet::from([hall]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };

    let result = search_plan(
        &snapshot,
        &goal,
        &semantics,
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    );

    let plan = match result {
        PlanSearchResult::Found(plan) => plan,
        other => panic!("expected plan, got {other:?}"),
    };
    assert!(
        plan.steps
            .iter()
            .all(|step| step.op_kind != PlannerOpKind::ConsultRecord)
    );
}

#[test]
fn planned_plan_carries_searched_opportunity_key() {
    let actor = entity(1);
    let office = entity(3);
    let hall = entity(12);
    let mut view = TestBeliefView::default();
    view.alive.extend([actor, office, hall]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(office, EntityKind::Office);
    view.kinds.insert(hall, EntityKind::Place);
    view.effective_places.insert(actor, hall);
    view.effective_places.insert(office, hall);
    view.entities_at.insert(hall, vec![actor, office]);
    view.carry_capacities.insert(actor, LoadUnits(10));
    view.entity_loads.insert(actor, LoadUnits(0));
    view.office_holder_beliefs.insert(
        office,
        worldwake_core::InstitutionalBeliefRead::Certain(None),
    );
    view.office_data.insert(
        office,
        worldwake_core::OfficeData {
            title: "War Chief".to_string(),
            seat: hall,
            jurisdiction: BTreeSet::from([hall]),
            succession_law: worldwake_core::SuccessionLaw::Force,
            eligibility_rules: Vec::new(),
            succession_period_ticks: 10,
            vacancy_since: Some(Tick(2)),
        },
    );

    let (registry, handlers) = build_registry();
    let semantics = build_semantics_table(&registry);
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([office]),
        &BTreeSet::from([hall]),
        0,
    );
    let goal_key = GoalKey::from(GoalKind::ClaimOffice { office });
    let opportunity = worldwake_core::OpportunityKey {
        goal_key,
        anchor: worldwake_core::OpportunityAnchor::Place(hall),
    };
    let goal = GoalOffer {
        anchor: opportunity.anchor,
        key: goal_key,
        evidence_entities: BTreeSet::from([office]),
        evidence_places: BTreeSet::from([hall]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };

    let result = search_plan(
        &snapshot,
        &goal,
        &semantics,
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    );

    let plan = match result {
        PlanSearchResult::Found(plan) => plan,
        other => panic!("expected plan, got {other:?}"),
    };

    assert_eq!(plan.goal, goal_key);
    assert_eq!(plan.opportunity, opportunity);
}

#[test]
fn search_trace_records_force_claim_root_candidate_outcomes() {
    let actor = entity(1);
    let office = entity(3);
    let hall = entity(12);
    let mut view = TestBeliefView::default();
    view.alive.extend([actor, office, hall]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(office, EntityKind::Office);
    view.kinds.insert(hall, EntityKind::Place);
    view.effective_places.insert(actor, hall);
    view.effective_places.insert(office, hall);
    view.entities_at.insert(hall, vec![actor, office]);
    view.carry_capacities.insert(actor, LoadUnits(10));
    view.entity_loads.insert(actor, LoadUnits(0));
    view.office_holder_beliefs.insert(
        office,
        worldwake_core::InstitutionalBeliefRead::Certain(None),
    );
    view.office_data.insert(
        office,
        worldwake_core::OfficeData {
            title: "War Chief".to_string(),
            seat: hall,
            jurisdiction: BTreeSet::from([hall]),
            succession_law: worldwake_core::SuccessionLaw::Force,
            eligibility_rules: Vec::new(),
            succession_period_ticks: 10,
            vacancy_since: Some(Tick(2)),
        },
    );

    let (registry, handlers) = build_registry();
    let semantics = build_semantics_table(&registry);
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([office]),
        &BTreeSet::from([hall]),
        0,
    );
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::ClaimOffice { office }),
        evidence_entities: BTreeSet::from([office]),
        evidence_places: BTreeSet::from([hall]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let mut expansions = Vec::new();

    let result = search_plan(
        &snapshot,
        &goal,
        &semantics,
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        Some(&mut expansions),
    );
    let plan = match result {
        PlanSearchResult::Found(plan) => plan,
        other => panic!("expected plan, got {other:?}"),
    };
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::PressForceClaim);

    let root = expansions
        .iter()
        .find(|summary| summary.depth == 0)
        .expect("root expansion summary should be recorded");
    let press_force_claim = root
        .root_candidates
        .iter()
        .find(|candidate| candidate.op_kind == Some(PlannerOpKind::PressForceClaim))
        .expect("force-claim root candidate should be traced");
    assert_eq!(
        press_force_claim.outcome,
        crate::decision_trace::RootCandidateOutcome::Expanded
    );

    let declare_support = root
        .root_candidates
        .iter()
        .find(|candidate| candidate.op_kind == Some(PlannerOpKind::DeclareSupport))
        .expect("declare_support root candidate should be traced");
    assert_eq!(
        declare_support.outcome,
        crate::decision_trace::RootCandidateOutcome::Skipped(
            crate::decision_trace::RootCandidateSkipReason::PayloadOverride(
                crate::decision_trace::PayloadOverrideFailureReason::UnsupportedGoal,
            ),
        )
    );
}

#[test]
fn search_trace_records_omitted_relevant_operator_when_no_matching_action_def_exists() {
    let actor = entity(1);
    let place = entity(10);
    let office = entity(11);
    let mut view = TestBeliefView::default();
    view.alive.extend([actor, place, office]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(place, EntityKind::Place);
    view.kinds.insert(office, EntityKind::Office);
    view.effective_places.insert(actor, place);
    view.effective_places.insert(office, place);
    view.entities_at.insert(place, vec![actor, office]);

    let registry = ActionDefRegistry::new();
    let handlers = worldwake_sim::ActionHandlerRegistry::new();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::ClaimOffice { office }),
        evidence_entities: BTreeSet::from([office]),
        evidence_places: BTreeSet::from([place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([office]),
        &BTreeSet::from([place]),
        0,
    );
    let mut expansions = Vec::new();

    let _result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        Some(&mut expansions),
    );

    let root = expansions
        .iter()
        .find(|summary| summary.depth == 0)
        .expect("root expansion summary should be recorded");
    assert!(
        !root
            .root_candidates
            .iter()
            .any(|candidate| { candidate.op_kind == Some(PlannerOpKind::AskWitness) })
    );
    assert!(root.root_candidates.is_empty());
    assert!(root.root_omissions.iter().any(|omission| {
        omission.op_kind == PlannerOpKind::PressForceClaim
            && omission.reason
                == crate::decision_trace::RootOperatorOmissionReason::NoMatchingActionDef
    }));
}

#[test]
fn search_trace_annotates_root_candidate_with_omitted_anchor_reason() {
    let actor = entity(1);
    let town = entity(10);
    let listener = entity(20);
    let subject = entity(21);
    let reason = OmissionReason::OverBudget {
        budget: 5,
        candidates_seen: 12,
    };
    let mut view = TestBeliefView::default();
    view.alive.extend([actor, town, listener]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(town, EntityKind::Place);
    view.kinds.insert(listener, EntityKind::Agent);
    view.effective_places.insert(actor, town);
    view.effective_places.insert(listener, town);
    view.entities_at.insert(town, vec![actor]);
    view.belief_stores.insert(
        actor,
        AgentBeliefStore {
            observation_omission_log: ObservationOmissionLog {
                entries: VecDeque::from([ObservationOmission {
                    omitted_entity: listener,
                    reason,
                    observed_tick: Tick(7),
                }]),
            },
            ..AgentBeliefStore::default()
        },
    );

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Entity(listener),
        key: GoalKey::from(GoalKind::ShareBelief {
            listener,
            topic: TellTopic::EntityBelief { subject },
            communication_class: worldwake_core::CommunicationClass::Gossip,
        }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot =
        build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::from([town]), 0);
    let mut expansions = Vec::new();

    let _result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(7),
        None,
        Some(&mut expansions),
    );

    let root = expansions
        .iter()
        .find(|summary| summary.depth == 0)
        .expect("root expansion summary should be recorded");
    let tell_candidate = root
        .root_candidates
        .iter()
        .find(|candidate| {
            candidate.op_kind == Some(PlannerOpKind::Tell)
                && candidate.authoritative_targets == vec![listener]
        })
        .expect("synthesized tell candidate should be traced");
    assert_eq!(tell_candidate.omitted_anchor, Some(reason));
}

#[test]
fn fulfill_post_notice_search_finds_travel_then_post_notice_progress_barrier() {
    let actor = entity(1);
    let origin = entity(10);
    let posting_place = entity(11);
    let mut view = TestBeliefView::default();
    view.alive.extend([actor, origin, posting_place]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(origin, EntityKind::Place);
    view.kinds.insert(posting_place, EntityKind::Place);
    view.effective_places.insert(actor, origin);
    view.entities_at.insert(origin, vec![actor]);
    view.adjacent
        .insert(origin, vec![(posting_place, NonZeroU32::new(1).unwrap())]);
    view.adjacent
        .insert(posting_place, vec![(origin, NonZeroU32::new(1).unwrap())]);

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(posting_place),
        key: GoalKey::from(GoalKind::PostNotice {
            posting: ArtifactPostingContext {
                posting_place,
                issuing_authority: None,
                expires_at: Some(Tick(7)),
                jurisdiction: Some(posting_place),
            },
            topic: NoticeTopic::ThreatWarning {
                place: posting_place,
            },
        }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([posting_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let mut expansions = Vec::new();

    let result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        Some(&mut expansions),
    );
    let plan = match result {
        PlanSearchResult::Found(plan) => plan,
        other => panic!(
            "search should find a Travel+PostNotice plan, got {other:?} with expansions {expansions:?}"
        ),
    };

    assert_eq!(plan.terminal_kind, PlanTerminalKind::ProgressBarrier);
    assert_eq!(plan.steps.len(), 2);
    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Travel);
    assert_eq!(plan.steps[1].op_kind, PlannerOpKind::PostNotice);
    assert_eq!(
        plan.steps[1]
            .payload_override
            .as_ref()
            .and_then(ActionPayload::as_post_notice),
        Some(&worldwake_sim::PostNoticeActionPayload {
            posting_place,
            issuing_authority: None,
            expires_at: Some(Tick(7)),
            jurisdiction: Some(posting_place),
            topic: NoticeTopic::ThreatWarning {
                place: posting_place,
            },
        })
    );
}

#[test]
fn fulfill_post_notice_search_finds_same_place_post_notice_progress_barrier() {
    let actor = entity(1);
    let posting_place = entity(11);
    let mut view = TestBeliefView::default();
    view.alive.extend([actor, posting_place]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(posting_place, EntityKind::Place);
    view.effective_places.insert(actor, posting_place);
    view.entities_at.insert(posting_place, vec![actor]);

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(posting_place),
        key: GoalKey::from(GoalKind::PostNotice {
            posting: ArtifactPostingContext {
                posting_place,
                issuing_authority: None,
                expires_at: Some(Tick(7)),
                jurisdiction: Some(posting_place),
            },
            topic: NoticeTopic::ThreatWarning {
                place: posting_place,
            },
        }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([posting_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let mut expansions = Vec::new();

    let result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        Some(&mut expansions),
    );
    let plan = match result {
        PlanSearchResult::Found(plan) => plan,
        other => panic!(
            "search should find a same-place PostNotice plan, got {other:?} with expansions {expansions:?}"
        ),
    };

    assert_eq!(plan.terminal_kind, PlanTerminalKind::ProgressBarrier);
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::PostNotice);
    assert_eq!(
        plan.steps[0]
            .payload_override
            .as_ref()
            .and_then(ActionPayload::as_post_notice),
        Some(&worldwake_sim::PostNoticeActionPayload {
            posting_place,
            issuing_authority: None,
            expires_at: Some(Tick(7)),
            jurisdiction: Some(posting_place),
            topic: NoticeTopic::ThreatWarning {
                place: posting_place,
            },
        })
    );
}

#[test]
fn fulfill_post_bounty_search_finds_travel_then_post_bounty_progress_barrier() {
    let actor = entity(1);
    let origin = entity(10);
    let posting_place = entity(11);
    let target = entity(20);
    let mut view = TestBeliefView::default();
    view.alive.extend([actor, origin, posting_place, target]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(origin, EntityKind::Place);
    view.kinds.insert(posting_place, EntityKind::Place);
    view.kinds.insert(target, EntityKind::Agent);
    view.effective_places.insert(actor, origin);
    view.effective_places.insert(target, posting_place);
    view.entities_at.insert(origin, vec![actor]);
    view.entities_at.insert(posting_place, vec![target]);
    view.adjacent
        .insert(origin, vec![(posting_place, NonZeroU32::new(1).unwrap())]);
    view.adjacent
        .insert(posting_place, vec![(origin, NonZeroU32::new(1).unwrap())]);

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(posting_place),
        key: GoalKey::from(GoalKind::PostBounty {
            posting: ArtifactPostingContext {
                posting_place,
                issuing_authority: None,
                expires_at: Some(Tick(9)),
                jurisdiction: Some(posting_place),
            },
            terms: BountyTerms {
                target: BountyTarget::EliminateEntity { target },
                proof_requirement: ProofRequirement::SelfReport,
                reward_commodity: CommodityKind::Coin,
                reward_quantity: Quantity(12),
                reward_source: RewardSource::PersonalFunds { issuer: actor },
                claim_place: posting_place,
            },
        }),
        evidence_entities: BTreeSet::from([target]),
        evidence_places: BTreeSet::from([posting_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let mut expansions = Vec::new();

    let result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        Some(&mut expansions),
    );
    let plan = match result {
        PlanSearchResult::Found(plan) => plan,
        other => panic!(
            "search should find a Travel+PostBounty plan, got {other:?} with expansions {expansions:?}"
        ),
    };

    assert_eq!(plan.terminal_kind, PlanTerminalKind::ProgressBarrier);
    assert_eq!(plan.steps.len(), 2);
    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Travel);
    assert_eq!(plan.steps[1].op_kind, PlannerOpKind::PostBounty);
    assert_eq!(
        plan.steps[1]
            .payload_override
            .as_ref()
            .and_then(ActionPayload::as_post_bounty),
        Some(&worldwake_sim::PostBountyActionPayload {
            posting_place,
            issuing_authority: None,
            expires_at: Some(Tick(9)),
            jurisdiction: Some(posting_place),
            target: BountyTarget::EliminateEntity { target },
            proof_requirement: ProofRequirement::SelfReport,
            reward_commodity: CommodityKind::Coin,
            reward_quantity: Quantity(12),
            reward_source: RewardSource::PersonalFunds { issuer: actor },
            claim_place: posting_place,
        })
    );
}

#[test]
fn search_trace_records_trade_omission_when_goal_side_target_derivation_fails() {
    let actor = entity(1);
    let town = entity(10);
    let seller_a = entity(20);
    let seller_b = entity(21);
    let mut view = TestBeliefView::default();
    view.alive.extend([actor, town, seller_a, seller_b]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(town, EntityKind::Place);
    view.kinds.insert(seller_a, EntityKind::Agent);
    view.kinds.insert(seller_b, EntityKind::Agent);
    view.effective_places.insert(actor, town);
    view.entities_at.insert(town, vec![actor]);
    view.needs.insert(
        actor,
        HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
    );
    view.thresholds.insert(actor, DriveThresholds::default());

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        evidence_entities: BTreeSet::from([seller_a, seller_b]),
        evidence_places: BTreeSet::from([town]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([seller_a, seller_b]),
        &BTreeSet::from([town]),
        0,
    );
    let mut expansions = Vec::new();

    let _result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        Some(&mut expansions),
    );

    let root = expansions
        .iter()
        .find(|summary| summary.depth == 0)
        .expect("root expansion summary should be recorded");
    assert!(
        !root
            .root_candidates
            .iter()
            .any(|candidate| { candidate.op_kind == Some(PlannerOpKind::AskWitness) })
    );
    assert!(root.root_candidates.is_empty());
    assert!(root.root_omissions.iter().any(|omission| {
        omission.op_kind == PlannerOpKind::Trade
            && omission.reason
                == crate::decision_trace::RootOperatorOmissionReason::SynthesisTargetDerivationFailed
    }));
}

#[test]
fn search_trace_records_ask_witness_omission_when_no_stale_epistemic_subjects_exist() {
    let actor = entity(1);
    let town = entity(10);
    let remote = entity(11);
    let subject = entity(20);
    let mut view = TestBeliefView::default();
    view.alive.extend([actor, town, remote, subject]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(town, EntityKind::Place);
    view.kinds.insert(remote, EntityKind::Place);
    view.kinds.insert(subject, EntityKind::Facility);
    view.current_tick = Tick(50);
    view.effective_places.insert(actor, town);
    view.entities_at.insert(town, vec![actor]);
    view.epistemic_profiles.insert(actor, epistemic_profile());
    view.known_entity_beliefs.insert(
        actor,
        vec![(
            subject,
            believed_entity_state_at(
                remote,
                Tick(50),
                Some(ResourceSource {
                    commodity: CommodityKind::Bread,
                    available_quantity: Quantity(4),
                    max_quantity: Quantity(4),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                    extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                    extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                }),
            ),
        )],
    );

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(remote),
        key: GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Bread,
        }),
        evidence_entities: BTreeSet::from([subject]),
        evidence_places: BTreeSet::from([remote]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([subject]),
        &BTreeSet::from([town, remote]),
        2,
    );
    let mut expansions = Vec::new();

    let _result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(50),
        None,
        Some(&mut expansions),
    );

    let root = expansions
        .iter()
        .find(|summary| summary.depth == 0)
        .expect("root expansion summary should be recorded");
    assert!(root.root_omissions.iter().any(|omission| {
        omission.op_kind == PlannerOpKind::AskWitness
            && omission.reason
                == crate::decision_trace::RootOperatorOmissionReason::ConditionalBarrierUnavailable
            && omission.detail
                == Some(
                    crate::decision_trace::RootOperatorOmissionDetail::AskWitness(
                        crate::decision_trace::AskWitnessOmissionDetail::NoStaleEpistemicSubjects,
                    ),
                )
    }));
}

#[test]
fn search_trace_records_ask_witness_omission_when_no_witness_affordance_exists() {
    let actor = entity(1);
    let town = entity(10);
    let remote = entity(11);
    let subject = entity(20);
    let mut view = TestBeliefView::default();
    view.alive.extend([actor, town, remote, subject]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(town, EntityKind::Place);
    view.kinds.insert(remote, EntityKind::Place);
    view.kinds.insert(subject, EntityKind::Facility);
    view.current_tick = Tick(50);
    view.effective_places.insert(actor, town);
    view.entities_at.insert(town, vec![actor]);
    view.epistemic_profiles.insert(actor, epistemic_profile());
    view.known_entity_beliefs.insert(
        actor,
        vec![(
            subject,
            believed_entity_state_at(
                remote,
                Tick(0),
                Some(ResourceSource {
                    commodity: CommodityKind::Bread,
                    available_quantity: Quantity(4),
                    max_quantity: Quantity(4),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                    extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                    extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                }),
            ),
        )],
    );

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(remote),
        key: GoalKey::from(GoalKind::RestockCommodity {
            commodity: CommodityKind::Bread,
        }),
        evidence_entities: BTreeSet::from([subject]),
        evidence_places: BTreeSet::from([remote]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([subject]),
        &BTreeSet::from([town, remote]),
        2,
    );
    let mut expansions = Vec::new();

    let _result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(50),
        None,
        Some(&mut expansions),
    );

    let root = expansions
        .iter()
        .find(|summary| summary.depth == 0)
        .expect("root expansion summary should be recorded");
    assert!(root.root_omissions.iter().any(|omission| {
        omission.op_kind == PlannerOpKind::AskWitness
            && omission.reason
                == crate::decision_trace::RootOperatorOmissionReason::ConditionalBarrierUnavailable
            && omission.detail
                == Some(
                    crate::decision_trace::RootOperatorOmissionDetail::AskWitness(
                        crate::decision_trace::AskWitnessOmissionDetail::NoWitnessAffordance,
                    ),
                )
    }));
}

#[test]
fn search_trace_omits_trade_root_candidate_without_trade_disposition_profile() {
    let actor = entity(1);
    let town = entity(10);
    let seller = entity(20);
    let seller_lot = entity(100);
    let mut view = TestBeliefView::default();
    view.alive.extend([actor, town, seller, seller_lot]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(town, EntityKind::Place);
    view.kinds.insert(seller, EntityKind::Agent);
    view.kinds.insert(seller_lot, EntityKind::ItemLot);
    view.effective_places.insert(actor, town);
    view.effective_places.insert(seller, town);
    view.effective_places.insert(seller_lot, town);
    view.entities_at
        .insert(town, vec![actor, seller, seller_lot]);
    view.lot_commodities
        .insert(seller_lot, CommodityKind::Bread);
    view.lot_sellers.insert(seller_lot, seller);
    view.direct_possessors.insert(seller_lot, seller);
    view.direct_possessions
        .entry(seller)
        .or_default()
        .push(seller_lot);
    view.needs.insert(
        actor,
        HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
    );
    view.thresholds.insert(actor, DriveThresholds::default());
    view.merchandise_profiles.insert(
        seller,
        MerchandiseProfile {
            sale_kinds: BTreeSet::from([CommodityKind::Bread]),
            home_facility: None,
        },
    );
    view.commodity_quantities
        .insert((seller, CommodityKind::Bread), Quantity(3));
    view.commodity_quantities
        .insert((actor, CommodityKind::Coin), Quantity(5));

    let (registry, handlers) = build_registry();
    let goal = acquire_goal(CommodityKind::Bread);
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &BTreeSet::from([seller]),
        &BTreeSet::from([town]),
        0,
    );
    let mut expansions = Vec::new();

    let result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        Some(&mut expansions),
    );

    assert!(!result.is_found());
    let root = expansions
        .iter()
        .find(|summary| summary.depth == 0)
        .expect("root expansion summary should be recorded");
    assert!(
        root.root_candidates
            .iter()
            .all(|candidate| candidate.op_kind != Some(PlannerOpKind::Trade)),
        "trade root candidate should be absent when no trade disposition profile exists"
    );
}

// ── S23-004: Place-scoped blocker pruning in plan search ──

#[test]
fn place_scoped_blocker_prunes_candidate_at_blocked_place() {
    // Actor at town with local bread. Blocker says "consume bread is blocked
    // at town". The eat candidate is at town → should be pruned → no plan.
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
    let goal = consume_goal(CommodityKind::Bread);

    // Without blocker: plan is found.
    let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
    let no_blocker = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    );
    assert!(
        no_blocker.is_found(),
        "without blocker, consume plan should be found"
    );

    // With place-scoped blocker at town: plan should NOT be found.
    let mut blocked = BlockerMemory::default();
    blocked.record(Blocker {
        blocker_key: BlockerKey {
            goal_key: goal.key,
            place: Some(town),
            target: None,
            action_def: None,
        },
        blocking_fact: BlockingFact::SourceDepleted,
        diagnostic_context: None,
        observed_tick: Tick(0),
        expires_tick: Tick(100),
        clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
        baseline_snapshot: None,
    });
    let with_blocker = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &blocked,
        Tick(1),
        None,
        None,
    );
    assert!(
        !with_blocker.is_found(),
        "place-scoped blocker at actor's place should prune all candidates"
    );
}

#[test]
fn place_scoped_blocker_does_not_prune_candidate_at_different_place() {
    // Actor at town with bread at town. Blocker at field (different place).
    // Eat candidate is at town → blocker at field should NOT prune it.
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
    let goal = consume_goal(CommodityKind::Bread);
    let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);

    let mut blocked = BlockerMemory::default();
    blocked.record(Blocker {
        blocker_key: BlockerKey {
            goal_key: goal.key,
            place: Some(field), // different place
            target: None,
            action_def: None,
        },
        blocking_fact: BlockingFact::SourceDepleted,
        diagnostic_context: None,
        observed_tick: Tick(0),
        expires_tick: Tick(100),
        clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
        baseline_snapshot: None,
    });
    let result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &blocked,
        Tick(1),
        None,
        None,
    );
    assert!(
        result.is_found(),
        "blocker at different place should not prune local candidate"
    );
}

#[test]
fn travel_action_uses_destination_as_place_for_blocker_check() {
    // Actor at town, bread at field. Blocker at field. Travel-to-field
    // should be pruned because the travel destination is the blocked place.
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
    let mut bread_belief = believed_entity_state_at(field, Tick(1), None);
    bread_belief.believed_kind = Some(EntityKind::ItemLot);
    bread_belief
        .last_known_inventory
        .insert(CommodityKind::Bread, Quantity(1));
    remember_entity(&mut view, actor, bread, bread_belief);
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
    let goal = consume_goal(CommodityKind::Bread);
    let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);

    // Without blocker: plan includes travel to field.
    let no_blocker = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    );
    assert!(
        no_blocker.is_found(),
        "baseline without blocker should find plan"
    );

    // With blocker at field: travel-to-field should be pruned → no plan.
    let mut blocked = BlockerMemory::default();
    blocked.record(Blocker {
        blocker_key: BlockerKey {
            goal_key: goal.key,
            place: Some(field),
            target: None,
            action_def: None,
        },
        blocking_fact: BlockingFact::SourceDepleted,
        diagnostic_context: None,
        observed_tick: Tick(0),
        expires_tick: Tick(100),
        clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
        baseline_snapshot: None,
    });
    let with_blocker = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &blocked,
        Tick(1),
        None,
        None,
    );
    assert!(
        !with_blocker.is_found(),
        "blocker at travel destination should prune travel-to-field, leaving no viable plan"
    );
}

#[test]
fn candidate_pruned_by_blocker_records_place_blocker_trace() {
    // Same setup as place_scoped_blocker_prunes test but with trace collection.
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
    let goal = consume_goal(CommodityKind::Bread);
    let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);

    let mut blocked = BlockerMemory::default();
    blocked.record(Blocker {
        blocker_key: BlockerKey {
            goal_key: goal.key,
            place: Some(town),
            target: None,
            action_def: None,
        },
        blocking_fact: BlockingFact::SourceDepleted,
        diagnostic_context: None,
        observed_tick: Tick(0),
        expires_tick: Tick(100),
        clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
        baseline_snapshot: None,
    });

    let mut summaries = Vec::new();
    let _result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &blocked,
        Tick(1),
        None,
        Some(&mut summaries),
    );

    // At least one expansion summary should exist (root expansion).
    assert!(
        !summaries.is_empty(),
        "search should produce at least one expansion summary"
    );
    let root = &summaries[0];
    // At least one root candidate should be filtered with PlaceBlocker.
    let has_place_blocker = root.root_candidates.iter().any(|c| {
        matches!(
            c.outcome,
            crate::decision_trace::RootCandidateOutcome::Filtered(
                crate::decision_trace::RootCandidateFilterReason::PlaceBlocker { .. }
            )
        )
    });
    assert!(
        has_place_blocker,
        "trace should record PlaceBlocker filter for pruned candidates"
    );

    // Verify the PlaceBlocker carries the correct place and fact.
    let place_blocker_trace = root
        .root_candidates
        .iter()
        .find_map(|c| match &c.outcome {
            crate::decision_trace::RootCandidateOutcome::Filtered(
                crate::decision_trace::RootCandidateFilterReason::PlaceBlocker {
                    place,
                    blocking_fact,
                },
            ) => Some((*place, *blocking_fact)),
            _ => None,
        })
        .expect("should find a PlaceBlocker trace entry");
    assert_eq!(place_blocker_trace.0, Some(town));
    assert_eq!(place_blocker_trace.1, BlockingFact::SourceDepleted);
}

// ── Remote pursuit search tests ──────────────────────────────────────

#[test]
fn remote_pursuit_travel_then_attack_for_raid_target() {
    let actor = entity(1);
    let target = entity(2);
    let actor_place = entity(10);
    let remote_place = entity(11);

    let mut view = TestBeliefView {
        current_tick: Tick(10),
        ..TestBeliefView::default()
    };
    view.alive
        .extend([actor, target, actor_place, remote_place]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(target, EntityKind::Agent);
    view.kinds.insert(actor_place, EntityKind::Place);
    view.kinds.insert(remote_place, EntityKind::Place);
    view.effective_places.insert(actor, actor_place);
    // Target believed at remote place.
    view.effective_places.insert(target, remote_place);
    view.entities_at.insert(actor_place, vec![actor]);
    view.entities_at.insert(remote_place, vec![target]);
    view.thresholds.insert(actor, DriveThresholds::default());
    // Connect places bidirectionally.
    view.adjacent.insert(
        actor_place,
        vec![(remote_place, NonZeroU32::new(2).unwrap())],
    );
    view.adjacent.insert(
        remote_place,
        vec![(actor_place, NonZeroU32::new(2).unwrap())],
    );
    // Actor believes target is at remote_place.
    view.known_entity_beliefs.insert(
        actor,
        vec![(
            target,
            BelievedEntityState {
                believed_kind: None,
                last_known_place: Some(remote_place),
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
                    Tick(9),
                    PerceptionSource::DirectObservation,
                )
            },
        )],
    );

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Entity(target),
        key: GoalKey::from(GoalKind::RaidTarget { target }),
        evidence_entities: BTreeSet::from([target]),
        evidence_places: BTreeSet::from([remote_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(10),
        None,
        None,
    );
    let plan = result
        .into_plan()
        .expect("search should find a Travel+Attack plan for remote raid target");

    assert!(
        plan.steps.len() >= 2,
        "plan should have at least 2 steps (Travel + Attack), got {}",
        plan.steps.len()
    );
    assert_eq!(
        plan.steps[0].op_kind,
        PlannerOpKind::Travel,
        "first step should be Travel to remote place"
    );
    assert_eq!(
        plan.steps[1].op_kind,
        PlannerOpKind::Attack,
        "second step should be Attack at remote place"
    );
}

#[test]
fn remote_pursuit_travel_then_attack_for_engage_hostile() {
    let actor = entity(1);
    let target = entity(2);
    let actor_place = entity(10);
    let remote_place = entity(11);

    let mut view = TestBeliefView {
        current_tick: Tick(10),
        ..TestBeliefView::default()
    };
    view.alive
        .extend([actor, target, actor_place, remote_place]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(target, EntityKind::Agent);
    view.kinds.insert(actor_place, EntityKind::Place);
    view.kinds.insert(remote_place, EntityKind::Place);
    view.effective_places.insert(actor, actor_place);
    view.effective_places.insert(target, remote_place);
    view.entities_at.insert(actor_place, vec![actor]);
    view.entities_at.insert(remote_place, vec![target]);
    view.thresholds.insert(actor, DriveThresholds::default());
    view.hostiles.insert(actor, vec![target]);
    view.adjacent.insert(
        actor_place,
        vec![(remote_place, NonZeroU32::new(2).unwrap())],
    );
    view.adjacent.insert(
        remote_place,
        vec![(actor_place, NonZeroU32::new(2).unwrap())],
    );
    view.known_entity_beliefs.insert(
        actor,
        vec![(
            target,
            BelievedEntityState {
                believed_kind: None,
                last_known_place: Some(remote_place),
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
                    Tick(9),
                    PerceptionSource::DirectObservation,
                )
            },
        )],
    );

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Entity(target),
        key: GoalKey::from(GoalKind::EngageHostile { target }),
        evidence_entities: BTreeSet::from([target]),
        evidence_places: BTreeSet::from([remote_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(10),
        None,
        None,
    );
    let plan = result
        .into_plan()
        .expect("search should find a Travel+Attack plan for remote engage hostile");

    assert!(
        plan.steps.len() >= 2,
        "plan should have at least 2 steps (Travel + Attack), got {}",
        plan.steps.len()
    );
    assert_eq!(
        plan.steps[0].op_kind,
        PlannerOpKind::Travel,
        "first step should be Travel to remote place"
    );
    assert_eq!(
        plan.steps[1].op_kind,
        PlannerOpKind::Attack,
        "second step should be Attack at remote place"
    );
}

#[test]
fn explore_location_search_finds_travel_plan_to_target_place() {
    let actor = entity(1);
    let actor_place = entity(10);
    let target_place = entity(11);

    let mut view = TestBeliefView {
        current_tick: Tick(10),
        ..TestBeliefView::default()
    };
    view.alive.extend([actor, actor_place, target_place]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(actor_place, EntityKind::Place);
    view.kinds.insert(target_place, EntityKind::Place);
    view.effective_places.insert(actor, actor_place);
    view.entities_at.insert(actor_place, vec![actor]);
    view.entities_at.entry(target_place).or_default();
    view.thresholds.insert(actor, DriveThresholds::default());
    view.adjacent.insert(
        actor_place,
        vec![(target_place, NonZeroU32::new(2).unwrap())],
    );
    view.adjacent.insert(
        target_place,
        vec![(actor_place, NonZeroU32::new(2).unwrap())],
    );

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::Place(target_place),
        key: GoalKey::from(GoalKind::ExploreLocation {
            target_place,
            motivating_need: worldwake_core::ExplorationMotivation::NeedDriven(
                HomeostaticNeedId::Hunger,
            ),
            hypothesis: worldwake_core::HypothesisKind::MayContainCommodity {
                commodity: CommodityKind::Apple,
            },
        }),
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::from([target_place]),
        obligation_source: None,
        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    };
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(10),
        None,
        None,
    );
    let plan = result
        .into_plan()
        .expect("search should find a Travel plan for ExploreLocation");

    assert_eq!(
        plan.steps.len(),
        1,
        "exploration should complete as soon as travel reaches the target place"
    );
    assert_eq!(
        plan.steps[0].op_kind,
        PlannerOpKind::Travel,
        "plan should contain a single Travel step to the explored place"
    );
    assert_eq!(
        plan.steps[0].targets,
        vec![PlanningEntityRef::Authoritative(target_place)]
    );
}

#[test]
fn search_empty_beliefs_exploration_fallback_returns_nearest_travel_barrier() {
    let actor = entity(1);
    let actor_place = entity(10);
    let near_place = entity(11);
    let far_place = entity(12);

    let mut view = TestBeliefView::default();
    view.alive
        .extend([actor, actor_place, near_place, far_place]);
    view.kinds.insert(actor, EntityKind::Agent);
    view.kinds.insert(actor_place, EntityKind::Place);
    view.kinds.insert(near_place, EntityKind::Place);
    view.kinds.insert(far_place, EntityKind::Place);
    view.effective_places.insert(actor, actor_place);
    view.entities_at.insert(actor_place, vec![actor]);
    view.entities_at.entry(near_place).or_default();
    view.entities_at.entry(far_place).or_default();
    view.adjacent.insert(
        actor_place,
        vec![
            (near_place, NonZeroU32::new(2).unwrap()),
            (far_place, NonZeroU32::new(5).unwrap()),
        ],
    );
    view.adjacent
        .insert(near_place, vec![(actor_place, NonZeroU32::new(2).unwrap())]);
    view.adjacent
        .insert(far_place, vec![(actor_place, NonZeroU32::new(5).unwrap())]);

    let (registry, handlers) = build_registry();
    let goal = GoalOffer {
        anchor: worldwake_core::OpportunityAnchor::None,
        key: GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
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
    let snapshot = build_planning_snapshot(
        &view,
        actor,
        &goal.evidence_entities,
        &goal.evidence_places,
        1,
    );
    let result = search_plan(
        &snapshot,
        &goal,
        &build_semantics_table(&registry),
        &registry,
        &handlers,
        &ProfileFixture::default(),
        &RecipeRegistry::new(),
        &BlockerMemory::default(),
        Tick(0),
        None,
        None,
    );
    let plan = result
        .into_plan()
        .expect("exploration fallback should return a nearest travel barrier");

    assert_eq!(plan.terminal_kind, PlanTerminalKind::ProgressBarrier);
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].op_kind, PlannerOpKind::Travel);
    assert_eq!(
        plan.steps[0].targets,
        vec![PlanningEntityRef::Authoritative(near_place)]
    );
}
