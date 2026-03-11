use crate::{
    AbortReason, ActionDef, ActionError, ActionHandler, ActionInstance, ActionStatus,
    DeterministicRng, ReplanNeeded,
};
use worldwake_core::{EventLog, EventTag, WorldTxn};

pub(crate) struct FailedActionTermination {
    pub status: ActionStatus,
    pub reason: AbortReason,
    pub event_tag: EventTag,
}

pub(crate) fn finalize_failed_action(
    def: &ActionDef,
    instance: &mut ActionInstance,
    handler: &ActionHandler,
    mut txn: WorldTxn<'_>,
    event_log: &mut EventLog,
    rng: &mut DeterministicRng,
    termination: &FailedActionTermination,
) -> Result<ReplanNeeded, ActionError> {
    (handler.on_abort)(def, instance, &termination.reason, rng, &mut txn)?;
    release_reservations(&mut txn, &instance.reservation_ids)?;
    instance.status = termination.status;
    txn.add_tag(termination.event_tag);
    add_targets(&mut txn, &instance.targets);
    let replan_needed = ReplanNeeded {
        agent: instance.actor,
        failed_action_def: instance.def_id,
        failed_instance: instance.instance_id,
        reason: termination.reason.clone(),
        tick: txn.tick(),
    };
    let _ = txn.commit(event_log);
    Ok(replan_needed)
}

pub(crate) fn release_reservations(
    txn: &mut WorldTxn<'_>,
    reservation_ids: &[worldwake_core::ReservationId],
) -> Result<(), ActionError> {
    for reservation_id in reservation_ids.iter().rev().copied() {
        txn.release_reservation(reservation_id)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }
    Ok(())
}

pub(crate) fn add_targets(txn: &mut WorldTxn<'_>, targets: &[worldwake_core::EntityId]) {
    for target in targets {
        txn.add_target(*target);
    }
}
