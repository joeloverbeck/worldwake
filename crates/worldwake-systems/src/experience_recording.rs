use worldwake_core::{EntityId, ReliabilityRecord, SourceKey, Tick, WorldTxn};
use worldwake_sim::ActionError;

fn update_source_reliability(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    key: SourceKey,
    current_tick: Tick,
    update: impl FnOnce(&mut ReliabilityRecord),
) -> Result<(), ActionError> {
    let mut reliability = txn
        .get_component_source_reliability(actor)
        .cloned()
        .unwrap_or_default();
    let record = reliability.sources.entry(key).or_insert(ReliabilityRecord {
        successful_acquisitions: 0,
        failed_attempts: 0,
        last_attempt_tick: current_tick,
    });
    update(record);
    record.last_attempt_tick = current_tick;

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
) -> Result<(), ActionError> {
    update_source_reliability(txn, actor, key, current_tick, |record| {
        record.successful_acquisitions = record.successful_acquisitions.saturating_add(1);
    })
}

pub(crate) fn record_failed_source_attempt(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    key: SourceKey,
    current_tick: Tick,
) -> Result<(), ActionError> {
    update_source_reliability(txn, actor, key, current_tick, |record| {
        record.failed_attempts = record.failed_attempts.saturating_add(1);
    })
}
