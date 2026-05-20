use worldwake_core::{EntityId, EventId, TestimonyReliabilityEntry, TopicScope};

pub fn expect_testimony_reliability_update(
    source: EntityId,
    topic: TopicScope,
    before: &TestimonyReliabilityEntry,
    after: &TestimonyReliabilityEntry,
    observation_event: EventId,
) {
    assert_eq!(
        after.observations(),
        before.observations().saturating_add(1),
        "expected exactly one testimony reliability observation for source={source:?} topic={topic:?}"
    );
    assert!(
        after.provenance_events.contains(&observation_event),
        "expected reliability update for source={source:?} topic={topic:?} to retain provenance event {observation_event:?}; provenance={:?}",
        after.provenance_events
    );

    let counter_deltas = [
        after
            .direct_confirmations
            .saturating_sub(before.direct_confirmations),
        after
            .direct_refutations
            .saturating_sub(before.direct_refutations),
        after.stale_claims.saturating_sub(before.stale_claims),
        after
            .contradicted_claims
            .saturating_sub(before.contradicted_claims),
    ];
    assert_eq!(
        counter_deltas.iter().sum::<u32>(),
        1,
        "expected one reliability counter to advance for source={source:?} topic={topic:?}; before={before:?} after={after:?}"
    );
}
