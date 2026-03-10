use worldwake_core::{
    CommodityConsumableProfile, CommodityKind, EntityId, EntityKind, Quantity, RecipeId,
    ResourceSource, TickRange, UniqueItemKind, WorkstationTag,
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
}
