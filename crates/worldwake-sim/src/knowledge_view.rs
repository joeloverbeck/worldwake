use worldwake_core::{CommodityKind, EntityId, EntityKind, Quantity, TickRange};

pub trait KnowledgeView {
    fn is_alive(&self, entity: EntityId) -> bool;
    fn entity_kind(&self, entity: EntityId) -> Option<EntityKind>;
    fn effective_place(&self, entity: EntityId) -> Option<EntityId>;
    fn entities_at(&self, place: EntityId) -> Vec<EntityId>;
    fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity;
    fn has_control(&self, entity: EntityId) -> bool;
    fn reservation_conflicts(&self, entity: EntityId, range: TickRange) -> bool;
}
