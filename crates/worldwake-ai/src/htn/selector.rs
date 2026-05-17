use crate::goal_model::GoalOffer;
use crate::htn::{
    BeliefPredicate, CommodityTemplate, EntityTemplate, MethodPrecondition, MethodRegistry,
    MethodSchema, RecipeTemplate,
};
use worldwake_core::{
    AgentSchemaContextProfile, ArtifactKind, ArtifactLegalEffect, BountyTarget, CommodityKind,
    EntityId, GoalKind, GoalKindDiscriminant, MotiveSourceDiscriminant, MotiveSourceRef, RecipeId,
};
use worldwake_sim::RuntimeBeliefView;

#[must_use]
pub fn select_method<'r>(
    actor: EntityId,
    goal: &GoalOffer,
    registry: &'r MethodRegistry,
    profile: &AgentSchemaContextProfile,
    belief_view: &dyn RuntimeBeliefView,
    motives: &[MotiveSourceRef],
) -> Option<&'r MethodSchema> {
    let goal_kind = GoalKindDiscriminant::from(&goal.key.kind);

    registry
        .methods_for(goal_kind)
        .iter()
        .filter_map(|id| registry.get(*id))
        .filter(|method| !profile.disabled_methods.contains(&method.id))
        .filter(|method| preconditions_satisfied(actor, method, goal, belief_view, motives))
        .map(|method| (method, motive_score(method, motives)))
        .max_by(|(a, score_a), (b, score_b)| score_a.cmp(score_b).then_with(|| b.id.cmp(&a.id)))
        .map(|(method, _)| method)
}

fn preconditions_satisfied(
    actor: EntityId,
    method: &MethodSchema,
    goal: &GoalOffer,
    belief_view: &dyn RuntimeBeliefView,
    motives: &[MotiveSourceRef],
) -> bool {
    method
        .preconditions
        .iter()
        .all(|precondition| evaluate_precondition(actor, precondition, goal, belief_view, motives))
}

fn evaluate_precondition(
    actor: EntityId,
    precondition: &MethodPrecondition,
    goal: &GoalOffer,
    belief_view: &dyn RuntimeBeliefView,
    motives: &[MotiveSourceRef],
) -> bool {
    match precondition {
        MethodPrecondition::BeliefHolds(predicate) => {
            evaluate_belief_predicate(actor, predicate, goal, belief_view)
        }
        MethodPrecondition::MotiveSourcePresent(discriminant) => motives
            .iter()
            .any(|source| MotiveSourceDiscriminant::from(&source.source) == *discriminant),
        MethodPrecondition::AgentRole(_) => true,
        MethodPrecondition::LocationKnown(criterion) => match criterion {
            crate::htn::EntityCriterion::Workstation(tag) => candidate_places(goal)
                .into_iter()
                .any(|place| !belief_view.matching_workstations_at(place, *tag).is_empty()),
            crate::htn::EntityCriterion::ResourceSource(commodity) => {
                resolve_commodity(goal, *commodity, belief_view).is_some_and(|commodity| {
                    candidate_places(goal)
                        .into_iter()
                        .any(|place| !belief_view.resource_sources_at(place, commodity).is_empty())
                })
            }
            crate::htn::EntityCriterion::Seller(commodity) => {
                resolve_commodity(goal, *commodity, belief_view).is_some_and(|commodity| {
                    candidate_places(goal)
                        .into_iter()
                        .any(|place| !belief_view.listed_sale_lots_at(place, commodity).is_empty())
                })
            }
            crate::htn::EntityCriterion::Target(template) => {
                resolve_entity(actor, goal, *template, belief_view).is_some()
            }
            crate::htn::EntityCriterion::Witness { .. }
            | crate::htn::EntityCriterion::ViolationEvidence { .. }
            | crate::htn::EntityCriterion::Ledger { .. } => false,
        },
    }
}

fn evaluate_belief_predicate(
    actor: EntityId,
    predicate: &BeliefPredicate,
    goal: &GoalOffer,
    belief_view: &dyn RuntimeBeliefView,
) -> bool {
    match predicate {
        BeliefPredicate::BountyRecordExists { bounty } => {
            resolve_entity(actor, goal, *bounty, belief_view)
                .is_some_and(|bounty| bounty_artifact_state(actor, bounty, belief_view).is_some())
        }
        BeliefPredicate::BountyExpired { bounty } => {
            resolve_entity(actor, goal, *bounty, belief_view)
                .and_then(|bounty| bounty_artifact_state(actor, bounty, belief_view))
                .is_some_and(|artifact| {
                    !matches!(
                        artifact.legal_effect,
                        ArtifactLegalEffect::Active { expires_at: _ }
                    )
                })
        }
        BeliefPredicate::TargetLastSeenKnown { target } => {
            resolve_entity(actor, goal, *target, belief_view).is_some_and(|target| {
                belief_view
                    .believed_target_location(actor, target)
                    .value
                    .is_some()
            })
        }
        BeliefPredicate::WitnessNamesKnown { violation } => {
            resolve_entity(actor, goal, *violation, belief_view)
                .is_some_and(|violation| has_witness_report_for(actor, violation, belief_view))
        }
        BeliefPredicate::InstitutionalRecordBelievedExtant { violation } => {
            resolve_entity(actor, goal, *violation, belief_view)
                .is_some_and(|violation| belief_view.record_data(violation).is_some())
        }
        BeliefPredicate::ResourceSourceKnown { commodity } => {
            resolve_commodity(goal, *commodity, belief_view).is_some_and(|commodity| {
                candidate_places(goal)
                    .into_iter()
                    .any(|place| !belief_view.resource_sources_at(place, commodity).is_empty())
            })
        }
        BeliefPredicate::SellerKnown { commodity } => {
            resolve_commodity(goal, *commodity, belief_view).is_some_and(|commodity| {
                candidate_places(goal)
                    .into_iter()
                    .any(|place| !belief_view.listed_sale_lots_at(place, commodity).is_empty())
            })
        }
        BeliefPredicate::OwnedCommodityBelowThreshold {
            commodity,
            threshold,
        } => resolve_commodity(goal, *commodity, belief_view)
            .is_some_and(|commodity| belief_view.commodity_quantity(actor, commodity) < *threshold),
        BeliefPredicate::OwnsInputsForRecipe { recipe } => resolve_recipe(goal, *recipe)
            .and_then(|recipe| belief_view.recipe_definition(recipe))
            .is_some_and(|recipe| {
                recipe.inputs.iter().all(|(commodity, quantity)| {
                    belief_view.commodity_quantity(actor, *commodity) >= *quantity
                })
            }),
        BeliefPredicate::EscorteeBelievedSafeAt { escortee } => {
            resolve_entity(actor, goal, *escortee, belief_view).is_some_and(|escortee| {
                belief_view
                    .believed_target_location(actor, escortee)
                    .value
                    .is_some()
            })
        }
        BeliefPredicate::AllyOrBountyOfficeAvailable => {
            !belief_view.known_institutional_beliefs(actor).is_empty()
        }
        BeliefPredicate::TargetBelievedDangerous { target } => {
            resolve_entity(actor, goal, *target, belief_view).is_some_and(|target| {
                belief_view.visible_hostiles_for(actor).contains(&target)
                    || belief_view.hostile_targets_of(actor).contains(&target)
            })
        }
    }
}

fn motive_score(method: &MethodSchema, motives: &[MotiveSourceRef]) -> u32 {
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

fn candidate_places(goal: &GoalOffer) -> Vec<EntityId> {
    let mut places = goal.evidence_places.iter().copied().collect::<Vec<_>>();
    if let worldwake_core::OpportunityAnchor::Place(place) = goal.anchor {
        places.push(place);
    }
    places.sort();
    places.dedup();
    places
}

fn resolve_entity(
    actor: EntityId,
    goal: &GoalOffer,
    template: EntityTemplate,
    belief_view: &dyn RuntimeBeliefView,
) -> Option<EntityId> {
    match template {
        EntityTemplate::GoalPrimaryEntity => primary_goal_entity(goal),
        EntityTemplate::GoalSecondaryEntity => secondary_goal_entity(goal),
        EntityTemplate::GoalPlace => match goal.anchor {
            worldwake_core::OpportunityAnchor::Place(place) => Some(place),
            worldwake_core::OpportunityAnchor::Entity(entity) => {
                belief_view.believed_target_location(actor, entity).value
            }
            worldwake_core::OpportunityAnchor::None => None,
        },
        EntityTemplate::BountyTarget => bounty_terms(actor, goal, belief_view).and_then(|terms| {
            if let BountyTarget::EliminateEntity { target } = terms.target {
                Some(target)
            } else {
                None
            }
        }),
        EntityTemplate::Violation => {
            if let GoalKind::InvestigateViolation { violation_id, .. } = goal.key.kind {
                u32::try_from(violation_id.0).ok().map(|slot| EntityId {
                    slot,
                    generation: 0,
                })
            } else {
                goal.evidence_entities.iter().next().copied()
            }
        }
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
            worldwake_core::OpportunityAnchor::Entity(entity)
            | worldwake_core::OpportunityAnchor::Place(entity) => Some(entity),
            worldwake_core::OpportunityAnchor::None => {
                goal.evidence_entities.iter().next().copied()
            }
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

fn resolve_commodity(
    goal: &GoalOffer,
    template: CommodityTemplate,
    belief_view: &dyn RuntimeBeliefView,
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
            let recipe_id = resolve_recipe(goal, recipe)?;
            belief_view
                .recipe_definition(recipe_id)?
                .inputs
                .get(usize::from(ordinal))
                .map(|(commodity, _)| *commodity)
        }
        CommodityTemplate::Fixed(commodity) => Some(commodity),
    }
}

fn resolve_recipe(goal: &GoalOffer, template: RecipeTemplate) -> Option<RecipeId> {
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

fn bounty_terms(
    actor: EntityId,
    goal: &GoalOffer,
    belief_view: &dyn RuntimeBeliefView,
) -> Option<worldwake_core::BelievedBountyTerms> {
    let GoalKind::FulfillBounty { bounty } = goal.key.kind else {
        return None;
    };
    bounty_artifact_state(actor, bounty, belief_view).and_then(|artifact| artifact.bounty_terms)
}

fn bounty_artifact_state(
    actor: EntityId,
    bounty: EntityId,
    belief_view: &dyn RuntimeBeliefView,
) -> Option<worldwake_core::BelievedArtifactState> {
    belief_view
        .known_entity_beliefs(actor)
        .into_iter()
        .find_map(|(entity, belief)| {
            (entity == bounty)
                .then_some(belief.believed_artifact)
                .flatten()
        })
        .filter(|artifact| artifact.kind == ArtifactKind::Bounty)
}

fn has_witness_report_for(
    actor: EntityId,
    violation: EntityId,
    belief_view: &dyn RuntimeBeliefView,
) -> bool {
    belief_view
        .known_entity_beliefs(actor)
        .into_iter()
        .any(|(entity, belief)| {
            entity == violation
                && matches!(
                    belief.source,
                    worldwake_core::PerceptionSource::Report { .. }
                )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::htn::{BeliefPredicate, MethodPrecondition, MotiveBias, build_method_registry};
    use std::collections::{BTreeMap, BTreeSet};
    use worldwake_core::{
        BeliefConfidencePolicy, BodyCostPerTick, DemandObservation, DriveThresholds, EntityKind,
        HomeostaticNeeds, InTransitOnEdge, IntentionDispositionProfile, LoadUnits,
        MetabolismProfile, MethodSchemaId, MotiveSource, OpportunityAnchor, Permille, Quantity,
        ResourceSource, Tick, TickRange, TradeDispositionProfile, UniqueItemKind, WorkstationTag,
    };
    use worldwake_sim::belief_view::{BeliefStatus, BeliefValue};
    use worldwake_sim::{
        ActionDuration, ActionPayload, BelievedAuthorityView, CombatBeliefView, ControlBeliefView,
        DurationExpr, EconomicBeliefView, EntityBeliefView, FacilityBeliefView,
        InventoryBeliefView, LocalPhysicalObservationView, ProfileBeliefView, RecipeDefinition,
        RuntimeBeliefView, SocialBeliefView, SpatialBeliefView, TemporalBeliefView,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn motive(source: MotiveSource) -> MotiveSourceRef {
        MotiveSourceRef {
            source,
            introduced_tick: Tick(0),
        }
    }

    fn goal(kind: GoalKind) -> GoalOffer {
        GoalOffer {
            key: worldwake_core::GoalKey::from(kind),
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            motive_sources: Vec::new(),
            acquisition_quantity: None,
        }
    }

    fn method(
        id: u32,
        goal_kind: GoalKindDiscriminant,
        preconditions: Vec<MethodPrecondition>,
        motive_bias: Vec<MotiveBias>,
    ) -> MethodSchema {
        MethodSchema {
            id: MethodSchemaId(id),
            goal_kind,
            preconditions,
            subgoals: Vec::new(),
            expected_artifacts: Vec::new(),
            required_claims: Vec::new(),
            failure_modes: Vec::new(),
            explanation_template: crate::htn::ExplanationTemplateId(id),
            motive_bias,
            planning_budget_hint: None,
        }
    }

    fn bias(motive_variant: MotiveSourceDiscriminant, weight: u16) -> MotiveBias {
        MotiveBias {
            motive_variant,
            weight: pm(weight),
        }
    }

    fn registry(methods: impl IntoIterator<Item = MethodSchema>) -> MethodRegistry {
        let mut registry = MethodRegistry::default();
        for method in methods {
            registry.insert(method);
        }
        registry
    }

    fn recipe(inputs: Vec<(CommodityKind, Quantity)>) -> RecipeDefinition {
        RecipeDefinition {
            name: "Test recipe".to_string(),
            inputs,
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: std::num::NonZeroU32::new(1).unwrap(),
            required_workstation_tag: None,
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::zero(),
        }
    }

    fn produce_goal_with_evidence(recipe_id: RecipeId, place: EntityId) -> GoalOffer {
        let mut goal = goal(GoalKind::ProduceCommodity { recipe_id });
        goal.evidence_places.insert(place);
        goal
    }

    #[derive(Default)]
    struct TestBeliefView {
        target_locations: BTreeMap<EntityId, EntityId>,
        commodities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        resource_sources: BTreeMap<(EntityId, CommodityKind), Vec<EntityId>>,
        sale_lots: BTreeMap<(EntityId, CommodityKind), Vec<EntityId>>,
        recipes: BTreeMap<RecipeId, RecipeDefinition>,
        visible_hostiles: Vec<EntityId>,
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
            BeliefValue {
                value: self.target_locations.get(&target).copied(),
                confidence: Permille::new_unchecked(1000),
                acquired_tick: Tick(0),
                claimed_event_tick: Some(Tick(0)),
                status: BeliefStatus::Certain,
            }
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
            None
        }

        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }

        fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
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

        fn knows_recipe(&self, _actor: EntityId, recipe: RecipeId) -> bool {
            self.recipes.contains_key(&recipe)
        }

        fn recipe_definition(&self, recipe: RecipeId) -> Option<RecipeDefinition> {
            self.recipes.get(&recipe).cloned()
        }

        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
        }

        fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
            self.commodities
                .get(&(holder, kind))
                .copied()
                .unwrap_or(Quantity(0))
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
            self.recipes.keys().copied().collect()
        }
    }

    impl CombatBeliefView for TestBeliefView {
        fn combat_profile(&self, _agent: EntityId) -> Option<worldwake_core::CombatProfile> {
            None
        }

        fn wounds(&self, _agent: EntityId) -> Vec<worldwake_core::Wound> {
            Vec::new()
        }

        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            self.visible_hostiles.clone()
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

        fn merchandise_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::MerchandiseProfile> {
            None
        }
    }

    impl SocialBeliefView for TestBeliefView {
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

    impl worldwake_sim::PoliticalBeliefView for TestBeliefView {}
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

        fn resource_sources_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.resource_sources
                .get(&(place, commodity))
                .cloned()
                .unwrap_or_default()
        }
    }

    impl RuntimeBeliefView for TestBeliefView {}

    #[test]
    fn select_method_returns_top_ranked_method_by_motive_score() {
        let registry = registry([
            method(
                1,
                GoalKindDiscriminant::RestockCommodity,
                Vec::new(),
                vec![bias(MotiveSourceDiscriminant::Greed, 300)],
            ),
            method(
                2,
                GoalKindDiscriminant::RestockCommodity,
                Vec::new(),
                vec![bias(MotiveSourceDiscriminant::Greed, 700)],
            ),
        ]);

        let selected = select_method(
            entity(1),
            &goal(GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            }),
            &registry,
            &AgentSchemaContextProfile::default(),
            &TestBeliefView::default(),
            &[motive(MotiveSource::Greed {
                opportunity: worldwake_core::OpportunityKey {
                    goal_key: worldwake_core::GoalKey::from(GoalKind::RestockCommodity {
                        commodity: CommodityKind::Bread,
                    }),
                    anchor: OpportunityAnchor::None,
                },
            })],
        )
        .expect("matching method should be selected");

        assert_eq!(selected.id, MethodSchemaId(2));
    }

    #[test]
    fn select_method_honors_disabled_methods_denylist() {
        let registry = registry([
            method(
                1,
                GoalKindDiscriminant::RestockCommodity,
                Vec::new(),
                vec![bias(MotiveSourceDiscriminant::Greed, 300)],
            ),
            method(
                2,
                GoalKindDiscriminant::RestockCommodity,
                Vec::new(),
                vec![bias(MotiveSourceDiscriminant::Greed, 700)],
            ),
        ]);
        let mut profile = AgentSchemaContextProfile::default();
        profile.disabled_methods.insert(MethodSchemaId(2));

        let selected = select_method(
            entity(1),
            &goal(GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            }),
            &registry,
            &profile,
            &TestBeliefView::default(),
            &[motive(MotiveSource::Greed {
                opportunity: worldwake_core::OpportunityKey {
                    goal_key: worldwake_core::GoalKey::from(GoalKind::RestockCommodity {
                        commodity: CommodityKind::Bread,
                    }),
                    anchor: OpportunityAnchor::None,
                },
            })],
        )
        .expect("non-disabled method should be selected");

        assert_eq!(selected.id, MethodSchemaId(1));
    }

    #[test]
    fn select_method_skips_methods_with_failed_preconditions() {
        let target = entity(9);
        let registry = registry([
            method(
                1,
                GoalKindDiscriminant::EngageHostile,
                vec![MethodPrecondition::BeliefHolds(
                    BeliefPredicate::TargetLastSeenKnown {
                        target: EntityTemplate::Fixed(target),
                    },
                )],
                Vec::new(),
            ),
            method(
                2,
                GoalKindDiscriminant::EngageHostile,
                Vec::new(),
                Vec::new(),
            ),
        ]);

        let selected = select_method(
            entity(1),
            &goal(GoalKind::EngageHostile { target }),
            &registry,
            &AgentSchemaContextProfile::default(),
            &TestBeliefView::default(),
            &[],
        )
        .expect("fallback method should be selected");

        assert_eq!(selected.id, MethodSchemaId(2));
    }

    #[test]
    fn select_method_tie_breaks_by_lower_method_schema_id() {
        let registry = registry([
            method(
                20,
                GoalKindDiscriminant::RestockCommodity,
                Vec::new(),
                vec![bias(MotiveSourceDiscriminant::Greed, 500)],
            ),
            method(
                10,
                GoalKindDiscriminant::RestockCommodity,
                Vec::new(),
                vec![bias(MotiveSourceDiscriminant::Greed, 500)],
            ),
        ]);

        let selected = select_method(
            entity(1),
            &goal(GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            }),
            &registry,
            &AgentSchemaContextProfile::default(),
            &TestBeliefView::default(),
            &[motive(MotiveSource::Greed {
                opportunity: worldwake_core::OpportunityKey {
                    goal_key: worldwake_core::GoalKey::from(GoalKind::RestockCommodity {
                        commodity: CommodityKind::Bread,
                    }),
                    anchor: OpportunityAnchor::None,
                },
            })],
        )
        .expect("matching method should be selected");

        assert_eq!(selected.id, MethodSchemaId(10));
    }

    #[test]
    fn select_method_returns_none_when_no_method_matches_goal_kind() {
        let registry = registry([method(
            1,
            GoalKindDiscriminant::RestockCommodity,
            Vec::new(),
            Vec::new(),
        )]);

        let selected = select_method(
            entity(1),
            &goal(GoalKind::Sleep),
            &registry,
            &AgentSchemaContextProfile::default(),
            &TestBeliefView::default(),
            &[],
        );

        assert!(selected.is_none());
    }

    #[test]
    fn recipe_input_resource_source_precondition_resolves_first_recipe_input() {
        let actor = entity(1);
        let source_place = entity(10);
        let source = entity(20);
        let recipe_id = RecipeId(7);
        let mut view = TestBeliefView::default();
        view.recipes
            .insert(recipe_id, recipe(vec![(CommodityKind::Grain, Quantity(2))]));
        view.resource_sources
            .insert((source_place, CommodityKind::Grain), vec![source]);
        let registry = registry([method(
            42,
            GoalKindDiscriminant::ProduceCommodity,
            vec![MethodPrecondition::BeliefHolds(
                BeliefPredicate::ResourceSourceKnown {
                    commodity: CommodityTemplate::RecipeInput {
                        recipe: RecipeTemplate::GoalRecipe,
                        ordinal: 0,
                    },
                },
            )],
            Vec::new(),
        )]);

        let selected = select_method(
            actor,
            &produce_goal_with_evidence(recipe_id, source_place),
            &registry,
            &AgentSchemaContextProfile::default(),
            &view,
            &[],
        )
        .expect("recipe-input commodity should resolve through recipe definition");

        assert_eq!(selected.id, MethodSchemaId(42));
    }

    #[test]
    fn recipe_input_resource_source_precondition_fails_closed_for_invalid_ordinal() {
        let source_place = entity(10);
        let recipe_id = RecipeId(7);
        let mut view = TestBeliefView::default();
        view.recipes
            .insert(recipe_id, recipe(vec![(CommodityKind::Grain, Quantity(2))]));
        view.resource_sources
            .insert((source_place, CommodityKind::Grain), vec![entity(20)]);
        let registry = registry([method(
            42,
            GoalKindDiscriminant::ProduceCommodity,
            vec![MethodPrecondition::BeliefHolds(
                BeliefPredicate::ResourceSourceKnown {
                    commodity: CommodityTemplate::RecipeInput {
                        recipe: RecipeTemplate::GoalRecipe,
                        ordinal: 1,
                    },
                },
            )],
            Vec::new(),
        )]);

        let selected = select_method(
            entity(1),
            &produce_goal_with_evidence(recipe_id, source_place),
            &registry,
            &AgentSchemaContextProfile::default(),
            &view,
            &[],
        );

        assert!(
            selected.is_none(),
            "out-of-range recipe input ordinal must not satisfy a method precondition"
        );
    }

    #[test]
    fn canonical_produce_with_gather_selects_from_known_recipe_input_source() {
        let source_place = entity(10);
        let recipe_id = RecipeId(7);
        let mut view = TestBeliefView::default();
        view.recipes
            .insert(recipe_id, recipe(vec![(CommodityKind::Grain, Quantity(2))]));
        view.resource_sources
            .insert((source_place, CommodityKind::Grain), vec![entity(20)]);

        let registry = build_method_registry();
        let selected = select_method(
            entity(1),
            &produce_goal_with_evidence(recipe_id, source_place),
            &registry,
            &AgentSchemaContextProfile::default(),
            &view,
            &[],
        )
        .expect("canonical produce_with_gather should select from recipe input source beliefs");

        assert_eq!(selected.id, MethodSchemaId(5));
    }

    #[test]
    fn select_method_is_deterministic_across_repeated_calls() {
        let registry = registry([
            method(
                1,
                GoalKindDiscriminant::RestockCommodity,
                Vec::new(),
                vec![bias(MotiveSourceDiscriminant::Greed, 500)],
            ),
            method(
                2,
                GoalKindDiscriminant::RestockCommodity,
                Vec::new(),
                vec![bias(MotiveSourceDiscriminant::Greed, 500)],
            ),
        ]);
        let goal = goal(GoalKind::RestockCommodity {
            commodity: CommodityKind::Bread,
        });
        let motives = [motive(MotiveSource::Greed {
            opportunity: worldwake_core::OpportunityKey {
                goal_key: goal.key,
                anchor: OpportunityAnchor::None,
            },
        })];

        let first = select_method(
            entity(1),
            &goal,
            &registry,
            &AgentSchemaContextProfile::default(),
            &TestBeliefView::default(),
            &motives,
        )
        .expect("matching method should be selected") as *const MethodSchema;
        let second = select_method(
            entity(1),
            &goal,
            &registry,
            &AgentSchemaContextProfile::default(),
            &TestBeliefView::default(),
            &motives,
        )
        .expect("matching method should be selected") as *const MethodSchema;

        assert_eq!(first, second);
    }
}
