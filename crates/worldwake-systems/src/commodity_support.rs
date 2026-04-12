use worldwake_core::{CommodityKind, EntityId, Quantity, World, WorldTxn};
use worldwake_sim::{ActionAbortRequestReason, ActionError};

pub(crate) fn ensure_accessible_quantity(
    world: &World,
    holder: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
) -> Result<(), ActionError> {
    let available = world.controlled_commodity_quantity(holder, commodity);
    if available < quantity {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::HolderLacksAccessibleCommodity {
                holder,
                commodity,
                quantity,
            },
        ));
    }
    Ok(())
}

pub(crate) fn resolve_controlled_lots(
    txn: &mut WorldTxn<'_>,
    holder: EntityId,
    commodity: CommodityKind,
    quantity: Quantity,
    place: EntityId,
    context: &str,
) -> Result<Vec<(EntityId, Quantity)>, ActionError> {
    let mut remaining = quantity;
    let mut selected = Vec::new();
    let mut lots = txn
        .query_item_lot()
        .filter_map(|(entity, lot)| {
            (lot.commodity == commodity
                && txn.can_exercise_control(holder, entity).is_ok()
                && txn.effective_place(entity) == Some(place))
            .then_some((entity, lot.quantity))
        })
        .collect::<Vec<_>>();
    lots.sort_by_key(|(entity, _)| *entity);

    for (lot_id, available) in lots {
        if remaining == Quantity(0) {
            break;
        }
        if available > remaining {
            let (_, split_off) = txn
                .split_lot(lot_id, remaining)
                .map_err(|error| ActionError::InternalError(error.to_string()))?;
            selected.push((split_off, remaining));
            remaining = Quantity(0);
            break;
        }

        selected.push((lot_id, available));
        remaining = remaining
            .checked_sub(available)
            .ok_or_else(|| ActionError::InternalError(context.to_string()))?;
    }

    if remaining != Quantity(0) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::HolderLacksAccessibleCommodity {
                holder,
                commodity,
                quantity,
            },
        ));
    }

    Ok(selected)
}
