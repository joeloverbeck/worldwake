use std::collections::BTreeSet;
use worldwake_core::{
    ActionDefId, BodyCostPerTick, CommodityKind, EntityId, EntityKind, EventTag, PerceptionSource,
    RecordedViolation, SocialObservation, SocialObservationDetail, ViolationId, ViolationKind,
    VisibilitySpec, World, WorldTxn,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerId,
    ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress, ActionState,
    CommitOutcome, Constraint, DeterministicRng, DurationExpr, Interruptibility,
    InvestigateActionPayload, PerAgentBeliefView, Precondition, RuntimeBeliefView, TargetSpec,
};

pub fn register_investigate_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(
            start_investigate,
            tick_investigate,
            commit_investigate,
            abort_investigate,
        )
        .with_affordance_payloads(enumerate_investigate_payloads)
        .with_payload_override_validator(validate_investigate_payload_override)
        .with_authoritative_payload_validator(validate_investigate_payload_authoritatively),
    );
    let id = ActionDefId(defs.len() as u32);
    defs.register(investigate_action_def(id, handler))
}

fn investigate_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "investigate".to_string(),
        domain: worldwake_sim::ActionDomain::Generic,
        actor_constraints: vec![Constraint::ActorAlive, Constraint::ActorNotIncapacitated],
        targets: vec![TargetSpec::ActorPlace],
        preconditions: vec![
            Precondition::TargetExists(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Place,
            },
        ],
        reservation_requirements: Vec::new(),
        duration: DurationExpr::ActorInvestigationDisposition,
        body_cost_per_tick: BodyCostPerTick::zero(),
        interruptibility: Interruptibility::FreelyInterruptible,
        commit_conditions: vec![
            Precondition::TargetExists(0),
            Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Place,
            },
        ],
        visibility: VisibilitySpec::SamePlace,
        causal_event_tags: BTreeSet::from([EventTag::Discovery]),
        payload: ActionPayload::None,
        handler,
    }
}

fn start_investigate(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let place = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    let violation_id = investigate_payload(&instance.payload)
        .map_err(ActionError::PreconditionFailed)?
        .violation_id;
    let memory = txn
        .get_component_violation_memory(instance.actor)
        .ok_or_else(|| {
            ActionError::PreconditionFailed(format!(
                "actor {} lacks ViolationMemory",
                instance.actor
            ))
        })?;
    let Some(record) = memory.unresolved_by_id(violation_id, txn.tick()) else {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {} has no active violation {}",
            instance.actor, violation_id.0
        )));
    };
    let Some(state) = investigable_state_for_record(record, place) else {
        return Err(ActionError::PreconditionFailed(format!(
            "violation {} is not investigable at place {}",
            violation_id.0, place
        )));
    };
    Ok(Some(state))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_investigate(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn commit_investigate(
    _def: &ActionDef,
    instance: &ActionInstance,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let (violation_id, subject, place, commodity) = investigate_state(instance)?;
    let belief = PerAgentBeliefView::from_world(instance.actor, txn);
    let owner_is_investigating_actor = belief.believed_owner_of(subject) == Some(instance.actor);

    let mut store = txn
        .get_component_agent_belief_store(instance.actor)
        .cloned()
        .ok_or_else(|| {
            ActionError::InternalError(format!(
                "live agent {} lacks AgentBeliefStore",
                instance.actor
            ))
        })?;
    store.record_social_observation(SocialObservation {
        detail: SocialObservationDetail::WitnessedAbsence {
            missing_entity: subject,
            expected_place: place,
        },
        place,
        observed_tick: txn.tick(),
        source: PerceptionSource::DirectObservation,
    });
    if owner_is_investigating_actor {
        store.record_social_observation(SocialObservation {
            detail: SocialObservationDetail::SuspectedTheft {
                missing_entity: subject,
                expected_place: place,
                suspect: None,
            },
            place,
            observed_tick: txn.tick(),
            source: PerceptionSource::DirectObservation,
        });
    }
    txn.set_component_agent_belief_store(instance.actor, store)
        .map_err(|err| ActionError::InternalError(err.to_string()))?;

    let mut memory = txn
        .get_component_violation_memory(instance.actor)
        .cloned()
        .ok_or_else(|| {
            ActionError::InternalError(format!(
                "live agent {} lacks ViolationMemory",
                instance.actor
            ))
        })?;
    let Some(record) = memory.unresolved_by_id(violation_id, txn.tick()) else {
        return Err(ActionError::PreconditionFailed(format!(
            "violation {} is no longer active at commit",
            violation_id.0
        )));
    };
    let Some((expected_subject, expected_commodity)) = investigable_binding(record, place) else {
        return Err(ActionError::PreconditionFailed(format!(
            "violation {} no longer matches place {}",
            violation_id.0, place
        )));
    };
    if expected_subject != subject || expected_commodity != commodity {
        return Err(ActionError::PreconditionFailed(format!(
            "violation {} no longer matches bound investigate state",
            violation_id.0
        )));
    }
    if let Some(profile) = txn.get_component_violation_disposition_profile(instance.actor) {
        let resolved = memory.resolve_id(
            violation_id,
            txn.tick(),
            profile.violation_memory_retention_ticks,
        );
        if !resolved {
            return Err(ActionError::PreconditionFailed(format!(
                "violation {} expired before resolution",
                violation_id.0
            )));
        }
        if owner_is_investigating_actor {
            memory.record(
                ViolationKind::SuspectedTheft {
                    missing_entity: subject,
                    expected_place: place,
                    suspect: None,
                },
                txn.tick(),
                profile.violation_memory_retention_ticks,
            );
        }
        txn.set_component_violation_memory(instance.actor, memory)
            .map_err(|err| ActionError::InternalError(err.to_string()))?;
    }

    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_investigate(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _reason: &AbortReason,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

fn enumerate_investigate_payloads(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    view: &dyn RuntimeBeliefView,
) -> Vec<ActionPayload> {
    let Some(place) = targets.first().copied() else {
        return Vec::new();
    };
    view.active_violation_records(actor)
        .into_iter()
        .filter(|record| investigable_binding(record, place).is_some())
        .map(|record| {
            ActionPayload::Investigate(InvestigateActionPayload {
                violation_id: record.id,
            })
        })
        .collect()
}

fn validate_investigate_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(place) = targets.first().copied() else {
        return false;
    };
    let Some(payload) = payload.as_investigate() else {
        return false;
    };
    view.active_violation_records(actor)
        .into_iter()
        .any(|record| {
            record.id == payload.violation_id && investigable_binding(&record, place).is_some()
        })
}

fn validate_investigate_payload_authoritatively(
    _def: &ActionDef,
    _registry: &ActionDefRegistry,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    world: &World,
) -> Result<(), ActionError> {
    let Some(place) = targets.first().copied() else {
        return Err(ActionError::InvalidTarget(actor));
    };
    let payload = investigate_payload(payload).map_err(ActionError::PreconditionFailed)?;
    let memory = world.get_component_violation_memory(actor).ok_or_else(|| {
        ActionError::PreconditionFailed(format!("actor {actor} lacks ViolationMemory"))
    })?;
    let Some(record) = memory
        .violations
        .iter()
        .find(|record| record.id == payload.violation_id)
    else {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {} has no violation {}",
            actor, payload.violation_id.0
        )));
    };
    if investigable_binding(record, place).is_none() {
        return Err(ActionError::PreconditionFailed(format!(
            "violation {} is not investigable at place {}",
            payload.violation_id.0, place
        )));
    }
    Ok(())
}

fn investigable_state_for_record(
    record: &RecordedViolation,
    place: EntityId,
) -> Option<ActionState> {
    let (subject, commodity) = investigable_binding(record, place)?;
    Some(ActionState::Investigate {
        violation_id: record.id,
        subject,
        place,
        commodity,
    })
}

fn investigable_binding(
    record: &RecordedViolation,
    place: EntityId,
) -> Option<(EntityId, Option<CommodityKind>)> {
    match &record.kind {
        ViolationKind::EntityMissing {
            entity,
            expected_place,
        } if *expected_place == place => Some((*entity, None)),
        ViolationKind::SupplyDepleted {
            commodity,
            source,
            place: violation_place,
        } if *violation_place == place => Some((*source, Some(*commodity))),
        ViolationKind::EntityMissing { .. }
        | ViolationKind::SupplyDepleted { .. }
        | ViolationKind::EntityDead { .. }
        | ViolationKind::SuspectedTheft { .. } => None,
    }
}

fn investigate_state(
    instance: &ActionInstance,
) -> Result<(ViolationId, EntityId, EntityId, Option<CommodityKind>), ActionError> {
    match instance.local_state {
        Some(ActionState::Investigate {
            violation_id,
            subject,
            place,
            commodity,
        }) => Ok((violation_id, subject, place, commodity)),
        _ => Err(ActionError::InternalError(format!(
            "investigate action instance {} is missing investigate state",
            instance.instance_id
        ))),
    }
}

fn investigate_payload(payload: &ActionPayload) -> Result<&InvestigateActionPayload, String> {
    payload
        .as_investigate()
        .ok_or_else(|| "investigate action requires investigate payload".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, AgentBeliefStore, BelievedEntityState, CauseRef, CombatProfile,
        ControlSource, EventLog, PerceptionSource, Permille, Quantity, Seed, Tick,
        ViolationDispositionProfile, ViolationMemory, WitnessData, World, Wound, WoundCause,
        WoundId, WoundList,
    };
    use worldwake_sim::{
        abort_action, get_affordances, start_action, tick_action, ActionExecutionAuthority,
        ActionExecutionContext, ActionInstanceId, Affordance, DeterministicRng,
        ExternalAbortReason, PerAgentBeliefView, TickOutcome,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    fn new_world() -> World {
        World::new(build_prototype_world()).unwrap()
    }

    fn first_two_places(world: &World) -> (EntityId, EntityId) {
        let places = world.topology().place_ids().collect::<Vec<_>>();
        (places[0], places[1])
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

    fn spawn_actor(world: &mut World, place: EntityId) -> EntityId {
        let mut txn = new_txn(world, 1);
        let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, place).unwrap();
        txn.set_component_agent_belief_store(actor, AgentBeliefStore::default())
            .unwrap();
        txn.set_component_violation_memory(actor, ViolationMemory::default())
            .unwrap();
        txn.set_component_combat_profile(
            actor,
            CombatProfile::new(
                pm(1000),
                pm(700),
                pm(600),
                pm(550),
                pm(75),
                pm(20),
                pm(15),
                pm(120),
                pm(30),
                nz(6),
                nz(10),
            ),
        )
        .unwrap();
        commit_txn(txn);
        actor
    }

    fn set_violation_profile(world: &mut World, actor: EntityId, duration: u32, retention: u32) {
        let mut txn = new_txn(world, 1);
        txn.set_component_violation_disposition_profile(
            actor,
            ViolationDispositionProfile {
                investigation_duration_ticks: nz(duration),
                violation_memory_retention_ticks: retention,
                investigation_motive_weight: pm(500),
                ownership_motive_bonus: pm(200),
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    fn record_violation(
        world: &mut World,
        actor: EntityId,
        violation: ViolationKind,
        observed_tick: u64,
        ttl: u32,
    ) -> ViolationId {
        let mut txn = new_txn(world, observed_tick);
        let mut memory = txn
            .get_component_violation_memory(actor)
            .cloned()
            .unwrap_or_default();
        let violation_id = memory.record(violation, Tick(observed_tick), ttl);
        txn.set_component_violation_memory(actor, memory).unwrap();
        commit_txn(txn);
        violation_id
    }

    fn create_item_lot_at_place(
        world: &mut World,
        place: EntityId,
        owner: Option<EntityId>,
    ) -> EntityId {
        let mut txn = new_txn(world, 1);
        let lot = txn.create_item_lot(CommodityKind::Bread, Quantity(1)).unwrap();
        txn.set_ground_location(lot, place).unwrap();
        if let Some(owner) = owner {
            txn.set_owner(lot, owner).unwrap();
        }
        commit_txn(txn);
        lot
    }

    fn mark_entity_known(world: &mut World, actor: EntityId, entity: EntityId, place: EntityId) {
        let mut txn = new_txn(world, 1);
        let mut store = txn
            .get_component_agent_belief_store(actor)
            .cloned()
            .expect("actor should have a belief store");
        store.update_entity(
            entity,
            BelievedEntityState {
                last_known_place: Some(place),
                last_known_inventory: BTreeMap::new(),
                workstation_tag: None,
                resource_source: None,
                alive: true,
                wounds: Vec::new(),
                last_known_courage: None,
                observed_tick: Tick(1),
                source: PerceptionSource::DirectObservation,
            },
        );
        txn.set_component_agent_belief_store(actor, store).unwrap();
        commit_txn(txn);
    }

    fn set_incapacitated(world: &mut World, actor: EntityId) {
        let mut txn = new_txn(world, 1);
        txn.set_component_wound_list(
            actor,
            WoundList {
                wounds: vec![Wound {
                    id: WoundId(1),
                    body_part: worldwake_core::BodyPart::Torso,
                    cause: WoundCause::Combat {
                        attacker: actor,
                        weapon: worldwake_core::CombatWeaponRef::Unarmed,
                    },
                    severity: pm(700),
                    inflicted_at: Tick(1),
                    bleed_rate_per_tick: pm(0),
                }],
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    fn setup_registries() -> (ActionDefRegistry, ActionHandlerRegistry, ActionDefId) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let def_id = register_investigate_action(&mut defs, &mut handlers);
        (defs, handlers, def_id)
    }

    fn investigate_affordance(
        world: &World,
        actor: EntityId,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
    ) -> Affordance {
        get_affordances(
            &PerAgentBeliefView::from_world(actor, world),
            actor,
            defs,
            handlers,
        )
        .into_iter()
        .find(|affordance| defs.get(affordance.def_id).unwrap().name == "investigate")
        .expect("investigate affordance should exist")
    }

    fn investigate_affordance_for_id(
        world: &World,
        actor: EntityId,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        violation_id: ViolationId,
    ) -> Affordance {
        get_affordances(
            &PerAgentBeliefView::from_world(actor, world),
            actor,
            defs,
            handlers,
        )
        .into_iter()
        .find(|affordance| {
            defs.get(affordance.def_id).unwrap().name == "investigate"
                && affordance
                    .payload_override
                    .as_ref()
                    .and_then(ActionPayload::as_investigate)
                    .is_some_and(|payload| payload.violation_id == violation_id)
        })
        .expect("investigate affordance should exist for violation id")
    }

    #[test]
    fn register_investigate_action_creates_expected_definition() {
        let (defs, handlers, def_id) = setup_registries();
        let def = defs.get(def_id).unwrap();

        assert!(handlers.get(def.handler).is_some());
        assert_eq!(def.name, "investigate");
        assert_eq!(def.domain, worldwake_sim::ActionDomain::Generic);
        assert_eq!(def.targets, vec![TargetSpec::ActorPlace]);
        assert_eq!(def.duration, DurationExpr::ActorInvestigationDisposition);
        assert_eq!(def.interruptibility, Interruptibility::FreelyInterruptible);
        assert_eq!(def.visibility, VisibilitySpec::SamePlace);
        assert_eq!(def.causal_event_tags, BTreeSet::from([EventTag::Discovery]));
        assert!(def
            .actor_constraints
            .contains(&Constraint::ActorNotIncapacitated));
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn investigate_action_commits_witnessed_absence_and_extends_violation_memory() {
        let mut world = new_world();
        let (place, _) = first_two_places(&world);
        let missing = entity(30);
        let actor = spawn_actor(&mut world, place);
        set_violation_profile(&mut world, actor, 2, 50);
        let violation_id = record_violation(
            &mut world,
            actor,
            ViolationKind::EntityMissing {
                entity: missing,
                expected_place: place,
            },
            1,
            5,
        );

        let (defs, handlers, def_id) = setup_registries();
        let affordance = investigate_affordance(&world, actor, &defs, &handlers);
        assert_eq!(affordance.def_id, def_id);
        assert_eq!(affordance.bound_targets, vec![place]);
        assert_eq!(
            affordance.payload_override,
            Some(ActionPayload::Investigate(InvestigateActionPayload {
                violation_id
            }))
        );

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([7; 32]));
        let mut next_instance_id = ActionInstanceId(1);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            &mut next_instance_id,
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap();

        assert_eq!(
            active_actions
                .get(&instance_id)
                .unwrap()
                .remaining_duration
                .ticks(),
            2
        );

        assert_eq!(
            tick_action(
                instance_id,
                &defs,
                &handlers,
                ActionExecutionAuthority {
                    world: &mut world,
                    event_log: &mut event_log,
                    active_actions: &mut active_actions,
                    rng: &mut rng,
                },
                ActionExecutionContext {
                    tick: Tick(3),
                    cause: CauseRef::Bootstrap,
                },
            )
            .unwrap(),
            TickOutcome::Continuing
        );

        match tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            ActionExecutionContext {
                tick: Tick(4),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap()
        {
            TickOutcome::Committed { .. } => {}
            other => panic!("expected committed investigate action, got {other:?}"),
        }

        let store = world.get_component_agent_belief_store(actor).unwrap();
        assert!(store.social_observations.iter().any(|observation| {
            observation.kind() == worldwake_core::SocialObservationKind::WitnessedAbsence
                && observation.detail
                    == SocialObservationDetail::WitnessedAbsence {
                        missing_entity: missing,
                        expected_place: place,
                    }
                && observation.place == place
                && observation.observed_tick == Tick(4)
                && observation.source == PerceptionSource::DirectObservation
        }));

        let memory = world.get_component_violation_memory(actor).unwrap();
        assert!(memory.violations.iter().any(|record| {
            record.id == violation_id
                && record.kind
                    == ViolationKind::EntityMissing {
                        entity: missing,
                        expected_place: place,
                    }
                && record.observed_tick == Tick(1)
                && record.resolved_tick == Some(Tick(4))
                && record.expires_tick == Tick(54)
        }));

        assert!(
            !event_log.events_by_tag(EventTag::Discovery).is_empty(),
            "investigate commit should emit a discovery-tagged event"
        );
    }

    #[test]
    fn investigate_action_fails_when_actor_is_not_at_the_violation_place() {
        let mut world = new_world();
        let (_, other_place) = first_two_places(&world);
        let expected_place = first_two_places(&world).0;
        let actor = spawn_actor(&mut world, other_place);
        let violation_id = record_violation(
            &mut world,
            actor,
            ViolationKind::EntityMissing {
                entity: entity(30),
                expected_place,
            },
            1,
            5,
        );

        let (defs, handlers, _) = setup_registries();
        let def_id = defs
            .iter()
            .find(|def| def.name == "investigate")
            .map(|def| def.id)
            .unwrap();
        let affordance = Affordance {
            def_id,
            actor,
            bound_targets: vec![other_place],
            payload_override: Some(ActionPayload::Investigate(InvestigateActionPayload {
                violation_id,
            })),
            explanation: None,
        };

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([8; 32]));
        let mut next_instance_id = ActionInstanceId(1);
        let err = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            &mut next_instance_id,
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::PreconditionFailed(format!(
                "violation {} is not investigable at place {}",
                violation_id.0, other_place
            ))
        );
    }

    #[test]
    fn investigate_affordances_remain_distinct_for_same_place_violations() {
        let mut world = new_world();
        let (place, _) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place);
        let first_id = record_violation(
            &mut world,
            actor,
            ViolationKind::EntityMissing {
                entity: entity(30),
                expected_place: place,
            },
            1,
            5,
        );
        let second_id = record_violation(
            &mut world,
            actor,
            ViolationKind::SupplyDepleted {
                commodity: CommodityKind::Apple,
                source: entity(31),
                place,
            },
            2,
            5,
        );

        let (defs, handlers, _) = setup_registries();
        let payload_ids = get_affordances(
            &PerAgentBeliefView::from_world(actor, &world),
            actor,
            &defs,
            &handlers,
        )
        .into_iter()
        .filter(|affordance| defs.get(affordance.def_id).unwrap().name == "investigate")
        .filter_map(|affordance| {
            affordance
                .payload_override
                .and_then(|payload| payload.as_investigate().cloned())
                .map(|payload| payload.violation_id)
        })
        .collect::<Vec<_>>();

        assert_eq!(payload_ids, vec![first_id, second_id]);
    }

    #[test]
    fn suspected_theft_violation_is_not_exposed_as_generic_investigate_affordance() {
        let mut world = new_world();
        let (place, _) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place);
        let suspected_id = record_violation(
            &mut world,
            actor,
            ViolationKind::SuspectedTheft {
                missing_entity: entity(30),
                expected_place: place,
                suspect: None,
            },
            1,
            5,
        );

        let (defs, handlers, _) = setup_registries();
        let payload_ids = get_affordances(
            &PerAgentBeliefView::from_world(actor, &world),
            actor,
            &defs,
            &handlers,
        )
        .into_iter()
        .filter(|affordance| defs.get(affordance.def_id).unwrap().name == "investigate")
        .filter_map(|affordance| {
            affordance
                .payload_override
                .and_then(|payload| payload.as_investigate().cloned())
                .map(|payload| payload.violation_id)
        })
        .collect::<Vec<_>>();

        assert!(
            !payload_ids.contains(&suspected_id),
            "SuspectedTheft should not be exposed as a generic investigate affordance"
        );
    }

    #[test]
    fn suspected_theft_payload_fails_authoritative_investigate_validation() {
        let mut world = new_world();
        let (place, _) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place);
        let violation_id = record_violation(
            &mut world,
            actor,
            ViolationKind::SuspectedTheft {
                missing_entity: entity(30),
                expected_place: place,
                suspect: Some(entity(40)),
            },
            1,
            5,
        );

        let (defs, handlers, def_id) = setup_registries();
        let affordance = Affordance {
            def_id,
            actor,
            bound_targets: vec![place],
            payload_override: Some(ActionPayload::Investigate(InvestigateActionPayload {
                violation_id,
            })),
            explanation: None,
        };

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([21; 32]));
        let mut next_instance_id = ActionInstanceId(1);
        let err = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            &mut next_instance_id,
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::PreconditionFailed(format!(
                "violation {} is not investigable at place {}",
                violation_id.0, place
            ))
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn investigate_commit_resolves_only_selected_same_place_violation() {
        let mut world = new_world();
        let (place, _) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place);
        set_violation_profile(&mut world, actor, 2, 50);
        let first_id = record_violation(
            &mut world,
            actor,
            ViolationKind::EntityMissing {
                entity: entity(30),
                expected_place: place,
            },
            1,
            5,
        );
        let second_id = record_violation(
            &mut world,
            actor,
            ViolationKind::SupplyDepleted {
                commodity: CommodityKind::Apple,
                source: entity(31),
                place,
            },
            2,
            5,
        );

        let (defs, handlers, def_id) = setup_registries();
        let affordance = Affordance {
            def_id,
            actor,
            bound_targets: vec![place],
            payload_override: Some(ActionPayload::Investigate(InvestigateActionPayload {
                violation_id: first_id,
            })),
            explanation: None,
        };

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([13; 32]));
        let mut next_instance_id = ActionInstanceId(1);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            &mut next_instance_id,
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap();

        assert_eq!(
            tick_action(
                instance_id,
                &defs,
                &handlers,
                ActionExecutionAuthority {
                    world: &mut world,
                    event_log: &mut event_log,
                    active_actions: &mut active_actions,
                    rng: &mut rng,
                },
                ActionExecutionContext {
                    tick: Tick(3),
                    cause: CauseRef::Bootstrap,
                },
            )
            .unwrap(),
            TickOutcome::Continuing
        );

        match tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            ActionExecutionContext {
                tick: Tick(4),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap()
        {
            TickOutcome::Committed { .. } => {}
            other => panic!("expected committed investigate action, got {other:?}"),
        }

        let memory = world.get_component_violation_memory(actor).unwrap();
        let first_record = memory
            .violations
            .iter()
            .find(|record| record.id == first_id)
            .expect("first violation should remain recorded");
        let second_record = memory
            .violations
            .iter()
            .find(|record| record.id == second_id)
            .expect("second violation should remain recorded");

        assert_eq!(first_record.resolved_tick, Some(Tick(4)));
        assert_eq!(second_record.resolved_tick, None);

        let payload_ids = get_affordances(
            &PerAgentBeliefView::from_world(actor, &world),
            actor,
            &defs,
            &handlers,
        )
        .into_iter()
        .filter(|next_affordance| next_affordance.def_id == def_id)
        .filter_map(|next_affordance| {
            next_affordance
                .payload_override
                .and_then(|payload| payload.as_investigate().cloned())
                .map(|payload| payload.violation_id)
        })
        .collect::<Vec<_>>();

        assert_eq!(payload_ids, vec![second_id]);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn owner_investigating_missing_owned_entity_records_suspected_theft() {
        let mut world = new_world();
        let (place, _) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place);
        set_violation_profile(&mut world, actor, 2, 50);
        let missing = create_item_lot_at_place(&mut world, place, Some(actor));
        let violation_id = record_violation(
            &mut world,
            actor,
            ViolationKind::EntityMissing {
                entity: missing,
                expected_place: place,
            },
            1,
            5,
        );

        let (defs, handlers, _) = setup_registries();
        let affordance =
            investigate_affordance_for_id(&world, actor, &defs, &handlers, violation_id);

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([31; 32]));
        let mut next_instance_id = ActionInstanceId(1);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            &mut next_instance_id,
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap();

        assert_eq!(
            tick_action(
                instance_id,
                &defs,
                &handlers,
                ActionExecutionAuthority {
                    world: &mut world,
                    event_log: &mut event_log,
                    active_actions: &mut active_actions,
                    rng: &mut rng,
                },
                ActionExecutionContext {
                    tick: Tick(3),
                    cause: CauseRef::Bootstrap,
                },
            )
            .unwrap(),
            TickOutcome::Continuing
        );

        match tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            ActionExecutionContext {
                tick: Tick(4),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap()
        {
            TickOutcome::Committed { .. } => {}
            other => panic!("expected committed investigate action, got {other:?}"),
        }

        let store = world.get_component_agent_belief_store(actor).unwrap();
        assert!(store.social_observations.iter().any(|observation| {
            observation.detail
                == SocialObservationDetail::WitnessedAbsence {
                    missing_entity: missing,
                    expected_place: place,
                }
        }));
        assert!(store.social_observations.iter().any(|observation| {
            observation.detail
                == SocialObservationDetail::SuspectedTheft {
                    missing_entity: missing,
                    expected_place: place,
                    suspect: None,
                }
                && observation.place == place
                && observation.observed_tick == Tick(4)
                && observation.source == PerceptionSource::DirectObservation
        }));

        let memory = world.get_component_violation_memory(actor).unwrap();
        assert!(memory.violations.iter().any(|record| {
            record.id == violation_id
                && record.kind
                    == ViolationKind::EntityMissing {
                        entity: missing,
                        expected_place: place,
                    }
                && record.resolved_tick == Some(Tick(4))
        }));
        assert!(memory.violations.iter().any(|record| {
            record.id != violation_id
                && record.kind
                    == ViolationKind::SuspectedTheft {
                        missing_entity: missing,
                        expected_place: place,
                        suspect: None,
                    }
                && record.observed_tick == Tick(4)
                && record.resolved_tick.is_none()
                && record.expires_tick == Tick(54)
        }));
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn non_owner_investigating_missing_entity_does_not_record_suspected_theft() {
        let mut world = new_world();
        let (place, _) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place);
        let owner = spawn_actor(&mut world, place);
        set_violation_profile(&mut world, actor, 2, 50);
        let missing = create_item_lot_at_place(&mut world, place, Some(owner));
        mark_entity_known(&mut world, actor, missing, place);
        let violation_id = record_violation(
            &mut world,
            actor,
            ViolationKind::EntityMissing {
                entity: missing,
                expected_place: place,
            },
            1,
            5,
        );

        let (defs, handlers, _) = setup_registries();
        let affordance =
            investigate_affordance_for_id(&world, actor, &defs, &handlers, violation_id);

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([32; 32]));
        let mut next_instance_id = ActionInstanceId(1);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            &mut next_instance_id,
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap();

        assert_eq!(
            tick_action(
                instance_id,
                &defs,
                &handlers,
                ActionExecutionAuthority {
                    world: &mut world,
                    event_log: &mut event_log,
                    active_actions: &mut active_actions,
                    rng: &mut rng,
                },
                ActionExecutionContext {
                    tick: Tick(3),
                    cause: CauseRef::Bootstrap,
                },
            )
            .unwrap(),
            TickOutcome::Continuing
        );

        match tick_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            ActionExecutionContext {
                tick: Tick(4),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap()
        {
            TickOutcome::Committed { .. } => {}
            other => panic!("expected committed investigate action, got {other:?}"),
        }

        let store = world.get_component_agent_belief_store(actor).unwrap();
        assert!(store.social_observations.iter().any(|observation| {
            observation.detail
                == SocialObservationDetail::WitnessedAbsence {
                    missing_entity: missing,
                    expected_place: place,
                }
        }));
        assert!(!store.social_observations.iter().any(|observation| {
            observation.kind() == worldwake_core::SocialObservationKind::SuspectedTheft
        }));

        let memory = world.get_component_violation_memory(actor).unwrap();
        assert!(memory.violations.iter().any(|record| {
            record.id == violation_id
                && record.kind
                    == ViolationKind::EntityMissing {
                        entity: missing,
                        expected_place: place,
                    }
                && record.resolved_tick == Some(Tick(4))
        }));
        assert!(!memory.violations.iter().any(|record| {
            matches!(record.kind, ViolationKind::SuspectedTheft { .. })
        }));
    }

    #[test]
    fn investigate_action_rejects_stale_violation_payload() {
        let mut world = new_world();
        let (place, _) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place);
        let violation_id = record_violation(
            &mut world,
            actor,
            ViolationKind::EntityMissing {
                entity: entity(30),
                expected_place: place,
            },
            1,
            1,
        );

        let (defs, handlers, def_id) = setup_registries();
        let affordance = Affordance {
            def_id,
            actor,
            bound_targets: vec![place],
            payload_override: Some(ActionPayload::Investigate(InvestigateActionPayload {
                violation_id,
            })),
            explanation: None,
        };

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([12; 32]));
        let mut next_instance_id = ActionInstanceId(1);
        let err = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            &mut next_instance_id,
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::PreconditionFailed(format!(
                "actor {} has no active violation {}",
                actor, violation_id.0
            ))
        );
    }

    #[test]
    fn investigate_action_falls_back_to_three_ticks_without_profile() {
        let mut world = new_world();
        let (place, _) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place);
        let violation_id = record_violation(
            &mut world,
            actor,
            ViolationKind::EntityMissing {
                entity: entity(30),
                expected_place: place,
            },
            1,
            5,
        );

        let (defs, handlers, _) = setup_registries();
        let affordance =
            investigate_affordance_for_id(&world, actor, &defs, &handlers, violation_id);

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([9; 32]));
        let mut next_instance_id = ActionInstanceId(1);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            &mut next_instance_id,
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap();

        assert_eq!(
            active_actions
                .get(&instance_id)
                .unwrap()
                .remaining_duration
                .ticks(),
            3
        );
    }

    #[test]
    fn investigate_action_start_gate_rejects_incapacitated_actor() {
        let mut world = new_world();
        let (place, _) = first_two_places(&world);
        let actor = spawn_actor(&mut world, place);
        set_violation_profile(&mut world, actor, 2, 50);
        let violation_id = record_violation(
            &mut world,
            actor,
            ViolationKind::EntityMissing {
                entity: entity(30),
                expected_place: place,
            },
            1,
            5,
        );
        set_incapacitated(&mut world, actor);

        let (defs, handlers, def_id) = setup_registries();
        let affordance = Affordance {
            def_id,
            actor,
            bound_targets: vec![place],
            payload_override: Some(ActionPayload::Investigate(InvestigateActionPayload {
                violation_id,
            })),
            explanation: None,
        };

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([10; 32]));
        let mut next_instance_id = ActionInstanceId(1);
        let err = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            &mut next_instance_id,
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap_err();

        assert_eq!(
            err,
            ActionError::ConstraintFailed("ActorNotIncapacitated".to_string())
        );
    }

    #[test]
    fn aborting_investigate_produces_no_social_observation() {
        let mut world = new_world();
        let (place, _) = first_two_places(&world);
        let missing = entity(30);
        let actor = spawn_actor(&mut world, place);
        set_violation_profile(&mut world, actor, 2, 50);
        let violation_id = record_violation(
            &mut world,
            actor,
            ViolationKind::EntityMissing {
                entity: missing,
                expected_place: place,
            },
            1,
            5,
        );

        let (defs, handlers, _) = setup_registries();
        let affordance =
            investigate_affordance_for_id(&world, actor, &defs, &handlers, violation_id);

        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([11; 32]));
        let mut next_instance_id = ActionInstanceId(1);
        let instance_id = start_action(
            &affordance,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            &mut next_instance_id,
            ActionExecutionContext {
                tick: Tick(2),
                cause: CauseRef::Bootstrap,
            },
        )
        .unwrap();

        let before_abort = world
            .get_component_violation_memory(actor)
            .unwrap()
            .violations
            .clone();

        abort_action(
            instance_id,
            &defs,
            &handlers,
            ActionExecutionAuthority {
                world: &mut world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            ActionExecutionContext {
                tick: Tick(3),
                cause: CauseRef::Bootstrap,
            },
            ExternalAbortReason::Other,
        )
        .unwrap();

        let store = world.get_component_agent_belief_store(actor).unwrap();
        assert!(store.social_observations.is_empty());
        assert_eq!(
            world
                .get_component_violation_memory(actor)
                .unwrap()
                .violations,
            before_abort
        );
    }
}
