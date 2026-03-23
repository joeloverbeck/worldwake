use worldwake_core::{CauseRef, EntityId, Tick, VisibilitySpec, WitnessData, WorldTxn};
use worldwake_sim::{PerAgentBeliefView, RuntimeBeliefView, TickInputError};

pub(super) fn abandon_expired_facility_queues(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
) -> Result<bool, TickInputError> {
    let limit = {
        let view = PerAgentBeliefView::from_world(agent, world);
        let Some(limit) = view.facility_queue_patience_ticks(agent) else {
            return Ok(false);
        };
        limit
    };

    abandon_expired_facility_queues_with_limit(world, event_log, agent, tick, limit)
}

pub(super) fn abandon_expired_facility_queues_with_limit(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
    limit: std::num::NonZeroU32,
) -> Result<bool, TickInputError> {
    let expired_facilities = {
        let view = PerAgentBeliefView::from_world(agent, world);
        let Some(place) = view.effective_place(agent) else {
            return Ok(false);
        };

        view.entities_at(place)
            .into_iter()
            .filter(|facility| view.has_exclusive_facility_policy(*facility))
            .filter(|facility| {
                view.facility_grant(*facility)
                    .is_none_or(|grant| grant.actor != agent)
            })
            .filter(|facility| {
                view.facility_queue_join_tick(*facility, agent)
                    .is_some_and(|queued_at| tick >= queued_at + u64::from(limit.get()))
            })
            .collect::<Vec<_>>()
    };

    let mut changed = false;
    for facility in expired_facilities {
        let Some(mut queue) = world.get_component_facility_use_queue(facility).cloned() else {
            continue;
        };
        if !queue.remove_actor(agent) {
            continue;
        }

        let mut txn = WorldTxn::new(
            world,
            tick,
            CauseRef::SystemTick(tick),
            None,
            world.effective_place(facility),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        txn.set_component_facility_use_queue(facility, queue)
            .map_err(|error| TickInputError::new(error.to_string()))?;
        let _ = txn.commit(event_log);
        changed = true;
    }

    Ok(changed)
}
