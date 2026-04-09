use std::collections::BTreeSet;
use std::num::NonZeroU32;
use worldwake_core::{ContentionPolicy, ContentionQueue, EntityId, WorldTxn};

pub(crate) fn corpse_contention_policy() -> ContentionPolicy {
    ContentionPolicy {
        grant_hold_ticks: NonZeroU32::new(5).unwrap(),
        auto_promote: true,
        max_waiters: None,
    }
}

pub(crate) fn care_contention_policy() -> ContentionPolicy {
    ContentionPolicy {
        grant_hold_ticks: NonZeroU32::new(5).unwrap(),
        auto_promote: true,
        max_waiters: None,
    }
}

fn clear_contention_intent_for_entity(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    entity: EntityId,
) -> Result<(), String> {
    let Some(mut intents) = txn.get_component_contention_intents(actor).cloned() else {
        return Ok(());
    };
    if intents.intents.remove(&entity).is_none() {
        return Ok(());
    }
    if intents.intents.is_empty() {
        txn.clear_component_contention_intents(actor)
            .map_err(|err| err.to_string())?;
    } else {
        txn.set_component_contention_intents(actor, intents)
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

pub(crate) fn clear_entity_contention_state(
    txn: &mut WorldTxn<'_>,
    entity: EntityId,
) -> Result<(), String> {
    if let Some(queue) = txn.get_component_contention_queue(entity).cloned() {
        let mut actors = BTreeSet::new();
        if let Some(grant) = queue.granted {
            actors.insert(grant.actor);
        }
        actors.extend(queue.waiting.values().map(|waiter| waiter.actor));
        for actor in actors {
            clear_contention_intent_for_entity(txn, actor, entity)?;
        }
        txn.clear_component_contention_queue(entity)
            .map_err(|err| err.to_string())?;
    }
    if txn.get_component_contention_policy(entity).is_some() {
        txn.clear_component_contention_policy(entity)
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

pub(crate) fn install_corpse_contention_state(
    txn: &mut WorldTxn<'_>,
    entity: EntityId,
) -> Result<(), String> {
    clear_entity_contention_state(txn, entity)?;
    txn.set_component_contention_queue(entity, ContentionQueue::default())
        .map_err(|err| err.to_string())?;
    txn.set_component_contention_policy(entity, corpse_contention_policy())
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub(crate) fn ensure_care_contention_state(
    txn: &mut WorldTxn<'_>,
    entity: EntityId,
) -> Result<(), String> {
    if txn.get_component_dead_at(entity).is_some() {
        return Ok(());
    }
    if txn.get_component_contention_queue(entity).is_none() {
        txn.set_component_contention_queue(entity, ContentionQueue::default())
            .map_err(|err| err.to_string())?;
    }
    if txn.get_component_contention_policy(entity).is_none() {
        txn.set_component_contention_policy(entity, care_contention_policy())
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}
