use std::collections::BTreeSet;
use worldwake_core::{load_of_entity, CarryCapacity, EntityId, LoadUnits, WorldTxn};
use worldwake_sim::ActionError;

pub(crate) fn controlled_entity_ids(txn: &WorldTxn<'_>, root: EntityId) -> BTreeSet<EntityId> {
    let mut controlled = BTreeSet::new();
    let mut frontier = vec![root];

    while let Some(entity) = frontier.pop() {
        if !controlled.insert(entity) {
            continue;
        }
        frontier.extend(txn.possessions_of(entity));
        frontier.extend(txn.direct_contents_of(entity));
    }

    controlled
}

pub(crate) fn controlled_entity_load(
    txn: &WorldTxn<'_>,
    root: EntityId,
) -> Result<LoadUnits, ActionError> {
    let total = controlled_entity_ids(txn, root)
        .into_iter()
        .try_fold(0_u32, |total, entity| {
            total
                .checked_add(
                    load_of_entity(txn, entity)
                        .map_err(|err| {
                            ActionError::InternalError(format!(
                                "failed to compute controlled load for {entity}: {err}"
                            ))
                        })?
                        .0,
                )
                .ok_or_else(|| ActionError::InternalError("controlled load overflowed".to_string()))
        })?;

    Ok(LoadUnits(total))
}

pub(crate) fn carried_entities(txn: &WorldTxn<'_>, actor: EntityId) -> BTreeSet<EntityId> {
    let mut carried = BTreeSet::new();
    let mut frontier = txn.possessions_of(actor);

    while let Some(entity) = frontier.pop() {
        if !carried.insert(entity) {
            continue;
        }
        frontier.extend(txn.possessions_of(entity));
        frontier.extend(txn.direct_contents_of(entity));
    }

    carried
}

pub(crate) fn carried_load(txn: &WorldTxn<'_>, actor: EntityId) -> Result<LoadUnits, ActionError> {
    let total = carried_entities(txn, actor)
        .into_iter()
        .try_fold(0_u32, |total, entity| {
            total
                .checked_add(
                    load_of_entity(txn, entity)
                        .map_err(|err| {
                            ActionError::InternalError(format!(
                                "failed to compute carried load for {entity}: {err}"
                            ))
                        })?
                        .0,
                )
                .ok_or_else(|| ActionError::InternalError("carried load overflowed".to_string()))
        })?;

    Ok(LoadUnits(total))
}

pub(crate) fn remaining_capacity(
    txn: &WorldTxn<'_>,
    actor: EntityId,
) -> Result<LoadUnits, ActionError> {
    let CarryCapacity(capacity) = txn
        .get_component_carry_capacity(actor)
        .copied()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("actor {actor} lacks CarryCapacity"))
        })?;
    let current = carried_load(txn, actor)?;
    capacity
        .0
        .checked_sub(current.0)
        .map(LoadUnits)
        .ok_or_else(|| {
            ActionError::InternalError(format!(
                "actor {actor} is over carry capacity: load {} exceeds capacity {}",
                current.0, capacity.0
            ))
        })
}

pub(crate) fn move_entity_to_direct_possession(
    txn: &mut WorldTxn<'_>,
    entity: EntityId,
    holder: EntityId,
    place: EntityId,
) -> Result<(), ActionError> {
    if txn.direct_container(entity).is_some() {
        txn.remove_from_container(entity)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    if txn.possessor_of(entity).is_some() {
        txn.clear_possessor(entity)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    if txn.effective_place(entity) != Some(place) {
        txn.set_ground_location(entity, place)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    txn.set_possessor(entity, holder)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;
    txn.add_target(entity);
    Ok(())
}
