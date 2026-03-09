use crate::{
    CommodityKind, Container, EntityId, ItemLot, LoadUnits, UniqueItem, UniqueItemKind, World,
    WorldError,
};
use std::collections::BTreeSet;

pub fn load_per_unit(commodity: CommodityKind) -> LoadUnits {
    match commodity {
        CommodityKind::Water => LoadUnits(2),
        CommodityKind::Firewood => LoadUnits(3),
        CommodityKind::Apple
        | CommodityKind::Grain
        | CommodityKind::Bread
        | CommodityKind::Medicine
        | CommodityKind::Coin
        | CommodityKind::Waste => LoadUnits(1),
    }
}

pub fn load_of_lot(lot: &ItemLot) -> LoadUnits {
    let per_unit = load_per_unit(lot.commodity).0;
    let total = lot
        .quantity
        .0
        .checked_mul(per_unit)
        .expect("load_of_lot overflowed u32 load accounting");
    LoadUnits(total)
}

pub fn load_of_unique_item_kind(kind: UniqueItemKind) -> LoadUnits {
    match kind {
        UniqueItemKind::SimpleTool | UniqueItemKind::Artifact => LoadUnits(5),
        UniqueItemKind::Weapon => LoadUnits(10),
        UniqueItemKind::Contract => LoadUnits(1),
        UniqueItemKind::OfficeInsignia => LoadUnits(2),
        UniqueItemKind::Misc => LoadUnits(3),
    }
}

pub fn load_of_unique_item(item: &UniqueItem) -> LoadUnits {
    load_of_unique_item_kind(item.kind)
}

pub fn load_of_entity(world: &World, entity_id: EntityId) -> Result<LoadUnits, WorldError> {
    require_live_entity(world, entity_id)?;

    if let Some(lot) = world.get_component_item_lot(entity_id) {
        return Ok(load_of_lot(lot));
    }
    if let Some(item) = world.get_component_unique_item(entity_id) {
        return Ok(load_of_unique_item(item));
    }

    Ok(LoadUnits(0))
}

pub fn current_container_load(
    world: &World,
    container_id: EntityId,
    contained: impl IntoIterator<Item = EntityId>,
) -> Result<LoadUnits, WorldError> {
    let _container = require_live_container(world, container_id)?;
    let mut seen = BTreeSet::new();
    let mut total = 0u32;

    for entity_id in contained {
        if !seen.insert(entity_id) {
            return Err(WorldError::InvariantViolation(format!(
                "duplicate contained entity {entity_id} supplied for container {container_id}"
            )));
        }

        total = total
            .checked_add(load_of_entity(world, entity_id)?.0)
            .ok_or_else(|| {
                WorldError::InvariantViolation(format!(
                    "container {container_id} load overflowed u32 accounting"
                ))
            })?;
    }

    Ok(LoadUnits(total))
}

pub fn remaining_container_capacity(
    world: &World,
    container_id: EntityId,
    contained: impl IntoIterator<Item = EntityId>,
) -> Result<LoadUnits, WorldError> {
    let container_component = require_live_container(world, container_id)?;
    let current = current_container_load(world, container_id, contained)?;

    container_component
        .capacity
        .0
        .checked_sub(current.0)
        .map(LoadUnits)
        .ok_or_else(|| {
            WorldError::InvariantViolation(format!(
                "container {container_id} is over capacity: load {} exceeds capacity {}",
                current.0, container_component.capacity.0
            ))
        })
}

fn require_live_entity(world: &World, entity_id: EntityId) -> Result<(), WorldError> {
    let meta = world
        .entity_meta(entity_id)
        .ok_or(WorldError::EntityNotFound(entity_id))?;
    if meta.archived_at.is_some() {
        return Err(WorldError::ArchivedEntity(entity_id));
    }

    Ok(())
}

fn require_live_container(world: &World, container_id: EntityId) -> Result<&Container, WorldError> {
    require_live_entity(world, container_id)?;
    world.get_component_container(container_id)
        .ok_or(WorldError::ComponentNotFound {
            entity: container_id,
            component_type: "Container",
        })
}

#[cfg(test)]
mod tests {
    use super::{
        current_container_load, load_of_entity, load_of_lot, load_of_unique_item,
        load_of_unique_item_kind, load_per_unit, remaining_container_capacity,
    };
    use crate::{
        CommodityKind, Container, EntityId, LoadUnits, Quantity, Tick, Topology, UniqueItem,
        UniqueItemKind, World, WorldError,
    };
    use std::collections::BTreeMap;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn test_world() -> World {
        World::new(Topology::new()).unwrap()
    }

    fn test_container(capacity: u32) -> Container {
        Container {
            capacity: LoadUnits(capacity),
            allowed_commodities: None,
            allows_unique_items: true,
            allows_nested_containers: false,
        }
    }

    #[test]
    fn commodity_load_table_matches_ticket_values() {
        assert_eq!(load_per_unit(CommodityKind::Apple), LoadUnits(1));
        assert_eq!(load_per_unit(CommodityKind::Grain), LoadUnits(1));
        assert_eq!(load_per_unit(CommodityKind::Bread), LoadUnits(1));
        assert_eq!(load_per_unit(CommodityKind::Water), LoadUnits(2));
        assert_eq!(load_per_unit(CommodityKind::Firewood), LoadUnits(3));
        assert_eq!(load_per_unit(CommodityKind::Medicine), LoadUnits(1));
        assert_eq!(load_per_unit(CommodityKind::Coin), LoadUnits(1));
        assert_eq!(load_per_unit(CommodityKind::Waste), LoadUnits(1));
    }

    #[test]
    fn unique_item_load_table_matches_ticket_values() {
        assert_eq!(load_of_unique_item_kind(UniqueItemKind::SimpleTool), LoadUnits(5));
        assert_eq!(load_of_unique_item_kind(UniqueItemKind::Weapon), LoadUnits(10));
        assert_eq!(load_of_unique_item_kind(UniqueItemKind::Contract), LoadUnits(1));
        assert_eq!(load_of_unique_item_kind(UniqueItemKind::Artifact), LoadUnits(5));
        assert_eq!(
            load_of_unique_item_kind(UniqueItemKind::OfficeInsignia),
            LoadUnits(2)
        );
        assert_eq!(load_of_unique_item_kind(UniqueItemKind::Misc), LoadUnits(3));
    }

    #[test]
    fn load_of_lot_multiplies_quantity_by_per_unit_weight() {
        let apples = crate::ItemLot {
            commodity: CommodityKind::Apple,
            quantity: Quantity(10),
            provenance: Vec::new(),
        };
        let water = crate::ItemLot {
            commodity: CommodityKind::Water,
            quantity: Quantity(5),
            provenance: Vec::new(),
        };

        assert_eq!(load_of_lot(&apples), LoadUnits(10));
        assert_eq!(load_of_lot(&water), LoadUnits(10));
    }

    #[test]
    fn load_of_unique_item_uses_kind_weight() {
        let weapon = UniqueItem {
            kind: UniqueItemKind::Weapon,
            name: Some("Rusty Sword".to_string()),
            metadata: BTreeMap::from([("condition".to_string(), "worn".to_string())]),
        };

        assert_eq!(load_of_unique_item(&weapon), LoadUnits(10));
    }

    #[test]
    fn load_of_entity_dispatches_items_and_returns_zero_for_live_non_items() {
        let mut world = test_world();
        let lot = world
            .create_item_lot(CommodityKind::Firewood, Quantity(4), Tick(1))
            .unwrap();
        let item = world
            .create_unique_item(
                UniqueItemKind::Artifact,
                Some("Court Seal"),
                BTreeMap::new(),
                Tick(2),
            )
            .unwrap();
        let office = world.create_office("Ledger Hall", Tick(3)).unwrap();

        assert_eq!(load_of_entity(&world, lot).unwrap(), LoadUnits(12));
        assert_eq!(load_of_entity(&world, item).unwrap(), LoadUnits(5));
        assert_eq!(load_of_entity(&world, office).unwrap(), LoadUnits(0));
    }

    #[test]
    fn load_of_entity_rejects_missing_and_archived_entities() {
        let mut world = test_world();
        let archived = world
            .create_unique_item(
                UniqueItemKind::Contract,
                Some("Grain Charter"),
                BTreeMap::new(),
                Tick(1),
            )
            .unwrap();
        world.archive_entity(archived, Tick(2)).unwrap();

        let missing = entity(999);

        assert!(matches!(
            load_of_entity(&world, missing),
            Err(WorldError::EntityNotFound(actual)) if actual == missing
        ));
        assert!(matches!(
            load_of_entity(&world, archived),
            Err(WorldError::ArchivedEntity(actual)) if actual == archived
        ));
    }

    #[test]
    fn current_container_load_sums_explicit_contents() {
        let mut world = test_world();
        let container = world.create_container(test_container(30), Tick(1)).unwrap();
        let lot = world
            .create_item_lot(CommodityKind::Water, Quantity(4), Tick(2))
            .unwrap();
        let item = world
            .create_unique_item(
                UniqueItemKind::Weapon,
                Some("Rusty Sword"),
                BTreeMap::new(),
                Tick(3),
            )
            .unwrap();
        let office = world.create_office("Ledger Hall", Tick(4)).unwrap();

        assert_eq!(
            current_container_load(&world, container, [lot, item, office]).unwrap(),
            LoadUnits(18)
        );
    }

    #[test]
    fn current_container_load_rejects_invalid_container_entities() {
        let mut world = test_world();
        let office = world.create_office("Ledger Hall", Tick(1)).unwrap();
        let archived_container = world.create_container(test_container(10), Tick(2)).unwrap();
        world.archive_entity(archived_container, Tick(3)).unwrap();
        let missing = entity(404);

        assert!(matches!(
            current_container_load(&world, missing, []),
            Err(WorldError::EntityNotFound(actual)) if actual == missing
        ));
        assert!(matches!(
            current_container_load(&world, archived_container, []),
            Err(WorldError::ArchivedEntity(actual)) if actual == archived_container
        ));
        assert!(matches!(
            current_container_load(&world, office, []),
            Err(WorldError::ComponentNotFound {
                entity,
                component_type: "Container",
            }) if entity == office
        ));
    }

    #[test]
    fn current_container_load_rejects_duplicate_contained_entities() {
        let mut world = test_world();
        let container = world.create_container(test_container(20), Tick(1)).unwrap();
        let lot = world
            .create_item_lot(CommodityKind::Bread, Quantity(3), Tick(2))
            .unwrap();

        let err = current_container_load(&world, container, [lot, lot]).unwrap_err();

        assert!(matches!(err, WorldError::InvariantViolation(_)));
    }

    #[test]
    fn remaining_container_capacity_subtracts_current_load() {
        let mut world = test_world();
        let container = world.create_container(test_container(25), Tick(1)).unwrap();
        let lot = world
            .create_item_lot(CommodityKind::Apple, Quantity(5), Tick(2))
            .unwrap();
        let item = world
            .create_unique_item(
                UniqueItemKind::SimpleTool,
                Some("Hammer"),
                BTreeMap::new(),
                Tick(3),
            )
            .unwrap();

        assert_eq!(
            remaining_container_capacity(&world, container, [lot, item]).unwrap(),
            LoadUnits(15)
        );
    }

    #[test]
    fn remaining_container_capacity_rejects_over_capacity_contents() {
        let mut world = test_world();
        let container = world.create_container(test_container(10), Tick(1)).unwrap();
        let item = world
            .create_unique_item(
                UniqueItemKind::Weapon,
                Some("Rusty Sword"),
                BTreeMap::new(),
                Tick(2),
            )
            .unwrap();
        let lot = world
            .create_item_lot(CommodityKind::Firewood, Quantity(1), Tick(3))
            .unwrap();

        let err = remaining_container_capacity(&world, container, [item, lot]).unwrap_err();

        assert!(matches!(err, WorldError::InvariantViolation(_)));
    }
}
