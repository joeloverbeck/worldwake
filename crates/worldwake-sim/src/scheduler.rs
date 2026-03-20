use crate::{
    abort_action, start_action, tick_action, ActionAbortRequestReason, ActionDefRegistry,
    ActionError, ActionExecutionAuthority, ActionExecutionContext, ActionHandlerRegistry,
    ActionInstance, ActionInstanceId, Affordance, CommitOutcome, DeterministicRng,
    ExternalAbortReason, InputEvent, InputQueue, InterruptReason, ReplanNeeded,
    ResolvedRequestTrace, SystemManifest, TickOutcome,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldwake_core::{ActionDefId, EntityId, EventLog, Tick, World};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ActionStartFailureReason {
    ReservationUnavailable(EntityId),
    PreconditionFailed(String),
    InvalidTarget(EntityId),
    AbortRequested(ActionAbortRequestReason),
}

impl ActionStartFailureReason {
    #[must_use]
    pub fn from_action_error(error: &ActionError) -> Option<Self> {
        match error {
            ActionError::ReservationUnavailable(entity) => {
                Some(Self::ReservationUnavailable(*entity))
            }
            ActionError::PreconditionFailed(detail) => {
                Some(Self::PreconditionFailed(detail.clone()))
            }
            ActionError::InvalidTarget(target) => Some(Self::InvalidTarget(*target)),
            ActionError::AbortRequested(reason) => Some(Self::AbortRequested(reason.clone())),
            ActionError::UnknownActionInstance(_)
            | ActionError::UnknownActionDef(_)
            | ActionError::UnknownActionHandler(_)
            | ActionError::InvalidActionStatus { .. }
            | ActionError::InterruptBlocked { .. }
            | ActionError::ConstraintFailed(_)
            | ActionError::InternalError(_) => None,
        }
    }

    #[must_use]
    pub fn as_action_error(&self) -> ActionError {
        match self {
            Self::ReservationUnavailable(entity) => ActionError::ReservationUnavailable(*entity),
            Self::PreconditionFailed(detail) => ActionError::PreconditionFailed(detail.clone()),
            Self::InvalidTarget(target) => ActionError::InvalidTarget(*target),
            Self::AbortRequested(reason) => ActionError::AbortRequested(reason.clone()),
        }
    }

    #[must_use]
    pub fn debug_summary(&self) -> String {
        format!("{:?}", self.as_action_error())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionStartFailure {
    pub tick: Tick,
    pub actor: EntityId,
    pub def_id: ActionDefId,
    pub request: ResolvedRequestTrace,
    pub reason: ActionStartFailureReason,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommittedAction {
    pub actor: EntityId,
    pub def_id: ActionDefId,
    pub instance_id: ActionInstanceId,
    pub tick: Tick,
    pub outcome: CommitOutcome,
}

pub struct SchedulerActionRuntime<'a> {
    pub action_defs: &'a ActionDefRegistry,
    pub action_handlers: &'a ActionHandlerRegistry,
    pub world: &'a mut World,
    pub event_log: &'a mut EventLog,
    pub rng: &'a mut DeterministicRng,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Scheduler {
    current_tick: Tick,
    active_actions: BTreeMap<ActionInstanceId, ActionInstance>,
    system_manifest: SystemManifest,
    input_queue: InputQueue,
    pending_replans: Vec<ReplanNeeded>,
    committed_actions: Vec<CommittedAction>,
    action_start_failures: Vec<ActionStartFailure>,
    next_instance_id: ActionInstanceId,
}

impl Scheduler {
    #[must_use]
    pub fn new(system_manifest: SystemManifest) -> Self {
        Self::new_with_tick(Tick(0), system_manifest)
    }

    #[must_use]
    pub fn new_with_tick(tick: Tick, system_manifest: SystemManifest) -> Self {
        Self {
            current_tick: tick,
            active_actions: BTreeMap::new(),
            system_manifest,
            input_queue: InputQueue::new(),
            pending_replans: Vec::new(),
            committed_actions: Vec::new(),
            action_start_failures: Vec::new(),
            next_instance_id: ActionInstanceId(0),
        }
    }

    #[must_use]
    pub const fn current_tick(&self) -> Tick {
        self.current_tick
    }

    #[must_use]
    pub fn active_actions(&self) -> &BTreeMap<ActionInstanceId, ActionInstance> {
        &self.active_actions
    }

    #[must_use]
    pub const fn system_manifest(&self) -> &SystemManifest {
        &self.system_manifest
    }

    #[must_use]
    pub const fn input_queue(&self) -> &InputQueue {
        &self.input_queue
    }

    pub fn input_queue_mut(&mut self) -> &mut InputQueue {
        &mut self.input_queue
    }

    #[must_use]
    pub fn pending_replans(&self) -> &[ReplanNeeded] {
        &self.pending_replans
    }

    pub fn drain_pending_replans(&mut self) -> Vec<ReplanNeeded> {
        std::mem::take(&mut self.pending_replans)
    }

    pub fn retain_replan(&mut self, replan: ReplanNeeded) {
        self.pending_replans.push(replan);
    }

    #[must_use]
    pub fn committed_actions(&self) -> &[CommittedAction] {
        &self.committed_actions
    }

    pub fn retain_committed_action(&mut self, committed: CommittedAction) {
        self.committed_actions.push(committed);
    }

    pub fn take_committed_actions_for(&mut self, actor: EntityId) -> Vec<CommittedAction> {
        let mut taken = Vec::new();
        self.committed_actions.retain(|committed| {
            if committed.actor == actor {
                taken.push(committed.clone());
                false
            } else {
                true
            }
        });
        taken
    }

    #[must_use]
    pub fn action_start_failures(&self) -> &[ActionStartFailure] {
        &self.action_start_failures
    }

    pub fn record_action_start_failure(&mut self, failure: ActionStartFailure) {
        self.action_start_failures.push(failure);
    }

    pub fn drain_action_start_failures(&mut self) -> Vec<ActionStartFailure> {
        std::mem::take(&mut self.action_start_failures)
    }

    pub fn take_action_start_failures_for(&mut self, actor: EntityId) -> Vec<ActionStartFailure> {
        let mut taken = Vec::new();
        self.action_start_failures.retain(|failure| {
            if failure.actor == actor {
                taken.push(failure.clone());
                false
            } else {
                true
            }
        });
        taken
    }

    pub(crate) fn drain_current_tick_inputs(&mut self) -> Vec<InputEvent> {
        self.input_queue.drain_tick(self.current_tick)
    }

    pub fn allocate_instance_id(&mut self) -> ActionInstanceId {
        let instance_id = self.next_instance_id;
        self.next_instance_id = ActionInstanceId(
            self.next_instance_id
                .0
                .checked_add(1)
                .expect("scheduler action instance id overflowed"),
        );
        instance_id
    }

    pub fn insert_action(&mut self, instance: ActionInstance) {
        let replaced = self.active_actions.insert(instance.instance_id, instance);
        assert!(
            replaced.is_none(),
            "scheduler action instance id already exists in active actions"
        );
    }

    pub fn remove_action(&mut self, id: ActionInstanceId) -> Option<ActionInstance> {
        self.active_actions.remove(&id)
    }

    pub(crate) fn active_action_actor(&self, id: ActionInstanceId) -> Option<EntityId> {
        self.active_actions.get(&id).map(|instance| instance.actor)
    }

    pub(crate) fn start_affordance(
        &mut self,
        affordance: &Affordance,
        runtime: SchedulerActionRuntime<'_>,
        context: ActionExecutionContext,
    ) -> Result<ActionInstanceId, ActionError> {
        let SchedulerActionRuntime {
            action_defs,
            action_handlers,
            world,
            event_log,
            rng,
        } = runtime;
        start_action(
            affordance,
            action_defs,
            action_handlers,
            ActionExecutionAuthority {
                active_actions: &mut self.active_actions,
                world,
                event_log,
                rng,
            },
            &mut self.next_instance_id,
            context,
        )
    }

    pub(crate) fn abort_active_action(
        &mut self,
        id: ActionInstanceId,
        runtime: SchedulerActionRuntime<'_>,
        context: ActionExecutionContext,
        reason: ExternalAbortReason,
    ) -> Result<crate::ReplanNeeded, ActionError> {
        let SchedulerActionRuntime {
            action_defs,
            action_handlers,
            world,
            event_log,
            rng,
        } = runtime;
        abort_action(
            id,
            action_defs,
            action_handlers,
            ActionExecutionAuthority {
                active_actions: &mut self.active_actions,
                world,
                event_log,
                rng,
            },
            context,
            reason,
        )
    }

    pub fn interrupt_active_action(
        &mut self,
        id: ActionInstanceId,
        runtime: SchedulerActionRuntime<'_>,
        context: ActionExecutionContext,
        reason: InterruptReason,
    ) -> Result<crate::ReplanNeeded, ActionError> {
        let SchedulerActionRuntime {
            action_defs,
            action_handlers,
            world,
            event_log,
            rng,
        } = runtime;
        crate::interrupt_action(
            id,
            action_defs,
            action_handlers,
            ActionExecutionAuthority {
                active_actions: &mut self.active_actions,
                world,
                event_log,
                rng,
            },
            context,
            reason,
        )
    }

    pub(crate) fn tick_active_action(
        &mut self,
        id: ActionInstanceId,
        runtime: SchedulerActionRuntime<'_>,
        context: ActionExecutionContext,
    ) -> Result<TickOutcome, ActionError> {
        let SchedulerActionRuntime {
            action_defs,
            action_handlers,
            world,
            event_log,
            rng,
        } = runtime;
        tick_action(
            id,
            action_defs,
            action_handlers,
            ActionExecutionAuthority {
                active_actions: &mut self.active_actions,
                world,
                event_log,
                rng,
            },
            context,
        )
    }

    pub fn increment_tick(&mut self) {
        self.current_tick = Tick(
            self.current_tick
                .0
                .checked_add(1)
                .expect("scheduler tick overflowed"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{ActionStartFailure, ActionStartFailureReason, CommittedAction, Scheduler};
    use crate::{
        ActionDuration, ActionInstance, ActionInstanceId, ActionPayload, ActionState, ActionStatus,
        CommitOutcome, InputKind, RequestAttemptTrace, RequestBindingKind, RequestProvenance,
        ResolvedRequestTrace, SystemManifest,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use worldwake_core::{ActionDefId, EntityId, ReservationId, Tick};

    fn assert_traits<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    const fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    const fn sample_request(input_sequence_no: u64) -> ResolvedRequestTrace {
        ResolvedRequestTrace {
            attempt: RequestAttemptTrace {
                input_sequence_no,
                provenance: RequestProvenance::External,
            },
            binding: RequestBindingKind::BestEffortFallback,
        }
    }

    fn sample_action(instance_id: u64) -> ActionInstance {
        ActionInstance {
            instance_id: ActionInstanceId(instance_id),
            def_id: ActionDefId(7),
            payload: ActionPayload::None,
            actor: entity(2),
            targets: vec![entity(3)],
            start_tick: Tick(11),
            remaining_duration: ActionDuration::new(4),
            status: ActionStatus::Active,
            reservation_ids: vec![ReservationId(9)],
            local_state: Some(ActionState::Empty),
        }
    }

    #[test]
    fn scheduler_satisfies_required_traits() {
        assert_traits::<Scheduler>();
    }

    #[test]
    fn new_starts_at_tick_zero_with_empty_scheduler_state() {
        let manifest = SystemManifest::canonical();
        let scheduler = Scheduler::new(manifest.clone());

        assert_eq!(scheduler.current_tick(), Tick(0));
        assert!(scheduler.active_actions().is_empty());
        assert!(scheduler.input_queue().is_empty());
        assert_eq!(scheduler.input_queue().next_sequence_no(), 0);
        assert_eq!(scheduler.system_manifest(), &manifest);
    }

    #[test]
    fn new_with_tick_preserves_provided_tick() {
        let scheduler = Scheduler::new_with_tick(Tick(23), SystemManifest::canonical());

        assert_eq!(scheduler.current_tick(), Tick(23));
        assert!(scheduler.active_actions().is_empty());
        assert!(scheduler.input_queue().is_empty());
    }

    #[test]
    fn allocate_instance_id_returns_monotonic_ids() {
        let mut scheduler = Scheduler::new(SystemManifest::canonical());

        assert_eq!(scheduler.allocate_instance_id(), ActionInstanceId(0));
        assert_eq!(scheduler.allocate_instance_id(), ActionInstanceId(1));
        assert_eq!(scheduler.allocate_instance_id(), ActionInstanceId(2));
    }

    #[test]
    #[should_panic(expected = "scheduler action instance id overflowed")]
    fn allocate_instance_id_panics_on_overflow() {
        let mut scheduler = Scheduler::new(SystemManifest::canonical());
        scheduler.next_instance_id = ActionInstanceId(u64::MAX);

        let _ = scheduler.allocate_instance_id();
    }

    #[test]
    fn insert_action_and_remove_action_manage_active_set() {
        let mut scheduler = Scheduler::new(SystemManifest::canonical());
        let instance = sample_action(5);

        scheduler.insert_action(instance.clone());

        assert_eq!(
            scheduler.active_actions().get(&instance.instance_id),
            Some(&instance)
        );
        assert_eq!(
            scheduler.remove_action(instance.instance_id),
            Some(instance.clone())
        );
        assert!(scheduler.active_actions().is_empty());
        assert_eq!(scheduler.remove_action(instance.instance_id), None);
    }

    #[test]
    fn active_actions_iterate_in_sorted_instance_order() {
        let mut scheduler = Scheduler::new(SystemManifest::canonical());

        scheduler.insert_action(sample_action(9));
        scheduler.insert_action(sample_action(3));
        scheduler.insert_action(sample_action(6));

        let ids = scheduler
            .active_actions()
            .keys()
            .copied()
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec![
                ActionInstanceId(3),
                ActionInstanceId(6),
                ActionInstanceId(9)
            ]
        );
    }

    #[test]
    fn increment_tick_advances_by_one() {
        let mut scheduler = Scheduler::new_with_tick(Tick(41), SystemManifest::canonical());

        scheduler.increment_tick();

        assert_eq!(scheduler.current_tick(), Tick(42));
    }

    #[test]
    #[should_panic(expected = "scheduler tick overflowed")]
    fn increment_tick_panics_on_overflow() {
        let mut scheduler = Scheduler::new_with_tick(Tick(u64::MAX), SystemManifest::canonical());

        scheduler.increment_tick();
    }

    #[test]
    fn bincode_roundtrip_preserves_active_actions_inputs_and_next_id() {
        let mut scheduler = Scheduler::new_with_tick(Tick(14), SystemManifest::canonical());
        scheduler.input_queue_mut().enqueue(
            Tick(14),
            InputKind::SwitchControl {
                from: None,
                to: Some(entity(7)),
            },
        );
        scheduler.input_queue_mut().enqueue(
            Tick(16),
            InputKind::RequestAction {
                actor: entity(2),
                def_id: ActionDefId(8),
                targets: vec![entity(9)],
                payload_override: None,
                mode: crate::ActionRequestMode::Strict,
                provenance: crate::RequestProvenance::External,
            },
        );
        scheduler.insert_action(sample_action(4));
        scheduler.insert_action(sample_action(11));
        assert_eq!(scheduler.allocate_instance_id(), ActionInstanceId(0));
        assert_eq!(scheduler.allocate_instance_id(), ActionInstanceId(1));

        let bytes = bincode::serialize(&scheduler).unwrap();
        let mut restored: Scheduler = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored, scheduler);
        assert_eq!(restored.allocate_instance_id(), ActionInstanceId(2));
        assert_eq!(restored.input_queue().peek_tick(Tick(14)).len(), 1);
        assert_eq!(restored.input_queue().peek_tick(Tick(16)).len(), 1);
    }

    #[test]
    fn committed_actions_can_be_retained_and_taken_per_actor() {
        let mut scheduler = Scheduler::new(SystemManifest::canonical());
        let actor = entity(2);
        let other = entity(8);
        let committed = CommittedAction {
            actor,
            def_id: ActionDefId(7),
            instance_id: ActionInstanceId(9),
            tick: Tick(11),
            outcome: CommitOutcome::empty(),
        };
        let other_committed = CommittedAction {
            actor: other,
            def_id: ActionDefId(3),
            instance_id: ActionInstanceId(4),
            tick: Tick(12),
            outcome: CommitOutcome::empty(),
        };

        scheduler.retain_committed_action(committed.clone());
        scheduler.retain_committed_action(other_committed.clone());

        assert_eq!(scheduler.take_committed_actions_for(actor), vec![committed]);
        assert_eq!(scheduler.committed_actions(), &[other_committed]);
    }

    #[test]
    fn action_start_failure_drain_returns_all_then_empty() {
        let mut scheduler = Scheduler::new(SystemManifest::canonical());
        let f1 = ActionStartFailure {
            tick: Tick(1),
            actor: entity(2),
            def_id: ActionDefId(3),
            request: sample_request(7),
            reason: ActionStartFailureReason::PreconditionFailed("precondition failed".into()),
        };
        let f2 = ActionStartFailure {
            tick: Tick(1),
            actor: entity(4),
            def_id: ActionDefId(5),
            request: sample_request(8),
            reason: ActionStartFailureReason::ReservationUnavailable(entity(11)),
        };

        scheduler.record_action_start_failure(f1.clone());
        scheduler.record_action_start_failure(f2.clone());

        let drained = scheduler.drain_action_start_failures();
        assert_eq!(drained, vec![f1, f2]);
        assert!(scheduler.drain_action_start_failures().is_empty());
    }

    #[test]
    fn action_start_failure_read_access() {
        let mut scheduler = Scheduler::new(SystemManifest::canonical());
        assert!(scheduler.action_start_failures().is_empty());

        let failure = ActionStartFailure {
            tick: Tick(5),
            actor: entity(7),
            def_id: ActionDefId(9),
            request: sample_request(9),
            reason: ActionStartFailureReason::InvalidTarget(entity(3)),
        };
        scheduler.record_action_start_failure(failure.clone());

        assert_eq!(scheduler.action_start_failures().len(), 1);
        assert_eq!(scheduler.action_start_failures()[0], failure);
    }

    #[test]
    fn action_start_failures_can_be_taken_per_actor() {
        let mut scheduler = Scheduler::new(SystemManifest::canonical());
        let actor = entity(7);
        let other = entity(8);
        let actor_failure = ActionStartFailure {
            tick: Tick(5),
            actor,
            def_id: ActionDefId(9),
            request: sample_request(10),
            reason: ActionStartFailureReason::InvalidTarget(entity(3)),
        };
        let other_failure = ActionStartFailure {
            tick: Tick(6),
            actor: other,
            def_id: ActionDefId(4),
            request: sample_request(11),
            reason: ActionStartFailureReason::ReservationUnavailable(entity(11)),
        };

        scheduler.record_action_start_failure(actor_failure.clone());
        scheduler.record_action_start_failure(other_failure.clone());

        assert_eq!(
            scheduler.take_action_start_failures_for(actor),
            vec![actor_failure]
        );
        assert_eq!(scheduler.action_start_failures(), &[other_failure]);
    }

    #[test]
    fn action_start_failure_satisfies_required_traits() {
        assert_traits::<ActionStartFailure>();
        assert_traits::<ActionStartFailureReason>();
    }
}
