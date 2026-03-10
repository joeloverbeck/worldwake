use crate::{CommodityKind, World, WorldError};

pub fn total_live_lot_quantity(world: &World, commodity: CommodityKind) -> u64 {
    world
        .query_item_lot()
        .filter(|(_, lot)| lot.commodity == commodity)
        .map(|(_, lot)| u64::from(lot.quantity.0))
        .sum()
}

pub fn total_authoritative_commodity_quantity(world: &World, commodity: CommodityKind) -> u64 {
    total_live_lot_quantity(world, commodity)
        + world
            .query_resource_source()
            .filter(|(_, source)| source.commodity == commodity)
            .map(|(_, source)| u64::from(source.available_quantity.0))
            .sum::<u64>()
}

pub fn verify_live_lot_conservation(
    world: &World,
    commodity: CommodityKind,
    expected_total: u64,
) -> Result<(), WorldError> {
    let actual_total = total_live_lot_quantity(world, commodity);
    if actual_total == expected_total {
        return Ok(());
    }

    Err(WorldError::InvariantViolation(format!(
        "live-lot conservation violation for {commodity:?}: expected {expected_total}, found {actual_total}"
    )))
}

pub fn verify_authoritative_conservation(
    world: &World,
    commodity: CommodityKind,
    expected_total: u64,
) -> Result<(), WorldError> {
    let actual_total = total_authoritative_commodity_quantity(world, commodity);
    if actual_total == expected_total {
        return Ok(());
    }

    Err(WorldError::InvariantViolation(format!(
        "authoritative conservation violation for {commodity:?}: expected {expected_total}, found {actual_total}"
    )))
}

#[cfg(test)]
mod tests {
    use super::{
        total_authoritative_commodity_quantity, total_live_lot_quantity,
        verify_authoritative_conservation, verify_live_lot_conservation,
    };
    use crate::{
        CauseRef, CommodityKind, EventId, Quantity, ResourceSource, Tick, Topology, VisibilitySpec,
        WitnessData, World, WorldError, WorldTxn,
    };

    fn test_world() -> World {
        World::new(Topology::new()).unwrap()
    }

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::Hidden,
            WitnessData::default(),
        )
    }

    #[test]
    fn total_live_lot_quantity_is_zero_when_no_matching_lots_exist() {
        let world = test_world();

        assert_eq!(total_live_lot_quantity(&world, CommodityKind::Apple), 0);
    }

    #[test]
    fn total_live_lot_quantity_sums_only_matching_live_lots() {
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

        assert_eq!(total_live_lot_quantity(&world, CommodityKind::Grain), 11);
        assert_eq!(total_live_lot_quantity(&world, CommodityKind::Apple), 5);
        assert_eq!(total_live_lot_quantity(&world, CommodityKind::Water), 0);
    }

    #[test]
    fn total_authoritative_commodity_quantity_includes_live_resource_sources() {
        let mut world = test_world();
        let (place, archived_place) = {
            let mut txn = new_txn(&mut world, 1);
            let place = txn.create_entity(crate::EntityKind::Facility);
            let archived_place = txn.create_entity(crate::EntityKind::Facility);
            let _ = txn.commit(&mut crate::EventLog::new());
            (place, archived_place)
        };
        world
            .create_item_lot(CommodityKind::Coin, Quantity(8), Tick(1))
            .unwrap();
        world
            .create_item_lot(CommodityKind::Coin, Quantity(3), Tick(2))
            .unwrap();
        let mut txn = new_txn(&mut world, 2);
        txn.set_component_resource_source(
            place,
            ResourceSource {
                commodity: CommodityKind::Coin,
                available_quantity: Quantity(5),
                max_quantity: Quantity(9),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        )
        .unwrap();
        txn.set_component_resource_source(
            archived_place,
            ResourceSource {
                commodity: CommodityKind::Coin,
                available_quantity: Quantity(4),
                max_quantity: Quantity(9),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        )
        .unwrap();
        let _ = txn.commit(&mut crate::EventLog::new());
        world.archive_entity(archived_place, Tick(3)).unwrap();

        assert_eq!(total_live_lot_quantity(&world, CommodityKind::Coin), 11);
        assert_eq!(
            total_authoritative_commodity_quantity(&world, CommodityKind::Coin),
            16
        );
    }

    #[test]
    fn verify_live_lot_conservation_accepts_matching_totals_and_rejects_mismatches() {
        let mut world = test_world();
        world
            .create_item_lot(CommodityKind::Coin, Quantity(8), Tick(1))
            .unwrap();
        world
            .create_item_lot(CommodityKind::Coin, Quantity(3), Tick(2))
            .unwrap();

        assert!(verify_live_lot_conservation(&world, CommodityKind::Coin, 11).is_ok());

        let err = verify_live_lot_conservation(&world, CommodityKind::Coin, 10).unwrap_err();
        assert!(matches!(err, WorldError::InvariantViolation(_)));
        assert!(err
            .to_string()
            .contains("live-lot conservation violation for Coin: expected 10, found 11"));
    }

    #[test]
    fn verify_authoritative_conservation_counts_sources_and_lots_together() {
        let mut world = test_world();
        let place = {
            let mut txn = new_txn(&mut world, 1);
            let place = txn.create_entity(crate::EntityKind::Facility);
            let _ = txn.commit(&mut crate::EventLog::new());
            place
        };
        world
            .create_item_lot(CommodityKind::Apple, Quantity(3), Tick(1))
            .unwrap();
        let mut txn = new_txn(&mut world, 2);
        txn.set_component_resource_source(
            place,
            ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(7),
                max_quantity: Quantity(7),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        )
        .unwrap();
        let _ = txn.commit(&mut crate::EventLog::new());

        verify_authoritative_conservation(&world, CommodityKind::Apple, 10).unwrap();

        let err = verify_authoritative_conservation(&world, CommodityKind::Apple, 9).unwrap_err();
        assert!(matches!(err, WorldError::InvariantViolation(_)));
        assert!(err
            .to_string()
            .contains("authoritative conservation violation for Apple: expected 9, found 10"));
    }

    #[test]
    fn verify_live_lot_conservation_tracks_split_and_merge_without_double_counting_archived_sources(
    ) {
        let mut world = test_world();
        let lot = world
            .create_item_lot(CommodityKind::Waste, Quantity(10), Tick(1))
            .unwrap();

        verify_live_lot_conservation(&world, CommodityKind::Waste, 10).unwrap();

        let (_, split_off) = world
            .split_lot(lot, Quantity(4), Tick(2), Some(EventId(7)))
            .unwrap();
        verify_live_lot_conservation(&world, CommodityKind::Waste, 10).unwrap();

        world
            .merge_lots(lot, split_off, Tick(3), Some(EventId(8)))
            .unwrap();
        verify_live_lot_conservation(&world, CommodityKind::Waste, 10).unwrap();
        assert_eq!(total_live_lot_quantity(&world, CommodityKind::Waste), 10);
    }
}
