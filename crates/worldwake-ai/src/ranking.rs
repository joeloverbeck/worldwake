use crate::{
    classify_band, derive_danger_pressure, derive_pain_pressure,
    enterprise::{market_signal_for_place, opportunity_signal},
    GoalPriorityClass, GroundedGoal, RankedGoal,
};
use std::cmp::Ordering;
use worldwake_core::{
    belief_confidence, BelievedEntityState, BeliefConfidencePolicy, CommodityKind,
    CommodityPurpose, DriveThresholds, EntityId, GoalKind, HomeostaticNeeds, Permille, Tick,
    UtilityProfile,
};
use worldwake_sim::{GoalBeliefView, RecipeRegistry};

#[must_use]
pub fn rank_candidates(
    candidates: &[GroundedGoal],
    view: &dyn GoalBeliefView,
    agent: EntityId,
    current_tick: Tick,
    utility: &UtilityProfile,
    recipes: &RecipeRegistry,
) -> Vec<RankedGoal> {
    let context = RankingContext::new(view, agent, current_tick, utility);
    let mut ranked = candidates
        .iter()
        .filter(|candidate| !is_suppressed(candidate, &context))
        .map(|candidate| RankedGoal {
            grounded: candidate.clone(),
            priority_class: priority_class(candidate, &context, recipes),
            motive_score: motive_score(candidate, &context, recipes),
        })
        .collect::<Vec<_>>();

    ranked.sort_unstable_by(compare_ranked_goals);
    ranked
}

struct RankingContext<'a> {
    view: &'a dyn GoalBeliefView,
    agent: EntityId,
    current_tick: Tick,
    utility: &'a UtilityProfile,
    needs: Option<HomeostaticNeeds>,
    thresholds: Option<DriveThresholds>,
    danger_pressure: Permille,
}

impl<'a> RankingContext<'a> {
    fn new(
        view: &'a dyn GoalBeliefView,
        agent: EntityId,
        current_tick: Tick,
        utility: &'a UtilityProfile,
    ) -> Self {
        Self {
            view,
            agent,
            current_tick,
            utility,
            needs: view.homeostatic_needs(agent),
            thresholds: view.drive_thresholds(agent),
            danger_pressure: derive_danger_pressure(view, agent),
        }
    }

    fn self_care_high_or_above(&self) -> bool {
        self.max_self_care_class() >= GoalPriorityClass::High
    }

    fn danger_high_or_above(&self) -> bool {
        self.danger_class() >= GoalPriorityClass::High
    }

    fn danger_class(&self) -> GoalPriorityClass {
        self.thresholds
            .map_or(GoalPriorityClass::Background, |thresholds| {
                classify_band(self.danger_pressure, &thresholds.danger)
            })
    }

    fn max_self_care_class(&self) -> GoalPriorityClass {
        let Some(needs) = self.needs else {
            return GoalPriorityClass::Background;
        };
        let Some(thresholds) = self.thresholds else {
            return GoalPriorityClass::Background;
        };

        [
            classify_band(needs.hunger, &thresholds.hunger),
            classify_band(needs.thirst, &thresholds.thirst),
            classify_band(needs.fatigue, &thresholds.fatigue),
            classify_band(needs.bladder, &thresholds.bladder),
            classify_band(needs.dirtiness, &thresholds.dirtiness),
        ]
        .into_iter()
        .max()
        .unwrap_or(GoalPriorityClass::Background)
    }
}

fn is_suppressed(candidate: &GroundedGoal, context: &RankingContext<'_>) -> bool {
    matches!(
        candidate.key.kind,
        GoalKind::LootCorpse { .. } | GoalKind::BuryCorpse { .. } | GoalKind::ShareBelief { .. }
    ) && (context.danger_high_or_above() || context.self_care_high_or_above())
}

fn priority_class(
    candidate: &GroundedGoal,
    context: &RankingContext<'_>,
    recipes: &RecipeRegistry,
) -> GoalPriorityClass {
    match candidate.key.kind {
        GoalKind::ConsumeOwnedCommodity { commodity }
        | GoalKind::AcquireCommodity {
            commodity,
            purpose: CommodityPurpose::SelfConsume,
        } => self_consume_priority(commodity, context),
        GoalKind::AcquireCommodity {
            commodity: _,
            purpose: CommodityPurpose::Treatment,
        } => context
            .thresholds
            .map_or(GoalPriorityClass::Background, |thresholds| {
                classify_band(
                    derive_pain_pressure(context.view, context.agent),
                    &thresholds.pain,
                )
            }),
        GoalKind::AcquireCommodity {
            commodity: _,
            purpose: CommodityPurpose::RecipeInput(recipe_id),
        } => recipe_output_priority(recipe_id, context, recipes),
        GoalKind::AcquireCommodity { .. }
        | GoalKind::ProduceCommodity { .. }
        | GoalKind::SellCommodity { .. }
        | GoalKind::RestockCommodity { .. }
        | GoalKind::MoveCargo { .. } => GoalPriorityClass::Medium,
        GoalKind::Sleep => drive_priority(
            context,
            |needs| needs.fatigue,
            |thresholds| thresholds.fatigue,
        ),
        GoalKind::Relieve => drive_priority(
            context,
            |needs| needs.bladder,
            |thresholds| thresholds.bladder,
        ),
        GoalKind::Wash => drive_priority(
            context,
            |needs| needs.dirtiness,
            |thresholds| thresholds.dirtiness,
        ),
        GoalKind::EngageHostile { .. } | GoalKind::ReduceDanger => context.danger_class(),
        GoalKind::Heal { target } => {
            let target_pain = derive_pain_pressure(context.view, target);
            let pain_class = context
                .thresholds
                .map_or(GoalPriorityClass::Background, |thresholds| {
                    classify_band(target_pain, &thresholds.pain)
                });
            if context.danger_class() >= GoalPriorityClass::High {
                promote_priority_class(pain_class)
            } else {
                pain_class
            }
        }
        GoalKind::LootCorpse { .. }
        | GoalKind::BuryCorpse { .. }
        | GoalKind::ShareBelief { .. } => GoalPriorityClass::Low,
    }
}

fn self_consume_priority(
    commodity: CommodityKind,
    context: &RankingContext<'_>,
) -> GoalPriorityClass {
    relevant_self_consume_factors(commodity, context)
        .into_iter()
        .map(|(pressure, _, band)| classify_band(pressure, &band))
        .max()
        .unwrap_or(GoalPriorityClass::Background)
}

fn drive_priority(
    context: &RankingContext<'_>,
    pressure: impl Fn(HomeostaticNeeds) -> Permille,
    band: impl Fn(DriveThresholds) -> worldwake_core::ThresholdBand,
) -> GoalPriorityClass {
    match (context.needs, context.thresholds) {
        (Some(needs), Some(thresholds)) => classify_band(pressure(needs), &band(thresholds)),
        _ => GoalPriorityClass::Background,
    }
}

fn promote_priority_class(priority: GoalPriorityClass) -> GoalPriorityClass {
    match priority {
        GoalPriorityClass::Background => GoalPriorityClass::Low,
        GoalPriorityClass::Low => GoalPriorityClass::Medium,
        GoalPriorityClass::Medium => GoalPriorityClass::High,
        GoalPriorityClass::High | GoalPriorityClass::Critical => GoalPriorityClass::Critical,
    }
}

fn motive_score(
    candidate: &GroundedGoal,
    context: &RankingContext<'_>,
    recipes: &RecipeRegistry,
) -> u32 {
    match candidate.key.kind {
        GoalKind::ConsumeOwnedCommodity { commodity }
        | GoalKind::AcquireCommodity {
            commodity,
            purpose: CommodityPurpose::SelfConsume,
        } => relevant_self_consume_factors(commodity, context)
            .into_iter()
            .map(|(pressure, weight, _)| score_product(weight, pressure))
            .max()
            .unwrap_or(0),
        GoalKind::AcquireCommodity {
            commodity,
            purpose: CommodityPurpose::Treatment,
        } => treatment_score(commodity, context),
        GoalKind::AcquireCommodity {
            commodity: _,
            purpose: CommodityPurpose::RecipeInput(recipe_id),
        } => recipe_output_motive_score(recipe_id, context, recipes),
        GoalKind::AcquireCommodity { commodity, .. }
        | GoalKind::SellCommodity { commodity }
        | GoalKind::RestockCommodity { commodity } => enterprise_score(commodity, context),
        GoalKind::Sleep => drive_score(
            context,
            |needs| needs.fatigue,
            |utility| utility.fatigue_weight,
        ),
        GoalKind::Relieve => drive_score(
            context,
            |needs| needs.bladder,
            |utility| utility.bladder_weight,
        ),
        GoalKind::Wash => drive_score(
            context,
            |needs| needs.dirtiness,
            |utility| utility.dirtiness_weight,
        ),
        GoalKind::EngageHostile { .. } | GoalKind::ReduceDanger => {
            score_product(context.utility.danger_weight, context.danger_pressure)
        }
        GoalKind::Heal { target } => {
            let pain_score = score_product(
                context.utility.pain_weight,
                derive_pain_pressure(context.view, target),
            );
            if context.danger_pressure.value() == 0 {
                pain_score
            } else {
                pain_score + score_product(context.utility.danger_weight, context.danger_pressure)
            }
        }
        GoalKind::ProduceCommodity { recipe_id } => {
            let signal = recipes
                .get(recipe_id)
                .map_or(Permille::new_unchecked(0), |recipe| {
                    recipe
                        .outputs
                        .iter()
                        .map(|(commodity, _)| {
                            opportunity_signal(
                                context.view,
                                context.agent,
                                context.view.effective_place(context.agent),
                                *commodity,
                            )
                        })
                        .max_by_key(|signal| signal.value())
                        .unwrap_or(Permille::new_unchecked(0))
                });
            score_product(context.utility.enterprise_weight, signal)
        }
        GoalKind::MoveCargo {
            commodity,
            destination,
        } => {
            let signal =
                market_signal_for_place(context.view, context.agent, commodity, destination);
            score_product(context.utility.enterprise_weight, signal)
        }
        GoalKind::ShareBelief { subject, .. } => score_product(
            context.utility.social_weight,
            social_pressure_for_subject(context, subject),
        ),
        GoalKind::LootCorpse { .. } | GoalKind::BuryCorpse { .. } => 1,
    }
}

fn social_pressure_for_subject(context: &RankingContext<'_>, subject: EntityId) -> Permille {
    let belief = context
        .view
        .known_entity_beliefs(context.agent)
        .into_iter()
        .find_map(|(entity, belief)| (entity == subject).then_some(belief));

    belief.map_or(Permille::new_unchecked(0), |belief| {
        belief_pressure_from_state(&belief, context.current_tick)
    })
}

fn belief_pressure_from_state(state: &BelievedEntityState, current_tick: Tick) -> Permille {
    let staleness_ticks = current_tick.0.saturating_sub(state.observed_tick.0);
    belief_confidence(
        &state.source,
        staleness_ticks,
        &BeliefConfidencePolicy::default(),
    )
}

fn drive_score(
    context: &RankingContext<'_>,
    pressure: impl Fn(HomeostaticNeeds) -> Permille,
    weight: impl Fn(&UtilityProfile) -> Permille,
) -> u32 {
    match context.needs {
        Some(needs) => score_product(weight(context.utility), pressure(needs)),
        None => 0,
    }
}

fn treatment_score(commodity: CommodityKind, context: &RankingContext<'_>) -> u32 {
    if commodity.spec().treatment_profile.is_none() {
        return 0;
    }

    let pain_score = score_product(
        context.utility.pain_weight,
        derive_pain_pressure(context.view, context.agent),
    );
    if context.danger_pressure.value() == 0 {
        pain_score
    } else {
        pain_score + score_product(context.utility.danger_weight, context.danger_pressure)
    }
}

fn recipe_output_priority(
    recipe_id: worldwake_core::RecipeId,
    context: &RankingContext<'_>,
    recipes: &RecipeRegistry,
) -> GoalPriorityClass {
    recipes
        .get(recipe_id)
        .map_or(GoalPriorityClass::Background, |recipe| {
            recipe
                .outputs
                .iter()
                .map(|(commodity, _)| commodity_goal_priority(*commodity, context))
                .max()
                .unwrap_or(GoalPriorityClass::Background)
        })
}

fn commodity_goal_priority(
    commodity: CommodityKind,
    context: &RankingContext<'_>,
) -> GoalPriorityClass {
    let self_consume = self_consume_priority(commodity, context);
    if self_consume > GoalPriorityClass::Background {
        return self_consume;
    }

    if commodity.spec().treatment_profile.is_some() {
        return context
            .thresholds
            .map_or(GoalPriorityClass::Background, |thresholds| {
                classify_band(
                    derive_pain_pressure(context.view, context.agent),
                    &thresholds.pain,
                )
            });
    }

    if enterprise_score(commodity, context) > 0 {
        GoalPriorityClass::Medium
    } else {
        GoalPriorityClass::Background
    }
}

fn recipe_output_motive_score(
    recipe_id: worldwake_core::RecipeId,
    context: &RankingContext<'_>,
    recipes: &RecipeRegistry,
) -> u32 {
    recipes.get(recipe_id).map_or(0, |recipe| {
        recipe
            .outputs
            .iter()
            .map(|(commodity, _)| commodity_goal_motive_score(*commodity, context))
            .max()
            .unwrap_or(0)
    })
}

fn commodity_goal_motive_score(commodity: CommodityKind, context: &RankingContext<'_>) -> u32 {
    let self_consume = relevant_self_consume_factors(commodity, context)
        .into_iter()
        .map(|(pressure, weight, _)| score_product(weight, pressure))
        .max()
        .unwrap_or(0);
    if self_consume > 0 {
        return self_consume;
    }

    let treatment = treatment_score(commodity, context);
    if treatment > 0 {
        return treatment;
    }

    enterprise_score(commodity, context)
}

fn relevant_self_consume_factors(
    commodity: CommodityKind,
    context: &RankingContext<'_>,
) -> Vec<(Permille, Permille, worldwake_core::ThresholdBand)> {
    let Some(needs) = context.needs else {
        return Vec::new();
    };
    let Some(thresholds) = context.thresholds else {
        return Vec::new();
    };

    let Some(profile) = commodity.spec().consumable_profile else {
        return Vec::new();
    };

    let mut factors = Vec::new();
    if profile.hunger_relief_per_unit.value() > 0 {
        factors.push((
            needs.hunger,
            context.utility.hunger_weight,
            thresholds.hunger,
        ));
    }
    if profile.thirst_relief_per_unit.value() > 0 {
        factors.push((
            needs.thirst,
            context.utility.thirst_weight,
            thresholds.thirst,
        ));
    }
    factors
}

fn enterprise_score(commodity: CommodityKind, context: &RankingContext<'_>) -> u32 {
    let signal = opportunity_signal(
        context.view,
        context.agent,
        context.view.effective_place(context.agent),
        commodity,
    );
    score_product(context.utility.enterprise_weight, signal)
}

fn score_product(weight: Permille, pressure: Permille) -> u32 {
    u32::from(weight.value()) * u32::from(pressure.value())
}

fn compare_ranked_goals(left: &RankedGoal, right: &RankedGoal) -> Ordering {
    right
        .priority_class
        .cmp(&left.priority_class)
        .then_with(|| right.motive_score.cmp(&left.motive_score))
        .then_with(|| {
            goal_kind_discriminant(left.grounded.key.kind)
                .cmp(&goal_kind_discriminant(right.grounded.key.kind))
        })
        .then_with(|| {
            left.grounded
                .key
                .commodity
                .cmp(&right.grounded.key.commodity)
        })
        .then_with(|| left.grounded.key.entity.cmp(&right.grounded.key.entity))
        .then_with(|| left.grounded.key.place.cmp(&right.grounded.key.place))
}

fn goal_kind_discriminant(kind: GoalKind) -> u8 {
    match kind {
        GoalKind::ConsumeOwnedCommodity { .. } => 0,
        GoalKind::AcquireCommodity { .. } => 1,
        GoalKind::Sleep => 2,
        GoalKind::Relieve => 3,
        GoalKind::Wash => 4,
        GoalKind::EngageHostile { .. } => 5,
        GoalKind::ReduceDanger => 6,
        GoalKind::Heal { .. } => 7,
        GoalKind::ProduceCommodity { .. } => 8,
        GoalKind::SellCommodity { .. } => 9,
        GoalKind::RestockCommodity { .. } => 10,
        GoalKind::MoveCargo { .. } => 11,
        GoalKind::LootCorpse { .. } => 12,
        GoalKind::BuryCorpse { .. } => 13,
        GoalKind::ShareBelief { .. } => 14,
    }
}

#[cfg(test)]
mod tests {
    use super::rank_candidates;
    use crate::{GoalKey, GoalKind, GoalPriorityClass, GroundedGoal};
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        belief_confidence, BeliefConfidencePolicy, BelievedEntityState, BodyCostPerTick, BodyPart,
        CombatProfile, CommodityConsumableProfile, CommodityKind, CommodityPurpose,
        DemandObservation, DemandObservationReason, DeprivationKind, DriveThresholds, EntityId,
        EntityKind, HomeostaticNeeds, InTransitOnEdge, LoadUnits, MerchandiseProfile,
        MetabolismProfile, PerceptionSource, Permille, Quantity, RecipeId, ResourceSource, Tick,
        TickRange, TradeDispositionProfile, UniqueItemKind, UtilityProfile, WorkstationTag, Wound,
        WoundCause, WoundId,
    };
    use worldwake_sim::{
        ActionDuration, ActionPayload, DurationExpr, RecipeDefinition, RecipeRegistry,
        RuntimeBeliefView,
    };

    #[derive(Default)]
    struct TestBeliefView {
        alive: BTreeSet<EntityId>,
        needs: BTreeMap<EntityId, HomeostaticNeeds>,
        thresholds: BTreeMap<EntityId, DriveThresholds>,
        wounds: BTreeMap<EntityId, Vec<Wound>>,
        hostiles: BTreeMap<EntityId, Vec<EntityId>>,
        attackers: BTreeMap<EntityId, Vec<EntityId>>,
        merchandise_profiles: BTreeMap<EntityId, MerchandiseProfile>,
        demand_memory: BTreeMap<EntityId, Vec<DemandObservation>>,
        beliefs: BTreeMap<EntityId, Vec<(EntityId, BelievedEntityState)>>,
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        item_lot_commodities: BTreeMap<EntityId, CommodityKind>,
    }

    worldwake_sim::impl_goal_belief_view!(TestBeliefView);

    impl RuntimeBeliefView for TestBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.contains(&entity)
        }
        fn entity_kind(&self, _entity: EntityId) -> Option<EntityKind> {
            None
        }
        fn effective_place(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }
        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }
        fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
            self.beliefs.get(&agent).cloned().unwrap_or_default()
        }
        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
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
        fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> {
            self.item_lot_commodities.get(&entity).copied()
        }
        fn item_lot_consumable_profile(
            &self,
            _entity: EntityId,
        ) -> Option<CommodityConsumableProfile> {
            None
        }
        fn direct_container(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }
        fn direct_possessor(&self, _entity: EntityId) -> Option<EntityId> {
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
        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }
        fn has_control(&self, _entity: EntityId) -> bool {
            false
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
        fn is_dead(&self, _entity: EntityId) -> bool {
            false
        }
        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }
        fn has_wounds(&self, entity: EntityId) -> bool {
            self.wounds
                .get(&entity)
                .is_some_and(|wounds| !wounds.is_empty())
        }
        fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
            self.needs.get(&agent).copied()
        }
        fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
            self.thresholds.get(&agent).copied()
        }
        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }
        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
        }
        fn travel_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::TravelDispositionProfile> {
            None
        }
        fn combat_profile(&self, _agent: EntityId) -> Option<CombatProfile> {
            None
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
            _place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
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

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn demand(place: EntityId, commodity: CommodityKind, quantity: u32) -> DemandObservation {
        DemandObservation {
            commodity,
            quantity: Quantity(quantity),
            place,
            tick: Tick(1),
            counterparty: None,
            reason: DemandObservationReason::WantedToBuyButNoSeller,
        }
    }

    fn believed_state(observed_tick: u64, source: PerceptionSource) -> BelievedEntityState {
        BelievedEntityState {
            last_known_place: Some(entity(99)),
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            observed_tick: Tick(observed_tick),
            source,
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

    fn goal(kind: GoalKind) -> GroundedGoal {
        GroundedGoal {
            key: GoalKey::from(kind),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
        }
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
        }
    }

    fn current_tick() -> Tick {
        Tick(10)
    }

    fn base_view(agent: EntityId) -> TestBeliefView {
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(pm(100), pm(100), pm(100), pm(100), pm(100)),
        );
        view.thresholds.insert(agent, DriveThresholds::default());
        view
    }

    #[test]
    fn hunger_candidate_becomes_critical_and_uses_weight_times_pressure() {
        let agent = entity(1);
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.critical(), pm(0), pm(0), pm(0), pm(0)),
        );

        let ranked = rank_candidates(
            &[goal(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        );

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Critical);
        assert_eq!(ranked[0].motive_score, 900 * 900);
    }

    #[test]
    fn enterprise_goals_are_capped_at_medium_even_with_full_signal() {
        let agent = entity(1);
        let market = entity(2);
        let mut view = base_view(agent);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(market),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 10)]);

        let ranked = rank_candidates(
            &[goal(GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        );

        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(ranked[0].motive_score, 200 * 1000);
    }

    #[test]
    fn recipe_input_goals_inherit_downstream_self_care_priority_and_score() {
        let agent = entity(1);
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.critical(), pm(0), pm(0), pm(0), pm(0)),
        );

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

        let ranked = rank_candidates(
            &[goal(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Firewood,
                purpose: CommodityPurpose::RecipeInput(recipe_id),
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
            &recipes,
        );

        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Critical);
        assert_eq!(ranked[0].motive_score, 900 * 900);
    }

    #[test]
    fn move_cargo_scoring_uses_goal_commodity_directly() {
        let agent = entity(1);
        let market = entity(2);
        let mut view = base_view(agent);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(market),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 10)]);

        let ranked = rank_candidates(
            &[goal(GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination: market,
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        );

        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(ranked[0].motive_score, 200 * 1000);
    }

    #[test]
    fn loot_candidates_are_low_and_suppressed_by_high_danger_or_self_care() {
        let agent = entity(1);
        let attacker = entity(9);
        let corpse = entity(3);
        let mut danger_view = base_view(agent);
        danger_view.attackers.insert(agent, vec![attacker]);

        let ranked = rank_candidates(
            &[goal(GoalKind::LootCorpse { corpse })],
            &danger_view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        );
        assert!(ranked.is_empty());

        let mut self_care_view = base_view(agent);
        let thresholds = DriveThresholds::default();
        self_care_view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.high(), pm(0), pm(0), pm(0), pm(0)),
        );

        let ranked = rank_candidates(
            &[goal(GoalKind::LootCorpse { corpse })],
            &self_care_view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        );
        assert!(ranked.is_empty());

        let ranked = rank_candidates(
            &[goal(GoalKind::BuryCorpse {
                corpse,
                burial_site: entity(4),
            })],
            &base_view(agent),
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        );
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Low);
        assert_eq!(ranked[0].motive_score, 1);
    }

    #[test]
    fn share_belief_is_low_priority_and_suppressed_by_high_danger_or_self_care() {
        let agent = entity(1);
        let listener = entity(2);
        let subject = entity(3);
        let attacker = entity(9);
        let mut danger_view = base_view(agent);
        danger_view.attackers.insert(agent, vec![attacker]);
        danger_view.beliefs.insert(
            agent,
            vec![(subject, believed_state(9, PerceptionSource::DirectObservation))],
        );

        let ranked = rank_candidates(
            &[goal(GoalKind::ShareBelief { listener, subject })],
            &danger_view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        );
        assert!(ranked.is_empty());

        let mut self_care_view = base_view(agent);
        let thresholds = DriveThresholds::default();
        self_care_view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.high(), pm(0), pm(0), pm(0), pm(0)),
        );
        self_care_view.beliefs.insert(
            agent,
            vec![(subject, believed_state(9, PerceptionSource::DirectObservation))],
        );

        let ranked = rank_candidates(
            &[goal(GoalKind::ShareBelief { listener, subject })],
            &self_care_view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        );
        assert!(ranked.is_empty());

        let mut calm_view = base_view(agent);
        calm_view.beliefs.insert(
            agent,
            vec![(subject, believed_state(9, PerceptionSource::DirectObservation))],
        );
        let ranked = rank_candidates(
            &[goal(GoalKind::ShareBelief { listener, subject })],
            &calm_view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        );

        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Low);
        assert_eq!(
            ranked[0].motive_score,
            150
                * u32::from(
                    belief_confidence(
                        &PerceptionSource::DirectObservation,
                        1,
                        &BeliefConfidencePolicy::default(),
                    )
                    .value(),
                )
        );
    }

    #[test]
    fn share_belief_scoring_tracks_social_weight_and_subject_belief_confidence() {
        let agent = entity(1);
        let listener = entity(2);
        let fresh_subject = entity(3);
        let rumor_subject = entity(4);
        let mut view = base_view(agent);
        view.beliefs.insert(
            agent,
            vec![
                (
                    fresh_subject,
                    believed_state(9, PerceptionSource::DirectObservation),
                ),
                (
                    rumor_subject,
                    believed_state(1, PerceptionSource::Rumor { chain_len: 3 }),
                ),
            ],
        );

        let baseline = utility();
        let stronger_social = UtilityProfile {
            social_weight: pm(300),
            ..baseline.clone()
        };
        let fresh_goal = goal(GoalKind::ShareBelief {
            listener,
            subject: fresh_subject,
        });
        let rumor_goal = goal(GoalKind::ShareBelief {
            listener,
            subject: rumor_subject,
        });

        let baseline_ranked = rank_candidates(
            &[fresh_goal.clone(), rumor_goal.clone()],
            &view,
            agent,
            current_tick(),
            &baseline,
            &RecipeRegistry::new(),
        );
        let boosted_ranked = rank_candidates(
            &[fresh_goal, rumor_goal],
            &view,
            agent,
            current_tick(),
            &stronger_social,
            &RecipeRegistry::new(),
        );

        let fresh_pressure = belief_confidence(
            &PerceptionSource::DirectObservation,
            1,
            &BeliefConfidencePolicy::default(),
        );
        let rumor_pressure = belief_confidence(
            &PerceptionSource::Rumor { chain_len: 3 },
            9,
            &BeliefConfidencePolicy::default(),
        );

        assert!(baseline_ranked[0].motive_score > baseline_ranked[1].motive_score);
        assert_eq!(baseline_ranked[0].motive_score, 150 * u32::from(fresh_pressure.value()));
        assert_eq!(baseline_ranked[1].motive_score, 150 * u32::from(rumor_pressure.value()));
        assert_eq!(boosted_ranked[0].motive_score, 300 * u32::from(fresh_pressure.value()));
    }

    #[test]
    fn share_belief_scoring_is_zero_without_social_weight_or_known_subject() {
        let agent = entity(1);
        let listener = entity(2);
        let known_subject = entity(3);
        let missing_subject = entity(4);
        let mut view = base_view(agent);
        view.beliefs.insert(
            agent,
            vec![(known_subject, believed_state(9, PerceptionSource::DirectObservation))],
        );

        let zero_social = UtilityProfile {
            social_weight: pm(0),
            ..utility()
        };
        let ranked = rank_candidates(
            &[
                goal(GoalKind::ShareBelief {
                    listener,
                    subject: known_subject,
                }),
                goal(GoalKind::ShareBelief {
                    listener,
                    subject: missing_subject,
                }),
            ],
            &view,
            agent,
            current_tick(),
            &zero_social,
            &RecipeRegistry::new(),
        );

        assert_eq!(ranked[0].motive_score, 0);
        assert_eq!(ranked[1].motive_score, 0);
    }

    #[test]
    fn medium_priority_enterprise_and_critical_self_care_outrank_share_belief() {
        let agent = entity(1);
        let listener = entity(2);
        let subject = entity(3);
        let market = entity(4);
        let mut enterprise_view = base_view(agent);
        enterprise_view.beliefs.insert(
            agent,
            vec![(subject, believed_state(9, PerceptionSource::DirectObservation))],
        );
        enterprise_view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(market),
            },
        );
        enterprise_view
            .demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 10)]);

        let enterprise_first = rank_candidates(
            &[
                goal(GoalKind::ShareBelief { listener, subject }),
                goal(GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread,
                }),
            ],
            &enterprise_view,
            agent,
            current_tick(),
            &UtilityProfile {
                enterprise_weight: pm(1),
                social_weight: pm(1000),
                ..utility()
            },
            &RecipeRegistry::new(),
        );
        assert!(matches!(
            enterprise_first[0].grounded.key.kind,
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread
            }
        ));

        let mut self_care_view = enterprise_view;
        let thresholds = DriveThresholds::default();
        self_care_view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.critical(), pm(0), pm(0), pm(0), pm(0)),
        );
        let self_care_first = rank_candidates(
            &[
                goal(GoalKind::ShareBelief { listener, subject }),
                goal(GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Bread,
                }),
            ],
            &self_care_view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        );
        assert!(matches!(
            self_care_first[0].grounded.key.kind,
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread
            }
        ));
    }

    #[test]
    fn enterprise_does_not_outrank_critical_self_care() {
        let agent = entity(1);
        let market = entity(2);
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(thresholds.hunger.critical(), pm(0), pm(0), pm(0), pm(0)),
        );
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(market),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 10)]);

        let ranked = rank_candidates(
            &[
                goal(GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread,
                }),
                goal(GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Bread,
                }),
            ],
            &view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        );

        assert!(matches!(
            ranked.first().map(|goal| goal.grounded.key.kind),
            Some(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread
            })
        ));
    }

    #[test]
    fn same_priority_candidates_sort_by_motive_then_kind_then_ids() {
        let agent = entity(1);
        let market = entity(2);
        let corpse_a = entity(10);
        let corpse_b = entity(11);
        let mut view = base_view(agent);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread, CommodityKind::Water]),
                home_market: Some(market),
            },
        );
        view.demand_memory.insert(
            agent,
            vec![
                demand(market, CommodityKind::Bread, 10),
                demand(market, CommodityKind::Water, 5),
            ],
        );

        let ranked = rank_candidates(
            &[
                goal(GoalKind::RestockCommodity {
                    commodity: CommodityKind::Water,
                }),
                goal(GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread,
                }),
                goal(GoalKind::LootCorpse { corpse: corpse_b }),
                goal(GoalKind::LootCorpse { corpse: corpse_a }),
            ],
            &view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        );

        assert!(matches!(
            ranked[0].grounded.key.kind,
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread
            }
        ));
        assert!(matches!(
            ranked[1].grounded.key.kind,
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Water
            }
        ));
        assert!(matches!(
            ranked[2].grounded.key.kind,
            GoalKind::LootCorpse { corpse } if corpse == corpse_a
        ));
        assert!(matches!(
            ranked[3].grounded.key.kind,
            GoalKind::LootCorpse { corpse } if corpse == corpse_b
        ));
    }

    #[test]
    fn opportunity_signal_is_zero_without_demand_memory() {
        let agent = entity(1);
        let market = entity(2);
        let mut view = base_view(agent);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(market),
            },
        );

        let ranked = rank_candidates(
            &[goal(GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            })],
            &view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        );

        assert_eq!(ranked[0].motive_score, 0);
    }

    #[test]
    fn heal_uses_target_pain_and_is_promoted_by_high_danger() {
        let agent = entity(1);
        let target = entity(7);
        let attacker = entity(9);
        let mut view = base_view(agent);
        view.wounds.insert(target, vec![wound(650)]);
        view.attackers.insert(agent, vec![attacker]);

        let ranked = rank_candidates(
            &[goal(GoalKind::Heal { target })],
            &view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        );

        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Critical);
        assert_eq!(ranked[0].motive_score, (400 * 650) + (300 * 550));
    }

    #[test]
    fn produce_commodity_uses_recipe_outputs_for_opportunity_signal() {
        let agent = entity(1);
        let market = entity(2);
        let mut view = base_view(agent);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(market),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 10)]);
        let mut recipes = RecipeRegistry::new();
        let recipe_id = recipes.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: None,
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::new(pm(1), pm(1), pm(1), pm(1)),
        });

        let ranked = rank_candidates(
            &[goal(GoalKind::ProduceCommodity { recipe_id })],
            &view,
            agent,
            current_tick(),
            &utility(),
            &recipes,
        );

        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(ranked[0].motive_score, 200 * 1000);
    }

    #[test]
    fn ranking_is_deterministic_for_identical_inputs() {
        let agent = entity(1);
        let market = entity(2);
        let mut view = base_view(agent);
        view.merchandise_profiles.insert(
            agent,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(market),
            },
        );
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 10)]);
        let candidates = vec![
            goal(GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            }),
            goal(GoalKind::LootCorpse { corpse: entity(5) }),
            goal(GoalKind::Sleep),
        ];

        let first = rank_candidates(
            &candidates,
            &view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        );
        let second = rank_candidates(
            &candidates,
            &view,
            agent,
            current_tick(),
            &utility(),
            &RecipeRegistry::new(),
        );

        assert_eq!(first, second);
    }

    #[test]
    fn simultaneous_critical_self_care_needs_rank_by_weighted_order() {
        let agent = entity(1);
        let mut view = base_view(agent);
        let thresholds = DriveThresholds::default();
        view.needs.insert(
            agent,
            HomeostaticNeeds::new(
                thresholds.hunger.critical(),
                thresholds.thirst.critical(),
                thresholds.fatigue.critical(),
                pm(0),
                pm(0),
            ),
        );
        let utility = UtilityProfile {
            hunger_weight: pm(800),
            thirst_weight: pm(600),
            fatigue_weight: pm(400),
            ..UtilityProfile::default()
        };

        let ranked = rank_candidates(
            &[
                goal(GoalKind::Sleep),
                goal(GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Water,
                }),
                goal(GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Bread,
                }),
            ],
            &view,
            agent,
            current_tick(),
            &utility,
            &RecipeRegistry::new(),
        );

        assert!(matches!(
            ranked[0].grounded.key.kind,
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread
            }
        ));
        assert!(matches!(
            ranked[1].grounded.key.kind,
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Water
            }
        ));
        assert!(matches!(ranked[2].grounded.key.kind, GoalKind::Sleep));
    }
}
