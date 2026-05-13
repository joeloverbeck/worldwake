use crate::{GoalBeliefView, commodity_opportunity_score};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldwake_core::{
    CommodityKind, DemandMemory, EntityId, HomeostaticNeeds, Quantity, WoundList,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TradeAcceptance {
    Accept,
    Reject { reason: TradeRejectionReason },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TradeRejectionReason {
    PostTradeStateWorse,
    InsufficientPayment,
    NoNeed,
}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
struct ValuationSnapshot {
    survival: u64,
    wound: u64,
    demand: u64,
    coin: u64,
}

#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn evaluate_trade_bundle(
    actor: EntityId,
    belief: &dyn GoalBeliefView,
    _needs: Option<&HomeostaticNeeds>,
    _wounds: Option<&WoundList>,
    current_coin: Quantity,
    offered: &[(CommodityKind, Quantity)],
    received: &[(CommodityKind, Quantity)],
    local_alternatives: &[(EntityId, CommodityKind, Quantity)],
    _demand_memory: Option<&DemandMemory>,
) -> TradeAcceptance {
    let current_holdings = build_current_holdings(actor, belief, current_coin);
    let alternative_supply = aggregate_local_alternatives(actor, local_alternatives);
    let current_snapshot = snapshot(actor, belief, &current_holdings, &alternative_supply);

    let Some(receipt_only_holdings) = apply_bundle_changes(&current_holdings, received, &[]) else {
        return TradeAcceptance::Reject {
            reason: TradeRejectionReason::PostTradeStateWorse,
        };
    };
    let receipt_only_snapshot =
        snapshot(actor, belief, &receipt_only_holdings, &alternative_supply);

    let Some(post_trade_holdings) = apply_bundle_changes(&current_holdings, received, offered)
    else {
        return TradeAcceptance::Reject {
            reason: insufficient_payment_reason(offered),
        };
    };
    let post_trade_snapshot = snapshot(actor, belief, &post_trade_holdings, &alternative_supply);

    if post_trade_snapshot > current_snapshot {
        return TradeAcceptance::Accept;
    }

    if receipt_only_snapshot <= current_snapshot {
        return TradeAcceptance::Reject {
            reason: TradeRejectionReason::NoNeed,
        };
    }

    if received
        .iter()
        .all(|(kind, _)| *kind == CommodityKind::Coin)
    {
        return TradeAcceptance::Reject {
            reason: TradeRejectionReason::InsufficientPayment,
        };
    }

    TradeAcceptance::Reject {
        reason: TradeRejectionReason::PostTradeStateWorse,
    }
}

fn build_current_holdings(
    actor: EntityId,
    belief: &dyn GoalBeliefView,
    current_coin: Quantity,
) -> BTreeMap<CommodityKind, u32> {
    let mut holdings = BTreeMap::new();
    for kind in CommodityKind::ALL {
        let quantity = if kind == CommodityKind::Coin {
            current_coin.0
        } else {
            belief.commodity_quantity(actor, kind).0
        };
        holdings.insert(kind, quantity);
    }
    holdings
}

fn aggregate_local_alternatives(
    actor: EntityId,
    local_alternatives: &[(EntityId, CommodityKind, Quantity)],
) -> BTreeMap<CommodityKind, u32> {
    let mut by_kind = BTreeMap::new();
    for (entity, kind, quantity) in local_alternatives {
        if *entity == actor {
            continue;
        }
        *by_kind.entry(*kind).or_insert(0) += quantity.0;
    }
    by_kind
}

fn apply_bundle_changes(
    current_holdings: &BTreeMap<CommodityKind, u32>,
    received: &[(CommodityKind, Quantity)],
    offered: &[(CommodityKind, Quantity)],
) -> Option<BTreeMap<CommodityKind, u32>> {
    let mut next = current_holdings.clone();

    for (kind, quantity) in received {
        let entry = next.entry(*kind).or_insert(0);
        *entry = entry.checked_add(quantity.0)?;
    }

    for (kind, quantity) in offered {
        let entry = next.entry(*kind).or_insert(0);
        *entry = entry.checked_sub(quantity.0)?;
    }

    Some(next)
}

fn snapshot(
    actor: EntityId,
    belief: &dyn GoalBeliefView,
    holdings: &BTreeMap<CommodityKind, u32>,
    alternative_supply: &BTreeMap<CommodityKind, u32>,
) -> ValuationSnapshot {
    let mut survival = 0_u64;
    let mut wound = 0_u64;
    let mut demand = 0_u64;
    for kind in CommodityKind::ALL {
        let breakdown =
            commodity_opportunity_score(actor, kind, belief, holdings, alternative_supply);
        survival += u64::from(breakdown.direct_survival_score);
        wound += u64::from(breakdown.treatment_score);
        demand += u64::from(
            breakdown
                .enterprise_score
                .saturating_add(breakdown.indirect_recipe_score),
        );
    }

    ValuationSnapshot {
        survival,
        wound,
        demand,
        coin: holdings
            .get(&CommodityKind::Coin)
            .copied()
            .unwrap_or(0)
            .into(),
    }
}

fn insufficient_payment_reason(offered: &[(CommodityKind, Quantity)]) -> TradeRejectionReason {
    if offered.iter().any(|(kind, _)| *kind == CommodityKind::Coin) {
        TradeRejectionReason::InsufficientPayment
    } else {
        TradeRejectionReason::PostTradeStateWorse
    }
}

#[cfg(test)]
mod tests {
    use super::{
        TradeAcceptance, TradeRejectionReason, apply_bundle_changes, build_current_holdings,
        evaluate_trade_bundle, snapshot,
    };
    use crate::BeliefRead;
    use crate::{
        CombatBeliefView, ControlBeliefView, EconomicBeliefView, EntityBeliefView,
        ProfileBeliefView, RecipeDefinition, RuntimeBeliefView, SpatialBeliefView,
        TemporalBeliefView, commodity_opportunity_score,
    };
    use std::collections::BTreeMap;
    use std::num::NonZeroU8;
    use worldwake_core::{
        BelievedEntityState, BodyPart, CombatProfile, CommodityConsumableProfile, CommodityKind,
        CommodityValuationProfile, DemandMemory, DemandObservation, DemandObservationReason,
        DriveThresholds, EntityId, EntityKind, HomeostaticNeeds, InTransitOnEdge, LoadUnits,
        MerchandiseProfile, MetabolismProfile, Permille, Quantity, RecipeId, ResourceSource,
        TellProfile, Tick, TickRange, TradeDispositionProfile, UniqueItemKind, WorkstationTag,
        Wound, WoundCause, WoundList,
    };

    #[derive(Default)]
    struct StubBeliefView {
        commodities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        needs: Option<HomeostaticNeeds>,
        wounds: Vec<Wound>,
        demand_memory: Vec<DemandObservation>,
        effective_place: Option<EntityId>,
        commodity_valuation_profile: Option<CommodityValuationProfile>,
        known_recipes: Vec<RecipeId>,
        recipe_definitions: BTreeMap<RecipeId, RecipeDefinition>,
        matching_workstations: BTreeMap<(EntityId, WorkstationTag), Vec<EntityId>>,
    }

    impl ControlBeliefView for StubBeliefView {
        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }

        fn has_control(&self, _entity: EntityId) -> bool {
            false
        }
    }

    impl crate::BelievedAuthorityView for StubBeliefView {
        fn believed_owner_of(&self, _entity: EntityId) -> BeliefRead<EntityId> {
            BeliefRead::Unknown
        }
    }

    impl EntityBeliefView for StubBeliefView {
        fn is_alive(&self, _entity: EntityId) -> bool {
            false
        }

        fn entity_kind(&self, _entity: EntityId) -> Option<EntityKind> {
            None
        }

        fn is_dead(&self, _entity: EntityId) -> bool {
            false
        }

        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
    }

    impl ProfileBeliefView for StubBeliefView {
        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            self.needs
        }

        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
        }

        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }
    }

    impl SpatialBeliefView for StubBeliefView {
        fn effective_place(&self, _entity: EntityId) -> Option<EntityId> {
            self.effective_place
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

    impl TemporalBeliefView for StubBeliefView {
        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }

        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
            Vec::new()
        }

        fn estimate_duration(
            &self,
            _actor: EntityId,
            duration: &crate::DurationExpr,
            _targets: &[EntityId],
            _payload: &crate::ActionPayload,
        ) -> Option<crate::ActionDuration> {
            duration.fixed_ticks().map(crate::ActionDuration::new)
        }
    }

    impl RuntimeBeliefView for StubBeliefView {}

    impl crate::SocialBeliefView for StubBeliefView {
        fn known_entity_beliefs(&self, _agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
            Vec::new()
        }

        fn intention_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::IntentionDispositionProfile> {
            None
        }

        fn tell_profile(&self, _agent: EntityId) -> Option<TellProfile> {
            None
        }

        fn belief_confidence_policy(
            &self,
            _agent: EntityId,
        ) -> worldwake_core::BeliefConfidencePolicy {
            worldwake_core::BeliefConfidencePolicy::default()
        }
    }

    impl crate::PoliticalBeliefView for StubBeliefView {}

    impl CombatBeliefView for StubBeliefView {
        fn combat_profile(&self, _agent: EntityId) -> Option<CombatProfile> {
            None
        }

        fn wounds(&self, _agent: EntityId) -> Vec<Wound> {
            self.wounds.clone()
        }

        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn has_wounds(&self, _entity: EntityId) -> bool {
            !self.wounds.is_empty()
        }
    }

    impl EconomicBeliefView for StubBeliefView {
        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
        }

        fn commodity_valuation_profile(
            &self,
            _agent: EntityId,
        ) -> Option<CommodityValuationProfile> {
            self.commodity_valuation_profile
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

        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            self.demand_memory.clone()
        }

        fn merchandise_profile(&self, _agent: EntityId) -> Option<MerchandiseProfile> {
            None
        }
    }

    impl crate::InventoryBeliefView for StubBeliefView {
        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
            false
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
        ) -> Option<CommodityConsumableProfile> {
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
            self.known_recipes.clone()
        }

        fn recipe_definition(&self, recipe: RecipeId) -> Option<RecipeDefinition> {
            self.recipe_definitions.get(&recipe).cloned()
        }
    }

    impl crate::FacilityBeliefView for StubBeliefView {
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }

        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn matching_workstations_at(&self, place: EntityId, tag: WorkstationTag) -> Vec<EntityId> {
            self.matching_workstations
                .get(&(place, tag))
                .cloned()
                .unwrap_or_default()
        }

        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
    }

    fn assert_traits<T: Clone + Eq + std::fmt::Debug>() {}

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn q(value: u32) -> Quantity {
        Quantity(value)
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn hunger(value: u16) -> HomeostaticNeeds {
        HomeostaticNeeds::new(pm(value), pm(0), pm(0), pm(0), pm(0))
    }

    fn thirst(value: u16) -> HomeostaticNeeds {
        HomeostaticNeeds::new(pm(0), pm(value), pm(0), pm(0), pm(0))
    }

    fn demand_memory(kind: CommodityKind, quantity: u32) -> DemandMemory {
        DemandMemory {
            observations: vec![DemandObservation {
                commodity: kind,
                quantity: q(quantity),
                place: entity(99),
                tick: Tick(7),
                counterparty: Some(entity(55)),
                reason: DemandObservationReason::WantedToBuyButNoSeller,
            }],
        }
    }

    fn valuation_profile(depth: u8, horizon: u8, decay: u16) -> CommodityValuationProfile {
        CommodityValuationProfile {
            recipe_opportunity_depth: NonZeroU8::new(depth).unwrap(),
            recipe_place_horizon: horizon,
            indirect_value_decay_per_step: pm(decay),
        }
    }

    fn recipe(
        name: &str,
        inputs: Vec<(CommodityKind, u32)>,
        outputs: Vec<(CommodityKind, u32)>,
        workstation: Option<WorkstationTag>,
    ) -> RecipeDefinition {
        RecipeDefinition {
            name: name.to_string(),
            inputs: inputs
                .into_iter()
                .map(|(kind, quantity)| (kind, q(quantity)))
                .collect(),
            outputs: outputs
                .into_iter()
                .map(|(kind, quantity)| (kind, q(quantity)))
                .collect(),
            work_ticks: std::num::NonZeroU32::new(3).unwrap(),
            required_workstation_tag: workstation,
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
        }
    }

    fn wound_list() -> WoundList {
        WoundList {
            wounds: vec![Wound {
                id: worldwake_core::WoundId(1),
                body_part: BodyPart::Torso,
                cause: WoundCause::Deprivation(worldwake_core::DeprivationKind::Starvation),
                severity: pm(700),
                inflicted_at: Tick(3),
                bleed_rate_per_tick: pm(0),
            }],
        }
    }

    #[test]
    fn valuation_types_satisfy_required_traits() {
        assert_traits::<TradeAcceptance>();
        assert_traits::<TradeRejectionReason>();
    }

    #[test]
    fn accepts_trade_when_post_trade_state_is_better() {
        let actor = entity(1);

        let view = StubBeliefView {
            needs: Some(hunger(900)),
            ..Default::default()
        };

        let acceptance = evaluate_trade_bundle(
            actor,
            &view,
            Some(&hunger(900)),
            None,
            q(3),
            &[(CommodityKind::Coin, q(1))],
            &[(CommodityKind::Bread, q(1))],
            &[],
            None,
        );

        assert_eq!(acceptance, TradeAcceptance::Accept);
    }

    #[test]
    fn rejects_trade_when_post_trade_state_is_worse() {
        let actor = entity(2);
        let mut view = StubBeliefView {
            needs: Some(hunger(900)),
            ..Default::default()
        };
        view.commodities.insert((actor, CommodityKind::Bread), q(1));

        let acceptance = evaluate_trade_bundle(
            actor,
            &view,
            Some(&hunger(900)),
            None,
            q(0),
            &[(CommodityKind::Bread, q(1))],
            &[(CommodityKind::Coin, q(1))],
            &[],
            None,
        );

        assert_eq!(
            acceptance,
            TradeAcceptance::Reject {
                reason: TradeRejectionReason::InsufficientPayment,
            }
        );
    }

    #[test]
    fn evaluates_without_homeostatic_needs_component() {
        let actor = entity(3);
        let view = StubBeliefView::default();

        let acceptance = evaluate_trade_bundle(
            actor,
            &view,
            None,
            None,
            q(1),
            &[(CommodityKind::Coin, q(1))],
            &[(CommodityKind::Firewood, q(1))],
            &[],
            None,
        );

        assert_eq!(
            acceptance,
            TradeAcceptance::Reject {
                reason: TradeRejectionReason::NoNeed,
            }
        );
    }

    #[test]
    fn high_need_accepts_survival_trade_that_no_need_actor_rejects() {
        let actor = entity(4);
        let no_need_view = StubBeliefView {
            needs: Some(HomeostaticNeeds::new_sated()),
            ..Default::default()
        };
        let high_need_view = StubBeliefView {
            needs: Some(hunger(900)),
            ..Default::default()
        };
        let offered = &[(CommodityKind::Coin, q(1))];
        let received = &[(CommodityKind::Bread, q(1))];

        let no_need = evaluate_trade_bundle(
            actor,
            &no_need_view,
            Some(&HomeostaticNeeds::new_sated()),
            None,
            q(1),
            offered,
            received,
            &[],
            None,
        );
        let high_need = evaluate_trade_bundle(
            actor,
            &high_need_view,
            Some(&hunger(900)),
            None,
            q(1),
            offered,
            received,
            &[],
            None,
        );

        assert_eq!(
            no_need,
            TradeAcceptance::Reject {
                reason: TradeRejectionReason::NoNeed,
            }
        );
        assert_eq!(high_need, TradeAcceptance::Accept);
    }

    #[test]
    fn local_alternative_supply_reduces_marginal_value_of_offer() {
        let actor = entity(5);
        let view = StubBeliefView::default();

        let acceptance = evaluate_trade_bundle(
            actor,
            &view,
            Some(&thirst(900)),
            None,
            q(1),
            &[(CommodityKind::Coin, q(1))],
            &[(CommodityKind::Water, q(1))],
            &[(entity(6), CommodityKind::Water, q(4))],
            None,
        );

        assert_eq!(
            acceptance,
            TradeAcceptance::Reject {
                reason: TradeRejectionReason::NoNeed,
            }
        );
    }

    #[test]
    fn demand_memory_can_make_non_consumable_stock_worth_acquiring() {
        let actor = entity(6);
        let view = StubBeliefView {
            demand_memory: demand_memory(CommodityKind::Firewood, 2).observations,
            ..Default::default()
        };

        let acceptance = evaluate_trade_bundle(
            actor,
            &view,
            None,
            None,
            q(1),
            &[(CommodityKind::Coin, q(1))],
            &[(CommodityKind::Firewood, q(1))],
            &[],
            Some(&demand_memory(CommodityKind::Firewood, 2)),
        );

        assert_eq!(acceptance, TradeAcceptance::Accept);
    }

    #[test]
    fn wounds_make_medicine_worth_acquiring() {
        let actor = entity(7);
        let view = StubBeliefView {
            wounds: wound_list().wounds,
            ..Default::default()
        };

        let acceptance = evaluate_trade_bundle(
            actor,
            &view,
            None,
            Some(&wound_list()),
            q(1),
            &[(CommodityKind::Coin, q(1))],
            &[(CommodityKind::Medicine, q(1))],
            &[],
            None,
        );

        assert_eq!(acceptance, TradeAcceptance::Accept);
    }

    #[test]
    fn rejects_impossible_bundle_that_spends_more_coin_than_actor_has() {
        let actor = entity(8);
        let view = StubBeliefView::default();

        let acceptance = evaluate_trade_bundle(
            actor,
            &view,
            Some(&HomeostaticNeeds::new_sated()),
            None,
            q(0),
            &[(CommodityKind::Coin, q(1))],
            &[(CommodityKind::Bread, q(1))],
            &[],
            None,
        );

        assert_eq!(
            acceptance,
            TradeAcceptance::Reject {
                reason: TradeRejectionReason::InsufficientPayment,
            }
        );
    }

    #[test]
    fn accepts_recipe_input_when_reachable_output_opportunity_is_positive() {
        let actor = entity(9);
        let place = entity(90);
        let recipe_id = RecipeId(0);
        let view = StubBeliefView {
            needs: Some(hunger(900)),
            effective_place: Some(place),
            commodity_valuation_profile: Some(valuation_profile(3, 0, 100)),
            known_recipes: vec![recipe_id],
            recipe_definitions: BTreeMap::from([(
                recipe_id,
                recipe(
                    "Bake Bread",
                    vec![(CommodityKind::Firewood, 1)],
                    vec![(CommodityKind::Bread, 1)],
                    Some(WorkstationTag::Mill),
                ),
            )]),
            matching_workstations: BTreeMap::from([(
                (place, WorkstationTag::Mill),
                vec![entity(91)],
            )]),
            ..Default::default()
        };
        let mut holdings = BTreeMap::new();
        holdings.insert(CommodityKind::Coin, 3);
        holdings.insert(CommodityKind::Firewood, 1);
        let breakdown = commodity_opportunity_score(
            actor,
            CommodityKind::Firewood,
            &view,
            &holdings,
            &BTreeMap::new(),
        );
        assert!(
            breakdown.indirect_recipe_score > 0,
            "reachable bread opportunity should give firewood positive indirect value; breakdown={breakdown:?}"
        );
        let current_holdings = build_current_holdings(actor, &view, q(3));
        let receipt_only_holdings =
            apply_bundle_changes(&current_holdings, &[(CommodityKind::Firewood, q(1))], &[])
                .expect("receiving firewood should be possible");
        let post_trade_holdings = apply_bundle_changes(
            &current_holdings,
            &[(CommodityKind::Firewood, q(1))],
            &[(CommodityKind::Coin, q(1))],
        )
        .expect("coin-for-firewood exchange should be possible");
        let current_snapshot = snapshot(actor, &view, &current_holdings, &BTreeMap::new());
        let receipt_only_snapshot =
            snapshot(actor, &view, &receipt_only_holdings, &BTreeMap::new());
        let post_trade_snapshot = snapshot(actor, &view, &post_trade_holdings, &BTreeMap::new());
        assert!(
            receipt_only_snapshot > current_snapshot,
            "receiving firewood should improve the receipt-only snapshot; current={current_snapshot:?} receipt={receipt_only_snapshot:?}"
        );
        assert!(
            post_trade_snapshot > current_snapshot,
            "coin-for-firewood exchange should improve the post-trade snapshot; current={current_snapshot:?} post={post_trade_snapshot:?}"
        );

        let acceptance = evaluate_trade_bundle(
            actor,
            &view,
            Some(&hunger(900)),
            None,
            q(3),
            &[(CommodityKind::Coin, q(1))],
            &[(CommodityKind::Firewood, q(1))],
            &[],
            None,
        );

        assert_eq!(acceptance, TradeAcceptance::Accept);
    }

    #[test]
    fn seller_rejects_selling_last_enabling_recipe_input_for_insufficient_coin() {
        let actor = entity(10);
        let place = entity(100);
        let recipe_id = RecipeId(0);
        let mut commodities = BTreeMap::new();
        commodities.insert((actor, CommodityKind::Firewood), q(1));
        let view = StubBeliefView {
            commodities,
            needs: Some(hunger(900)),
            effective_place: Some(place),
            commodity_valuation_profile: Some(valuation_profile(3, 0, 100)),
            known_recipes: vec![recipe_id],
            recipe_definitions: BTreeMap::from([(
                recipe_id,
                recipe(
                    "Bake Bread",
                    vec![(CommodityKind::Firewood, 1)],
                    vec![(CommodityKind::Bread, 1)],
                    Some(WorkstationTag::Mill),
                ),
            )]),
            matching_workstations: BTreeMap::from([(
                (place, WorkstationTag::Mill),
                vec![entity(101)],
            )]),
            ..Default::default()
        };

        let acceptance = evaluate_trade_bundle(
            actor,
            &view,
            Some(&hunger(900)),
            None,
            q(0),
            &[(CommodityKind::Firewood, q(1))],
            &[(CommodityKind::Coin, q(1))],
            &[],
            None,
        );

        assert_eq!(
            acceptance,
            TradeAcceptance::Reject {
                reason: TradeRejectionReason::InsufficientPayment,
            }
        );
    }

    #[test]
    fn seller_accepts_coin_for_firewood_when_recipe_opportunity_is_unreachable() {
        let actor = entity(11);
        let recipe_id = RecipeId(0);
        let mut commodities = BTreeMap::new();
        commodities.insert((actor, CommodityKind::Firewood), q(1));
        let view = StubBeliefView {
            commodities,
            needs: Some(hunger(900)),
            effective_place: Some(entity(110)),
            commodity_valuation_profile: Some(valuation_profile(3, 0, 100)),
            known_recipes: vec![recipe_id],
            recipe_definitions: BTreeMap::from([(
                recipe_id,
                recipe(
                    "Bake Bread",
                    vec![(CommodityKind::Firewood, 1)],
                    vec![(CommodityKind::Bread, 1)],
                    Some(WorkstationTag::Mill),
                ),
            )]),
            ..Default::default()
        };

        let acceptance = evaluate_trade_bundle(
            actor,
            &view,
            Some(&hunger(900)),
            None,
            q(0),
            &[(CommodityKind::Firewood, q(1))],
            &[(CommodityKind::Coin, q(1))],
            &[],
            None,
        );

        assert_eq!(acceptance, TradeAcceptance::Accept);
    }
}
