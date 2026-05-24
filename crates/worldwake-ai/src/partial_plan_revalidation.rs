use crate::{
    GoalOffer, PlannedSkeletonStep,
    htn::{BeliefPredicate, CommodityTemplate, EntityTemplate, RecipeTemplate},
};
use worldwake_core::{CommodityKind, EntityId, GoalKind, OpportunityAnchor, RecipeId};
use worldwake_sim::{
    RuntimeBeliefView,
    belief_view::{BeliefStatus, BeliefValue},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkeletonRevalidationVerdict {
    Reusable,
    Invalid(SkeletonRevalidationReason),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkeletonRevalidationReason {
    BeliefStale,
    BeliefContradicted,
    BeliefUnknown,
    UnsupportedPredicate,
}

#[derive(Clone, Copy)]
pub struct SkeletonRevalidationContext<'a> {
    pub actor: EntityId,
    pub goal: &'a GoalOffer,
    pub step: &'a PlannedSkeletonStep,
    pub view: &'a dyn RuntimeBeliefView,
}

#[must_use]
pub fn revalidate_skeleton_step(
    context: SkeletonRevalidationContext<'_>,
) -> SkeletonRevalidationVerdict {
    for predicate in &context.step.expected_pre {
        if let Err(reason) = predicate_holds(context, predicate) {
            return SkeletonRevalidationVerdict::Invalid(reason);
        }
    }

    SkeletonRevalidationVerdict::Reusable
}

fn predicate_holds(
    context: SkeletonRevalidationContext<'_>,
    predicate: &BeliefPredicate,
) -> Result<(), SkeletonRevalidationReason> {
    match predicate {
        BeliefPredicate::BountyRecordExists { bounty } => {
            let bounty = resolve_entity(context.goal, *bounty)?;
            if context.view.record_data(bounty).is_some() {
                Ok(())
            } else {
                Err(SkeletonRevalidationReason::BeliefUnknown)
            }
        }
        BeliefPredicate::BountyExpired { .. } => {
            Err(SkeletonRevalidationReason::UnsupportedPredicate)
        }
        BeliefPredicate::TargetLastSeenKnown { target } => {
            let target = resolve_entity(context.goal, *target)?;
            location_belief_holds(context.view.believed_target_location(context.actor, target))
        }
        BeliefPredicate::WitnessNamesKnown { violation } => {
            let violation = resolve_entity(context.goal, *violation)?;
            if context
                .view
                .known_entity_beliefs(context.actor)
                .into_iter()
                .any(|(subject, _)| subject == violation)
            {
                Ok(())
            } else {
                Err(SkeletonRevalidationReason::BeliefUnknown)
            }
        }
        BeliefPredicate::InstitutionalRecordBelievedExtant { violation } => {
            let violation = resolve_entity(context.goal, *violation)?;
            if context.view.record_data(violation).is_some() {
                Ok(())
            } else {
                Err(SkeletonRevalidationReason::BeliefUnknown)
            }
        }
        BeliefPredicate::ResourceSourceKnown { commodity } => {
            let commodity = resolve_commodity(context.goal, *commodity)?;
            if candidate_places(context.goal).into_iter().any(|place| {
                !context
                    .view
                    .resource_sources_at(place, commodity)
                    .is_empty()
            }) {
                Ok(())
            } else {
                Err(SkeletonRevalidationReason::BeliefUnknown)
            }
        }
        BeliefPredicate::SellerKnown { commodity } => {
            let commodity = resolve_commodity(context.goal, *commodity)?;
            if candidate_places(context.goal).into_iter().any(|place| {
                !context
                    .view
                    .listed_sale_lots_at(place, commodity)
                    .is_empty()
            }) {
                Ok(())
            } else {
                Err(SkeletonRevalidationReason::BeliefUnknown)
            }
        }
        BeliefPredicate::OwnedCommodityBelowThreshold {
            commodity,
            threshold,
        } => {
            let commodity = resolve_commodity(context.goal, *commodity)?;
            if context.view.commodity_quantity(context.actor, commodity) < *threshold {
                Ok(())
            } else {
                Err(SkeletonRevalidationReason::BeliefContradicted)
            }
        }
        BeliefPredicate::OwnsInputsForRecipe { recipe } => {
            let recipe = resolve_recipe(context.goal, *recipe)?;
            let Some(recipe) = context.view.recipe_definition(recipe) else {
                return Err(SkeletonRevalidationReason::BeliefUnknown);
            };
            if recipe.inputs.iter().all(|(commodity, quantity)| {
                context.view.commodity_quantity(context.actor, *commodity) >= *quantity
            }) {
                Ok(())
            } else {
                Err(SkeletonRevalidationReason::BeliefContradicted)
            }
        }
        BeliefPredicate::EscorteeBelievedSafeAt { escortee } => {
            let escortee = resolve_entity(context.goal, *escortee)?;
            location_belief_holds(
                context
                    .view
                    .believed_target_location(context.actor, escortee),
            )
        }
        BeliefPredicate::AllyOrBountyOfficeAvailable => {
            if context
                .view
                .known_institutional_beliefs(context.actor)
                .is_empty()
            {
                Err(SkeletonRevalidationReason::BeliefUnknown)
            } else {
                Ok(())
            }
        }
        BeliefPredicate::TargetBelievedDangerous { target } => {
            let target = resolve_entity(context.goal, *target)?;
            if context
                .view
                .visible_hostiles_for(context.actor)
                .contains(&target)
                || context
                    .view
                    .hostile_targets_of(context.actor)
                    .contains(&target)
            {
                Ok(())
            } else {
                Err(SkeletonRevalidationReason::BeliefUnknown)
            }
        }
    }
}

fn location_belief_holds(
    belief: BeliefValue<Option<EntityId>>,
) -> Result<(), SkeletonRevalidationReason> {
    match belief.status {
        BeliefStatus::Certain | BeliefStatus::Probable => {
            if belief.value.is_some() {
                Ok(())
            } else {
                Err(SkeletonRevalidationReason::BeliefUnknown)
            }
        }
        BeliefStatus::Stale => Err(SkeletonRevalidationReason::BeliefStale),
        BeliefStatus::Disputed | BeliefStatus::Contradicted => {
            Err(SkeletonRevalidationReason::BeliefContradicted)
        }
    }
}

fn resolve_entity(
    goal: &GoalOffer,
    template: EntityTemplate,
) -> Result<EntityId, SkeletonRevalidationReason> {
    match template {
        EntityTemplate::Fixed(entity) => Ok(entity),
        EntityTemplate::GoalPrimaryEntity
        | EntityTemplate::BountyTarget
        | EntityTemplate::Escortee => goal
            .key
            .entity
            .or_else(|| goal.evidence_entities.iter().next().copied())
            .ok_or(SkeletonRevalidationReason::UnsupportedPredicate),
        EntityTemplate::GoalSecondaryEntity => goal
            .evidence_entities
            .iter()
            .nth(1)
            .copied()
            .or(goal.key.place)
            .ok_or(SkeletonRevalidationReason::UnsupportedPredicate),
        EntityTemplate::GoalPlace => {
            goal_place(goal).ok_or(SkeletonRevalidationReason::UnsupportedPredicate)
        }
        EntityTemplate::Violation | EntityTemplate::Institution => goal
            .key
            .entity
            .or_else(|| goal.evidence_entities.iter().next().copied())
            .ok_or(SkeletonRevalidationReason::UnsupportedPredicate),
    }
}

fn resolve_commodity(
    goal: &GoalOffer,
    template: CommodityTemplate,
) -> Result<CommodityKind, SkeletonRevalidationReason> {
    match template {
        CommodityTemplate::Fixed(commodity) => Ok(commodity),
        CommodityTemplate::GoalCommodity => goal
            .key
            .commodity
            .ok_or(SkeletonRevalidationReason::UnsupportedPredicate),
        CommodityTemplate::RecipeInput { .. } => {
            Err(SkeletonRevalidationReason::UnsupportedPredicate)
        }
    }
}

fn resolve_recipe(
    goal: &GoalOffer,
    template: RecipeTemplate,
) -> Result<RecipeId, SkeletonRevalidationReason> {
    match template {
        RecipeTemplate::Fixed(recipe) => Ok(RecipeId(recipe)),
        RecipeTemplate::GoalRecipe => match goal.key.kind {
            GoalKind::ProduceCommodity { recipe_id } => Ok(recipe_id),
            _ => Err(SkeletonRevalidationReason::UnsupportedPredicate),
        },
    }
}

fn candidate_places(goal: &GoalOffer) -> Vec<EntityId> {
    let mut places = Vec::new();
    if let Some(place) = goal_place(goal) {
        places.push(place);
    }
    for place in &goal.evidence_places {
        if !places.contains(place) {
            places.push(*place);
        }
    }
    places
}

fn goal_place(goal: &GoalOffer) -> Option<EntityId> {
    goal.key.place.or_else(|| match goal.anchor {
        OpportunityAnchor::Place(place) => Some(place),
        OpportunityAnchor::Entity(_) | OpportunityAnchor::None => None,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        SkeletonRevalidationContext, SkeletonRevalidationReason, SkeletonRevalidationVerdict,
        revalidate_skeleton_step,
    };
    use crate::{
        GoalOffer, PlannedSkeletonStep, PlannerOpKind,
        htn::{BeliefPredicate, CommodityTemplate, EntityTemplate, PayloadTemplate},
    };
    use std::collections::{BTreeMap, BTreeSet};
    use worldwake_core::{
        BeliefConfidencePolicy, BelievedEntityState, BelievedInstitutionalClaim, CombatProfile,
        CommodityKind, CommodityPurpose, DemandObservation, DriveThresholds, EntityId, EntityKind,
        GoalKey, GoalKind, HomeostaticNeeds, InTransitOnEdge, IntentionDispositionProfile,
        LoadUnits, MerchandiseProfile, MetabolismProfile, OpportunityAnchor, Permille, Quantity,
        RecipeId, ResourceSource, Tick, TickRange, TradeDispositionProfile, UniqueItemKind,
        WorkstationTag, Wound,
    };
    use worldwake_sim::{
        ActionDuration, ActionPayload, BelievedAuthorityView, CombatBeliefView, ControlBeliefView,
        DurationExpr, EconomicBeliefView, EntityBeliefView, FacilityBeliefView,
        InventoryBeliefView, LocalPhysicalObservationView, PoliticalBeliefView, ProfileBeliefView,
        RuntimeBeliefView, SocialBeliefView, SpatialBeliefView, TemporalBeliefView,
        belief_view::{BeliefStatus, BeliefValue},
    };

    #[derive(Default)]
    struct TestBeliefView {
        target_locations: BTreeMap<EntityId, BeliefValue<Option<EntityId>>>,
        sale_lots: BTreeMap<(EntityId, CommodityKind), Vec<EntityId>>,
        world_truth_reads: u32,
    }

    impl TestBeliefView {
        fn target_location(
            &mut self,
            actor: EntityId,
            target: EntityId,
            place: Option<EntityId>,
            status: BeliefStatus,
        ) {
            let _ = actor;
            self.target_locations.insert(
                target,
                BeliefValue {
                    value: place,
                    confidence: Permille::new_unchecked(1000),
                    acquired_tick: Tick(1),
                    claimed_event_tick: Some(Tick(1)),
                    status,
                },
            );
        }
    }

    impl ControlBeliefView for TestBeliefView {
        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }

        fn has_control(&self, _entity: EntityId) -> bool {
            false
        }
    }

    impl EntityBeliefView for TestBeliefView {
        fn is_alive(&self, _entity: EntityId) -> bool {
            true
        }

        fn entity_kind(&self, _entity: EntityId) -> Option<EntityKind> {
            None
        }

        fn believed_target_location(
            &self,
            _agent: EntityId,
            target: EntityId,
        ) -> BeliefValue<Option<EntityId>> {
            self.target_locations
                .get(&target)
                .copied()
                .unwrap_or(BeliefValue {
                    value: None,
                    confidence: Permille::ZERO,
                    acquired_tick: Tick(0),
                    claimed_event_tick: None,
                    status: BeliefStatus::Certain,
                })
        }

        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
    }

    impl ProfileBeliefView for TestBeliefView {
        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            None
        }

        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
        }

        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }
    }

    impl SpatialBeliefView for TestBeliefView {
        fn effective_place(&self, _entity: EntityId) -> Option<EntityId> {
            panic!("skeleton revalidation must not read authoritative effective_place");
        }

        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }

        fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            panic!("skeleton revalidation must not read authoritative entities_at");
        }

        fn adjacent_places(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn route_exists(&self, _from: EntityId, _to: EntityId) -> bool {
            false
        }

        fn in_transit_state(&self, _entity: EntityId) -> Option<InTransitOnEdge> {
            None
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            _place: EntityId,
        ) -> Vec<(EntityId, std::num::NonZeroU32)> {
            Vec::new()
        }
    }

    impl TemporalBeliefView for TestBeliefView {
        fn current_tick(&self) -> Tick {
            Tick(5)
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
            _duration: &DurationExpr,
            _targets: &[EntityId],
            _payload: &ActionPayload,
        ) -> Option<ActionDuration> {
            None
        }
    }

    impl InventoryBeliefView for TestBeliefView {
        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
            false
        }

        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
        }

        fn commodity_quantity(&self, _holder: EntityId, _kind: CommodityKind) -> Quantity {
            Quantity(0)
        }

        fn item_lot_commodity(&self, _entity: EntityId) -> Option<CommodityKind> {
            None
        }

        fn item_lot_consumable_profile(
            &self,
            _entity: EntityId,
        ) -> Option<worldwake_core::CommodityConsumableProfile> {
            None
        }

        fn direct_container(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn direct_possessor(&self, _entity: EntityId) -> Option<EntityId> {
            None
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

    impl CombatBeliefView for TestBeliefView {
        fn combat_profile(&self, _agent: EntityId) -> Option<CombatProfile> {
            None
        }

        fn wounds(&self, _agent: EntityId) -> Vec<Wound> {
            Vec::new()
        }

        fn hostile_targets_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
        }
    }

    impl EconomicBeliefView for TestBeliefView {
        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
        }

        fn controlled_commodity_quantity_at_place(
            &self,
            _agent: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Quantity {
            Quantity(0)
        }

        fn local_controlled_lots_for(
            &self,
            _agent: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn listed_sale_lots_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.sale_lots
                .get(&(place, commodity))
                .cloned()
                .unwrap_or_default()
        }

        fn seller_for_sale_lot(&self, _lot: EntityId) -> Option<EntityId> {
            None
        }

        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            Vec::new()
        }

        fn merchandise_profile(&self, _agent: EntityId) -> Option<MerchandiseProfile> {
            None
        }
    }

    impl SocialBeliefView for TestBeliefView {
        fn known_entity_beliefs(&self, _agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
            Vec::new()
        }

        fn belief_confidence_policy(&self, _agent: EntityId) -> BeliefConfidencePolicy {
            BeliefConfidencePolicy::default()
        }

        fn intention_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<IntentionDispositionProfile> {
            None
        }
    }

    impl PoliticalBeliefView for TestBeliefView {
        fn known_institutional_beliefs(&self, _agent: EntityId) -> Vec<BelievedInstitutionalClaim> {
            Vec::new()
        }
    }

    impl BelievedAuthorityView for TestBeliefView {}
    impl LocalPhysicalObservationView for TestBeliefView {}

    impl FacilityBeliefView for TestBeliefView {
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }

        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
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
    }

    impl RuntimeBeliefView for TestBeliefView {}

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn actor() -> EntityId {
        entity(1)
    }

    fn goal(place: EntityId, target: EntityId) -> GoalOffer {
        let mut evidence_entities = BTreeSet::new();
        evidence_entities.insert(target);
        let mut evidence_places = BTreeSet::new();
        evidence_places.insert(place);
        GoalOffer {
            key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
                quantity: worldwake_core::AcquisitionQuantity::single(),
            }),
            anchor: OpportunityAnchor::Place(place),
            evidence_entities,
            evidence_places,
            obligation_source: None,
            commitment_impact_if_ignored: Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: Some(worldwake_core::AcquisitionQuantity::single()),
        }
    }

    fn target_step(target: EntityTemplate) -> PlannedSkeletonStep {
        PlannedSkeletonStep {
            op: PlannerOpKind::AskWitness,
            target_template: PayloadTemplate::FromContext,
            expected_pre: vec![BeliefPredicate::TargetLastSeenKnown { target }],
        }
    }

    fn revalidate(
        view: &TestBeliefView,
        goal: &GoalOffer,
        step: &PlannedSkeletonStep,
    ) -> SkeletonRevalidationVerdict {
        revalidate_skeleton_step(SkeletonRevalidationContext {
            actor: actor(),
            goal,
            step,
            view,
        })
    }

    #[test]
    fn revalidate_returns_reusable_when_all_predicates_hold() {
        let target = entity(2);
        let place = entity(10);
        let goal = goal(place, target);
        let step = target_step(EntityTemplate::Fixed(target));
        let mut view = TestBeliefView::default();
        view.target_location(actor(), target, Some(place), BeliefStatus::Certain);

        assert_eq!(
            revalidate(&view, &goal, &step),
            SkeletonRevalidationVerdict::Reusable
        );
    }

    #[test]
    fn revalidate_returns_belief_stale_when_predicate_freshness_expired() {
        let target = entity(2);
        let place = entity(10);
        let goal = goal(place, target);
        let step = target_step(EntityTemplate::Fixed(target));
        let mut view = TestBeliefView::default();
        view.target_location(actor(), target, Some(place), BeliefStatus::Stale);

        assert_eq!(
            revalidate(&view, &goal, &step),
            SkeletonRevalidationVerdict::Invalid(SkeletonRevalidationReason::BeliefStale)
        );
    }

    #[test]
    fn revalidate_returns_belief_contradicted_when_predicate_explicitly_contradicted() {
        let target = entity(2);
        let place = entity(10);
        let goal = goal(place, target);
        let step = target_step(EntityTemplate::Fixed(target));
        let mut view = TestBeliefView::default();
        view.target_location(actor(), target, Some(place), BeliefStatus::Contradicted);

        assert_eq!(
            revalidate(&view, &goal, &step),
            SkeletonRevalidationVerdict::Invalid(SkeletonRevalidationReason::BeliefContradicted)
        );
    }

    #[test]
    fn revalidate_returns_belief_unknown_when_predicate_has_no_belief() {
        let target = entity(2);
        let place = entity(10);
        let goal = goal(place, target);
        let step = target_step(EntityTemplate::Fixed(target));
        let view = TestBeliefView::default();

        assert_eq!(
            revalidate(&view, &goal, &step),
            SkeletonRevalidationVerdict::Invalid(SkeletonRevalidationReason::BeliefUnknown)
        );
    }

    #[test]
    fn revalidate_returns_unsupported_when_template_cannot_be_resolved() {
        let target = entity(2);
        let place = entity(10);
        let mut goal = goal(place, target);
        goal.key.entity = None;
        goal.evidence_entities.clear();
        let step = target_step(EntityTemplate::GoalPrimaryEntity);
        let view = TestBeliefView::default();

        assert_eq!(
            revalidate(&view, &goal, &step),
            SkeletonRevalidationVerdict::Invalid(SkeletonRevalidationReason::UnsupportedPredicate)
        );
    }

    #[test]
    fn revalidate_reads_only_belief_view_never_world_truth() {
        let target = entity(2);
        let place = entity(10);
        let goal = goal(place, target);
        let step = target_step(EntityTemplate::Fixed(target));
        let mut view = TestBeliefView::default();
        view.target_location(actor(), target, Some(place), BeliefStatus::Certain);

        assert_eq!(
            revalidate(&view, &goal, &step),
            SkeletonRevalidationVerdict::Reusable
        );
        assert_eq!(view.world_truth_reads, 0);
    }

    #[test]
    fn revalidate_iterates_stable_order() {
        let target = entity(2);
        let place = entity(10);
        let goal = goal(place, target);
        let step = PlannedSkeletonStep {
            op: PlannerOpKind::Trade,
            target_template: PayloadTemplate::FromContext,
            expected_pre: vec![
                BeliefPredicate::TargetLastSeenKnown {
                    target: EntityTemplate::Fixed(target),
                },
                BeliefPredicate::SellerKnown {
                    commodity: CommodityTemplate::Fixed(CommodityKind::Bread),
                },
            ],
        };
        let mut view = TestBeliefView::default();
        view.target_location(actor(), target, Some(place), BeliefStatus::Stale);
        view.sale_lots
            .insert((place, CommodityKind::Bread), vec![entity(50)]);

        let first = revalidate(&view, &goal, &step);
        let second = revalidate(&view, &goal, &step);

        assert_eq!(first, second);
        assert_eq!(
            first,
            SkeletonRevalidationVerdict::Invalid(SkeletonRevalidationReason::BeliefStale)
        );
    }
}
