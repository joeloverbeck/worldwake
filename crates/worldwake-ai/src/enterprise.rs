use worldwake_core::{CommodityKind, EntityId, Permille, Quantity};
use worldwake_sim::BeliefView;

pub(crate) fn restock_gap(
    view: &dyn BeliefView,
    agent: EntityId,
    fallback_place: Option<EntityId>,
    commodity: CommodityKind,
) -> Option<Quantity> {
    let profile = view.merchandise_profile(agent)?;
    if !profile.sale_kinds.contains(&commodity) {
        return None;
    }

    let market = profile.home_market.or(fallback_place)?;
    let observed_quantity = relevant_demand_quantity(view, agent, market, commodity);
    if observed_quantity == 0 {
        return None;
    }

    let current_stock = view.commodity_quantity(agent, commodity).0;
    (current_stock < observed_quantity).then_some(Quantity(observed_quantity - current_stock))
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
        .fold(0u32, |sum, observation| sum.saturating_add(observation.quantity.0))
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
