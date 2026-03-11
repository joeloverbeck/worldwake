use crate::BeliefView;
use std::collections::BTreeMap;
use worldwake_core::{
    CommodityKind, DemandMemory, EntityId, HomeostaticNeeds, Quantity, WoundList,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TradeAcceptance {
    Accept,
    Reject { reason: TradeRejectionReason },
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
    belief: &dyn BeliefView,
    needs: Option<&HomeostaticNeeds>,
    wounds: Option<&WoundList>,
    current_coin: Quantity,
    offered: &[(CommodityKind, Quantity)],
    received: &[(CommodityKind, Quantity)],
    local_alternatives: &[(EntityId, CommodityKind, Quantity)],
    demand_memory: Option<&DemandMemory>,
) -> TradeAcceptance {
    let current_holdings = build_current_holdings(actor, belief, current_coin);
    let alternative_supply = aggregate_local_alternatives(actor, local_alternatives);
    let current_snapshot = snapshot(
        &current_holdings,
        &alternative_supply,
        needs,
        wounds,
        demand_memory,
    );

    let Some(receipt_only_holdings) = apply_bundle_changes(&current_holdings, received, &[]) else {
        return TradeAcceptance::Reject {
            reason: TradeRejectionReason::PostTradeStateWorse,
        };
    };
    let receipt_only_snapshot = snapshot(
        &receipt_only_holdings,
        &alternative_supply,
        needs,
        wounds,
        demand_memory,
    );

    let Some(post_trade_holdings) = apply_bundle_changes(&current_holdings, received, offered)
    else {
        return TradeAcceptance::Reject {
            reason: insufficient_payment_reason(offered),
        };
    };
    let post_trade_snapshot = snapshot(
        &post_trade_holdings,
        &alternative_supply,
        needs,
        wounds,
        demand_memory,
    );

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
    belief: &dyn BeliefView,
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
    holdings: &BTreeMap<CommodityKind, u32>,
    alternative_supply: &BTreeMap<CommodityKind, u32>,
    needs: Option<&HomeostaticNeeds>,
    wounds: Option<&WoundList>,
    demand_memory: Option<&DemandMemory>,
) -> ValuationSnapshot {
    ValuationSnapshot {
        survival: survival_score(holdings, alternative_supply, needs),
        wound: wound_score(holdings, alternative_supply, wounds),
        demand: demand_score(holdings, alternative_supply, demand_memory),
        coin: holdings
            .get(&CommodityKind::Coin)
            .copied()
            .unwrap_or(0)
            .into(),
    }
}

fn survival_score(
    holdings: &BTreeMap<CommodityKind, u32>,
    alternative_supply: &BTreeMap<CommodityKind, u32>,
    needs: Option<&HomeostaticNeeds>,
) -> u64 {
    let Some(needs) = needs else {
        return 0;
    };

    let mut hunger_relief = 0_u64;
    let mut thirst_relief = 0_u64;

    for kind in CommodityKind::ALL {
        let Some(profile) = kind.spec().consumable_profile else {
            continue;
        };
        let quantity = accessible_quantity(holdings, alternative_supply, kind);
        hunger_relief += quantity * u64::from(profile.hunger_relief_per_unit.value());
        thirst_relief += quantity * u64::from(profile.thirst_relief_per_unit.value());
    }

    hunger_relief.min(u64::from(needs.hunger.value()))
        + thirst_relief.min(u64::from(needs.thirst.value()))
}

fn wound_score(
    holdings: &BTreeMap<CommodityKind, u32>,
    alternative_supply: &BTreeMap<CommodityKind, u32>,
    wounds: Option<&WoundList>,
) -> u64 {
    let Some(wounds) = wounds else {
        return 0;
    };
    if wounds.wounds.is_empty() {
        return 0;
    }

    let total_severity = wounds
        .wounds
        .iter()
        .map(|wound| u64::from(wound.severity.value()))
        .sum::<u64>();
    let wound_count = wounds.wounds.len() as u64;
    let accessible_medicine =
        accessible_quantity(holdings, alternative_supply, CommodityKind::Medicine);

    accessible_medicine.min(wound_count) * total_severity
}

fn demand_score(
    holdings: &BTreeMap<CommodityKind, u32>,
    alternative_supply: &BTreeMap<CommodityKind, u32>,
    demand_memory: Option<&DemandMemory>,
) -> u64 {
    let Some(demand_memory) = demand_memory else {
        return 0;
    };

    let mut remembered = BTreeMap::<CommodityKind, u64>::new();
    for observation in &demand_memory.observations {
        *remembered.entry(observation.commodity).or_insert(0) += u64::from(observation.quantity.0);
    }

    remembered
        .into_iter()
        .map(|(kind, remembered_quantity)| {
            accessible_quantity(holdings, alternative_supply, kind).min(remembered_quantity)
        })
        .sum()
}

fn accessible_quantity(
    holdings: &BTreeMap<CommodityKind, u32>,
    alternative_supply: &BTreeMap<CommodityKind, u32>,
    kind: CommodityKind,
) -> u64 {
    let held = u64::from(holdings.get(&kind).copied().unwrap_or(0));
    let alternatives = if kind == CommodityKind::Coin {
        0
    } else {
        u64::from(alternative_supply.get(&kind).copied().unwrap_or(0))
    };
    held + alternatives
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
    use super::{evaluate_trade_bundle, TradeAcceptance, TradeRejectionReason};
    use crate::BeliefView;
    use std::collections::BTreeMap;
    use worldwake_core::{
        BodyPart, CombatProfile, CommodityConsumableProfile, CommodityKind, DemandMemory,
        DemandObservation, DemandObservationReason, DriveThresholds, EntityId, EntityKind,
        HomeostaticNeeds, InTransitOnEdge, MerchandiseProfile, MetabolismProfile, Permille,
        Quantity, RecipeId, ResourceSource, Tick, TickRange, TradeDispositionProfile,
        UniqueItemKind, WorkstationTag, Wound, WoundCause, WoundList,
    };

    #[derive(Default)]
    struct StubBeliefView {
        commodities: BTreeMap<(EntityId, CommodityKind), Quantity>,
    }

    impl BeliefView for StubBeliefView {
        fn is_alive(&self, _entity: EntityId) -> bool {
            false
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
        ) -> Vec<(EntityId, std::num::NonZeroU32)> {
            Vec::new()
        }

        fn estimate_duration(
            &self,
            _actor: EntityId,
            duration: &crate::DurationExpr,
            _targets: &[EntityId],
            _payload: &crate::ActionPayload,
        ) -> Option<crate::ActionDuration> {
            duration
                .fixed_ticks()
                .map(crate::ActionDuration::Finite)
                .or(Some(crate::ActionDuration::Indefinite))
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

        let view = StubBeliefView::default();

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
        let mut view = StubBeliefView::default();
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
        let view = StubBeliefView::default();
        let offered = &[(CommodityKind::Coin, q(1))];
        let received = &[(CommodityKind::Bread, q(1))];

        let no_need = evaluate_trade_bundle(
            actor,
            &view,
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
            &view,
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
            Some(&demand_memory(CommodityKind::Firewood, 2)),
        );

        assert_eq!(acceptance, TradeAcceptance::Accept);
    }

    #[test]
    fn wounds_make_medicine_worth_acquiring() {
        let actor = entity(7);
        let view = StubBeliefView::default();

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
}
