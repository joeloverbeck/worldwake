use crate::{
    ActionDefId, ActionHandlerId, ActionInstance, ActionInstanceId, ActionState, ActionStatus,
    Interruptibility,
};
use serde::{Deserialize, Serialize};
use worldwake_core::{EntityId, WorldTxn};

pub type ActionStartFn = fn(&ActionInstance) -> Result<Option<ActionState>, ActionError>;
pub type ActionTickFn =
    for<'w> fn(&ActionInstance, &mut WorldTxn<'w>) -> Result<ActionProgress, ActionError>;
pub type ActionCommitFn = for<'w> fn(&ActionInstance, &mut WorldTxn<'w>) -> Result<(), ActionError>;
pub type ActionAbortFn =
    for<'w> fn(&ActionInstance, &AbortReason, &mut WorldTxn<'w>) -> Result<(), ActionError>;

#[derive(Copy, Clone)]
pub struct ActionHandler {
    pub on_start: ActionStartFn,
    pub on_tick: ActionTickFn,
    pub on_commit: ActionCommitFn,
    pub on_abort: ActionAbortFn,
}

impl ActionHandler {
    #[must_use]
    pub const fn new(
        on_start: ActionStartFn,
        on_tick: ActionTickFn,
        on_commit: ActionCommitFn,
        on_abort: ActionAbortFn,
    ) -> Self {
        Self {
            on_start,
            on_tick,
            on_commit,
            on_abort,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum ActionProgress {
    Continue,
    Complete,
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum ActionError {
    UnknownActionInstance(ActionInstanceId),
    UnknownActionDef(ActionDefId),
    UnknownActionHandler(ActionHandlerId),
    InvalidActionStatus {
        instance_id: ActionInstanceId,
        status: ActionStatus,
    },
    InterruptBlocked {
        instance_id: ActionInstanceId,
        interruptibility: Interruptibility,
    },
    ConstraintFailed(String),
    PreconditionFailed(String),
    ReservationUnavailable(EntityId),
    InvalidTarget(EntityId),
    InternalError(String),
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum AbortReason {
    CommitConditionFailed(String),
    Interrupted(String),
    ExternalAbort(String),
}

#[cfg(test)]
mod tests {
    use super::{AbortReason, ActionError, ActionHandler, ActionProgress};
    use crate::{ActionDefId, ActionInstance, ActionInstanceId, ActionState, ActionStatus};
    use serde::{de::DeserializeOwned, Serialize};
    use worldwake_core::{
        build_prototype_world, CauseRef, ControlSource, EntityId, ReservationId, Tick,
        VisibilitySpec, WitnessData, World, WorldTxn,
    };

    fn sample_instance() -> ActionInstance {
        ActionInstance {
            instance_id: ActionInstanceId(9),
            def_id: ActionDefId(2),
            actor: EntityId {
                slot: 3,
                generation: 1,
            },
            targets: vec![EntityId {
                slot: 7,
                generation: 1,
            }],
            start_tick: Tick(12),
            remaining_ticks: 3,
            status: ActionStatus::Active,
            reservation_ids: vec![ReservationId(5)],
            local_state: Some(ActionState::Empty),
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_start(_instance: &ActionInstance) -> Result<Option<ActionState>, ActionError> {
        Ok(Some(ActionState::Empty))
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_tick(
        _instance: &ActionInstance,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        Ok(ActionProgress::Continue)
    }

    fn create_agent_on_commit(
        _instance: &ActionInstance,
        txn: &mut WorldTxn<'_>,
    ) -> Result<(), ActionError> {
        txn.create_agent("Aster", ControlSource::Ai)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
        Ok(())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_abort(
        _instance: &ActionInstance,
        _reason: &AbortReason,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    fn assert_copy_traits<T: Copy + Clone + Eq + Ord + std::hash::Hash + std::fmt::Debug>() {}

    fn assert_clone_traits<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn action_supporting_types_satisfy_required_traits() {
        assert_copy_traits::<ActionProgress>();
        assert_clone_traits::<ActionError>();
        assert_clone_traits::<AbortReason>();
    }

    #[test]
    fn action_handler_hooks_are_callable() {
        let handler = ActionHandler::new(noop_start, noop_tick, create_agent_on_commit, noop_abort);
        let mut world = World::new(build_prototype_world()).unwrap();
        let instance = sample_instance();
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );

        assert_eq!(
            (handler.on_start)(&instance).unwrap(),
            Some(ActionState::Empty)
        );
        assert_eq!(
            (handler.on_tick)(&instance, &mut txn).unwrap(),
            ActionProgress::Continue
        );
        (handler.on_abort)(
            &instance,
            &AbortReason::ExternalAbort("test".to_string()),
            &mut txn,
        )
        .unwrap();
    }

    #[test]
    fn action_handler_on_commit_can_mutate_world_through_world_txn() {
        let handler = ActionHandler::new(noop_start, noop_tick, create_agent_on_commit, noop_abort);
        let mut world = World::new(build_prototype_world()).unwrap();
        let before = world
            .entities_of_kind(worldwake_core::EntityKind::Agent)
            .count();
        let instance = sample_instance();
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );

        (handler.on_commit)(&instance, &mut txn).unwrap();

        let after = txn.query_agent_data().count();
        assert_eq!(after, before + 1);
    }
}
