use crate::{
    abort_action, start_action, tick_action, ActionDefRegistry, ActionError,
    ActionExecutionAuthority, ActionExecutionContext, ActionHandlerRegistry, ActionInstance,
    ActionInstanceId, Affordance, DeterministicRng, InputEvent, InputQueue, SystemManifest,
    TickOutcome,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldwake_core::{EntityId, EventLog, Tick, World};

pub(crate) struct SchedulerActionRuntime<'a> {
    pub(crate) action_defs: &'a ActionDefRegistry,
    pub(crate) action_handlers: &'a ActionHandlerRegistry,
    pub(crate) world: &'a mut World,
    pub(crate) event_log: &'a mut EventLog,
    pub(crate) rng: &'a mut DeterministicRng,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Scheduler {
    current_tick: Tick,
    active_actions: BTreeMap<ActionInstanceId, ActionInstance>,
    system_manifest: SystemManifest,
    input_queue: InputQueue,
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
        reason: String,
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
    use super::Scheduler;
    use crate::{
        ActionDefId, ActionDuration, ActionInstance, ActionInstanceId, ActionPayload, ActionState,
        ActionStatus, InputKind, SystemManifest,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use worldwake_core::{EntityId, ReservationId, Tick};

    fn assert_traits<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    const fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
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
            remaining_duration: ActionDuration::Finite(4),
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
}
