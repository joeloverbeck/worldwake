use crate::{ActionHandler, ActionHandlerId};

#[derive(Clone, Default)]
pub struct ActionHandlerRegistry {
    handlers: Vec<ActionHandler>,
}

impl ActionHandlerRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, handler: ActionHandler) -> ActionHandlerId {
        let id = ActionHandlerId(self.handlers.len() as u32);
        self.handlers.push(handler);
        id
    }

    #[must_use]
    pub fn get(&self, id: ActionHandlerId) -> Option<&ActionHandler> {
        self.handlers.get(id.0 as usize)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ActionHandler> {
        self.handlers.iter()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::ActionHandlerRegistry;
    use crate::{
        AbortReason, ActionDef, ActionDefId, ActionError, ActionHandler, ActionHandlerId,
        ActionInstance, ActionInstanceId, ActionPayload, ActionProgress, ActionState,
        ActionStatus, Constraint, DurationExpr, Interruptibility, Precondition, ReservationReq,
        TargetSpec,
    };
    use std::collections::BTreeSet;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, BodyCostPerTick, CauseRef, ControlSource, EntityId, EventTag,
        ReservationId, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
    };

    fn sample_instance() -> ActionInstance {
        ActionInstance {
            instance_id: ActionInstanceId(6),
            def_id: ActionDefId(1),
            actor: EntityId {
                slot: 4,
                generation: 1,
            },
            targets: vec![],
            start_tick: Tick(9),
            remaining_ticks: 2,
            status: ActionStatus::Active,
            reservation_ids: vec![ReservationId(8)],
            local_state: None,
        }
    }

    fn sample_def() -> ActionDef {
        ActionDef {
            id: ActionDefId(1),
            name: "sample".to_string(),
            actor_constraints: vec![Constraint::ActorAlive],
            targets: vec![TargetSpec::SpecificEntity(EntityId {
                slot: 9,
                generation: 1,
            })],
            preconditions: vec![Precondition::TargetExists(0)],
            reservation_requirements: vec![ReservationReq { target_index: 0 }],
            duration: DurationExpr::Fixed(NonZeroU32::new(2).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![Precondition::ActorAlive],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::from([EventTag::WorldMutation]),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    fn start_a(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<Option<ActionState>, ActionError> {
        Ok(None)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn start_b(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<Option<ActionState>, ActionError> {
        Ok(Some(ActionState::Empty))
    }

    #[allow(clippy::unnecessary_wraps)]
    fn tick_a(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        Ok(ActionProgress::Continue)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn commit_a(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn abort_a(
        _def: &ActionDef,
        _instance: &ActionInstance,
        _reason: &AbortReason,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    fn commit_b(
        _def: &ActionDef,
        instance: &ActionInstance,
        txn: &mut WorldTxn<'_>,
    ) -> Result<(), ActionError> {
        let _ = instance.instance_id;
        txn.create_agent("Bram", ControlSource::Ai)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
        Ok(())
    }

    #[test]
    fn registry_starts_empty() {
        let registry = ActionHandlerRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn register_assigns_sequential_ids_and_get_returns_handlers() {
        let mut registry = ActionHandlerRegistry::new();
        let first = ActionHandler::new(start_a, tick_a, commit_a, abort_a);
        let second = ActionHandler::new(start_b, tick_a, commit_b, abort_a);

        let first_id = registry.register(first);
        let second_id = registry.register(second);

        assert_eq!(first_id, ActionHandlerId(0));
        assert_eq!(second_id, ActionHandlerId(1));
        assert!(registry.get(ActionHandlerId(2)).is_none());

        let retrieved_first = registry.get(first_id).unwrap();
        let retrieved_second = registry.get(second_id).unwrap();
        let instance = sample_instance();
        let def = sample_def();
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );

        assert_eq!((retrieved_first.on_start)(&def, &instance, &mut txn).unwrap(), None);
        assert_eq!(
            (retrieved_second.on_start)(&def, &instance, &mut txn).unwrap(),
            Some(ActionState::Empty)
        );
    }

    #[test]
    fn iter_returns_registration_order() {
        let mut registry = ActionHandlerRegistry::new();
        registry.register(ActionHandler::new(start_a, tick_a, commit_a, abort_a));
        registry.register(ActionHandler::new(start_b, tick_a, commit_b, abort_a));

        let instance = sample_instance();
        let def = sample_def();
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );
        let starts = registry
            .iter()
            .map(|handler| (handler.on_start)(&def, &instance, &mut txn).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(starts, vec![None, Some(ActionState::Empty)]);
    }

    #[test]
    fn retrieved_handler_can_mutate_world_through_world_txn() {
        let mut registry = ActionHandlerRegistry::new();
        let handler_id = registry.register(ActionHandler::new(start_a, tick_a, commit_b, abort_a));
        let instance = sample_instance();
        let def = sample_def();
        let mut world = World::new(build_prototype_world()).unwrap();
        let before = world.query_agent_data().count();
        let mut txn = WorldTxn::new(
            &mut world,
            Tick(1),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        );

        (registry.get(handler_id).unwrap().on_commit)(&def, &instance, &mut txn).unwrap();

        let after = txn.query_agent_data().count();
        assert_eq!(after, before + 1);
    }
}
