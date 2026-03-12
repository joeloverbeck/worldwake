use std::collections::BTreeMap;
use worldwake_core::{CommodityKind, EntityId, Permille, Quantity};
use worldwake_sim::BeliefView;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct EnterpriseSignals {
    restock_gaps: BTreeMap<CommodityKind, Quantity>,
}

impl EnterpriseSignals {
    pub(crate) fn restock_gap(&self, commodity: CommodityKind) -> Option<Quantity> {
        self.restock_gaps.get(&commodity).copied()
    }
}

pub(crate) fn analyze_candidate_enterprise(
    view: &dyn BeliefView,
    agent: EntityId,
    fallback_place: Option<EntityId>,
) -> EnterpriseSignals {
    let Some(profile) = view.merchandise_profile(agent) else {
        return EnterpriseSignals::default();
    };
    let Some(market) = profile.home_market.or(fallback_place) else {
        return EnterpriseSignals::default();
    };

    let mut restock_gaps = BTreeMap::new();
    for commodity in profile.sale_kinds {
        if let Some(gap) = restock_gap_for_market(view, agent, market, commodity) {
            restock_gaps.insert(commodity, gap);
        }
    }

    EnterpriseSignals { restock_gaps }
}

pub(crate) fn opportunity_signal(
    view: &dyn BeliefView,
    agent: EntityId,
    fallback_place: Option<EntityId>,
    commodity: CommodityKind,
) -> Permille {
    let market = view
        .merchandise_profile(agent)
        .and_then(|profile| profile.home_market.or(fallback_place));
    let Some(market) = market else {
        return Permille::new_unchecked(0);
    };

    market_signal_for_place(view, agent, commodity, market)
}

pub(crate) fn market_signal_for_place(
    view: &dyn BeliefView,
    agent: EntityId,
    commodity: CommodityKind,
    place: EntityId,
) -> Permille {
    let demand = relevant_demand_quantity(view, agent, place, commodity);
    let stock = view.commodity_quantity(agent, commodity).0;
    if demand == 0 {
        return Permille::new_unchecked(0);
    }

    let deficit = demand.saturating_sub(stock);
    let delivered = stock.min(demand);
    let dominant = deficit.max(delivered);
    permille_ratio(dominant, demand)
}

fn relevant_demand_quantity(
    view: &dyn BeliefView,
    agent: EntityId,
    market: EntityId,
    commodity: CommodityKind,
) -> u32 {
    view.demand_memory(agent)
        .into_iter()
        .filter(|observation| observation.place == market && observation.commodity == commodity)
        .fold(0u32, |sum, observation| {
            sum.saturating_add(observation.quantity.0)
        })
}

fn restock_gap_for_market(
    view: &dyn BeliefView,
    agent: EntityId,
    market: EntityId,
    commodity: CommodityKind,
) -> Option<Quantity> {
    let observed_quantity = relevant_demand_quantity(view, agent, market, commodity);
    if observed_quantity == 0 {
        return None;
    }

    let current_stock = view.commodity_quantity(agent, commodity).0;
    (current_stock < observed_quantity).then_some(Quantity(observed_quantity - current_stock))
}

pub(crate) fn restock_gap_at_destination(
    view: &dyn BeliefView,
    agent: EntityId,
    destination: EntityId,
    commodity: CommodityKind,
) -> Option<Quantity> {
    let observed_quantity = relevant_demand_quantity(view, agent, destination, commodity);
    if observed_quantity == 0 {
        return None;
    }

    let current_stock_at_destination = view
        .controlled_commodity_quantity_at_place(agent, destination, commodity)
        .0;
    (current_stock_at_destination < observed_quantity)
        .then_some(Quantity(observed_quantity - current_stock_at_destination))
}

fn permille_ratio(numerator: u32, denominator: u32) -> Permille {
    if numerator == 0 || denominator == 0 {
        return Permille::new_unchecked(0);
    }

    let scaled = numerator
        .saturating_mul(1000)
        .checked_div(denominator)
        .unwrap_or(u32::MAX)
        .min(1000);
    Permille::new(scaled as u16).unwrap()
}

#[cfg(test)]
mod tests {
    use super::{relevant_demand_quantity, restock_gap_at_destination, restock_gap_for_market};
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        CombatProfile, CommodityConsumableProfile, CommodityKind, DemandObservation,
        DemandObservationReason, DriveThresholds, EntityId, EntityKind, HomeostaticNeeds,
        InTransitOnEdge, LoadUnits, MerchandiseProfile, MetabolismProfile, Quantity, RecipeId,
        ResourceSource, Tick, TickRange, TradeDispositionProfile, UniqueItemKind, WorkstationTag,
        Wound,
    };
    use worldwake_sim::{
        estimate_duration_from_beliefs, ActionDuration, ActionPayload, BeliefView, DurationExpr,
    };

    #[derive(Default)]
    struct TestBeliefView {
        demand_memory: BTreeMap<EntityId, Vec<DemandObservation>>,
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        controlled_quantities: BTreeMap<(EntityId, EntityId, CommodityKind), Quantity>,
    }

    impl BeliefView for TestBeliefView {
        fn is_alive(&self, _entity: EntityId) -> bool {
            true
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
            agent: EntityId,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Quantity {
            self.controlled_quantities
                .get(&(agent, place, commodity))
                .copied()
                .unwrap_or(Quantity(0))
        }

        fn local_controlled_lots_for(
            &self,
            _agent: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
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

        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
        }

        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            None
        }

        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
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

        fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation> {
            self.demand_memory.get(&agent).cloned().unwrap_or_default()
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

    #[test]
    fn restock_gap_at_destination_returns_gap_when_understocked_at_destination() {
        let agent = entity(1);
        let market = entity(2);
        let mut view = TestBeliefView::default();
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 7)]);
        view.controlled_quantities
            .insert((agent, market, CommodityKind::Bread), Quantity(2));

        assert_eq!(
            restock_gap_at_destination(&view, agent, market, CommodityKind::Bread),
            Some(Quantity(5))
        );
    }

    #[test]
    fn restock_gap_at_destination_returns_none_when_no_demand() {
        let agent = entity(1);
        let market = entity(2);
        let mut view = TestBeliefView::default();
        view.controlled_quantities
            .insert((agent, market, CommodityKind::Bread), Quantity(1));

        assert_eq!(
            relevant_demand_quantity(&view, agent, market, CommodityKind::Bread),
            0
        );
        assert_eq!(
            restock_gap_at_destination(&view, agent, market, CommodityKind::Bread),
            None
        );
    }

    #[test]
    fn restock_gap_at_destination_returns_none_when_fully_stocked() {
        let agent = entity(1);
        let market = entity(2);
        let mut view = TestBeliefView::default();
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 4)]);
        view.controlled_quantities
            .insert((agent, market, CommodityKind::Bread), Quantity(4));

        assert_eq!(
            restock_gap_at_destination(&view, agent, market, CommodityKind::Bread),
            None
        );
    }

    #[test]
    fn restock_gap_at_destination_ignores_stock_at_other_places() {
        let agent = entity(1);
        let market = entity(2);
        let remote = entity(3);
        let mut view = TestBeliefView::default();
        view.demand_memory
            .insert(agent, vec![demand(market, CommodityKind::Bread, 5)]);
        view.commodity_quantities
            .insert((agent, CommodityKind::Bread), Quantity(5));
        view.controlled_quantities
            .insert((agent, remote, CommodityKind::Bread), Quantity(5));

        assert_eq!(
            restock_gap_for_market(&view, agent, market, CommodityKind::Bread),
            None
        );
        assert_eq!(
            restock_gap_at_destination(&view, agent, market, CommodityKind::Bread),
            Some(Quantity(5))
        );
    }
}
