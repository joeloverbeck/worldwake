use crate::{CommodityKind, World, WorldError};

pub fn total_commodity_quantity(world: &World, commodity: CommodityKind) -> u64 {
    world
        .query_item_lot()
        .filter(|(_, lot)| lot.commodity == commodity)
        .map(|(_, lot)| u64::from(lot.quantity.0))
        .sum()
}

pub fn verify_conservation(
    world: &World,
    commodity: CommodityKind,
    expected_total: u64,
) -> Result<(), WorldError> {
    let actual_total = total_commodity_quantity(world, commodity);
    if actual_total == expected_total {
        return Ok(());
    }

    Err(WorldError::InvariantViolation(format!(
        "conservation violation for {commodity:?}: expected {expected_total}, found {actual_total}"
    )))
}

#[cfg(test)]
mod tests {
    use super::{total_commodity_quantity, verify_conservation};
    use crate::{CommodityKind, EventId, Quantity, Tick, Topology, World, WorldError};

    fn test_world() -> World {
        World::new(Topology::new()).unwrap()
    }

    #[test]
    fn total_commodity_quantity_is_zero_when_no_matching_lots_exist() {
        let world = test_world();

        assert_eq!(total_commodity_quantity(&world, CommodityKind::Apple), 0);
    }

    #[test]
    fn total_commodity_quantity_sums_only_matching_live_lots() {
        let mut world = test_world();
        let archived = world
            .create_item_lot(CommodityKind::Grain, Quantity(9), Tick(1))
            .unwrap();
        world
            .create_item_lot(CommodityKind::Grain, Quantity(4), Tick(2))
            .unwrap();
        world
            .create_item_lot(CommodityKind::Grain, Quantity(7), Tick(3))
            .unwrap();
        world
            .create_item_lot(CommodityKind::Apple, Quantity(5), Tick(4))
            .unwrap();
        world.archive_entity(archived, Tick(5)).unwrap();

        assert_eq!(total_commodity_quantity(&world, CommodityKind::Grain), 11);
        assert_eq!(total_commodity_quantity(&world, CommodityKind::Apple), 5);
        assert_eq!(total_commodity_quantity(&world, CommodityKind::Water), 0);
    }

    #[test]
    fn verify_conservation_accepts_matching_totals_and_rejects_mismatches() {
        let mut world = test_world();
        world
            .create_item_lot(CommodityKind::Coin, Quantity(8), Tick(1))
            .unwrap();
        world
            .create_item_lot(CommodityKind::Coin, Quantity(3), Tick(2))
            .unwrap();

        assert!(verify_conservation(&world, CommodityKind::Coin, 11).is_ok());

        let err = verify_conservation(&world, CommodityKind::Coin, 10).unwrap_err();
        assert!(matches!(err, WorldError::InvariantViolation(_)));
        assert!(err
            .to_string()
            .contains("conservation violation for Coin: expected 10, found 11"));
    }

    #[test]
    fn verify_conservation_tracks_split_and_merge_without_double_counting_archived_sources() {
        let mut world = test_world();
        let lot = world
            .create_item_lot(CommodityKind::Waste, Quantity(10), Tick(1))
            .unwrap();

        verify_conservation(&world, CommodityKind::Waste, 10).unwrap();

        let (_, split_off) = world
            .split_lot(lot, Quantity(4), Tick(2), Some(EventId(7)))
            .unwrap();
        verify_conservation(&world, CommodityKind::Waste, 10).unwrap();

        world
            .merge_lots(lot, split_off, Tick(3), Some(EventId(8)))
            .unwrap();
        verify_conservation(&world, CommodityKind::Waste, 10).unwrap();
        assert_eq!(total_commodity_quantity(&world, CommodityKind::Waste), 10);
    }
}
