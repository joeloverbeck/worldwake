use std::collections::BTreeSet;

use worldwake_core::{
    ActionDefId, BodyCostPerTick, EntityId, EntityKind, EventLog, EventTag, EvidenceKind,
    ExpectationOutcome, ExpectationRecord, ExpectationState, ExpectationStore, LastSeenMemory,
    LastSeenProvenance, LastSeenRecord, SearchCondition, SearchResult, VisibilitySpec, World,
    WorldTxn, is_incapacitated,
};
use worldwake_sim::{
    AbortReason, ActionDef, ActionDefRegistry, ActionError, ActionHandler, ActionHandlerId,
    ActionHandlerRegistry, ActionInstance, ActionPayload, ActionProgress, ActionState,
    CommitOutcome, Constraint, DeterministicRng, DurationExpr, Interruptibility,
    PerAgentBeliefView, Precondition, RuntimeBeliefView, SearchPlaceActionPayload, TargetSpec,
};

pub fn register_search_place_action(
    defs: &mut ActionDefRegistry,
    handlers: &mut ActionHandlerRegistry,
) -> ActionDefId {
    let handler = handlers.register(
        ActionHandler::new(
            start_search_place,
            tick_search_place,
            commit_search_place,
            abort_search_place,
        )
        .with_affordance_payloads(enumerate_search_place_payloads)
        .with_payload_override_validator(validate_search_place_payload_override)
        .with_authoritative_payload_validator(validate_search_place_payload_authoritatively),
    );
    let id = ActionDefId(defs.len() as u32);
    defs.register(search_place_action_def(id, handler))
}

fn search_place_action_def(id: ActionDefId, handler: ActionHandlerId) -> ActionDef {
    ActionDef {
        id,
        name: "search_place".to_string(),
        domain: worldwake_core::ActionDomain::Epistemic,
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
        attention_cost: worldwake_core::Permille::ZERO,
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
        binding_strictness: worldwake_sim::BindingStrictness::AnyLegalTarget,
    }
}

fn search_place_payload(payload: &ActionPayload) -> Result<&SearchPlaceActionPayload, String> {
    payload
        .as_search_place()
        .ok_or_else(|| "search_place action requires search_place payload".to_string())
}

fn overdue_expectation_for_subject_at_place(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    subject: EntityId,
    place: EntityId,
) -> Option<ExpectationRecord> {
    view.expectation_store(actor)?
        .records
        .values()
        .find(|record| {
            record.owner == actor
                && record.subject == subject
                && record.expected_place == place
                && record.state == ExpectationState::Overdue
        })
        .copied()
}

fn enumerate_search_place_payloads(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    view: &dyn RuntimeBeliefView,
) -> Vec<ActionPayload> {
    let Some(place) = targets.first().copied() else {
        return Vec::new();
    };
    if view.violation_disposition_profile(actor).is_none() {
        return Vec::new();
    }
    let Some(store) = view.expectation_store(actor) else {
        return Vec::new();
    };

    let mut subjects = BTreeSet::new();
    for record in store.records.values() {
        if record.owner == actor
            && record.expected_place == place
            && record.state == ExpectationState::Overdue
        {
            subjects.insert(record.subject);
        }
    }

    subjects
        .into_iter()
        .map(|subject| ActionPayload::SearchPlace(SearchPlaceActionPayload { subject }))
        .collect()
}

fn validate_search_place_payload_override(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    payload: &ActionPayload,
    view: &dyn RuntimeBeliefView,
) -> bool {
    let Some(place) = targets.first().copied() else {
        return false;
    };
    if view.violation_disposition_profile(actor).is_none() {
        return false;
    }
    let Some(payload) = payload.as_search_place() else {
        return false;
    };

    overdue_expectation_for_subject_at_place(view, actor, payload.subject, place).is_some()
}

fn validate_search_place_payload_authoritatively(
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
    if world
        .get_component_violation_disposition_profile(actor)
        .is_none()
    {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} lacks ViolationDispositionProfile"
        )));
    }
    let payload = search_place_payload(payload).map_err(ActionError::PreconditionFailed)?;
    let view = PerAgentBeliefView::from_world(actor, world);
    if overdue_expectation_for_subject_at_place(&view, actor, payload.subject, place).is_none() {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {actor} cannot search place {place} for subject {}",
            payload.subject
        )));
    }
    Ok(())
}

fn start_search_place(
    _def: &ActionDef,
    instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<Option<ActionState>, ActionError> {
    let place = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    let payload =
        search_place_payload(&instance.payload).map_err(ActionError::PreconditionFailed)?;
    let view = PerAgentBeliefView::from_world(instance.actor, txn);
    if overdue_expectation_for_subject_at_place(&view, instance.actor, payload.subject, place)
        .is_none()
    {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {} cannot search place {} for subject {}",
            instance.actor, place, payload.subject
        )));
    }
    Ok(Some(ActionState::Empty))
}

#[allow(clippy::unnecessary_wraps)]
fn tick_search_place(
    _def: &ActionDef,
    _instance: &mut ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<ActionProgress, ActionError> {
    Ok(ActionProgress::Continue)
}

fn search_result_for_subject(
    txn: &WorldTxn<'_>,
    place: EntityId,
    subject: EntityId,
) -> SearchResult {
    if txn.effective_place(subject) == Some(place) {
        if txn.get_component_dead_at(subject).is_some() {
            return SearchResult::FoundDead { entity: subject };
        }
        let condition =
            txn.get_component_wound_list(subject)
                .map_or(SearchCondition::Healthy, |wounds| {
                    if txn
                        .get_component_combat_profile(subject)
                        .is_some_and(|profile| is_incapacitated(wounds, profile))
                    {
                        SearchCondition::Unconscious
                    } else if wounds.wounds.is_empty() {
                        SearchCondition::Healthy
                    } else {
                        SearchCondition::Wounded
                    }
                });
        return SearchResult::FoundAlive {
            entity: subject,
            condition,
        };
    }

    let evidence_kinds = txn
        .get_component_scene_evidence(place)
        .map(|scene| {
            scene
                .evidence
                .iter()
                .filter_map(|entry| match entry.kind {
                    EvidenceKind::BloodTrail {
                        caused_by: Some(entity),
                        ..
                    } if entity == subject => Some(entry.kind),
                    EvidenceKind::MovementTrace { entity, .. } if entity == subject => {
                        Some(entry.kind)
                    }
                    EvidenceKind::ContainerTampered { .. }
                    | EvidenceKind::BloodTrail { .. }
                    | EvidenceKind::DisturbanceMarker { .. }
                    | EvidenceKind::MovementTrace { .. } => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if evidence_kinds.is_empty() {
        SearchResult::NothingFound
    } else {
        SearchResult::FoundEvidence { evidence_kinds }
    }
}

fn remember_last_seen(memory: &mut LastSeenMemory, record: LastSeenRecord) {
    if memory
        .records
        .get(&record.subject)
        .is_some_and(|existing| existing.observed_tick >= record.observed_tick)
    {
        return;
    }
    memory.records.insert(record.subject, record);
    let capacity = usize::from(memory.capacity);
    if memory.records.len() <= capacity {
        return;
    }

    let excess = memory.records.len() - capacity;
    let mut eviction_order = memory
        .records
        .iter()
        .map(|(subject, existing)| (existing.observed_tick, *subject))
        .collect::<Vec<_>>();
    eviction_order.sort_unstable();
    for (_, subject) in eviction_order.into_iter().take(excess) {
        memory.records.remove(&subject);
    }
}

fn resolve_overdue_expectations_for_subject_at_place(
    store: &mut ExpectationStore,
    actor: EntityId,
    subject: EntityId,
    place: EntityId,
    outcome: ExpectationOutcome,
) {
    for record in store.records.values_mut() {
        if record.owner == actor
            && record.subject == subject
            && record.expected_place == place
            && record.state == ExpectationState::Overdue
        {
            record.state = ExpectationState::Resolved { outcome };
        }
    }
}

fn commit_search_place(
    _def: &ActionDef,
    instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _event_log: &EventLog,
    _rng: &mut DeterministicRng,
    txn: &mut WorldTxn<'_>,
) -> Result<CommitOutcome, ActionError> {
    let place = *instance
        .targets
        .first()
        .ok_or(ActionError::InvalidTarget(instance.actor))?;
    let payload =
        search_place_payload(&instance.payload).map_err(ActionError::PreconditionFailed)?;
    let view = PerAgentBeliefView::from_world(instance.actor, txn);
    if overdue_expectation_for_subject_at_place(&view, instance.actor, payload.subject, place)
        .is_none()
    {
        return Err(ActionError::PreconditionFailed(format!(
            "actor {} cannot search place {} for subject {} at commit",
            instance.actor, place, payload.subject
        )));
    }

    match search_result_for_subject(txn, place, payload.subject) {
        SearchResult::FoundAlive { entity, condition } => {
            let mut memory = txn
                .get_component_last_seen_memory(instance.actor)
                .cloned()
                .unwrap_or_default();
            remember_last_seen(
                &mut memory,
                LastSeenRecord {
                    subject: entity,
                    place,
                    observed_tick: txn.tick(),
                    source: instance.actor,
                    provenance: LastSeenProvenance::DirectObservation,
                },
            );
            txn.set_component_last_seen_memory(instance.actor, memory)
                .map_err(|err| ActionError::InternalError(err.to_string()))?;

            let outcome = match condition {
                SearchCondition::Healthy => ExpectationOutcome::FoundSafe { at_place: place },
                SearchCondition::Wounded | SearchCondition::Unconscious => {
                    ExpectationOutcome::FoundWounded { at_place: place }
                }
            };
            let mut store = txn
                .get_component_expectation_store(instance.actor)
                .cloned()
                .unwrap_or_default();
            resolve_overdue_expectations_for_subject_at_place(
                &mut store,
                instance.actor,
                entity,
                place,
                outcome,
            );
            txn.set_component_expectation_store(instance.actor, store)
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
        }
        SearchResult::FoundDead { entity } => {
            let mut memory = txn
                .get_component_last_seen_memory(instance.actor)
                .cloned()
                .unwrap_or_default();
            remember_last_seen(
                &mut memory,
                LastSeenRecord {
                    subject: entity,
                    place,
                    observed_tick: txn.tick(),
                    source: instance.actor,
                    provenance: LastSeenProvenance::DirectObservation,
                },
            );
            txn.set_component_last_seen_memory(instance.actor, memory)
                .map_err(|err| ActionError::InternalError(err.to_string()))?;

            let mut store = txn
                .get_component_expectation_store(instance.actor)
                .cloned()
                .unwrap_or_default();
            resolve_overdue_expectations_for_subject_at_place(
                &mut store,
                instance.actor,
                entity,
                place,
                ExpectationOutcome::FoundDead { at_place: place },
            );
            txn.set_component_expectation_store(instance.actor, store)
                .map_err(|err| ActionError::InternalError(err.to_string()))?;
        }
        SearchResult::FoundEvidence { .. } | SearchResult::NothingFound => {}
    }

    Ok(CommitOutcome::empty())
}

#[allow(clippy::unnecessary_wraps)]
fn abort_search_place(
    _def: &ActionDef,
    _instance: &ActionInstance,
    _context: &worldwake_sim::ActionExecutionContext<'_>,
    _reason: &AbortReason,
    _event_log: &EventLog,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence_support::emit_evidence;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        CauseRef, CombatProfile, ControlSource, DeadAt, ExpectationBasis, ExpectationId, Permille,
        PrototypePlace, SceneEvidence, Seed, Tick, ViolationDispositionProfile, WitnessData, Wound,
        WoundCause, WoundId, WoundList, build_prototype_world, prototype_place_entity,
    };
    use worldwake_sim::{
        ActionExecutionAuthority, ActionHandlerRegistry, ActionInstanceId, Affordance, TickOutcome,
        get_affordances, start_action, tick_action,
    };

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            worldwake_core::CauseRef::Bootstrap,
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

    fn set_violation_profile(world: &mut World, actor: EntityId) {
        let mut txn = new_txn(world, 1);
        txn.set_component_violation_disposition_profile(
            actor,
            ViolationDispositionProfile {
                investigation_duration_ticks: nz(3),
                violation_memory_retention_ticks: 12,
                investigation_motive_weight: pm(400),
                ownership_motive_bonus: pm(200),
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    fn seed_expectation(
        world: &mut World,
        actor: EntityId,
        subject: EntityId,
        expected_place: EntityId,
        state: ExpectationState,
    ) -> ExpectationId {
        let mut txn = new_txn(world, 1);
        let mut store = txn
            .get_component_expectation_store(actor)
            .cloned()
            .unwrap_or_default();
        let id = ExpectationId(store.records.len() as u64 + 1);
        store.records.insert(
            id,
            ExpectationRecord {
                id,
                owner: actor,
                subject,
                expected_place,
                deadline_tick: Tick(5),
                grace_ticks: 1,
                basis: ExpectationBasis::RoutineReturn,
                state,
                created_tick: Tick(1),
            },
        );
        txn.set_component_expectation_store(actor, store).unwrap();
        commit_txn(txn);
        id
    }

    fn kill_entity(world: &mut World, entity: EntityId, tick: u64) {
        let mut txn = new_txn(world, tick);
        txn.set_component_dead_at(
            entity,
            DeadAt {
                tick: Tick(tick),
                cause: worldwake_core::DeathCause::CombatWounds,
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    fn incapacitate_entity(world: &mut World, entity: EntityId, tick: u64) {
        let mut txn = new_txn(world, tick);
        if txn.get_component_combat_profile(entity).is_none() {
            txn.set_component_combat_profile(
                entity,
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
        }
        txn.set_component_wound_list(
            entity,
            WoundList {
                wounds: vec![Wound {
                    id: WoundId(1),
                    body_part: worldwake_core::BodyPart::Torso,
                    cause: WoundCause::Combat {
                        attacker: entity,
                        weapon: worldwake_core::CombatWeaponRef::Unarmed,
                    },
                    severity: Permille::new(1000).unwrap(),
                    inflicted_at: Tick(tick),
                    bleed_rate_per_tick: Permille::ZERO,
                }],
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    fn wound_entity(world: &mut World, entity: EntityId, tick: u64) {
        let mut txn = new_txn(world, tick);
        if txn.get_component_combat_profile(entity).is_none() {
            txn.set_component_combat_profile(
                entity,
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
        }
        txn.set_component_wound_list(
            entity,
            WoundList {
                wounds: vec![Wound {
                    id: WoundId(1),
                    body_part: worldwake_core::BodyPart::LeftArm,
                    cause: WoundCause::Combat {
                        attacker: entity,
                        weapon: worldwake_core::CombatWeaponRef::Unarmed,
                    },
                    severity: Permille::new(200).unwrap(),
                    inflicted_at: Tick(tick),
                    bleed_rate_per_tick: Permille::new(50).unwrap(),
                }],
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    fn setup_fixture() -> (World, EntityId, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::VillageSquare);
        let other_place = prototype_place_entity(PrototypePlace::ForestPath);
        let actor;
        let subject;
        {
            let mut txn = new_txn(&mut world, 1);
            actor = txn.create_agent("Searcher", ControlSource::Ai).unwrap();
            subject = txn.create_agent("Missing", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(subject, other_place).unwrap();
            commit_txn(txn);
        }
        set_violation_profile(&mut world, actor);
        (world, actor, subject, place)
    }

    fn setup_registries() -> (ActionDefRegistry, ActionHandlerRegistry, ActionDefId) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let def_id = register_search_place_action(&mut defs, &mut handlers);
        (defs, handlers, def_id)
    }

    fn run_action_to_completion(
        world: &mut World,
        defs: &ActionDefRegistry,
        handlers: &ActionHandlerRegistry,
        affordance: &Affordance,
    ) -> TickOutcome {
        let mut event_log = EventLog::new();
        let mut active_actions = BTreeMap::new();
        let mut rng = DeterministicRng::new(Seed([9; 32]));
        let mut next_instance_id = ActionInstanceId(1);
        let instance_id = start_action(
            affordance,
            defs,
            handlers,
            ActionExecutionAuthority {
                world,
                event_log: &mut event_log,
                active_actions: &mut active_actions,
                rng: &mut rng,
            },
            &mut next_instance_id,
            worldwake_sim::ActionExecutionContext::without_recipes(CauseRef::Bootstrap, Tick(1)),
        )
        .unwrap();

        for tick in 2..=6 {
            let outcome = tick_action(
                instance_id,
                defs,
                handlers,
                ActionExecutionAuthority {
                    world,
                    event_log: &mut event_log,
                    active_actions: &mut active_actions,
                    rng: &mut rng,
                },
                worldwake_sim::ActionExecutionContext::without_recipes(
                    CauseRef::Bootstrap,
                    Tick(tick),
                ),
            )
            .unwrap();
            if !matches!(outcome, TickOutcome::Continuing) {
                return outcome;
            }
        }

        panic!("action did not commit within expected search duration window");
    }

    fn search_affordance(
        world: &World,
        actor: EntityId,
        def_id: ActionDefId,
        subject: EntityId,
    ) -> Affordance {
        let view = PerAgentBeliefView::from_world(actor, world);
        let (defs, handlers, _) = setup_registries();
        get_affordances(&view, actor, &defs, &handlers)
            .into_iter()
            .find(|affordance| {
                affordance.def_id == def_id
                    && affordance.payload_override
                        == Some(ActionPayload::SearchPlace(SearchPlaceActionPayload {
                            subject,
                        }))
            })
            .expect("search_place affordance should exist")
    }

    fn last_seen_record(
        world: &World,
        actor: EntityId,
        subject: EntityId,
    ) -> Option<LastSeenRecord> {
        world
            .get_component_last_seen_memory(actor)
            .and_then(|memory| memory.records.get(&subject).copied())
    }

    #[test]
    fn search_place_affordance_enumerates_overdue_subjects_at_actor_place() {
        let (mut world, actor, subject, place) = setup_fixture();
        seed_expectation(&mut world, actor, subject, place, ExpectationState::Overdue);
        let (defs, handlers, def_id) = setup_registries();
        let view = PerAgentBeliefView::from_world(actor, &world);

        let affordances = get_affordances(&view, actor, &defs, &handlers);
        let affordance = affordances
            .iter()
            .find(|affordance| affordance.def_id == def_id)
            .expect("overdue expectation should produce search_place affordance");

        assert_eq!(affordance.bound_targets, vec![place]);
        assert_eq!(
            affordance.payload_override,
            Some(ActionPayload::SearchPlace(SearchPlaceActionPayload {
                subject
            }))
        );
    }

    #[test]
    fn search_place_commit_resolves_found_safe_and_records_last_seen() {
        let (mut world, actor, subject, place) = setup_fixture();
        seed_expectation(&mut world, actor, subject, place, ExpectationState::Overdue);
        let mut txn = new_txn(&mut world, 2);
        txn.set_ground_location(subject, place).unwrap();
        commit_txn(txn);
        let (defs, handlers, def_id) = setup_registries();
        let affordance = search_affordance(&world, actor, def_id, subject);

        let outcome = run_action_to_completion(&mut world, &defs, &handlers, &affordance);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));

        let record =
            last_seen_record(&world, actor, subject).expect("found subject should be recorded");
        assert_eq!(record.place, place);
        assert_eq!(record.observed_tick, Tick(4));
        assert_eq!(record.source, actor);
        assert_eq!(record.provenance, LastSeenProvenance::DirectObservation);

        let store = world
            .get_component_expectation_store(actor)
            .expect("expectation store should exist");
        let resolved = store.records.values().next().unwrap();
        assert_eq!(
            resolved.state,
            ExpectationState::Resolved {
                outcome: ExpectationOutcome::FoundSafe { at_place: place }
            }
        );
    }

    #[test]
    fn search_place_commit_resolves_wounded_and_incapacitated_subjects() {
        let (mut world, actor, subject, place) = setup_fixture();
        seed_expectation(&mut world, actor, subject, place, ExpectationState::Overdue);
        let mut txn = new_txn(&mut world, 2);
        txn.set_ground_location(subject, place).unwrap();
        commit_txn(txn);
        wound_entity(&mut world, subject, 2);
        let (defs, handlers, def_id) = setup_registries();
        let affordance = search_affordance(&world, actor, def_id, subject);
        let outcome = run_action_to_completion(&mut world, &defs, &handlers, &affordance);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        let store = world.get_component_expectation_store(actor).unwrap();
        let resolved = store.records.values().next().unwrap();
        assert_eq!(
            resolved.state,
            ExpectationState::Resolved {
                outcome: ExpectationOutcome::FoundWounded { at_place: place }
            }
        );

        let (mut world, actor, subject, place) = setup_fixture();
        seed_expectation(&mut world, actor, subject, place, ExpectationState::Overdue);
        let mut txn = new_txn(&mut world, 2);
        txn.set_ground_location(subject, place).unwrap();
        commit_txn(txn);
        incapacitate_entity(&mut world, subject, 2);
        let (defs, handlers, def_id) = setup_registries();
        let affordance = search_affordance(&world, actor, def_id, subject);
        let outcome = run_action_to_completion(&mut world, &defs, &handlers, &affordance);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        let store = world.get_component_expectation_store(actor).unwrap();
        let resolved = store.records.values().next().unwrap();
        assert_eq!(
            resolved.state,
            ExpectationState::Resolved {
                outcome: ExpectationOutcome::FoundWounded { at_place: place }
            }
        );
    }

    #[test]
    fn search_place_commit_resolves_found_dead() {
        let (mut world, actor, subject, place) = setup_fixture();
        seed_expectation(&mut world, actor, subject, place, ExpectationState::Overdue);
        let mut txn = new_txn(&mut world, 2);
        txn.set_ground_location(subject, place).unwrap();
        commit_txn(txn);
        kill_entity(&mut world, subject, 2);
        let (defs, handlers, def_id) = setup_registries();
        let affordance = search_affordance(&world, actor, def_id, subject);

        let outcome = run_action_to_completion(&mut world, &defs, &handlers, &affordance);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));

        let store = world.get_component_expectation_store(actor).unwrap();
        let resolved = store.records.values().next().unwrap();
        assert_eq!(
            resolved.state,
            ExpectationState::Resolved {
                outcome: ExpectationOutcome::FoundDead { at_place: place }
            }
        );
    }

    #[test]
    fn search_place_evidence_only_leaves_expectation_overdue() {
        let (mut world, actor, subject, place) = setup_fixture();
        seed_expectation(&mut world, actor, subject, place, ExpectationState::Overdue);
        let mut txn = new_txn(&mut world, 2);
        if txn.get_component_scene_evidence(place).is_none() {
            txn.set_component_scene_evidence(place, SceneEvidence::default())
                .unwrap();
        }
        commit_txn(txn);
        let mut txn = new_txn(&mut world, 2);
        emit_evidence(
            &mut txn,
            place,
            EvidenceKind::MovementTrace {
                entity: subject,
                departed_from: place,
                direction: prototype_place_entity(PrototypePlace::ForestPath),
                observed_at: Tick(2),
            },
            10,
        )
        .unwrap();
        commit_txn(txn);
        let (defs, handlers, def_id) = setup_registries();
        let affordance = search_affordance(&world, actor, def_id, subject);

        let outcome = run_action_to_completion(&mut world, &defs, &handlers, &affordance);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        let store = world.get_component_expectation_store(actor).unwrap();
        let unresolved = store.records.values().next().unwrap();
        assert_eq!(unresolved.state, ExpectationState::Overdue);
        assert!(last_seen_record(&world, actor, subject).is_none());
    }

    #[test]
    fn search_place_without_find_or_evidence_leaves_expectation_overdue() {
        let (mut world, actor, subject, place) = setup_fixture();
        seed_expectation(&mut world, actor, subject, place, ExpectationState::Overdue);
        let (defs, handlers, def_id) = setup_registries();
        let affordance = search_affordance(&world, actor, def_id, subject);

        let outcome = run_action_to_completion(&mut world, &defs, &handlers, &affordance);
        assert!(matches!(outcome, TickOutcome::Committed { .. }));
        let store = world.get_component_expectation_store(actor).unwrap();
        let unresolved = store.records.values().next().unwrap();
        assert_eq!(unresolved.state, ExpectationState::Overdue);
        assert!(last_seen_record(&world, actor, subject).is_none());
    }

    #[test]
    fn authoritative_validation_rejects_non_overdue_payload() {
        let (mut world, actor, subject, place) = setup_fixture();
        seed_expectation(&mut world, actor, subject, place, ExpectationState::Active);
        let (defs, _handlers, def_id) = setup_registries();
        let def = defs.get(def_id).unwrap();

        let error = validate_search_place_payload_authoritatively(
            def,
            &defs,
            actor,
            &[place],
            &ActionPayload::SearchPlace(SearchPlaceActionPayload { subject }),
            &world,
        )
        .expect_err("active expectations should be rejected");

        assert!(matches!(error, ActionError::PreconditionFailed(_)));
    }
}
