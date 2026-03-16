use crate::{generate_candidates, rank_candidates, GoalPriorityClass};
use worldwake_core::{BlockedIntentMemory, EntityId, GoalKind, Tick, UtilityProfile};
use worldwake_sim::{GoalBeliefView, RecipeRegistry};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoalExplanation {
    pub goal: GoalKind,
    pub priority_class: GoalPriorityClass,
    pub motive_value: u32,
    pub evidence_entities: Vec<EntityId>,
    pub evidence_places: Vec<EntityId>,
    pub competing_goals: Vec<(GoalKind, GoalPriorityClass, u32)>,
}

#[must_use]
pub fn explain_goal(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    goal: &GoalKind,
    blocked: &BlockedIntentMemory,
    recipes: &RecipeRegistry,
    utility: &UtilityProfile,
    current_tick: Tick,
) -> Option<GoalExplanation> {
    let candidates = generate_candidates(view, agent, blocked, recipes, current_tick);
    let ranked = rank_candidates(&candidates, view, agent, current_tick, utility, recipes);
    let target = ranked
        .iter()
        .find(|candidate| candidate.grounded.key.kind == *goal)?;

    Some(GoalExplanation {
        goal: *goal,
        priority_class: target.priority_class,
        motive_value: target.motive_score,
        evidence_entities: target.grounded.evidence_entities.iter().copied().collect(),
        evidence_places: target.grounded.evidence_places.iter().copied().collect(),
        competing_goals: ranked
            .iter()
            .filter(|candidate| candidate.grounded.key.kind != *goal)
            .map(|candidate| {
                (
                    candidate.grounded.key.kind,
                    candidate.priority_class,
                    candidate.motive_score,
                )
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::{explain_goal, GoalExplanation};
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        BlockedIntentMemory, CommodityConsumableProfile, CommodityKind, CommodityPurpose,
        DemandObservation, DriveThresholds, EntityId, EntityKind, GoalKind, GrantedFacilityUse,
        HomeostaticNeeds, InTransitOnEdge, LoadUnits, MerchandiseProfile, MetabolismProfile,
        Permille, PlaceTag, Quantity, RecipeId, ResourceSource, Tick, TickRange,
        TradeDispositionProfile, TravelDispositionProfile, UniqueItemKind, UtilityProfile,
        WorkstationTag, Wound,
    };
    use worldwake_sim::{
        estimate_duration_from_beliefs, ActionDuration, ActionPayload, DurationExpr,
        RecipeRegistry, RuntimeBeliefView,
    };

    #[derive(Default)]
    struct TestBeliefView {
        alive: BTreeSet<EntityId>,
        entity_kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        entities_at: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessions: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessors: BTreeMap<EntityId, EntityId>,
        consumable_profiles: BTreeMap<EntityId, CommodityConsumableProfile>,
        lot_commodities: BTreeMap<EntityId, CommodityKind>,
        controlled_entities: BTreeSet<EntityId>,
        controllable: BTreeSet<(EntityId, EntityId)>,
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        homeostatic_needs: BTreeMap<EntityId, HomeostaticNeeds>,
        drive_thresholds: BTreeMap<EntityId, DriveThresholds>,
    }

    worldwake_sim::impl_goal_belief_view!(TestBeliefView);

    impl RuntimeBeliefView for TestBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.contains(&entity)
        }

        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.entity_kinds.get(&entity).copied()
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

        fn adjacent_places(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
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
            agent: EntityId,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Quantity {
            self.local_controlled_lots_for(agent, place, commodity)
                .into_iter()
                .fold(Quantity(0), |acc, lot| {
                    Quantity(acc.0 + self.commodity_quantity(lot, commodity).0)
                })
        }

        fn local_controlled_lots_for(
            &self,
            agent: EntityId,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Vec<EntityId> {
            self.entities_at(place)
                .into_iter()
                .filter(|entity| {
                    self.item_lot_commodity(*entity) == Some(commodity)
                        && self.can_control(agent, *entity)
                })
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

        fn has_exclusive_facility_policy(&self, _entity: EntityId) -> bool {
            false
        }

        fn facility_queue_position(&self, _facility: EntityId, _actor: EntityId) -> Option<u32> {
            None
        }

        fn facility_grant(&self, _facility: EntityId) -> Option<&GrantedFacilityUse> {
            None
        }

        fn facility_queue_join_tick(&self, _facility: EntityId, _actor: EntityId) -> Option<Tick> {
            None
        }

        fn facility_queue_patience_ticks(&self, _agent: EntityId) -> Option<NonZeroU32> {
            None
        }

        fn place_has_tag(&self, _place: EntityId, _tag: PlaceTag) -> bool {
            false
        }

        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
            self.controlled_entities.contains(&entity)
                || self.controllable.contains(&(actor, entity))
        }

        fn has_control(&self, entity: EntityId) -> bool {
            self.controlled_entities.contains(&entity)
        }

        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
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
            self.homeostatic_needs.get(&agent).copied()
        }

        fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
            self.drive_thresholds.get(&agent).copied()
        }
        fn belief_confidence_policy(
            &self,
            _agent: EntityId,
        ) -> worldwake_core::BeliefConfidencePolicy {
            worldwake_core::BeliefConfidencePolicy::default()
        }

        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }

        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
        }

        fn travel_disposition_profile(&self, _agent: EntityId) -> Option<TravelDispositionProfile> {
            None
        }

        fn combat_profile(&self, _agent: EntityId) -> Option<worldwake_core::CombatProfile> {
            None
        }

        fn wounds(&self, _agent: EntityId) -> Vec<Wound> {
            Vec::new()
        }

        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn agents_selling_at(&self, _place: EntityId, _commodity: CommodityKind) -> Vec<EntityId> {
            Vec::new()
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

        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            Vec::new()
        }

        fn merchandise_profile(&self, _agent: EntityId) -> Option<MerchandiseProfile> {
            None
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn in_transit_state(&self, _entity: EntityId) -> Option<InTransitOnEdge> {
            None
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            _place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            Vec::new()
        }

        fn estimate_duration(
            &self,
            actor: EntityId,
            duration: &DurationExpr,
            targets: &[EntityId],
            payload: &ActionPayload,
        ) -> Option<ActionDuration> {
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

    fn utility() -> UtilityProfile {
        UtilityProfile {
            hunger_weight: pm(900),
            thirst_weight: pm(800),
            fatigue_weight: pm(700),
            bladder_weight: pm(600),
            dirtiness_weight: pm(500),
            pain_weight: pm(400),
            danger_weight: pm(300),
            enterprise_weight: pm(200),
            social_weight: pm(150),
            courage: pm(500),
        }
    }

    fn hungry_and_tired_view() -> (TestBeliefView, EntityId, GoalKind) {
        let agent = entity(1);
        let place = entity(10);
        let bread = entity(20);
        let thresholds = DriveThresholds::default();
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, bread]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(place, EntityKind::Place);
        view.entity_kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(bread, place);
        view.entities_at.insert(place, vec![agent, bread]);
        view.direct_possessions.insert(agent, vec![bread]);
        view.direct_possessors.insert(bread, agent);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityKind::Bread.spec().consumable_profile.unwrap(),
        );
        view.controlled_entities.insert(agent);
        view.controllable.insert((agent, bread));
        view.commodity_quantities
            .insert((agent, CommodityKind::Bread), Quantity(1));
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(1));
        view.homeostatic_needs.insert(
            agent,
            HomeostaticNeeds::new(
                thresholds.hunger.high(),
                pm(0),
                thresholds.fatigue.medium(),
                pm(0),
                pm(0),
            ),
        );
        view.drive_thresholds.insert(agent, thresholds);

        (
            view,
            agent,
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            },
        )
    }

    #[test]
    fn explain_goal_returns_rank_and_evidence_for_emitted_goal() {
        let (view, agent, goal) = hungry_and_tired_view();
        let thresholds = DriveThresholds::default();

        let explanation = explain_goal(
            &view,
            agent,
            &goal,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            &utility(),
            Tick(5),
        )
        .unwrap();

        assert_eq!(explanation.goal, goal);
        assert_eq!(explanation.priority_class, crate::GoalPriorityClass::High);
        assert_eq!(
            explanation.motive_value,
            900 * u32::from(thresholds.hunger.high().value())
        );
        assert_eq!(explanation.evidence_entities, vec![entity(20)]);
        assert_eq!(explanation.evidence_places, vec![entity(10)]);
    }

    #[test]
    fn explain_goal_returns_none_for_non_emittable_goal() {
        let (view, agent, _goal) = hungry_and_tired_view();
        let missing_goal = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
        };

        let explanation = explain_goal(
            &view,
            agent,
            &missing_goal,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            &utility(),
            Tick(5),
        );

        assert_eq!(explanation, None);
    }

    #[test]
    fn explain_goal_lists_other_ranked_candidates_as_competitors() {
        let (view, agent, goal) = hungry_and_tired_view();
        let thresholds = DriveThresholds::default();

        let explanation = explain_goal(
            &view,
            agent,
            &goal,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            &utility(),
            Tick(5),
        )
        .unwrap();

        assert_eq!(
            explanation.competing_goals,
            vec![(
                GoalKind::Sleep,
                crate::GoalPriorityClass::Medium,
                700 * u32::from(thresholds.fatigue.medium().value()),
            )]
        );
    }

    #[test]
    fn explain_goal_is_deterministic_for_same_inputs() {
        let (view, agent, goal) = hungry_and_tired_view();

        let first = explain_goal(
            &view,
            agent,
            &goal,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            &utility(),
            Tick(5),
        );
        let second = explain_goal(
            &view,
            agent,
            &goal,
            &BlockedIntentMemory::default(),
            &RecipeRegistry::new(),
            &utility(),
            Tick(5),
        );

        assert_eq!(first, second);
    }

    #[test]
    fn goal_explanation_struct_satisfies_value_bounds() {
        fn assert_value_bounds<T: Clone + Eq + std::fmt::Debug + PartialEq>() {}

        assert_value_bounds::<GoalExplanation>();
    }
}
