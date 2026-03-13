use crate::{ActionDuration, ActionPayload, DurationExpr};
use std::num::NonZeroU32;
use worldwake_core::{
    CombatProfile, CommodityConsumableProfile, CommodityKind, CommodityTreatmentProfile,
    DemandObservation, DriveThresholds, EntityId, EntityKind, HomeostaticNeeds, InTransitOnEdge,
    LoadUnits, MerchandiseProfile, MetabolismProfile, PlaceTag, Quantity, RecipeId, ResourceSource,
    TickRange, TradeDispositionProfile, TravelDispositionProfile, UniqueItemKind, WorkstationTag,
    Wound,
};

pub trait BeliefView {
    fn is_alive(&self, entity: EntityId) -> bool;
    fn entity_kind(&self, entity: EntityId) -> Option<EntityKind>;
    fn effective_place(&self, entity: EntityId) -> Option<EntityId>;
    fn is_in_transit(&self, entity: EntityId) -> bool;
    fn entities_at(&self, place: EntityId) -> Vec<EntityId>;
    fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId>;
    fn adjacent_places(&self, place: EntityId) -> Vec<EntityId>;
    fn knows_recipe(&self, actor: EntityId, recipe: RecipeId) -> bool;
    fn unique_item_count(&self, holder: EntityId, kind: UniqueItemKind) -> u32;
    fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity;
    fn controlled_commodity_quantity_at_place(
        &self,
        agent: EntityId,
        place: EntityId,
        commodity: CommodityKind,
    ) -> Quantity;
    fn local_controlled_lots_for(
        &self,
        agent: EntityId,
        place: EntityId,
        commodity: CommodityKind,
    ) -> Vec<EntityId>;
    fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind>;
    fn item_lot_consumable_profile(&self, entity: EntityId) -> Option<CommodityConsumableProfile>;
    fn direct_container(&self, entity: EntityId) -> Option<EntityId>;
    fn direct_possessor(&self, entity: EntityId) -> Option<EntityId>;
    fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag>;
    fn place_has_tag(&self, place: EntityId, tag: PlaceTag) -> bool {
        let _ = (place, tag);
        false
    }
    fn resource_source(&self, entity: EntityId) -> Option<ResourceSource>;
    fn has_production_job(&self, entity: EntityId) -> bool;
    fn can_control(&self, actor: EntityId, entity: EntityId) -> bool;
    fn has_control(&self, entity: EntityId) -> bool;
    fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits>;
    fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits>;
    fn reservation_conflicts(&self, entity: EntityId, range: TickRange) -> bool;
    fn reservation_ranges(&self, entity: EntityId) -> Vec<TickRange>;
    fn is_dead(&self, entity: EntityId) -> bool;
    fn is_incapacitated(&self, entity: EntityId) -> bool;
    fn has_wounds(&self, entity: EntityId) -> bool;
    fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds>;
    fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds>;
    fn metabolism_profile(&self, agent: EntityId) -> Option<MetabolismProfile>;
    fn trade_disposition_profile(&self, agent: EntityId) -> Option<TradeDispositionProfile>;
    fn travel_disposition_profile(&self, agent: EntityId) -> Option<TravelDispositionProfile>;
    fn combat_profile(&self, agent: EntityId) -> Option<CombatProfile>;
    fn wounds(&self, agent: EntityId) -> Vec<Wound>;
    fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId>;
    fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId>;
    fn agents_selling_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>;
    fn known_recipes(&self, agent: EntityId) -> Vec<RecipeId>;
    fn matching_workstations_at(&self, place: EntityId, tag: WorkstationTag) -> Vec<EntityId>;
    fn resource_sources_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>;
    fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation>;
    fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile>;
    fn corpse_entities_at(&self, place: EntityId) -> Vec<EntityId>;
    fn in_transit_state(&self, entity: EntityId) -> Option<InTransitOnEdge>;
    fn adjacent_places_with_travel_ticks(&self, place: EntityId) -> Vec<(EntityId, NonZeroU32)>;
    fn estimate_duration(
        &self,
        actor: EntityId,
        duration: &DurationExpr,
        targets: &[EntityId],
        payload: &ActionPayload,
    ) -> Option<ActionDuration>;
}

#[must_use]
pub fn estimate_duration_from_beliefs(
    view: &dyn BeliefView,
    actor: EntityId,
    duration: &DurationExpr,
    targets: &[EntityId],
    payload: &ActionPayload,
) -> Option<ActionDuration> {
    match *duration {
        DurationExpr::Fixed(ticks) => Some(ActionDuration::Finite(ticks.get())),
        DurationExpr::TargetConsumable { target_index } => {
            let target = targets.get(usize::from(target_index)).copied()?;
            let profile = view.item_lot_consumable_profile(target)?;
            Some(ActionDuration::Finite(
                profile.consumption_ticks_per_unit.get(),
            ))
        }
        DurationExpr::TravelToTarget { target_index } => {
            let target = targets.get(usize::from(target_index)).copied()?;
            let origin = view.effective_place(actor)?;
            view.adjacent_places_with_travel_ticks(origin)
                .into_iter()
                .find_map(|(adjacent, ticks)| {
                    (adjacent == target).then_some(ActionDuration::Finite(ticks.get()))
                })
        }
        DurationExpr::ActorMetabolism { kind } => {
            let profile = view.metabolism_profile(actor)?;
            let ticks = match kind {
                crate::MetabolismDurationKind::Toilet => profile.toilet_ticks.get(),
                crate::MetabolismDurationKind::Wash => profile.wash_ticks.get(),
            };
            Some(ActionDuration::Finite(ticks))
        }
        DurationExpr::ActorTradeDisposition => view
            .trade_disposition_profile(actor)
            .map(|profile| ActionDuration::Finite(profile.negotiation_round_ticks.get())),
        DurationExpr::Indefinite => Some(ActionDuration::Indefinite),
        DurationExpr::CombatWeapon => {
            let combat = payload.as_combat()?;
            match combat.weapon {
                worldwake_core::CombatWeaponRef::Unarmed => view
                    .combat_profile(actor)
                    .map(|profile| ActionDuration::Finite(profile.unarmed_attack_ticks.get())),
                worldwake_core::CombatWeaponRef::Commodity(kind) => kind
                    .spec()
                    .combat_weapon_profile
                    .map(|profile| ActionDuration::Finite(profile.attack_duration_ticks.get())),
            }
        }
        DurationExpr::TargetTreatment {
            target_index,
            commodity,
        } => {
            if view.commodity_quantity(actor, commodity) == Quantity(0) {
                return None;
            }
            let target = targets.get(usize::from(target_index)).copied()?;
            let wounds = view.wounds(target);
            if wounds.is_empty() {
                return None;
            }
            let CommodityTreatmentProfile {
                treatment_ticks_per_unit,
                severity_reduction_per_tick,
                ..
            } = commodity.spec().treatment_profile?;
            let wound_load = wounds.iter().fold(0u32, |acc, wound| {
                acc.saturating_add(u32::from(wound.severity.value()))
            });
            let severity_per_tick = u32::from(severity_reduction_per_tick.value()).max(1);
            let wound_ticks = wound_load.div_ceil(severity_per_tick).max(1);
            Some(ActionDuration::Finite(
                treatment_ticks_per_unit.get().max(wound_ticks),
            ))
        }
    }
}
