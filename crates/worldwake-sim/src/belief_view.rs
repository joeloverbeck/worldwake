use crate::{ActionDuration, ActionPayload, DurationExpr};
use std::num::NonZeroU32;
use worldwake_core::{
    CommodityConsumableProfile, CommodityKind, DemandObservation, DriveThresholds, EntityId,
    EntityKind, HomeostaticNeeds, InTransitOnEdge, MerchandiseProfile, Quantity, RecipeId,
    ResourceSource, TickRange, UniqueItemKind, WorkstationTag, Wound,
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
    fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind>;
    fn item_lot_consumable_profile(&self, entity: EntityId) -> Option<CommodityConsumableProfile>;
    fn direct_container(&self, entity: EntityId) -> Option<EntityId>;
    fn direct_possessor(&self, entity: EntityId) -> Option<EntityId>;
    fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag>;
    fn resource_source(&self, entity: EntityId) -> Option<ResourceSource>;
    fn has_production_job(&self, entity: EntityId) -> bool;
    fn can_control(&self, actor: EntityId, entity: EntityId) -> bool;
    fn has_control(&self, entity: EntityId) -> bool;
    fn reservation_conflicts(&self, entity: EntityId, range: TickRange) -> bool;
    fn is_dead(&self, entity: EntityId) -> bool;
    fn is_incapacitated(&self, entity: EntityId) -> bool;
    fn has_wounds(&self, entity: EntityId) -> bool;
    fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds>;
    fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds>;
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
