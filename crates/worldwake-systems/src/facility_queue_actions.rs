use std::collections::BTreeSet;
use std::num::NonZeroU32;
use worldwake_core::{
    ActionDefId, BodyCostPerTick, CommodityKind, Discrepancy, EntityId, EntityKind, EventTag,
    ExpectationId, Permille, Quantity, VisibilitySpec, WorkstationMarker, WorkstationTag, World,
    WorldTxn, WoundCause,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerId,
    ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress, Constraint,
    DeterministicRng, DurationExpr, EffectActionRef, EffectEntityRef, EffectEvaluationContext,
    EffectMode, EffectPrecondition, EffectSchema, EffectSink, EffectStep, Interruptibility,
    Precondition, QueueForFacilityUsePayload, TargetSpec, apply_effects_with_context,
};

pub fn register_queue_for_facility_use_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(
            start_queue_for_facility_use,
            tick_queue_for_facility_use,
            commit_queue_for_facility_use,
            abort_queue_for_facility_use,
        )
        .with_payload_override_validator(validate_queue_payload_override)
        .with_authoritative_payload_validator(validate_queue_payload_authoritatively),
    );
    let id = ActionDefId(defs.len() as u32);
    defs.register(queue_for_facility_use_action_def(id, handler))
}

fn queue_for_facility_use_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "queue_for_facility_use".to_string(),
        domain: worldwake_core::ActionDomain::Production,
        actor_constraints: vec![Constraint::ActorAlive, Constraint::ActorNotInTransit],
        targets: vec![TargetSpec::EntityAtActorPlaceAnyOf {
            kinds: [EntityKind::Facility, EntityKind::Place],
        }],
        preconditions: vec![
            Precondition::TargetExists(0),
            Precondition::TargetAtActorPlace(0),
        ],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::Fixed(NonZeroU32::MIN),
        body_cost_per_tick: BodyCostPerTick::new(pm(1), pm(1), pm(1), pm(1), pm(1)),
        attention_cost: Permille::ZERO,
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: vec![
            Precondition::TargetExists(0),
            Precondition::TargetAtActorPlace(0),
        ],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::WorldMutation]),
        payload: ActionPayload::None,
        handler,
        binding_strictness: worldwake_sim::BindingStrictness::EquivalentWorkstationTagAtSamePlace,
        guard_template: None,
        expectation_template: vec![],
        effect_schema: queue_for_facility_use_effect_schema(),
    }
}

fn queue_for_facility_use_effect_schema() -> EffectSchema {
    EffectSchema {
        preconditions: Vec::new(),
        steps: vec![EffectStep::EnqueueContention {
            actor: EffectEntityRef::Actor,
            entity: EffectEntityRef::Target { index: 0 },
            intended_action: EffectActionRef::PayloadQueueIntendedAction,
        }],
    }
}

fn queue_payload<'a>(
    def: &ActionDef,
    payload: &'a ActionPayload,
) -> Result<&'a QueueForFacilityUsePayload, ActionError> {
    payload.as_queue_for_facility_use().ok_or_else(|| {
        ActionError::PreconditionFailed(format!(
            "action def {} requires QueueForFacilityUse payload",
            def.id
        ))
    })
}

fn validate_queue_payload_override(
    def: &ActionDef,
    _actor: EntityId,
    _targets: &[EntityId],
    payload: &ActionPayload,
    _view: &dyn worldwake_sim::RuntimeBeliefView,
) -> bool {
    payload
        .as_queue_for_facility_use()
        .is_some_and(|queue| queue.intended_action != def.id)
}

fn validate_queue_payload_authoritatively(
    def: &ActionDef,
    registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let payload = queue_payload(def, payload)?;
    let entity = *targets.first().ok_or(ActionError::InvalidTarget(actor))?;

    validate_contention_queue_admission(world, actor, entity)?;

    let intended_def = registry.get(payload.intended_action).ok_or_else(|| {
        ActionError::PreconditionFailed(format!(
            "intended action {:?} is not registered",
            payload.intended_action
        ))
    })?;
    if intended_def.domain == worldwake_core::ActionDomain::Needs && intended_def.name == "sleep" {
        if world.entity_kind(entity) != Some(EntityKind::Place)
            || world.get_component_rest_capacity(entity).is_none()
        {
            return Err(ActionError::PreconditionFailed(format!(
                "entity {entity} is not a rest-capable place"
            )));
        }
        return Ok(());
    }

    let facility_marker = world
        .get_component_workstation_marker(entity)
        .copied()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("facility {entity} lacks workstation marker"))
        })?;
    let intended_tag = exclusive_facility_workstation_tag(intended_def).ok_or_else(|| {
        ActionError::PreconditionFailed(format!(
            "intended action {:?} is not an exclusive facility operation",
            payload.intended_action
        ))
    })?;

    if facility_marker != WorkstationMarker(intended_tag) {
        return Err(ActionError::PreconditionFailed(format!(
            "facility {entity} workstation {:?} does not match intended action {:?} workstation {:?}",
            facility_marker.0, payload.intended_action, intended_tag
        )));
    }

    Ok(())
}

pub(crate) fn exclusive_facility_workstation_tag(def: &ActionDef) -> Option<WorkstationTag> {
    match (&def.targets[..], &def.payload) {
        (
            [
                TargetSpec::EntityAtActorPlace {
                    kind: EntityKind::Facility,
                },
            ],
            ActionPayload::Harvest(payload),
        ) => Some(payload.required_workstation_tag),
        (
            [
                TargetSpec::EntityAtActorPlace {
                    kind: EntityKind::Facility,
                },
            ],
            ActionPayload::Craft(payload),
        ) => Some(payload.required_workstation_tag),
        _ => None,
    }
}

pub(crate) fn validate_contention_queue_admission(
    world: &World,
    actor: EntityId,
    entity: EntityId,
) -> Result<(), ActionError> {
    if world.get_component_contention_policy(entity).is_none() {
        return Err(ActionError::PreconditionFailed(format!(
            "entity {entity} lacks ContentionPolicy"
        )));
    }

    let queue = world
        .get_component_contention_queue(entity)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("entity {entity} lacks ContentionQueue"))
        })?;
    if queue.has_actor(actor) {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} is already queued or granted at entity {entity}"
        )));
    }

    Ok(())
}

pub(crate) fn enqueue_for_contention(
    txn: &mut WorldTxn<'_>,
    actor: EntityId,
    entity: EntityId,
    intended_action: ActionDefId,
) -> Result<(), ActionError> {
    let mut queue = txn
        .get_component_contention_queue(entity)
        .cloned()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("entity {entity} lacks ContentionQueue"))
        })?;
    let max_waiters = txn
        .get_component_contention_policy(entity)
        .map(|policy| policy.max_waiters)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!("entity {entity} lacks ContentionPolicy"))
        })?;

    queue
        .enqueue(actor, intended_action, txn.tick(), max_waiters)
        .map_err(|err| match err {
            worldwake_core::ContentionError::QueueFull => {
                ActionError::PreconditionFailed("contention_rejected".to_string())
            }
            other => ActionError::PreconditionFailed(format!("{other:?}")),
        })?;
    txn.set_component_contention_queue(entity, queue)
        .map_err(|err| ActionError::InternalError(err.to_string()))
}

#[allow(clippy::unnecessary_wraps)]
fn start_queue_for_facility_use(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<Option<worldwake_sim::ActionState>, ActionError> {
    Ok(None)
}

#[allow(clippy::unnecessary_wraps)]
fn tick_queue_for_facility_use(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_queue_for_facility_use(
    def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<worldwake_sim::CommitOutcome, ActionError> {
    let _ = queue_payload(def, &instance.payload)?;
    apply_queue_effect_schema(def, instance, txn)
}

struct QueueEffectSink<'txn, 'world> {
    txn: &'txn mut WorldTxn<'world>,
    action_error: Option<ActionError>,
}

impl QueueEffectSink<'_, '_> {
    fn record_error(&mut self, error: ActionError) -> Discrepancy {
        self.action_error = Some(error);
        Discrepancy::PartialExecutionDrift
    }

    fn take_error(self, discrepancy: Discrepancy) -> ActionError {
        self.action_error.unwrap_or_else(|| {
            ActionError::PreconditionFailed(format!("effect schema failed: {discrepancy:?}"))
        })
    }
}

impl EffectSink for QueueEffectSink<'_, '_> {
    fn check_precondition(
        &self,
        _precondition: &EffectPrecondition,
        _actor: EntityId,
        _targets: &[EntityId],
    ) -> Result<(), Discrepancy> {
        Ok(())
    }

    fn checkpoint(&mut self) -> usize {
        0
    }

    fn restore(&mut self, _checkpoint: usize) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn write_transfer(
        &mut self,
        _source: EntityId,
        _dest: EntityId,
        _commodity: CommodityKind,
        _quantity: Quantity,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn write_consume(
        &mut self,
        _source: EntityId,
        _commodity: CommodityKind,
        _quantity: Quantity,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn write_produce(
        &mut self,
        _sink: EntityId,
        _commodity: CommodityKind,
        _quantity: Quantity,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn write_wound(&mut self, _target: EntityId, _cause: WoundCause) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn write_event(&mut self, _tag: EventTag) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn assert_expectation_fulfilled(
        &mut self,
        _expectation: ExpectationId,
    ) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn consume_grant(&mut self, _grant: EntityId) -> Result<(), Discrepancy> {
        Err(Discrepancy::ImproperPlanningState)
    }

    fn enqueue_contention(
        &mut self,
        actor: EntityId,
        entity: EntityId,
        intended_action: ActionDefId,
    ) -> Result<(), Discrepancy> {
        enqueue_for_contention(self.txn, actor, entity, intended_action)
            .map_err(|err| self.record_error(err))
    }
}

fn apply_queue_effect_schema(
    def: &ActionDef,
    instance: &ActionInstance,
    txn: &mut WorldTxn<'_>,
) -> Result<worldwake_sim::CommitOutcome, ActionError> {
    let mut sink = QueueEffectSink {
        txn,
        action_error: None,
    };
    match apply_effects_with_context(
        &def.effect_schema,
        EffectEvaluationContext {
            actor: instance.actor,
            targets: &instance.targets,
            payload: &instance.payload,
            action_def_id: def.id,
        },
        &mut sink,
        EffectMode::Authoritative,
    ) {
        Ok(_) => Ok(worldwake_sim::CommitOutcome::empty()),
        Err(discrepancy) => Err(sink.take_error(discrepancy)),
    }
}

#[allow(clippy::unnecessary_wraps)]
fn abort_queue_for_facility_use(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _reason: &AbortReason,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

const fn pm(value: u16) -> Permille {
    Permille::new_unchecked(value)
}

#[cfg(test)]
mod tests {
    use super::register_queue_for_facility_use_action;
    use crate::{register_craft_actions, register_harvest_actions};
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        ActionDefId, BodyCostPerTick, CauseRef, CommodityKind, ContentionPolicy, ContentionQueue,
        ControlSource, EntityId, EntityKind, EventLog, EventTag, EventView, Permille,
        ProductionOutputOwner, ProductionOutputOwnershipPolicy, Quantity, ResourceSource, Seed,
        Tick, VisibilitySpec, WitnessData, WorkstationMarker, WorkstationTag, World, WorldTxn,
        build_prototype_world,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionError, ActionExecutionAuthority, ActionHandlerRegistry,
        ActionInstanceId, ActionPayload, DeterministicRng, DurationExpr, ExternalAbortReason,
        QueueForFacilityUsePayload, RecipeDefinition, RecipeRegistry, TickOutcome, abort_action,
        start_action, tick_action,
    };

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        )
    }

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    fn test_rng(byte: u8) -> DeterministicRng {
        DeterministicRng::new(Seed([byte; 32]))
    }

    fn build_recipe_registry() -> RecipeRegistry {
        let mut recipes = RecipeRegistry::new();
        let _ = recipes.register(RecipeDefinition {
            name: "Harvest Apples".to_string(),
            inputs: Vec::new(),
            outputs: vec![(CommodityKind::Apple, Quantity(2))],
            work_ticks: nz(2),
            required_workstation_tag: Some(WorkstationTag::OrchardRow),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::zero(),
        });
        let _ = recipes.register(RecipeDefinition {
            name: "Craft Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(1))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: nz(3),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::zero(),
        });
        recipes
    }

    fn setup_registries(
        recipes: &RecipeRegistry,
    ) -> (
        ActionDefRegistry,
        ActionHandlerRegistry,
        ActionDefId,
        ActionDefId,
        ActionDefId,
    ) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let queue_id = register_queue_for_facility_use_action(&mut defs, &mut handlers);
        let harvest_id = register_harvest_actions(&mut defs, &mut handlers, recipes)[0];
        let craft_id = register_craft_actions(&mut defs, &mut handlers, recipes)[0];
        (defs, handlers, queue_id, harvest_id, craft_id)
    }

    fn setup_world(with_policy: bool) -> (World, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (actor, facility) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let facility = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(facility, place).unwrap();
            txn.set_component_workstation_marker(
                facility,
                WorkstationMarker(WorkstationTag::OrchardRow),
            )
            .unwrap();
            txn.set_component_resource_source(
                facility,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(4),
                    max_quantity: Quantity(8),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                    extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                    extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                },
            )
            .unwrap();
            txn.set_component_contention_queue(facility, ContentionQueue::default())
                .unwrap();
            txn.set_component_production_output_ownership_policy(
                facility,
                ProductionOutputOwnershipPolicy {
                    output_owner: ProductionOutputOwner::Actor,
                },
            )
            .unwrap();
            if with_policy {
                txn.set_component_contention_policy(
                    facility,
                    ContentionPolicy {
                        grant_hold_ticks: nz(3),
                        auto_promote: true,
                        max_waiters: None,
                    },
                )
                .unwrap();
            }
            commit_txn(txn);
            (actor, facility)
        };
        (world, actor, facility)
    }

    fn queue_affordance(
        queue_id: ActionDefId,
        actor: EntityId,
        facility: EntityId,
        intended_action: ActionDefId,
    ) -> worldwake_sim::Affordance {
        worldwake_sim::Affordance {
            def_id: queue_id,
            actor,
            bound_targets: vec![facility],
            payload_override: Some(ActionPayload::QueueForFacilityUse(
                QueueForFacilityUsePayload { intended_action },
            )),
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        }
    }

    #[test]
    fn register_queue_for_facility_use_action_creates_expected_definition() {
        let recipes = build_recipe_registry();
        let (defs, _handlers, queue_id, _harvest_id, _craft_id) = setup_registries(&recipes);
        let def = defs.get(queue_id).unwrap();

        assert_eq!(def.name, "queue_for_facility_use");
        assert_eq!(def.domain, worldwake_core::ActionDomain::Production);
        assert_eq!(def.duration, DurationExpr::Fixed(NonZeroU32::MIN));
        assert_eq!(
            def.interruptibility,
            worldwake_sim::Interruptibility::FreelyInterruptible
        );
        assert_eq!(def.visibility, VisibilitySpec::SamePlace);
        assert!(def.causal_event_tags.contains(&EventTag::WorldMutation));
        assert_eq!(
            def.body_cost_per_tick,
            BodyCostPerTick::new(pm(1), pm(1), pm(1), pm(1), pm(1))
        );
        assert_eq!(def.payload, ActionPayload::None);
        assert_eq!(
            def.effect_schema.steps,
            vec![worldwake_sim::EffectStep::EnqueueContention {
                actor: worldwake_sim::EffectEntityRef::Actor,
                entity: worldwake_sim::EffectEntityRef::Target { index: 0 },
                intended_action: worldwake_sim::EffectActionRef::PayloadQueueIntendedAction,
            }]
        );
    }

    #[test]
    fn queue_for_facility_use_commit_enqueues_actor_with_intended_action_and_commit_tick() {
        let recipes = build_recipe_registry();
        let (defs, handlers, queue_id, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, actor, facility) = setup_world(true);
        let affordance = queue_affordance(queue_id, actor, facility, harvest_id);
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng(0x44);

        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_instance_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap();
        let outcome = tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(6)),
        )
        .unwrap();

        assert_eq!(
            outcome,
            TickOutcome::Committed {
                outcome: worldwake_sim::CommitOutcome::empty()
            }
        );
        let queue = world.get_component_contention_queue(facility).unwrap();
        let queued = queue.waiting.get(&0).unwrap();
        assert_eq!(queue.position_of(actor), Some(0));
        assert_eq!(queued.actor, actor);
        assert_eq!(queued.intended_action, harvest_id);
        assert_eq!(queued.queued_at, Tick(6));

        let record = log
            .get(*log.events_by_tag(EventTag::ActionCommitted).last().unwrap())
            .unwrap();
        assert_eq!(record.visibility(), VisibilitySpec::SamePlace);
        assert!(record.tags().contains(&EventTag::WorldMutation));
    }

    #[test]
    fn queue_for_facility_use_rejects_facility_without_policy() {
        let recipes = build_recipe_registry();
        let (defs, handlers, queue_id, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, actor, facility) = setup_world(false);
        let affordance = queue_affordance(queue_id, actor, facility, harvest_id);
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng(0x55);

        let err = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_instance_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap_err();

        assert!(
            matches!(err, ActionError::PreconditionFailed(message) if message.contains("ContentionPolicy"))
        );
    }

    #[test]
    fn queue_for_facility_use_rejects_actor_already_queued() {
        let recipes = build_recipe_registry();
        let (defs, handlers, queue_id, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, actor, facility) = setup_world(true);
        {
            let mut txn = new_txn(&mut world, 2);
            let mut queue = txn
                .get_component_contention_queue(facility)
                .cloned()
                .unwrap();
            queue.enqueue(actor, harvest_id, Tick(2), None).unwrap();
            txn.set_component_contention_queue(facility, queue).unwrap();
            commit_txn(txn);
        }
        let affordance = queue_affordance(queue_id, actor, facility, harvest_id);
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng(0x66);

        let err = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_instance_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap_err();

        assert!(
            matches!(err, ActionError::PreconditionFailed(message) if message.contains("already queued or granted"))
        );
    }

    #[test]
    fn queue_for_facility_use_rejects_actor_with_active_grant() {
        let recipes = build_recipe_registry();
        let (defs, handlers, queue_id, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, actor, facility) = setup_world(true);
        {
            let mut txn = new_txn(&mut world, 2);
            let mut queue = txn
                .get_component_contention_queue(facility)
                .cloned()
                .unwrap();
            queue.enqueue(actor, harvest_id, Tick(2), None).unwrap();
            queue.promote_head(Tick(3), nz(3));
            txn.set_component_contention_queue(facility, queue).unwrap();
            commit_txn(txn);
        }
        let affordance = queue_affordance(queue_id, actor, facility, harvest_id);
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng(0x77);

        let err = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_instance_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap_err();

        assert!(
            matches!(err, ActionError::PreconditionFailed(message) if message.contains("already queued or granted"))
        );
    }

    #[test]
    fn queue_for_facility_use_rejects_mismatched_intended_action() {
        let recipes = build_recipe_registry();
        let (defs, handlers, queue_id, _harvest_id, craft_id) = setup_registries(&recipes);
        let (mut world, actor, facility) = setup_world(true);
        let affordance = queue_affordance(queue_id, actor, facility, craft_id);
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng(0x88);

        let err = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_instance_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap_err();

        assert!(
            matches!(err, ActionError::PreconditionFailed(message) if message.contains("does not match intended action"))
        );
    }

    #[test]
    fn aborting_queue_for_facility_use_leaves_queue_unchanged() {
        let recipes = build_recipe_registry();
        let (defs, handlers, queue_id, harvest_id, _craft_id) = setup_registries(&recipes);
        let (mut world, actor, facility) = setup_world(true);
        let affordance = queue_affordance(queue_id, actor, facility, harvest_id);
        let mut log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut next_instance_id = ActionInstanceId(1);
        let mut rng = test_rng(0x99);

        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            &mut next_instance_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
        )
        .unwrap();
        let _ = abort_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                active_actions: &mut active_actions,
                world: &mut world,
                event_log: &mut log,
                rng: &mut rng,
            },
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(5)),
            ExternalAbortReason::Other,
        )
        .unwrap();

        let queue = world.get_component_contention_queue(facility).unwrap();
        assert!(queue.waiting.is_empty());
        assert_eq!(queue.granted, None);
    }
}
