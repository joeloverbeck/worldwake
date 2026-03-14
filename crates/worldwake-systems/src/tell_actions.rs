use std::collections::BTreeSet;
use std::num::NonZeroU32;
use worldwake_core::{
    ActionDefId, AgentBeliefStore, BodyCostPerTick, EntityId, EntityKind, EventTag,
    PerceptionProfile, PerceptionSource, TellProfile, VisibilitySpec, World, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionAbortRequestReason, ActionDef, ActionDefRegistry, ActionError,
    ActionHandler, ActionHandlerId, ActionHandlerRegistry, ActionInstance, ActionPayload,
    ActionProgress, ActionState, CommitOutcome, Constraint, DeterministicRng, DurationExpr,
    Interruptibility, PayloadEntityRole, Precondition, TargetSpec, TellActionPayload,
};

pub fn register_tell_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(start_tell, tick_tell, commit_tell, abort_tell)
            .with_affordance_payloads(enumerate_tell_payloads)
            .with_payload_override_validator(validate_tell_payload_override)
            .with_authoritative_payload_validator(validate_tell_payload_authoritatively),
    );
    let id = ActionDefId(defs.len() as u32);
    defs.register(tell_action_def(id, handler))
}

fn tell_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "tell".to_string(),
        domain: worldwake_sim::ActionDomain::Social,
        actor_constraints: vec![Constraint::ActorAlive],
        targets: vec![TargetSpec::EntityAtActorPlace {
            kind: EntityKind::Agent,
        }],
        preconditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetAtActorPlace(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Agent,
            },
            Precondition::TargetAlive(0),
        ],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::Fixed(NonZeroU32::new(2).unwrap()),
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: vec![
            Precondition::ActorAlive,
            Precondition::TargetExists(0),
            Precondition::TargetAtActorPlace(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Agent,
            },
            Precondition::TargetAlive(0),
        ],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::Social, EventTag::WorldMutation]),
        payload: ActionPayload::None,
        handler,
    }
}

fn tell_payload<'a>(
    def: &ActionDef,
    payload: &'a ActionPayload,
) -> Result<&'a TellActionPayload, ActionError> {
    payload.as_tell().ok_or_else(|| {
        ActionError::PreconditionFailed(format!(
            "action def {} requires Tell payload",
            def.id
        ))
    })
}

fn belief_chain_len(source: PerceptionSource) -> u8 {
    match source {
        PerceptionSource::DirectObservation | PerceptionSource::Inference => 0,
        PerceptionSource::Report { chain_len, .. } | PerceptionSource::Rumor { chain_len } => {
            chain_len
        }
    }
}

fn degrade_source(speaker: EntityId, source: PerceptionSource) -> PerceptionSource {
    match source {
        PerceptionSource::DirectObservation => PerceptionSource::Report {
            from: speaker,
            chain_len: 1,
        },
        PerceptionSource::Report { chain_len, .. } | PerceptionSource::Rumor { chain_len } => {
            PerceptionSource::Rumor {
                chain_len: chain_len.saturating_add(1),
            }
        }
        PerceptionSource::Inference => PerceptionSource::Rumor { chain_len: 1 },
    }
}

fn passes_acceptance_check(fidelity: u16, rng: &mut DeterministicRng) -> bool {
    match fidelity {
        0 => false,
        1000 => true,
        value => rng.next_range(0, 1000) < u32::from(value),
    }
}

fn validate_tell_context(
    txn: &WorldTxn<'_>,
    instance: &ActionInstance,
    payload: &TellActionPayload,
) -> Result<EntityId, ActionError> {
    let listener = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    if payload.listener != listener {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::PayloadEntityMismatch {
                role: PayloadEntityRole::Target,
                expected: listener,
                actual: payload.listener,
            },
        ));
    }

    let actor_place = txn.effective_place(instance.actor).ok_or({
        ActionError::AbortRequested(ActionAbortRequestReason::ActorNotPlaced {
            actor: instance.actor,
        })
    })?;
    if txn.effective_place(listener) != Some(actor_place) {
        return Err(ActionError::AbortRequested(
            ActionAbortRequestReason::TargetNotColocated {
                actor: instance.actor,
                target: listener,
            },
        ));
    }

    Ok(listener)
}

fn required_tell_profile_in_world(world: &World, entity: EntityId) -> Result<TellProfile, ActionError> {
    world
        .get_component_tell_profile(entity)
        .copied()
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "live agent {entity} lacks required TellProfile"
            ))
        })
}

fn required_tell_profile(world: &WorldTxn<'_>, entity: EntityId) -> Result<TellProfile, ActionError> {
    world
        .get_component_tell_profile(entity)
        .copied()
        .ok_or_else(|| {
            ActionError::InternalError(format!(
                "live agent {entity} lacks required TellProfile"
            ))
        })
}

fn required_perception_profile(
    world: &WorldTxn<'_>,
    entity: EntityId,
) -> Result<PerceptionProfile, ActionError> {
    world
        .get_component_perception_profile(entity)
        .copied()
        .ok_or_else(|| {
            ActionError::InternalError(format!(
                "live agent {entity} lacks required PerceptionProfile"
            ))
        })
}

fn required_belief_store(
    world: &WorldTxn<'_>,
    entity: EntityId,
) -> Result<AgentBeliefStore, ActionError> {
    world
        .get_component_agent_belief_store(entity)
        .cloned()
        .ok_or_else(|| {
            ActionError::InternalError(format!(
                "live agent {entity} lacks required AgentBeliefStore"
            ))
        })
}

fn validate_tell_payload_override(
    _def: &ActionDef,
    _actor: EntityId,
    _targets: &[EntityId],
    payload: &ActionPayload,
    _view: &dyn worldwake_sim::RuntimeBeliefView,
) -> bool {
    payload.as_tell().is_some()
}

fn enumerate_tell_payloads(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    view: &dyn worldwake_sim::RuntimeBeliefView,
) -> Vec<ActionPayload> {
    let Some(listener) = targets.first().copied() else {
        return Vec::new();
    };
    if listener == actor {
        return Vec::new();
    }

    let profile = view.tell_profile(actor).unwrap_or_default();
    let mut subjects = view
        .known_entity_beliefs(actor)
        .into_iter()
        .filter_map(|(subject, belief)| {
            (belief_chain_len(belief.source) <= profile.max_relay_chain_len)
                .then_some((belief.observed_tick, subject))
        })
        .collect::<Vec<_>>();
    subjects.sort_unstable_by(|(left_tick, left_subject), (right_tick, right_subject)| {
        right_tick
            .cmp(left_tick)
            .then_with(|| left_subject.cmp(right_subject))
    });
    subjects.truncate(usize::from(profile.max_tell_candidates));

    subjects
        .into_iter()
        .map(|(_, subject)| {
            ActionPayload::Tell(TellActionPayload {
                listener,
                subject_entity: subject,
            })
        })
        .collect()
}

fn validate_tell_payload_authoritatively(
    def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let payload = tell_payload(def, payload)?;
    let listener = *targets.first().ok_or(ActionError::InvalidTarget(actor))?;

    if payload.listener != listener {
        return Err(ActionError::PreconditionFailed(format!(
            "tell payload listener {} does not match bound target {}",
            payload.listener, listener
        )));
    }
    if listener == actor {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} cannot tell themselves"
        )));
    }

    let beliefs = world.get_component_agent_belief_store(actor).ok_or_else(|| {
        ActionError::PreconditionFailed(format!("actor {actor} lacks AgentBeliefStore"))
    })?;
    let belief = beliefs.get_entity(&payload.subject_entity).ok_or_else(|| {
        ActionError::PreconditionFailed(format!(
            "actor {actor} lacks belief about subject {}",
            payload.subject_entity
        ))
    })?;

    let relay_limit = required_tell_profile_in_world(world, actor)?.max_relay_chain_len;
    let chain_len = belief_chain_len(belief.source);
    if chain_len > relay_limit {
        return Err(ActionError::PreconditionFailed(format!(
            "subject {} chain length {} exceeds actor {actor} relay limit {}",
            payload.subject_entity, chain_len, relay_limit
        )));
    }

    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn start_tell(
    def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let _ = tell_payload(def, &instance.payload)?;
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_tell(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

#[allow(clippy::unnecessary_wraps)]
fn commit_tell(
    def: &ActionDef,
    instance: &ActionInstance,
    rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let payload = tell_payload(def, &instance.payload)?;
    let listener = validate_tell_context(txn, instance, payload)?;
    let speaker = instance.actor;

    let speaker_beliefs = required_belief_store(txn, speaker)?;
    let Some(speaker_belief) = speaker_beliefs.get_entity(&payload.subject_entity).cloned() else {
        return Ok(CommitOutcome::empty());
    };

    let relay_limit = required_tell_profile(txn, speaker)?.max_relay_chain_len;
    if belief_chain_len(speaker_belief.source) > relay_limit {
        return Ok(CommitOutcome::empty());
    }

    let listener_profile = required_tell_profile(txn, listener)?;
    if !passes_acceptance_check(listener_profile.acceptance_fidelity.value(), rng) {
        return Ok(CommitOutcome::empty());
    }

    let mut transferred = speaker_belief.clone();
    transferred.source = degrade_source(speaker, speaker_belief.source);

    let mut listener_beliefs = required_belief_store(txn, listener)?;
    listener_beliefs.update_entity(payload.subject_entity, transferred);
    listener_beliefs.enforce_capacity(&required_perception_profile(txn, listener)?, txn.tick());
    txn.set_component_agent_belief_store(listener, listener_beliefs)
        .map_err(|error| ActionError::InternalError(error.to_string()))?;
    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_tell(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _reason: &AbortReason,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{register_tell_action, validate_tell_payload_authoritatively};
    use std::collections::BTreeSet;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_believed_entity_state, build_prototype_world, ActionDefId, AgentBeliefStore,
        BeliefConfidencePolicy, BelievedEntityState, BodyCostPerTick, CauseRef, CombatProfile,
        CommodityConsumableProfile, CommodityKind, ControlSource, DemandObservation,
        DriveThresholds, EntityId, EntityKind, EventLog, EventTag, HomeostaticNeeds,
        InTransitOnEdge, LoadUnits, MerchandiseProfile, MetabolismProfile, Permille,
        PerceptionProfile, PerceptionSource, Quantity, RecipeId, ResourceSource, Seed,
        TellProfile, Tick, TickRange, TradeDispositionProfile, TravelDispositionProfile,
        UniqueItemKind, VisibilitySpec, WitnessData, WorkstationTag, World, WorldTxn, Wound,
    };
    use worldwake_sim::{
        get_affordances, ActionDefRegistry, ActionError, ActionHandlerRegistry, ActionInstance,
        ActionPayload, ActionState, ActionStatus, DeterministicRng, DurationExpr,
        Interruptibility, Precondition, RuntimeBeliefView, TargetSpec, TellActionPayload,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
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

    fn new_action_txn(
        world: &mut World,
        actor: EntityId,
        visibility: VisibilitySpec,
        tick: u64,
    ) -> WorldTxn<'_> {
        let place = world.effective_place(actor);
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            Some(actor),
            place,
            visibility,
            WitnessData::default(),
        )
    }

    fn test_rng(seed: u8) -> DeterministicRng {
        DeterministicRng::new(Seed([seed; 32]))
    }

    fn world_with_speaker_listener_and_subject(
        source: PerceptionSource,
    ) -> (World, EntityId, EntityId, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();

        let (speaker, listener, subject) = {
            let mut txn = new_txn(&mut world, 1);
            let speaker = txn.create_agent("Speaker", ControlSource::Ai).unwrap();
            let listener = txn.create_agent("Listener", ControlSource::Ai).unwrap();
            let subject = txn.create_agent("Subject", ControlSource::Ai).unwrap();
            txn.set_component_tell_profile(
                listener,
                TellProfile {
                    max_tell_candidates: 3,
                    max_relay_chain_len: 3,
                    acceptance_fidelity: Permille::new(1000).unwrap(),
                },
            )
            .unwrap();
            for entity in [speaker, listener, subject] {
                txn.set_ground_location(entity, place).unwrap();
            }
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (speaker, listener, subject)
        };

        let belief = build_believed_entity_state(&world, subject, Tick(2), source).unwrap();
        let mut store = world
            .get_component_agent_belief_store(speaker)
            .cloned()
            .unwrap_or_else(AgentBeliefStore::new);
        store.update_entity(subject, belief);

        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_agent_belief_store(speaker, store).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        (world, place, speaker, listener, subject)
    }

    fn tell_test_setup(
        source: PerceptionSource,
    ) -> (
        ActionDefRegistry,
        ActionHandlerRegistry,
        ActionDefId,
        World,
        EntityId,
        EntityId,
        EntityId,
        EntityId,
    ) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let (world, place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(source);
        (defs, handlers, tell_id, world, place, speaker, listener, subject)
    }

    fn tell_instance(
        tell_id: ActionDefId,
        speaker: EntityId,
        listener: EntityId,
        subject: EntityId,
    ) -> ActionInstance {
        ActionInstance {
            instance_id: worldwake_sim::ActionInstanceId(0),
            def_id: tell_id,
            payload: ActionPayload::Tell(TellActionPayload {
                listener,
                subject_entity: subject,
            }),
            actor: speaker,
            targets: vec![listener],
            start_tick: Tick(5),
            remaining_duration: worldwake_sim::ActionDuration::Finite(1),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        }
    }

    fn commit_tell_and_finalize_event(
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        tell_id: ActionDefId,
        world: &mut World,
        instance: &ActionInstance,
        seed: u8,
        tick: u64,
    ) {
        let def = defs.get(tell_id).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let mut rng = test_rng(seed);
        let mut txn = new_action_txn(world, instance.actor, def.visibility, tick);

        (handler.on_commit)(def, instance, &mut rng, &mut txn).unwrap();
        txn.add_tag(EventTag::ActionCommitted);
        for tag in &def.causal_event_tags {
            txn.add_tag(*tag);
        }
        for target in &instance.targets {
            txn.add_target(*target);
        }

        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
        assert_eq!(log.len(), 1);
    }

    fn commit_tell_result(
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        tell_id: ActionDefId,
        world: &mut World,
        instance: &ActionInstance,
        seed: u8,
        tick: u64,
    ) -> Result<worldwake_sim::CommitOutcome, ActionError> {
        let def = defs.get(tell_id).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let mut rng = test_rng(seed);
        let mut txn = new_action_txn(world, instance.actor, def.visibility, tick);

        (handler.on_commit)(def, instance, &mut rng, &mut txn)
    }

    #[derive(Default)]
    struct StubTellBeliefView {
        alive: std::collections::BTreeMap<EntityId, bool>,
        kinds: std::collections::BTreeMap<EntityId, EntityKind>,
        places: std::collections::BTreeMap<EntityId, EntityId>,
        beliefs: std::collections::BTreeMap<EntityId, Vec<(EntityId, BelievedEntityState)>>,
        tell_profiles: std::collections::BTreeMap<EntityId, TellProfile>,
    }

    impl RuntimeBeliefView for StubTellBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.get(&entity).copied().unwrap_or(false)
        }

        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.kinds.get(&entity).copied()
        }

        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.places.get(&entity).copied()
        }

        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }

        fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
            let mut entities = self
                .places
                .iter()
                .filter_map(|(entity, entity_place)| (*entity_place == place).then_some(*entity))
                .collect::<Vec<_>>();
            entities.sort();
            entities
        }

        fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
            self.beliefs.get(&agent).cloned().unwrap_or_default()
        }

        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn adjacent_places(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
            false
        }

        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
        }

        fn commodity_quantity(&self, _holder: EntityId, _kind: CommodityKind) -> Quantity {
            Quantity(0)
        }

        fn controlled_commodity_quantity_at_place(
            &self,
            _agent: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Quantity {
            Quantity(0)
        }

        fn local_controlled_lots_for(
            &self,
            _agent: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn item_lot_commodity(&self, _entity: EntityId) -> Option<CommodityKind> {
            None
        }

        fn item_lot_consumable_profile(
            &self,
            _entity: EntityId,
        ) -> Option<CommodityConsumableProfile> {
            None
        }

        fn direct_container(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn direct_possessor(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }

        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }

        fn has_control(&self, _entity: EntityId) -> bool {
            false
        }

        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }

        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
            Vec::new()
        }

        fn is_dead(&self, entity: EntityId) -> bool {
            !self.is_alive(entity)
        }

        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }

        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
        }

        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            None
        }

        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
        }

        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }

        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
        }

        fn travel_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<TravelDispositionProfile> {
            None
        }

        fn tell_profile(&self, agent: EntityId) -> Option<TellProfile> {
            self.tell_profiles.get(&agent).copied()
        }

        fn combat_profile(&self, _agent: EntityId) -> Option<CombatProfile> {
            None
        }

        fn wounds(&self, _agent: EntityId) -> Vec<Wound> {
            Vec::new()
        }

        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn agents_selling_at(&self, _place: EntityId, _commodity: CommodityKind) -> Vec<EntityId> {
            Vec::new()
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            Vec::new()
        }

        fn matching_workstations_at(
            &self,
            _place: EntityId,
            _tag: WorkstationTag,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            Vec::new()
        }

        fn merchandise_profile(&self, _agent: EntityId) -> Option<MerchandiseProfile> {
            None
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn in_transit_state(&self, _entity: EntityId) -> Option<InTransitOnEdge> {
            None
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            _place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            Vec::new()
        }

        fn estimate_duration(
            &self,
            _actor: EntityId,
            _duration: &DurationExpr,
            _targets: &[EntityId],
            _payload: &ActionPayload,
        ) -> Option<worldwake_sim::ActionDuration> {
            None
        }
    }

    fn collect_tell_affordances_from_view(
        view: &dyn RuntimeBeliefView,
        speaker: EntityId,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
    ) -> Vec<(EntityId, EntityId)> {
        get_affordances(view, speaker, defs, handlers)
            .into_iter()
            .filter_map(|affordance| {
                affordance
                    .payload_override
                    .and_then(|payload| payload.as_tell().cloned())
                    .map(|payload| (payload.listener, payload.subject_entity))
            })
            .collect()
    }

    #[test]
    fn register_tell_action_creates_expected_definition() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();

        assert_eq!(tell.name, "tell");
        assert_eq!(tell.domain, worldwake_sim::ActionDomain::Social);
        assert_eq!(
            tell.targets,
            vec![TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }]
        );
        assert_eq!(tell.duration, DurationExpr::Fixed(NonZeroU32::new(2).unwrap()));
        assert_eq!(tell.body_cost_per_tick, BodyCostPerTick::zero());
        assert_eq!(tell.interruptibility, Interruptibility::FreelyInterruptible);
        assert_eq!(tell.visibility, VisibilitySpec::SamePlace);
        assert_eq!(
            tell.causal_event_tags,
            BTreeSet::from([EventTag::Social, EventTag::WorldMutation])
        );
        assert!(handlers.get(tell.handler).is_some());
        assert_eq!(tell.payload, ActionPayload::None);
        assert!(tell.preconditions.contains(&Precondition::TargetAlive(0)));
        assert!(tell.commit_conditions.contains(&Precondition::TargetAlive(0)));
    }

    #[test]
    fn tell_payload_validator_rejects_non_tell_payload() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (world, _place, speaker, listener, _subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[listener],
            &ActionPayload::None,
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn tell_payload_validator_rejects_listener_target_mismatch() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (world, _place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);
        let other_listener = entity(999);

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[listener],
            &ActionPayload::Tell(TellActionPayload {
                listener: other_listener,
                subject_entity: subject,
            }),
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn tell_payload_validator_rejects_self_targeting() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (world, _place, speaker, _listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[speaker],
            &ActionPayload::Tell(TellActionPayload {
                listener: speaker,
                subject_entity: subject,
            }),
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn tell_payload_validator_rejects_unknown_subject_belief() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (world, _place, speaker, listener, _subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[listener],
            &ActionPayload::Tell(TellActionPayload {
                listener,
                subject_entity: entity(404),
            }),
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn tell_payload_validator_rejects_subjects_beyond_relay_limit() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (mut world, _place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::Rumor { chain_len: 4 });

        {
            let mut txn = new_txn(&mut world, 4);
            txn.set_component_tell_profile(
                speaker,
                TellProfile {
                    max_tell_candidates: 3,
                    max_relay_chain_len: 2,
                    acceptance_fidelity: Permille::new(800).unwrap(),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[listener],
            &ActionPayload::Tell(TellActionPayload {
                listener,
                subject_entity: subject,
            }),
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
    }

    #[test]
    fn tell_payload_validator_accepts_known_relayable_subject() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (world, _place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::Report {
                from: entity(77),
                chain_len: 2,
            });

        assert_eq!(
            validate_tell_payload_authoritatively(
                tell,
                &defs,
                speaker,
                &[listener],
                &ActionPayload::Tell(TellActionPayload {
                    listener,
                    subject_entity: subject,
                }),
                &world,
            ),
            Ok(())
        );
    }

    #[test]
    fn tell_payload_validator_rejects_missing_speaker_tell_profile() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let (mut world, _place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);
        {
            let mut txn = new_txn(&mut world, 4);
            txn.clear_component_tell_profile(speaker).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        let err = validate_tell_payload_authoritatively(
            tell,
            &defs,
            speaker,
            &[listener],
            &ActionPayload::Tell(TellActionPayload {
                listener,
                subject_entity: subject,
            }),
            &world,
        )
        .unwrap_err();

        assert!(matches!(err, ActionError::PreconditionFailed(_)));
        assert!(format!("{err:?}").contains("TellProfile"));
    }

    #[test]
    fn tell_action_starts_with_tell_payload() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let tell_id = register_tell_action(&mut defs, &mut handlers);
        let tell = defs.get(tell_id).unwrap();
        let handler = handlers.get(tell.handler).unwrap();
        let (mut world, _place, speaker, listener, subject) =
            world_with_speaker_listener_and_subject(PerceptionSource::DirectObservation);
        let instance = ActionInstance {
            instance_id: worldwake_sim::ActionInstanceId(0),
            def_id: tell_id,
            payload: ActionPayload::Tell(TellActionPayload {
                listener,
                subject_entity: subject,
            }),
            actor: speaker,
            targets: vec![listener],
            start_tick: Tick(5),
            remaining_duration: worldwake_sim::ActionDuration::Finite(2),
            status: worldwake_sim::ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: None,
        };
        let mut rng = test_rng(1);
        let mut txn = new_txn(&mut world, 5);

        assert_eq!(
            (handler.on_start)(tell, &instance, &mut rng, &mut txn).unwrap(),
            Some(ActionState::Empty)
        );
    }

    #[test]
    fn tell_commit_transfers_direct_observation_as_report_and_preserves_tick() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        let transferred = listener_store.get_entity(&subject).unwrap();
        assert_eq!(transferred.observed_tick, Tick(2));
        assert_eq!(
            transferred.source,
            PerceptionSource::Report {
                from: speaker,
                chain_len: 1,
            }
        );
    }

    #[test]
    fn tell_commit_degrades_report_to_rumor() {
        let report_source = PerceptionSource::Report {
            from: entity(77),
            chain_len: 2,
        };
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(report_source);
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let transferred = world
            .get_component_agent_belief_store(listener)
            .unwrap()
            .get_entity(&subject)
            .unwrap();
        assert_eq!(transferred.source, PerceptionSource::Rumor { chain_len: 3 });
    }

    #[test]
    fn tell_commit_degrades_rumor_to_deeper_rumor() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::Rumor { chain_len: 3 });
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let transferred = world
            .get_component_agent_belief_store(listener)
            .unwrap()
            .get_entity(&subject)
            .unwrap();
        assert_eq!(transferred.source, PerceptionSource::Rumor { chain_len: 4 });
    }

    #[test]
    fn tell_commit_degrades_inference_to_first_hand_rumor() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::Inference);
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let transferred = world
            .get_component_agent_belief_store(listener)
            .unwrap()
            .get_entity(&subject)
            .unwrap();
        assert_eq!(transferred.source, PerceptionSource::Rumor { chain_len: 1 });
    }

    #[test]
    fn tell_commit_skips_when_speaker_no_longer_has_subject_belief() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.set_component_agent_belief_store(speaker, AgentBeliefStore::new())
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        assert!(listener_store.get_entity(&subject).is_none());
    }

    #[test]
    fn tell_commit_fails_if_speaker_lacks_belief_store() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.clear_component_agent_belief_store(speaker).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        let err = commit_tell_result(&defs, &handlers, tell_id, &mut world, &instance, 1, 8)
            .unwrap_err();

        assert!(matches!(err, ActionError::InternalError(_)));
        assert!(format!("{err:?}").contains("AgentBeliefStore"));
        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        assert!(listener_store.get_entity(&subject).is_none());
    }

    #[test]
    fn tell_commit_fails_if_listener_lacks_belief_store() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.clear_component_agent_belief_store(listener).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        let err = commit_tell_result(&defs, &handlers, tell_id, &mut world, &instance, 1, 8)
            .unwrap_err();

        assert!(matches!(err, ActionError::InternalError(_)));
        assert!(format!("{err:?}").contains("AgentBeliefStore"));
    }

    #[test]
    fn tell_commit_fails_if_listener_lacks_tell_profile() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.clear_component_tell_profile(listener).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        let err = commit_tell_result(&defs, &handlers, tell_id, &mut world, &instance, 1, 8)
            .unwrap_err();

        assert!(matches!(err, ActionError::InternalError(_)));
        assert!(format!("{err:?}").contains("TellProfile"));
    }

    #[test]
    fn tell_commit_fails_if_listener_lacks_perception_profile() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.clear_component_perception_profile(listener).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        let err = commit_tell_result(&defs, &handlers, tell_id, &mut world, &instance, 1, 8)
            .unwrap_err();

        assert!(matches!(err, ActionError::InternalError(_)));
        assert!(format!("{err:?}").contains("PerceptionProfile"));
    }

    #[test]
    fn tell_commit_respects_listener_acceptance_fidelity() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.set_component_tell_profile(
                listener,
                TellProfile {
                    max_tell_candidates: 3,
                    max_relay_chain_len: 3,
                    acceptance_fidelity: Permille::new(0).unwrap(),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        assert!(listener_store.get_entity(&subject).is_none());
    }

    #[test]
    fn tell_commit_keeps_listener_newer_belief() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let newer = build_believed_entity_state(
            &world,
            subject,
            Tick(7),
            PerceptionSource::DirectObservation,
        )
        .unwrap();
        {
            let mut store = world
                .get_component_agent_belief_store(listener)
                .cloned()
                .unwrap_or_default();
            store.update_entity(subject, newer.clone());

            let mut txn = new_txn(&mut world, 7);
            txn.set_component_agent_belief_store(listener, store).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let retained = world
            .get_component_agent_belief_store(listener)
            .unwrap()
            .get_entity(&subject)
            .unwrap();
        assert_eq!(retained, &newer);
    }

    #[test]
    fn tell_commit_enforces_listener_memory_capacity() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let older_subject = {
            let place = world.topology().place_ids().next().unwrap();
            let mut txn = new_txn(&mut world, 4);
            let older_subject = txn.create_agent("OlderSubject", ControlSource::Ai).unwrap();
            txn.set_ground_location(older_subject, place).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            older_subject
        };
        let older_belief = build_believed_entity_state(
            &world,
            older_subject,
            Tick(1),
            PerceptionSource::DirectObservation,
        )
        .unwrap();
        {
            let mut store = world
                .get_component_agent_belief_store(listener)
                .cloned()
                .unwrap_or_default();
            store.update_entity(older_subject, older_belief);

            let mut txn = new_txn(&mut world, 6);
            txn.set_component_agent_belief_store(listener, store).unwrap();
            txn.set_component_perception_profile(
                listener,
                PerceptionProfile {
                    memory_capacity: 1,
                    memory_retention_ticks: 100,
                    observation_fidelity: Permille::new(1000).unwrap(),
                    confidence_policy: BeliefConfidencePolicy::default(),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        assert!(listener_store.get_entity(&older_subject).is_none());
        assert!(listener_store.get_entity(&subject).is_some());
        assert_eq!(listener_store.known_entities.len(), 1);
    }

    #[test]
    fn tell_commit_rechecks_relay_limit_against_current_belief() {
        let report_source = PerceptionSource::Report {
            from: entity(77),
            chain_len: 2,
        };
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(report_source);
        {
            let mut txn = new_txn(&mut world, 6);
            txn.set_component_tell_profile(
                speaker,
                TellProfile {
                    max_tell_candidates: 3,
                    max_relay_chain_len: 1,
                    acceptance_fidelity: Permille::new(800).unwrap(),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let instance = tell_instance(tell_id, speaker, listener, subject);

        commit_tell_and_finalize_event(&defs, &handlers, tell_id, &mut world, &instance, 1, 8);

        let listener_store = world.get_component_agent_belief_store(listener).unwrap();
        assert!(listener_store.get_entity(&subject).is_none());
    }

    #[test]
    fn tell_commit_relies_on_scheduler_event_transaction_shape() {
        let (defs, handlers, tell_id, mut world, _place, speaker, listener, subject) =
            tell_test_setup(PerceptionSource::DirectObservation);
        let def = defs.get(tell_id).unwrap();
        let handler = handlers.get(def.handler).unwrap();
        let instance = tell_instance(tell_id, speaker, listener, subject);
        let mut rng = test_rng(1);
        let mut txn = new_action_txn(&mut world, speaker, def.visibility, 8);

        (handler.on_commit)(def, &instance, &mut rng, &mut txn).unwrap();
        txn.add_tag(EventTag::ActionCommitted);
        for tag in &def.causal_event_tags {
            txn.add_tag(*tag);
        }
        for target in &instance.targets {
            txn.add_target(*target);
        }

        let mut log = EventLog::new();
        let event_id = txn.commit(&mut log);
        let record = log.get(event_id).unwrap();

        assert_eq!(record.actor_id, Some(speaker));
        assert_eq!(record.target_ids, vec![listener]);
        assert_eq!(record.visibility, VisibilitySpec::SamePlace);
        assert!(record.tags.contains(&EventTag::ActionCommitted));
        assert!(record.tags.contains(&EventTag::Social));
        assert!(record.tags.contains(&EventTag::WorldMutation));
    }

    #[test]
    fn tell_affordances_expand_live_colocated_listeners_across_relayable_subjects() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_tell_action(&mut defs, &mut handlers);
        let speaker = entity(1);
        let listener_a = entity(2);
        let listener_b = entity(3);
        let dead_listener = entity(4);
        let subject_a = entity(10);
        let subject_b = entity(11);
        let subject_c = entity(12);
        let place = entity(20);
        let mut view = StubTellBeliefView::default();

        for entity in [speaker, listener_a, listener_b, dead_listener] {
            view.kinds.insert(entity, EntityKind::Agent);
            view.places.insert(entity, place);
        }
        view.alive.insert(speaker, true);
        view.alive.insert(listener_a, true);
        view.alive.insert(listener_b, true);
        view.alive.insert(dead_listener, false);
        view.tell_profiles.insert(
            speaker,
            TellProfile {
                max_tell_candidates: 3,
                max_relay_chain_len: 3,
                acceptance_fidelity: Permille::new(800).unwrap(),
            },
        );
        view.beliefs.insert(
            speaker,
            vec![
                (
                    subject_a,
                    BelievedEntityState {
                        last_known_place: Some(entity(30)),
                        last_known_inventory: std::collections::BTreeMap::default(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        observed_tick: Tick(2),
                        source: PerceptionSource::DirectObservation,
                    },
                ),
                (
                    subject_b,
                    BelievedEntityState {
                        last_known_place: Some(entity(31)),
                        last_known_inventory: std::collections::BTreeMap::default(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        observed_tick: Tick(4),
                        source: PerceptionSource::Report {
                            from: entity(77),
                            chain_len: 2,
                        },
                    },
                ),
                (
                    subject_c,
                    BelievedEntityState {
                        last_known_place: Some(entity(32)),
                        last_known_inventory: std::collections::BTreeMap::default(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        observed_tick: Tick(6),
                        source: PerceptionSource::Inference,
                    },
                ),
            ],
        );

        let affordances = collect_tell_affordances_from_view(&view, speaker, &defs, &handlers);

        assert_eq!(
            affordances,
            vec![
                (listener_a, subject_a),
                (listener_a, subject_b),
                (listener_a, subject_c),
                (listener_b, subject_a),
                (listener_b, subject_b),
                (listener_b, subject_c),
            ]
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn tell_affordances_filter_relay_depth_and_limit_subjects_by_recency() {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_tell_action(&mut defs, &mut handlers);
        let speaker = entity(1);
        let listener = entity(2);
        let subject_a = entity(10);
        let subject_b = entity(11);
        let subject_c = entity(12);
        let subject_d = entity(13);
        let subject_e = entity(14);
        let place = entity(20);
        let mut view = StubTellBeliefView::default();

        for entity in [speaker, listener] {
            view.kinds.insert(entity, EntityKind::Agent);
            view.places.insert(entity, place);
            view.alive.insert(entity, true);
        }
        view.tell_profiles.insert(
            speaker,
            TellProfile {
                max_tell_candidates: 3,
                max_relay_chain_len: 2,
                acceptance_fidelity: Permille::new(800).unwrap(),
            },
        );
        view.beliefs.insert(
            speaker,
            vec![
                (
                    subject_a,
                    BelievedEntityState {
                        last_known_place: Some(entity(30)),
                        last_known_inventory: std::collections::BTreeMap::default(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        observed_tick: Tick(3),
                        source: PerceptionSource::DirectObservation,
                    },
                ),
                (
                    subject_b,
                    BelievedEntityState {
                        last_known_place: Some(entity(31)),
                        last_known_inventory: std::collections::BTreeMap::default(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        observed_tick: Tick(9),
                        source: PerceptionSource::Report {
                            from: entity(80),
                            chain_len: 2,
                        },
                    },
                ),
                (
                    subject_c,
                    BelievedEntityState {
                        last_known_place: Some(entity(32)),
                        last_known_inventory: std::collections::BTreeMap::default(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        observed_tick: Tick(9),
                        source: PerceptionSource::Inference,
                    },
                ),
                (
                    subject_d,
                    BelievedEntityState {
                        last_known_place: Some(entity(33)),
                        last_known_inventory: std::collections::BTreeMap::default(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        observed_tick: Tick(7),
                        source: PerceptionSource::Rumor { chain_len: 3 },
                    },
                ),
                (
                    subject_e,
                    BelievedEntityState {
                        last_known_place: Some(entity(34)),
                        last_known_inventory: std::collections::BTreeMap::default(),
                        workstation_tag: None,
                        resource_source: None,
                        alive: true,
                        wounds: Vec::new(),
                        observed_tick: Tick(5),
                        source: PerceptionSource::Rumor { chain_len: 1 },
                    },
                ),
            ],
        );

        let affordances = collect_tell_affordances_from_view(&view, speaker, &defs, &handlers);

        assert_eq!(
            affordances,
            vec![(listener, subject_b), (listener, subject_c), (listener, subject_e)]
        );
    }
}
