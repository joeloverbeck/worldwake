//! Assertions for route-segment blocker lifecycle goldens.

use std::num::NonZeroU32;

use worldwake_core::{
    Blocker, BlockerClearingCondition, BlockerMemory, BlockerScope, BlockingFact, EventId,
    EventLog, RouteSegment, Tick,
};

pub fn expect_route_blocker_lifecycle(
    event_log: &EventLog,
    segment: RouteSegment,
    observation_event: EventId,
    observed_tick: Tick,
    ttl: NonZeroU32,
) -> BlockerMemory {
    assert!(
        event_log.get(observation_event).is_some(),
        "route blocker source event {observation_event:?} must be present in the append-only log"
    );

    let expires_tick = Tick(observed_tick.0 + u64::from(ttl.get()));
    let mut memory = BlockerMemory::default();
    memory.record(Blocker {
        scope: BlockerScope::RouteSegment(segment),
        blocking_fact: BlockingFact::DangerTooHigh,
        diagnostic_context: None,
        observed_tick,
        expires_tick,
        clearing_condition: BlockerClearingCondition::TtlOnly,
        baseline_snapshot: None,
        source_event: Some(observation_event),
    });

    let stored = memory
        .route_segment_blocked(segment.from, segment.to, observed_tick)
        .expect("route segment should be blocked at the observation tick");
    assert_eq!(stored.source_event, Some(observation_event));
    assert_eq!(stored.clearing_condition, BlockerClearingCondition::TtlOnly);

    let last_persistent_tick = Tick(expires_tick.0.saturating_sub(1));
    assert!(
        memory
            .route_segment_blocked(segment.from, segment.to, last_persistent_tick)
            .is_some(),
        "route segment blocker should persist until the tick before TTL expiry"
    );

    let active_memory = memory.clone();
    memory.expire(expires_tick);
    assert!(
        memory
            .route_segment_blocked(segment.from, segment.to, expires_tick)
            .is_none(),
        "route segment blocker should clear at TTL expiry"
    );

    active_memory
}
