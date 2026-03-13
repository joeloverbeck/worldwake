use crate::{
    classify_band, derive_danger_pressure, derive_pain_pressure,
    enterprise::{market_signal_for_place, opportunity_signal},
    GoalPriorityClass, GroundedGoal, RankedGoal,
};
use std::cmp::Ordering;
use worldwake_core::{
    CommodityKind, CommodityPurpose, DriveThresholds, EntityId, GoalKind, HomeostaticNeeds,
    Permille, UtilityProfile,
};
use worldwake_sim::{BeliefView, RecipeRegistry};

#[must_use]
pub fn rank_candidates(
    candidates: &[GroundedGoal],
    view: &dyn BeliefView,
    agent: EntityId,
    utility: &UtilityProfile,
    recipes: &RecipeRegistry,
) -> Vec<RankedGoal> {
    let context = RankingContext::new(view, agent, utility);
    let mut ranked = candidates
        .iter()
        .filter(|candidate| !is_suppressed(candidate, &context))
        .map(|candidate| RankedGoal {
            grounded: candidate.clone(),
            priority_class: priority_class(candidate, &context),
            motive_score: motive_score(candidate, &context, recipes),
        })
        .collect::<Vec<_>>();

    ranked.sort_unstable_by(compare_ranked_goals);
    ranked
}

struct RankingContext<'a> {
    view: &'a dyn BeliefView,
    agent: EntityId,
    utility: &'a UtilityProfile,
    needs: Option<HomeostaticNeeds>,
    thresholds: Option<DriveThresholds>,
    danger_pressure: Permille,
}

impl<'a> RankingContext<'a> {
    fn new(view: &'a dyn BeliefView, agent: EntityId, utility: &'a UtilityProfile) -> Self {
        Self {
            view,
            agent,
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
        GoalKind::LootCorpse { .. } | GoalKind::BuryCorpse { .. }
    ) && (context.danger_high_or_above() || context.self_care_high_or_above())
}

fn priority_class(candidate: &GroundedGoal, context: &RankingContext<'_>) -> GoalPriorityClass {
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
        GoalKind::LootCorpse { .. } | GoalKind::BuryCorpse { .. } => GoalPriorityClass::Low,
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
        GoalKind::LootCorpse { .. } | GoalKind::BuryCorpse { .. } => 0,
    }
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
    }
}

#[cfg(test)]
mod tests {
    use super::rank_candidates;
    use crate::{GoalKey, GoalKind, GoalPriorityClass, GroundedGoal};
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        BodyCostPerTick, BodyPart, CombatProfile, CommodityConsumableProfile, CommodityKind,
        DemandObservation, DemandObservationReason, DeprivationKind, DriveThresholds, EntityId,
        EntityKind, HomeostaticNeeds, InTransitOnEdge, LoadUnits, MerchandiseProfile,
        MetabolismProfile, Permille, Quantity, RecipeId, ResourceSource, Tick, TickRange,
        TradeDispositionProfile, UniqueItemKind, UtilityProfile, WorkstationTag, Wound, WoundCause,
        WoundId,
    };
    use worldwake_sim::{
        ActionDuration, ActionPayload, BeliefView, DurationExpr, RecipeDefinition, RecipeRegistry,
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
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        item_lot_commodities: BTreeMap<EntityId, CommodityKind>,
    }

    impl BeliefView for TestBeliefView {
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
        }
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
            &utility(),
            &RecipeRegistry::new(),
        );

        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Medium);
        assert_eq!(ranked[0].motive_score, 200 * 1000);
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
            &utility(),
            &RecipeRegistry::new(),
        );
        assert_eq!(ranked[0].priority_class, GoalPriorityClass::Low);
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
            &utility(),
            &RecipeRegistry::new(),
        );
        let second = rank_candidates(
            &candidates,
            &view,
            agent,
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
