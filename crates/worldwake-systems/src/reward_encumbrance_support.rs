use worldwake_core::{
    CommodityKind, EntityId, Quantity, RewardEncumbrance, RewardReservation, World, WorldError,
    WorldTxn,
};

pub(crate) fn reserved_reward_quantity(
    world: &World,
    office: EntityId,
    commodity: CommodityKind,
) -> Quantity {
    world
        .get_component_reward_encumbrance(office)
        .map_or(Quantity(0), |encumbrance| {
            encumbrance.reserved_quantity(commodity)
        })
}

pub(crate) fn unencumbered_reward_quantity(
    world: &World,
    office: EntityId,
    commodity: CommodityKind,
) -> Quantity {
    world
        .controlled_commodity_quantity(office, commodity)
        .checked_sub(reserved_reward_quantity(world, office, commodity))
        .unwrap_or(Quantity(0))
}

pub(crate) fn reserve_bounty_reward(
    txn: &mut WorldTxn<'_>,
    office: EntityId,
    bounty_artifact: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
) -> Result<(), WorldError> {
    let reservation = RewardReservation {
        bounty_artifact,
        commodity,
        quantity,
    };
    let mut encumbrance = txn
        .get_component_reward_encumbrance(office)
        .cloned()
        .unwrap_or_else(|| RewardEncumbrance::from_reservation(reservation.clone()));
    if !encumbrance.contains_bounty(bounty_artifact) {
        encumbrance.reserve(reservation);
    }
    txn.set_component_reward_encumbrance(office, encumbrance)
}

pub(crate) fn release_bounty_reward(
    txn: &mut WorldTxn<'_>,
    office: EntityId,
    bounty_artifact: EntityId,
) -> Result<bool, WorldError> {
    let Some(mut encumbrance) = txn.get_component_reward_encumbrance(office).cloned() else {
        return Ok(false);
    };
    if !encumbrance.release(bounty_artifact) {
        return Ok(false);
    }
    if encumbrance.is_empty() {
        txn.clear_component_reward_encumbrance(office)?;
    } else {
        txn.set_component_reward_encumbrance(office, encumbrance)?;
    }
    Ok(true)
}
