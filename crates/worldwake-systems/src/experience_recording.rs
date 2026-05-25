use worldwake_core::{EntityId, EventId, ReliabilityRecord, SourceKey, Tick, WorldTxn};
use worldwake_sim::ActionError;

fn update_source_reliability(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    key: SourceKey,
    current_tick: Tick,
    provenance_event: Option<EventId>,
    update: impl FnOnce(&mut ReliabilityRecord),
) -> Result<(), ActionError> {
    let mut reliability = txn
        .get_component_source_reliability(actor)
        .cloned()
        .unwrap_or_default();
    let record = reliability
        .sources
        .entry(key)
        .or_insert_with(|| ReliabilityRecord::new(current_tick));
    update(record);
    record.last_attempt_tick = current_tick;
    if let Some(event_id) = provenance_event {
        record.push_provenance(event_id);
    }

    let profile = txn
        .get_component_preference_profile(actor)
        .copied()
        .unwrap_or_else(|| panic!("actor {actor} lacks PreferenceProfile"));
    reliability.enforce_limits(current_tick, &profile);

    txn.set_component_source_reliability(actor, reliability)
        .map_err(|err| ActionError::InternalError(err.to_string()))
}

pub(crate) fn record_successful_source_acquisition(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    key: SourceKey,
    current_tick: Tick,
    provenance_event: Option<EventId>,
) -> Result<(), ActionError> {
    update_source_reliability(txn, actor, key, current_tick, provenance_event, |record| {
        record.successful_acquisitions = record.successful_acquisitions.saturating_add(1);
    })
}

pub(crate) fn record_failed_source_attempt(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    key: SourceKey,
    current_tick: Tick,
    provenance_event: Option<EventId>,
) -> Result<(), ActionError> {
    update_source_reliability(txn, actor, key, current_tick, provenance_event, |record| {
        record.failed_attempts = record.failed_attempts.saturating_add(1);
    })
}
