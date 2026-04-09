use worldwake_core::{
    BlockedIntent, BlockerKey, BlockingFact, CauseRef, EntityId, Tick, VisibilitySpec, WitnessData,
    WorldTxn,
};
use worldwake_sim::{PerAgentBeliefView, SpatialBeliefView, TemporalBeliefView, TickInputError};

pub(super) fn abandon_expired_facility_queues(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
    structural_block_ticks: u32,
) -> Result<bool, TickInputError> {
    let limit = {
        let view = PerAgentBeliefView::from_world(agent, world);
        let Some(limit) = view.facility_queue_patience_ticks(agent) else {
            return Ok(false);
        };
        limit
    };

    abandon_expired_facility_queues_with_limit(
        world,
        event_log,
        agent,
        tick,
        limit,
        structural_block_ticks,
    )
}

pub(super) fn abandon_expired_facility_queues_with_limit(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
    limit: std::num::NonZeroU32,
    structural_block_ticks: u32,
) -> Result<bool, TickInputError> {
    let expired_facilities = {
        let view = PerAgentBeliefView::from_world(agent, world);
        let Some(place) = view.effective_place(agent) else {
            return Ok(false);
        };

        view.entities_at(place)
            .into_iter()
            .filter(|facility| view.has_contention_policy(*facility))
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
        let Some(mut queue) = world.get_component_contention_queue(facility).cloned() else {
            continue;
        };
        if !queue.remove_actor(agent) {
            continue;
        }
        let queued_intent = world
            .get_component_contention_intents(agent)
            .and_then(|intents| intents.intents.get(&facility).copied());
        let mut blocked_memory = world
            .get_component_blocked_intent_memory(agent)
            .cloned()
            .unwrap_or_default();
        let mut facility_queue_intents = world
            .get_component_contention_intents(agent)
            .cloned()
            .unwrap_or_default();
        facility_queue_intents.intents.remove(&facility);
        if let Some(intent) = queued_intent {
            blocked_memory.record(BlockedIntent {
                blocker_key: BlockerKey {
                    goal_key: intent.goal_key,
                    place: world.effective_place(agent),
                    target: Some(facility),
                    action_def: Some(intent.intended_action),
                },
                blocking_fact: BlockingFact::ExclusiveFacilityUnavailable,
                diagnostic_context: None,
                observed_tick: tick,
                expires_tick: tick + u64::from(structural_block_ticks),
                clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
                baseline_snapshot: None,
            });
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
        txn.set_component_contention_queue(facility, queue)
            .map_err(|error| TickInputError::new(error.to_string()))?;
        if blocked_memory.intents.is_empty() {
            txn.clear_component_blocked_intent_memory(agent)
                .map_err(|error| TickInputError::new(error.to_string()))?;
        } else {
            txn.set_component_blocked_intent_memory(agent, blocked_memory)
                .map_err(|error| TickInputError::new(error.to_string()))?;
        }
        if facility_queue_intents.intents.is_empty() {
            txn.clear_component_contention_intents(agent)
                .map_err(|error| TickInputError::new(error.to_string()))?;
        } else {
            txn.set_component_contention_intents(agent, facility_queue_intents)
                .map_err(|error| TickInputError::new(error.to_string()))?;
        }
        let _ = txn.commit(event_log);
        changed = true;
    }

    Ok(changed)
}
