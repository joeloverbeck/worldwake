use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    AgentBeliefStore, BelievedActivity, BelievedInstitutionalClaim, CauseRef, ComponentDelta,
    ComponentKind, ComponentValue, EntityId, EntityKind, EventLog, EventPayload, EventTag,
    EventView, EvidenceRef, InstitutionalBeliefKey, InstitutionalClaim,
    InstitutionalKnowledgeSource, MismatchKind, NoticeTopic, ObservationContext, PendingEvent,
    PerceptionSource, Permille, RelationDelta, RelationValue, SocialObservation,
    SocialObservationDetail, SocialObservationKind, StateDelta, TheftFacts, VisibilitySpec,
    WitnessData, World, WorldTxn, build_believed_entity_state,
};
use worldwake_sim::{
    ActionDefRegistry, ActionInstance, ActionInstanceId, PerceptionTraceEvent, SystemError,
    SystemExecutionContext,
};

#[derive(Copy, Clone)]
struct DiscoveryContext {
    tick: worldwake_core::Tick,
    observer: EntityId,
    place: Option<EntityId>,
}

struct DirectLocalObservationBatch {
    place: EntityId,
    observed_snapshots: BTreeMap<EntityId, worldwake_core::BelievedEntityState>,
    noticed_missing_subjects: BTreeSet<EntityId>,
}

#[derive(Copy, Clone)]
struct NoticeInternalizationContext<'a> {
    profile: &'a worldwake_core::PerceptionProfile,
    institutional_source: InstitutionalKnowledgeSource,
}

pub fn perception_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let SystemExecutionContext {
        world,
        event_log,
        rng,
        active_actions,
        action_defs,
        politics_trace: _,
        mut perception_trace,
        tick,
        system_id: _system_id,
    } = ctx;
    let event_ids = event_log.events_at_tick(tick).to_vec();
    let mut updated_stores = BTreeMap::<EntityId, AgentBeliefStore>::new();

    let direct_local_batches = observe_passive_local_entities(
        world,
        event_log,
        tick,
        rng,
        active_actions,
        action_defs,
        &mut updated_stores,
    );
    observe_active_actions(
        world,
        tick,
        active_actions,
        action_defs,
        &direct_local_batches,
        &mut updated_stores,
    );

    for event_id in event_ids {
        let Some(record) = event_log.get(event_id).cloned() else {
            continue;
        };
        let social_observations = social_observations_for_event(world, &record, tick);
        let institutional_claims = institutional_claims_for_event(&record);

        for witness in resolve_witnesses(world, &record) {
            process_witness_event(
                world,
                event_log,
                rng,
                active_actions,
                action_defs,
                &mut updated_stores,
                perception_trace.as_deref_mut(),
                tick,
                event_id,
                &record,
                witness,
                &social_observations,
                &institutional_claims,
            );
        }
    }

    if updated_stores.is_empty() {
        return Ok(());
    }

    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        None,
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.add_tag(EventTag::System)
        .add_tag(EventTag::WorldMutation);
    for (agent, store) in updated_stores {
        txn.set_component_agent_belief_store(agent, store)
            .map_err(|error| SystemError::new(error.to_string()))?;
    }
    let _ = txn.commit(event_log);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn process_witness_event(
    world: &World,
    event_log: &mut EventLog,
    rng: &mut worldwake_sim::DeterministicRng,
    active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
    updated_stores: &mut BTreeMap<EntityId, AgentBeliefStore>,
    mut perception_trace: Option<&mut worldwake_sim::PerceptionTraceSink>,
    tick: worldwake_core::Tick,
    event_id: worldwake_core::EventId,
    record: &worldwake_core::EventRecord,
    witness: EntityId,
    social_observations: &[worldwake_core::SocialObservation],
    institutional_claims: &[(InstitutionalBeliefKey, InstitutionalClaim)],
) {
    let Some(profile) = world.get_component_perception_profile(witness).copied() else {
        return;
    };
    let effective_fidelity = effective_observation_fidelity(
        world,
        witness,
        record.place_id(),
        profile,
        active_actions,
        action_defs,
    )
    .value();
    let passed = passes_observation_check(effective_fidelity, rng);
    if !passed {
        if let Some(ref mut sink) = perception_trace {
            sink.record(PerceptionTraceEvent {
                tick,
                sequence_in_tick: 0,
                observer: witness,
                event_id,
                effective_fidelity,
                observation_passed: false,
                entity_observations: vec![],
                institutional_claims: vec![],
            });
        }
        return;
    }

    let store = updated_stores.entry(witness).or_insert_with(|| {
        world
            .get_component_agent_belief_store(witness)
            .cloned()
            .unwrap_or_default()
    });

    let mut traced_entities = Vec::new();
    for (entity, observed) in record.observed_entities() {
        let snapshot =
            observed.to_believed_entity_state(record.tick(), PerceptionSource::DirectObservation);
        record_observed_snapshot(
            event_log,
            DiscoveryContext {
                tick,
                observer: witness,
                place: record.place_id().or(snapshot.last_known_place),
            },
            store,
            *entity,
            &snapshot,
            true,
            NoticeInternalizationContext {
                profile: &profile,
                institutional_source: InstitutionalKnowledgeSource::WitnessedEvent,
            },
        );
        if perception_trace.is_some() {
            traced_entities.push(*entity);
        }
    }

    for observation in social_observations {
        store.record_social_observation(*observation);
    }

    for (key, claim) in institutional_claims {
        store.record_institutional_belief(
            *key,
            BelievedInstitutionalClaim {
                claim: *claim,
                source: InstitutionalKnowledgeSource::WitnessedEvent,
                learned_tick: record.tick(),
                learned_at: record.place_id(),
            },
            &profile,
        );
    }

    if let Some(ref mut sink) = perception_trace {
        sink.record(PerceptionTraceEvent {
            tick,
            sequence_in_tick: 0,
            observer: witness,
            event_id,
            effective_fidelity,
            observation_passed: true,
            entity_observations: traced_entities,
            institutional_claims: institutional_claims.to_vec(),
        });
    }

    store.enforce_capacity(&profile, tick);
}

fn observe_passive_local_entities(
    world: &World,
    event_log: &mut EventLog,
    tick: worldwake_core::Tick,
    rng: &mut worldwake_sim::DeterministicRng,
    active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
    updated_stores: &mut BTreeMap<EntityId, AgentBeliefStore>,
) -> BTreeMap<EntityId, DirectLocalObservationBatch> {
    let mut batches = BTreeMap::new();
    let mut colocated_entities_by_place = BTreeMap::<EntityId, Vec<EntityId>>::new();

    for (agent, _) in world.query_agent_data() {
        if world.get_component_dead_at(agent).is_some() {
            continue;
        }
        let Some(profile) = world.get_component_perception_profile(agent).copied() else {
            continue;
        };
        let Some(place) = world.effective_place(agent) else {
            continue;
        };
        let colocated_entities = colocated_entities_by_place
            .entry(place)
            .or_insert_with(|| world.entities_effectively_at(place));

        let base_store = world
            .get_component_agent_belief_store(agent)
            .cloned()
            .unwrap_or_default();
        let effective_fidelity = effective_observation_fidelity(
            world,
            agent,
            Some(place),
            profile,
            active_actions,
            action_defs,
        )
        .value();
        let Some(batch) = collect_direct_local_observation_batch(
            world,
            agent,
            place,
            colocated_entities,
            tick,
            effective_fidelity,
            rng,
            &base_store,
        ) else {
            continue;
        };

        let mut store = base_store;
        apply_direct_local_observation_batch(
            event_log,
            DiscoveryContext {
                tick,
                observer: agent,
                place: Some(batch.place),
            },
            &mut store,
            &batch,
            &profile,
        );
        let doctrine_changed = project_local_bandit_rally_doctrine(
            world,
            agent,
            batch.place,
            tick,
            &mut store,
            &profile,
        );
        if !batch.observed_snapshots.is_empty() || doctrine_changed {
            updated_stores.insert(agent, store);
        }
        batches.insert(agent, batch);
    }

    batches
}

fn project_local_bandit_rally_doctrine(
    world: &World,
    observer: EntityId,
    place: EntityId,
    tick: worldwake_core::Tick,
    store: &mut AgentBeliefStore,
    profile: &worldwake_core::PerceptionProfile,
) -> bool {
    let Some(camp) = world.get_component_bandit_camp(place) else {
        return false;
    };
    if !world.factions_of(observer).contains(&camp.faction) {
        return false;
    }
    let Some(policy) = world.get_component_bandit_faction_policy(camp.faction) else {
        return false;
    };

    let key = InstitutionalBeliefKey::FactionRallyPointOf {
        faction: camp.faction,
    };
    let belief = BelievedInstitutionalClaim {
        claim: InstitutionalClaim::FactionRallyPoint {
            faction: camp.faction,
            rally_place: policy.rally_place,
            effective_tick: tick,
        },
        source: InstitutionalKnowledgeSource::DirectObservation,
        learned_tick: tick,
        learned_at: Some(place),
    };
    let before = store
        .get_institutional_beliefs(&key)
        .map(<[BelievedInstitutionalClaim]>::to_vec);
    store.replace_institutional_belief(key, belief.clone(), profile);
    before.as_deref() != Some(std::slice::from_ref(&belief))
}

fn observe_active_actions(
    world: &World,
    tick: worldwake_core::Tick,
    active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
    direct_local_batches: &BTreeMap<EntityId, DirectLocalObservationBatch>,
    updated_stores: &mut BTreeMap<EntityId, AgentBeliefStore>,
) {
    let mut active_by_actor = BTreeMap::<EntityId, &ActionInstance>::new();
    for instance in active_actions.values() {
        active_by_actor.entry(instance.actor).or_insert(instance);
    }

    for (agent, batch) in direct_local_batches {
        let Some(base_store) = updated_stores
            .get(agent)
            .cloned()
            .or_else(|| world.get_component_agent_belief_store(*agent).cloned())
        else {
            continue;
        };

        let mut store = base_store.clone();
        let mut changed = false;

        for subject in batch.observed_snapshots.keys() {
            let next_activity = match active_by_actor.get(subject) {
                Some(instance) => {
                    let Some(def) = action_defs.get(instance.def_id) else {
                        continue;
                    };
                    Some(BelievedActivity {
                        action_domain: def.domain,
                        target: instance.targets.first().copied(),
                        observed_tick: tick,
                    })
                }
                None => None,
            };

            if store.update_believed_activity(subject, next_activity) {
                changed = true;
            }
        }

        for subject in &batch.noticed_missing_subjects {
            if store.clear_believed_activity(subject) {
                changed = true;
            }

            // Departure-direction projection: when a known entity
            // departs and has an active travel action, project the
            // travel destination as their believed place.  This is
            // lawful co-location observation (Principles 7, 15) —
            // the observer was at the same place when the departure
            // happened and can see which direction the subject went.
            if let Some(instance) = active_by_actor.get(subject) {
                let is_travel = action_defs
                    .get(instance.def_id)
                    .is_some_and(|def| def.domain == worldwake_core::ActionDomain::Travel);
                if is_travel
                    && let Some(destination) = instance.targets.first().copied()
                    && store.update_departure_projection(subject, destination, tick)
                {
                    changed = true;
                }
            }
        }

        if changed {
            updated_stores.insert(*agent, store);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_direct_local_observation_batch(
    world: &World,
    observer: EntityId,
    place: EntityId,
    colocated_entities: &[EntityId],
    tick: worldwake_core::Tick,
    observation_fidelity: u16,
    rng: &mut worldwake_sim::DeterministicRng,
    store: &AgentBeliefStore,
) -> Option<DirectLocalObservationBatch> {
    let mut observed_snapshots = BTreeMap::new();
    for &entity in colocated_entities {
        if entity == observer {
            continue;
        }
        if !passes_observation_check(observation_fidelity, rng) {
            continue;
        }
        if let Some(snapshot) =
            build_believed_entity_state(world, entity, tick, PerceptionSource::DirectObservation)
        {
            observed_snapshots.insert(entity, snapshot);
        }
    }

    if passes_observation_check(observation_fidelity, rng)
        && let Some(snapshot) =
            build_believed_entity_state(world, place, tick, PerceptionSource::DirectObservation)
    {
        observed_snapshots.insert(place, snapshot);
    }

    let observed_entities = observed_snapshots.keys().copied().collect::<BTreeSet<_>>();
    let mut noticed_missing_subjects = BTreeSet::new();
    for (subject, belief) in store.iter_known_entities() {
        if belief.last_known_place != Some(place) {
            continue;
        }
        if observed_entities.contains(subject) {
            continue;
        }
        if world.effective_place(*subject) == Some(place) {
            continue;
        }
        if !passes_observation_check(observation_fidelity, rng) {
            continue;
        }
        noticed_missing_subjects.insert(*subject);
    }

    if observed_snapshots.is_empty() && noticed_missing_subjects.is_empty() {
        return None;
    }

    Some(DirectLocalObservationBatch {
        place,
        observed_snapshots,
        noticed_missing_subjects,
    })
}

fn apply_direct_local_observation_batch(
    event_log: &mut EventLog,
    context: DiscoveryContext,
    store: &mut AgentBeliefStore,
    batch: &DirectLocalObservationBatch,
    profile: &worldwake_core::PerceptionProfile,
) {
    for (subject, snapshot) in &batch.observed_snapshots {
        record_observed_snapshot(
            event_log,
            context,
            store,
            *subject,
            snapshot,
            false,
            NoticeInternalizationContext {
                profile,
                institutional_source: InstitutionalKnowledgeSource::DirectObservation,
            },
        );
    }

    for subject in &batch.noticed_missing_subjects {
        emit_discovery_event(event_log, context, *subject, MismatchKind::EntityMissing);
    }

    if !batch.observed_snapshots.is_empty() {
        store.enforce_capacity(profile, context.tick);
    }
}

fn record_observed_snapshot(
    event_log: &mut EventLog,
    context: DiscoveryContext,
    store: &mut AgentBeliefStore,
    subject: EntityId,
    snapshot: &worldwake_core::BelievedEntityState,
    include_place_change: bool,
    notice_context: NoticeInternalizationContext<'_>,
) {
    let prior = store.get_entity(&subject).cloned();
    if let Some(prior) = prior.as_ref() {
        for mismatch in detect_observation_mismatches(prior, snapshot, include_place_change) {
            emit_discovery_event(event_log, context, subject, mismatch);
        }
    }
    internalize_notice_beliefs(
        store,
        snapshot,
        context.tick,
        context.place.or(snapshot.last_known_place),
        notice_context.profile,
        notice_context.institutional_source,
    );
    store.record_entity_snapshot_claims(
        subject,
        snapshot,
        prior.as_ref(),
        context.tick,
        Some(snapshot.observed_tick),
        &notice_context.profile.confidence_policy,
    );
}

fn internalize_notice_beliefs(
    store: &mut AgentBeliefStore,
    snapshot: &worldwake_core::BelievedEntityState,
    current_tick: worldwake_core::Tick,
    learned_at: Option<EntityId>,
    profile: &worldwake_core::PerceptionProfile,
    source: InstitutionalKnowledgeSource,
) {
    let Some(artifact) = snapshot.believed_artifact.as_ref() else {
        return;
    };
    if artifact.state != worldwake_core::ArtifactState::Active {
        return;
    }

    let Some(topic) = artifact.notice_topic else {
        return;
    };

    let claim = match topic {
        NoticeTopic::Institutional { claim } => claim,
        NoticeTopic::OfficeVacancy { office } => InstitutionalClaim::OfficeHolder {
            office,
            holder: None,
            effective_tick: current_tick,
        },
        NoticeTopic::ThreatWarning { .. } | NoticeTopic::CommodityShortage { .. } => return,
    };
    let key = institutional_belief_key(claim);
    let belief = BelievedInstitutionalClaim {
        claim,
        source,
        learned_tick: current_tick,
        learned_at,
    };

    let already_known = store
        .get_institutional_beliefs(&key)
        .is_some_and(|beliefs| beliefs.iter().any(|existing| existing == &belief));
    if !already_known {
        store.record_institutional_belief(key, belief, profile);
    }
}

fn institutional_belief_key(claim: InstitutionalClaim) -> InstitutionalBeliefKey {
    match claim {
        InstitutionalClaim::OfficeHolder { office, .. } => {
            InstitutionalBeliefKey::OfficeHolderOf { office }
        }
        InstitutionalClaim::ForceControl { office, .. } => {
            InstitutionalBeliefKey::ForceControllerOf { office }
        }
        InstitutionalClaim::FactionMembership { faction, .. } => {
            InstitutionalBeliefKey::FactionMembersOf { faction }
        }
        InstitutionalClaim::FactionRallyPoint { faction, .. } => {
            InstitutionalBeliefKey::FactionRallyPointOf { faction }
        }
        InstitutionalClaim::SupportDeclaration {
            supporter, office, ..
        } => InstitutionalBeliefKey::SupportFor { supporter, office },
        InstitutionalClaim::Accusation {
            accused,
            violation_id,
            ..
        }
        | InstitutionalClaim::Verdict {
            accused,
            violation_id,
            ..
        } => InstitutionalBeliefKey::CrimeCase {
            accused,
            violation_id,
        },
        InstitutionalClaim::MissingPersonStatus { subject, .. } => {
            InstitutionalBeliefKey::MissingPersonStatus { subject }
        }
    }
}

fn detect_observation_mismatches(
    prior: &worldwake_core::BelievedEntityState,
    observed: &worldwake_core::BelievedEntityState,
    include_place_change: bool,
) -> Vec<MismatchKind> {
    let mut mismatches = Vec::new();

    if prior.alive != observed.alive {
        mismatches.push(MismatchKind::AliveStatusChanged);
    }

    let commodities = prior
        .last_known_inventory
        .keys()
        .chain(observed.last_known_inventory.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    for commodity in commodities {
        let believed = prior
            .last_known_inventory
            .get(&commodity)
            .copied()
            .unwrap_or(worldwake_core::Quantity(0));
        let seen = observed
            .last_known_inventory
            .get(&commodity)
            .copied()
            .unwrap_or(worldwake_core::Quantity(0));
        if believed != seen {
            mismatches.push(MismatchKind::InventoryDiscrepancy {
                commodity,
                believed,
                observed: seen,
            });
        }
    }

    let source_commodities = prior
        .resource_source
        .iter()
        .chain(observed.resource_source.iter())
        .map(|source| source.commodity)
        .collect::<BTreeSet<_>>();
    for commodity in source_commodities {
        let believed = prior
            .resource_source
            .as_ref()
            .filter(|source| source.commodity == commodity)
            .map_or(worldwake_core::Quantity(0), |source| {
                source.available_quantity
            });
        let seen = observed
            .resource_source
            .as_ref()
            .filter(|source| source.commodity == commodity)
            .map_or(worldwake_core::Quantity(0), |source| {
                source.available_quantity
            });
        if believed != seen {
            mismatches.push(MismatchKind::ResourceSourceDiscrepancy {
                commodity,
                believed,
                observed: seen,
            });
        }
    }

    if include_place_change
        && let (Some(believed_place), Some(observed_place)) =
            (prior.last_known_place, observed.last_known_place)
        && believed_place != observed_place
    {
        mismatches.push(MismatchKind::PlaceChanged {
            believed_place,
            observed_place,
        });
    }

    mismatches
}

fn emit_discovery_event(
    event_log: &mut EventLog,
    context: DiscoveryContext,
    subject: EntityId,
    mismatch: MismatchKind,
) {
    let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
        tick: context.tick,
        cause: CauseRef::SystemTick(context.tick),
        actor_id: Some(context.observer),
        action_name: None,
        target_ids: vec![subject],
        evidence: vec![EvidenceRef::Mismatch {
            observer: context.observer,
            subject,
            kind: mismatch,
        }],
        place_id: context.place,
        state_deltas: Vec::new(),
        observed_entities: BTreeMap::new(),
        visibility: VisibilitySpec::ParticipantsOnly,
        witness_data: WitnessData {
            direct_witnesses: BTreeSet::from([context.observer]),
            potential_witnesses: BTreeSet::from([context.observer]),
        },
        tags: BTreeSet::from([EventTag::Discovery, EventTag::WorldMutation]),
    }));
}

fn resolve_witnesses(world: &World, record: &impl EventView) -> Vec<EntityId> {
    let candidates = match record.visibility() {
        VisibilitySpec::ParticipantsOnly => record.witness_data().direct_witnesses.clone(),
        VisibilitySpec::SamePlace | VisibilitySpec::Hidden => {
            place_witnesses(world, record.place_id())
        }
        VisibilitySpec::AdjacentPlaces { max_hops } => {
            adjacent_place_witnesses(world, record.place_id(), max_hops)
        }
        VisibilitySpec::PublicRecord => BTreeSet::new(),
    };

    candidates
        .into_iter()
        .filter(|entity| world.entity_kind(*entity) == Some(EntityKind::Agent))
        .filter(|entity| world.get_component_dead_at(*entity).is_none())
        .collect()
}

fn place_witnesses(world: &World, place_id: Option<EntityId>) -> BTreeSet<EntityId> {
    let Some(place) = place_id else {
        return BTreeSet::new();
    };
    world.entities_effectively_at(place).into_iter().collect()
}

fn adjacent_place_witnesses(
    world: &World,
    place_id: Option<EntityId>,
    max_hops: u8,
) -> BTreeSet<EntityId> {
    let Some(origin) = place_id else {
        return BTreeSet::new();
    };
    let mut places = BTreeSet::from([origin]);
    let mut frontier = vec![(origin, 0u8)];

    while let Some((place, hops)) = frontier.pop() {
        if hops >= max_hops {
            continue;
        }

        let mut neighbors = world.topology().neighbors(place);
        neighbors.reverse();
        for neighbor in neighbors {
            if places.insert(neighbor) {
                frontier.push((neighbor, hops + 1));
            }
        }
    }

    places
        .into_iter()
        .flat_map(|place| world.entities_effectively_at(place))
        .collect()
}

fn passes_observation_check(fidelity: u16, rng: &mut worldwake_sim::DeterministicRng) -> bool {
    match fidelity {
        0 => false,
        1000 => true,
        value => rng.next_range(0, 1000) < u32::from(value),
    }
}

fn fatigue_observation_penalty(fatigue: Permille) -> Permille {
    if fatigue.value() <= 500 {
        Permille::ZERO
    } else {
        let penalty = (u32::from(fatigue.value()) - 500) * 300 / 500;
        Permille::new_unchecked(penalty as u16)
    }
}

fn active_attention_cost(
    agent: EntityId,
    active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
) -> Permille {
    for instance in active_actions.values() {
        if instance.actor == agent
            && let Some(def) = action_defs.get(instance.def_id)
        {
            return def.attention_cost;
        }
    }
    Permille::ZERO
}

fn effective_observation_fidelity(
    world: &World,
    observer: EntityId,
    place: Option<EntityId>,
    profile: worldwake_core::PerceptionProfile,
    active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
) -> Permille {
    let fatigue_penalty = world
        .get_component_homeostatic_needs(observer)
        .map_or(Permille::ZERO, |needs| {
            fatigue_observation_penalty(needs.fatigue)
        });
    let occupancy_penalty = active_attention_cost(observer, active_actions, action_defs);
    let place_concealment = place.and_then(|place| {
        world
            .get_component_place_visibility_profile(place)
            .map(|profile| profile.base_concealment)
    });

    ObservationContext {
        base_fidelity: profile.observation_fidelity,
        fatigue_penalty,
        occupancy_penalty,
        place_concealment: place_concealment.unwrap_or(Permille::ZERO),
        entity_concealment: Permille::ZERO,
    }
    .effective_fidelity()
}

fn social_observations_for_event(
    world: &World,
    record: &impl EventView,
    tick: worldwake_core::Tick,
) -> Vec<SocialObservation> {
    let Some(place) = record.place_id() else {
        return Vec::new();
    };
    let Some(actor) = record
        .actor_id()
        .filter(|actor| world.entity_kind(*actor) == Some(EntityKind::Agent))
    else {
        return Vec::new();
    };
    if let Some(observation) =
        suspected_theft_observation_for_event(world, record, tick, actor, place)
    {
        return vec![observation];
    }
    let targets = record
        .target_ids()
        .iter()
        .copied()
        .filter(|target| world.entity_kind(*target) == Some(EntityKind::Agent))
        .collect::<Vec<_>>();

    let Some(kind) = social_kind(record) else {
        return Vec::new();
    };

    targets
        .into_iter()
        .map(|target| SocialObservation {
            detail: match kind {
                SocialObservationKind::WitnessedCooperation => {
                    SocialObservationDetail::WitnessedCooperation {
                        actor,
                        counterpart: target,
                    }
                }
                SocialObservationKind::WitnessedConflict => {
                    SocialObservationDetail::WitnessedConflict { actor, target }
                }
                SocialObservationKind::WitnessedObligation => {
                    SocialObservationDetail::WitnessedObligation { actor, target }
                }
                SocialObservationKind::WitnessedTelling => {
                    SocialObservationDetail::WitnessedTelling {
                        speaker: actor,
                        listener: target,
                    }
                }
                SocialObservationKind::CoPresence
                | SocialObservationKind::WitnessedAbsence
                | SocialObservationKind::SuspectedTheft => {
                    unreachable!(
                        "perception event mapping only constructs actor-target social detail"
                    )
                }
            },
            place,
            observed_tick: tick,
            source: PerceptionSource::DirectObservation,
        })
        .collect()
}

fn suspected_theft_observation_for_event(
    world: &World,
    record: &impl EventView,
    tick: worldwake_core::Tick,
    actor: EntityId,
    place: EntityId,
) -> Option<SocialObservation> {
    if !record.tags().contains(&EventTag::Crime) || !record.tags().contains(&EventTag::Transfer) {
        return None;
    }

    let stolen_lot = record.target_ids().iter().copied().find(|target| {
        world.entity_kind(*target) == Some(EntityKind::ItemLot)
            && world.possessor_of(*target) == Some(actor)
    })?;
    world.owner_of(stolen_lot).filter(|owner| *owner != actor)?;
    let lot = world.get_component_item_lot(stolen_lot)?;

    Some(SocialObservation {
        detail: SocialObservationDetail::SuspectedTheft {
            theft: TheftFacts {
                missing_entity: stolen_lot,
                expected_place: place,
                commodity: lot.commodity,
                quantity: lot.quantity,
            },
            suspect: Some(actor),
        },
        place,
        observed_tick: tick,
        source: PerceptionSource::DirectObservation,
    })
}

fn social_kind(record: &impl EventView) -> Option<SocialObservationKind> {
    if record.tags().contains(&EventTag::Coercion) || record.tags().contains(&EventTag::Combat) {
        return Some(SocialObservationKind::WitnessedConflict);
    }
    if record.tags().contains(&EventTag::Political) || record.tags().contains(&EventTag::Trade) {
        return Some(SocialObservationKind::WitnessedCooperation);
    }
    if record.tags().contains(&EventTag::Social) && record.tags().contains(&EventTag::Transfer) {
        return Some(SocialObservationKind::WitnessedObligation);
    }
    if record.tags().contains(&EventTag::Social) {
        return Some(SocialObservationKind::WitnessedTelling);
    }
    None
}

fn institutional_claims_for_event(
    record: &impl EventView,
) -> Vec<(InstitutionalBeliefKey, InstitutionalClaim)> {
    if !record.tags().contains(&EventTag::Political) {
        return Vec::new();
    }

    let mut normalized = BTreeMap::new();
    for (key, claim) in force_control_claims_for_event(record) {
        normalized.insert(key, claim);
    }
    for delta in record.state_deltas() {
        let Some((key, claim)) = institutional_claim_from_delta(delta, record.tick()) else {
            continue;
        };
        normalized.insert(key, claim);
    }

    normalized.into_iter().collect()
}

fn force_control_claims_for_event(
    record: &impl EventView,
) -> Vec<(InstitutionalBeliefKey, InstitutionalClaim)> {
    enum ControllerProjection {
        Unspecified,
        None,
        Some(EntityId),
    }

    struct Projection {
        controller: ControllerProjection,
        contested: Option<bool>,
    }

    impl Default for Projection {
        fn default() -> Self {
            Self {
                controller: ControllerProjection::Unspecified,
                contested: None,
            }
        }
    }

    let mut by_office = BTreeMap::<EntityId, Projection>::new();

    for delta in record.state_deltas() {
        match delta {
            StateDelta::Relation(RelationDelta::Added {
                relation: RelationValue::OfficeController { office, controller },
                ..
            }) => {
                by_office.entry(*office).or_default().controller =
                    ControllerProjection::Some(*controller);
            }
            StateDelta::Relation(RelationDelta::Removed {
                relation: RelationValue::OfficeController { office, .. },
                ..
            }) => {
                by_office.entry(*office).or_default().controller = ControllerProjection::None;
            }
            StateDelta::Component(ComponentDelta::Set {
                entity,
                component_kind: ComponentKind::OfficeForceState,
                after: ComponentValue::OfficeForceState(state),
                ..
            }) => {
                by_office.entry(*entity).or_default().contested =
                    Some(state.contested_since.is_some());
            }
            _ => {}
        }
    }

    by_office
        .into_iter()
        .map(|(office, projection)| {
            let controller = match projection.controller {
                ControllerProjection::Unspecified | ControllerProjection::None => None,
                ControllerProjection::Some(controller) => Some(controller),
            };
            let contested = projection.contested.unwrap_or(false);
            (
                InstitutionalBeliefKey::ForceControllerOf { office },
                InstitutionalClaim::ForceControl {
                    office,
                    controller,
                    contested,
                    effective_tick: record.tick(),
                },
            )
        })
        .collect()
}

fn institutional_claim_from_delta(
    delta: &StateDelta,
    effective_tick: worldwake_core::Tick,
) -> Option<(InstitutionalBeliefKey, InstitutionalClaim)> {
    let StateDelta::Relation(relation_delta) = delta else {
        return None;
    };

    match relation_delta {
        RelationDelta::Added {
            relation: RelationValue::OfficeHolder { office, holder },
            ..
        } => Some((
            InstitutionalBeliefKey::OfficeHolderOf { office: *office },
            InstitutionalClaim::OfficeHolder {
                office: *office,
                holder: Some(*holder),
                effective_tick,
            },
        )),
        RelationDelta::Removed {
            relation: RelationValue::OfficeHolder { office, .. },
            ..
        } => Some((
            InstitutionalBeliefKey::OfficeHolderOf { office: *office },
            InstitutionalClaim::OfficeHolder {
                office: *office,
                holder: None,
                effective_tick,
            },
        )),
        RelationDelta::Added {
            relation:
                RelationValue::SupportDeclaration {
                    supporter,
                    office,
                    candidate,
                },
            ..
        } => Some((
            InstitutionalBeliefKey::SupportFor {
                supporter: *supporter,
                office: *office,
            },
            InstitutionalClaim::SupportDeclaration {
                office: *office,
                supporter: *supporter,
                candidate: Some(*candidate),
                effective_tick,
            },
        )),
        RelationDelta::Removed {
            relation:
                RelationValue::SupportDeclaration {
                    supporter, office, ..
                },
            ..
        } => Some((
            InstitutionalBeliefKey::SupportFor {
                supporter: *supporter,
                office: *office,
            },
            InstitutionalClaim::SupportDeclaration {
                office: *office,
                supporter: *supporter,
                candidate: None,
                effective_tick,
            },
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        active_attention_cost, fatigue_observation_penalty, perception_system, resolve_witnesses,
        social_kind, social_observations_for_event,
    };
    use crate::dispatch_table;
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        ActionDefId, ActionDomain, AgentBeliefStore, ArtifactHeader, ArtifactKind, ArtifactState,
        BanditCamp, BanditFactionPolicy, BeliefConfidencePolicy, BelievedActivity,
        BelievedContentionState, BelievedEntityState, BelievedEvidenceEntry, BelievedEvidenceState,
        BountyTarget, BountyTerms, CauseRef, CommodityKind, ComponentDelta, ComponentKind,
        ComponentValue, Container, ContentionGrant, ContentionQueue, ContentionWaiter,
        ControlSource, DeadAt, DisturbanceKind, EntityBeliefAspect, EntityKind, EventLog,
        EventPayload, EventTag, EventView, EvidenceKind, EvidenceRef, HomeostaticNeeds,
        InstitutionalBeliefKey, InstitutionalClaim, InstitutionalKnowledgeSource, LoadUnits,
        MismatchKind, NoticeContent, NoticeTopic, ObservedEntitySnapshot, OfficeForceState,
        PendingEvent, PerceptionProfile, PerceptionSource, Permille, PlaceVisibilityProfile,
        ProductionOutputOwner, ProductionOutputOwnershipPolicy, ProofRequirement, PrototypePlace,
        Quantity, RelationDelta, RelationKind, RelationValue, ResourceSource, RewardSource,
        SaleListing, SceneEvidence, Seed, SocialObservationDetail, SocialObservationKind,
        StateDelta, StockAssignment, StockAssignmentKind, TheftFacts, Tick, VisibilitySpec,
        WitnessData, WorkstationMarker, WorkstationTag, World, WorldTxn,
        build_observed_entity_snapshot, build_prototype_world, prototype_place_entity,
    };
    use worldwake_sim::{
        ActionDef, ActionDefRegistry, ActionDuration, ActionHandlerId, ActionInstance,
        ActionInstanceId, ActionPayload, ActionState, ActionStatus, Constraint, DeterministicRng,
        DurationExpr, Interruptibility, Precondition, ReservationReq, SystemExecutionContext,
        SystemId, TargetSpec,
    };

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

    fn profile(fidelity: u16) -> PerceptionProfile {
        PerceptionProfile {
            entity_memory_capacity: 8,
            entity_claim_capacity: 8,
            memory_retention_ticks: 32,
            observation_fidelity: Permille::new(fidelity).unwrap(),
            confidence_policy: BeliefConfidencePolicy::default(),
            institutional_memory_capacity: 20,
            consultation_speed_factor: Permille::new(500).unwrap(),
            contradiction_tolerance: Permille::new(300).unwrap(),
        }
    }

    fn discovery_records(event_log: &EventLog) -> Vec<&worldwake_core::EventRecord> {
        event_log
            .events_by_tag(EventTag::Discovery)
            .iter()
            .filter_map(|event_id| event_log.get(*event_id))
            .collect()
    }

    fn observed_from_world(
        world: &World,
        entities: &[worldwake_core::EntityId],
    ) -> BTreeMap<worldwake_core::EntityId, ObservedEntitySnapshot> {
        entities
            .iter()
            .filter_map(|entity| {
                build_observed_entity_snapshot(world, *entity).map(|snapshot| (*entity, snapshot))
            })
            .collect()
    }

    fn observed_snapshot(
        place: Option<worldwake_core::EntityId>,
        bread: u32,
    ) -> ObservedEntitySnapshot {
        let mut inventory = BTreeMap::new();
        if bread > 0 {
            inventory.insert(CommodityKind::Bread, Quantity(bread));
        }
        ObservedEntitySnapshot {
            believed_kind: None,
            last_known_place: place,
            last_known_inventory: inventory,
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            courage: None,
            artifact_state: None,
            contention_state: None,
            evidence_state: None,
        }
    }

    fn emit_political_relation_event(
        event_log: &mut EventLog,
        tick: Tick,
        place: worldwake_core::EntityId,
        actor: Option<worldwake_core::EntityId>,
        targets: Vec<worldwake_core::EntityId>,
        deltas: Vec<StateDelta>,
    ) {
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick,
            cause: CauseRef::Bootstrap,
            actor_id: actor,
            action_name: None,
            target_ids: targets,
            evidence: Vec::new(),
            place_id: Some(place),
            state_deltas: deltas,
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData::default(),
            tags: BTreeSet::from([EventTag::Political, EventTag::WorldMutation]),
        }));
    }

    fn register_test_action(
        defs: &mut ActionDefRegistry,
        domain: ActionDomain,
        name: &str,
    ) -> ActionDefId {
        let id = ActionDefId(defs.len() as u32);
        defs.register(ActionDef {
            id,
            name: name.to_string(),
            domain,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: vec![TargetSpec::SpecificEntity(worldwake_core::EntityId {
                slot: 999,
                generation: 1,
            })],
            preconditions: vec![Precondition::TargetExists(0)],
            reservation_requirements: vec![ReservationReq { target_index: 0 }],
            duration: DurationExpr::Fixed(NonZeroU32::new(3).unwrap()),
            body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![Precondition::TargetExists(0)],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::from([EventTag::ActionStarted]),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
        })
    }

    fn active_instance(
        def_id: ActionDefId,
        actor: worldwake_core::EntityId,
        targets: Vec<worldwake_core::EntityId>,
    ) -> ActionInstance {
        ActionInstance {
            instance_id: ActionInstanceId(0),
            def_id,
            payload: ActionPayload::None,
            actor,
            targets,
            start_tick: Tick(3),
            remaining_duration: ActionDuration::new(2),
            status: ActionStatus::Active,
            reservation_ids: Vec::new(),
            local_state: Some(ActionState::Empty),
            body_cost_override: None,
        }
    }

    fn stale_activity_belief(
        place: worldwake_core::EntityId,
        domain: ActionDomain,
        target: Option<worldwake_core::EntityId>,
    ) -> BelievedEntityState {
        let mut state = observed_snapshot(Some(place), 0)
            .to_believed_entity_state(Tick(2), PerceptionSource::DirectObservation);
        state.believed_activity = Some(BelievedActivity {
            action_domain: domain,
            target,
            observed_tick: Tick(2),
        });
        state
    }

    #[test]
    fn fatigue_observation_penalty_matches_spec_thresholds() {
        assert_eq!(fatigue_observation_penalty(Permille::ZERO), Permille::ZERO);
        assert_eq!(
            fatigue_observation_penalty(Permille::new(500).unwrap()),
            Permille::ZERO
        );
        assert_eq!(
            fatigue_observation_penalty(Permille::new(1000).unwrap()),
            Permille::new(300).unwrap()
        );
    }

    #[test]
    fn active_attention_cost_returns_zero_without_active_action() {
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        assert_eq!(
            active_attention_cost(
                worldwake_core::EntityId {
                    slot: 1,
                    generation: 0,
                },
                &active_actions,
                &action_defs,
            ),
            Permille::ZERO
        );
    }

    #[test]
    fn active_attention_cost_returns_registered_action_attention_cost() {
        let actor = worldwake_core::EntityId {
            slot: 3,
            generation: 0,
        };
        let target = worldwake_core::EntityId {
            slot: 4,
            generation: 0,
        };
        let mut action_defs = ActionDefRegistry::new();
        let def_id = register_test_action(&mut action_defs, ActionDomain::Combat, "fight");
        action_defs.get(def_id).expect("test action should exist");
        let active_actions = BTreeMap::from([(
            ActionInstanceId(0),
            active_instance(def_id, actor, vec![target]),
        )]);
        let expected = Permille::new(650).unwrap();

        let mut updated_defs = ActionDefRegistry::new();
        updated_defs.register(ActionDef {
            attention_cost: expected,
            ..action_defs.get(def_id).unwrap().clone()
        });

        assert_eq!(
            active_attention_cost(actor, &active_actions, &updated_defs),
            expected
        );
    }

    #[test]
    fn passive_perception_respects_place_visibility_profile() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();

        let (observer, subject) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let subject = txn.create_agent("Subject", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(subject, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            txn.set_component_place_visibility_profile(
                place,
                PlaceVisibilityProfile {
                    base_concealment: Permille::new(1000).unwrap(),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, subject)
        };

        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([0x90; 32]));

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &BTreeMap::new(),
            action_defs: &ActionDefRegistry::new(),
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world.get_component_agent_belief_store(observer).unwrap();
        assert!(
            beliefs.get_entity(&subject).is_none(),
            "full place concealment should reduce effective fidelity to zero"
        );
    }

    #[test]
    fn co_located_active_action_sets_believed_activity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, actor, target) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let actor = txn.create_agent("Actor", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            for entity in [observer, actor, target] {
                txn.set_ground_location(entity, place).unwrap();
            }
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, actor, target)
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([0x11; 32]));
        let mut action_defs = ActionDefRegistry::new();
        let def_id = register_test_action(&mut action_defs, ActionDomain::Production, "harvest");
        let active_actions = BTreeMap::from([(
            ActionInstanceId(0),
            active_instance(def_id, actor, vec![target]),
        )]);

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        let believed = beliefs
            .get_entity(&actor)
            .expect("colocated actor should be directly observed");
        assert_eq!(
            believed.believed_activity,
            Some(BelievedActivity {
                action_domain: ActionDomain::Production,
                target: Some(target),
                observed_tick: Tick(3),
            })
        );
    }

    #[test]
    fn active_action_respects_observation_fidelity_gate() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, actor, target) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let actor = txn.create_agent("Actor", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            for entity in [observer, actor, target] {
                txn.set_ground_location(entity, place).unwrap();
            }
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(0))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, actor, target)
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([0x12; 32]));
        let mut action_defs = ActionDefRegistry::new();
        let def_id = register_test_action(&mut action_defs, ActionDomain::Trade, "trade");
        let active_actions = BTreeMap::from([(
            ActionInstanceId(0),
            active_instance(def_id, actor, vec![target]),
        )]);

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should retain an empty belief store");
        assert!(
            beliefs.get_entity(&actor).is_none(),
            "fidelity zero should prevent direct observation and activity projection"
        );
    }

    #[test]
    fn agent_observes_place_without_scene_evidence() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::VillageSquare);
        let observer = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            observer
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([0x20; 32]));

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &BTreeMap::new(),
            action_defs: &ActionDefRegistry::new(),
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let belief = world
            .get_component_agent_belief_store(observer)
            .unwrap()
            .get_entity(&place)
            .expect("observer should believe current place without scene evidence");
        assert_eq!(belief.last_known_place, None);
        assert_eq!(belief.believed_evidence, None);
        assert_eq!(belief.observed_tick, Tick(3));
        assert_eq!(belief.source, PerceptionSource::DirectObservation);
    }

    #[test]
    fn passive_perception_projects_scene_evidence_for_current_place() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::VillageSquare);
        let observer = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            txn.set_component_scene_evidence(
                place,
                SceneEvidence {
                    evidence: vec![worldwake_core::EvidenceEntry {
                        id: worldwake_core::EvidenceEntryId(0),
                        kind: EvidenceKind::DisturbanceMarker {
                            place,
                            kind: DisturbanceKind::WildernessRelief,
                            created_at: Tick(2),
                        },
                        created_at: Tick(2),
                        decay_ticks: 50,
                    }],
                    next_entry_id: 1,
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            observer
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([0x21; 32]));

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &BTreeMap::new(),
            action_defs: &ActionDefRegistry::new(),
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let belief = world
            .get_component_agent_belief_store(observer)
            .unwrap()
            .get_entity(&place)
            .expect("observer should believe current place evidence");
        assert_eq!(
            belief.believed_evidence,
            Some(BelievedEvidenceState {
                entries: vec![BelievedEvidenceEntry {
                    kind: EvidenceKind::DisturbanceMarker {
                        place,
                        kind: DisturbanceKind::WildernessRelief,
                        created_at: Tick(2),
                    },
                    freshness: Tick(2),
                }],
                observed_tick: Tick(3),
            })
        );
    }

    #[test]
    fn passive_perception_clears_stale_place_evidence_after_reobservation() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::VillageSquare);
        let observer = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            txn.set_component_scene_evidence(
                place,
                SceneEvidence {
                    evidence: vec![worldwake_core::EvidenceEntry {
                        id: worldwake_core::EvidenceEntryId(0),
                        kind: EvidenceKind::DisturbanceMarker {
                            place,
                            kind: DisturbanceKind::WildernessRelief,
                            created_at: Tick(2),
                        },
                        created_at: Tick(2),
                        decay_ticks: 50,
                    }],
                    next_entry_id: 1,
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            observer
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([0x22; 32]));

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &BTreeMap::new(),
            action_defs: &ActionDefRegistry::new(),
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        {
            let mut txn = new_txn(&mut world, 4);
            txn.clear_component_scene_evidence(place).unwrap();
            let _ = txn.commit(&mut event_log);
        }

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &BTreeMap::new(),
            action_defs: &ActionDefRegistry::new(),
            politics_trace: None,
            perception_trace: None,
            tick: Tick(5),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let belief = world
            .get_component_agent_belief_store(observer)
            .unwrap()
            .get_entity(&place)
            .expect("observer should still know current place");
        assert_eq!(belief.observed_tick, Tick(5));
        assert_eq!(belief.believed_evidence, None);
    }

    #[test]
    fn idle_colocated_subject_clears_believed_activity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, actor) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let actor = txn.create_agent("Actor", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            let mut beliefs = AgentBeliefStore::new();
            beliefs.update_entity(
                actor,
                stale_activity_belief(place, ActionDomain::Production, None),
            );
            txn.set_component_agent_belief_store(observer, beliefs)
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, actor)
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([0x13; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        let believed = beliefs
            .get_entity(&actor)
            .expect("colocated subject should still be known");
        assert_eq!(believed.believed_activity, None);
        assert_eq!(believed.last_known_place, Some(place));
    }

    #[test]
    fn departed_subject_clears_believed_activity_when_no_longer_colocated() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let place = places[0];
        let other_place = *places.get(1).unwrap_or(&place);
        assert_ne!(
            place, other_place,
            "prototype world needs at least two places for departure coverage"
        );
        let (observer, actor) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let actor = txn.create_agent("Actor", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(actor, other_place).unwrap();
            let mut beliefs = AgentBeliefStore::new();
            beliefs.update_entity(
                actor,
                stale_activity_belief(place, ActionDomain::Travel, None),
            );
            txn.set_component_agent_belief_store(observer, beliefs)
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, actor)
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([0x14; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        let believed = beliefs
            .get_entity(&actor)
            .expect("departed subject should remain known");
        assert_eq!(believed.believed_activity, None);
        assert_eq!(believed.last_known_place, Some(place));
    }

    #[test]
    fn departed_subject_missing_cycle_clears_activity_and_emits_discovery() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let place = places[0];
        let other_place = *places.get(1).unwrap_or(&place);
        assert_ne!(
            place, other_place,
            "prototype world needs at least two places for departure coverage"
        );
        let (observer, actor) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let actor = txn.create_agent("Actor", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(actor, other_place).unwrap();
            let mut beliefs = AgentBeliefStore::new();
            beliefs.update_entity(
                actor,
                stale_activity_belief(place, ActionDomain::Travel, None),
            );
            txn.set_component_agent_belief_store(observer, beliefs)
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, actor)
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([0x15; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        let believed = beliefs
            .get_entity(&actor)
            .expect("departed subject should remain known");
        assert_eq!(believed.believed_activity, None);
        assert_eq!(believed.last_known_place, Some(place));
        assert_eq!(
            discovery_records(&event_log)[0].evidence(),
            vec![EvidenceRef::Mismatch {
                observer,
                subject: actor,
                kind: MismatchKind::EntityMissing,
            }]
        );
    }

    #[test]
    fn missing_displayed_facility_stock_emits_entity_missing_discovery() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let place = places[0];
        let other_place = *places.get(1).unwrap_or(&place);
        let (observer, stock_lot) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Merchant", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();

            let (facility, _stock_container, display_container) = txn
                .create_merchant_facility(place, observer, LoadUnits(200), Some(LoadUnits(100)))
                .unwrap();
            let display_container = display_container.expect("display container should exist");
            let stock_lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(2))
                .unwrap();
            txn.set_owner(stock_lot, observer).unwrap();
            txn.put_into_container(stock_lot, display_container)
                .unwrap();
            txn.set_component_stock_assignment(
                stock_lot,
                StockAssignment {
                    facility,
                    kind: StockAssignmentKind::Displayed,
                },
            )
            .unwrap();
            txn.set_component_sale_listing(stock_lot, SaleListing { listed_at: Tick(1) })
                .unwrap();

            let mut store = AgentBeliefStore::new();
            store.update_entity(
                stock_lot,
                observed_snapshot(Some(place), 2)
                    .to_believed_entity_state(Tick(1), PerceptionSource::DirectObservation),
            );
            txn.set_component_agent_belief_store(observer, store)
                .unwrap();

            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, stock_lot)
        };

        {
            let mut txn = new_txn(&mut world, 2);
            txn.set_ground_location(stock_lot, other_place).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([0x52; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert_eq!(
            discovery_records(&event_log)[0].evidence(),
            vec![EvidenceRef::Mismatch {
                observer,
                subject: stock_lot,
                kind: MismatchKind::EntityMissing,
            }]
        );
    }

    #[test]
    fn departed_subject_with_active_travel_projects_destination_as_believed_place() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let origin = places[0];
        let destination = *places.get(1).unwrap_or(&origin);
        assert_ne!(
            origin, destination,
            "prototype world needs at least two places for departure coverage"
        );
        let (observer, traveler) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let traveler = txn.create_agent("Traveler", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, origin).unwrap();
            // Place traveler at origin first, then put in transit.
            txn.set_ground_location(traveler, origin).unwrap();
            txn.set_in_transit(traveler).unwrap();
            // Observer believed the traveler was at origin.
            let mut beliefs = AgentBeliefStore::new();
            beliefs.update_entity(
                traveler,
                stale_activity_belief(origin, ActionDomain::Travel, Some(destination)),
            );
            txn.set_component_agent_belief_store(observer, beliefs)
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, traveler)
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([0x40; 32]));
        let mut action_defs = ActionDefRegistry::new();
        let travel_def = register_test_action(&mut action_defs, ActionDomain::Travel, "travel");
        let active_actions = BTreeMap::from([(
            ActionInstanceId(0),
            active_instance(travel_def, traveler, vec![destination]),
        )]);

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        let believed = beliefs
            .get_entity(&traveler)
            .expect("departed subject should remain known");
        // Departure-direction projection: the observer should now
        // believe the traveler is at the travel destination, not the
        // old origin — co-located agents observe which direction an
        // entity departs (Principles 7, 15).
        assert_eq!(
            believed.last_known_place,
            Some(destination),
            "departure-direction projection should update last_known_place to travel destination"
        );
        assert_eq!(believed.observed_tick, Tick(3));
        assert_eq!(believed.source, PerceptionSource::DirectObservation);
    }

    #[test]
    fn active_action_does_not_cross_place_boundaries_or_self_observe() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let place = places[0];
        let remote_place = *places.get(1).unwrap_or(&place);
        assert_ne!(
            place, remote_place,
            "prototype world needs at least two places for locality coverage"
        );
        let (observer, remote_actor, target) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let remote_actor = txn.create_agent("RemoteActor", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(remote_actor, remote_place).unwrap();
            txn.set_ground_location(target, remote_place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, remote_actor, target)
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([0x15; 32]));
        let mut action_defs = ActionDefRegistry::new();
        let def_id = register_test_action(&mut action_defs, ActionDomain::Travel, "travel");
        let active_actions = BTreeMap::from([
            (
                ActionInstanceId(0),
                active_instance(def_id, observer, vec![place]),
            ),
            (
                ActionInstanceId(1),
                active_instance(def_id, remote_actor, vec![target]),
            ),
        ]);

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        assert!(
            beliefs.get_entity(&observer).is_none(),
            "observers must not project their own active action into self belief entries"
        );
        assert!(
            beliefs.get_entity(&remote_actor).is_none(),
            "remote active actions must not project across place boundaries"
        );
    }

    #[test]
    fn same_place_event_updates_witness_belief_store() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, target) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(target, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            let mut observer_profile = profile(1000);
            observer_profile.entity_memory_capacity = 16;
            observer_profile.entity_claim_capacity = 16;
            txn.set_component_perception_profile(observer, observer_profile)
                .unwrap();
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(2))
                .unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_possessor(bread, target).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, target)
        };
        let mut event_log = EventLog::new();
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(3),
            cause: CauseRef::Bootstrap,
            actor_id: Some(target),
            action_name: None,
            target_ids: vec![target],
            evidence: Vec::new(),
            place_id: Some(place),
            state_deltas: Vec::new(),
            observed_entities: observed_from_world(&world, &[target]),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData::default(),
            tags: BTreeSet::new(),
        }));
        let mut rng = DeterministicRng::new(Seed([7; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        let believed = beliefs
            .get_entity(&target)
            .expect("same-place witness should gain a belief snapshot");
        assert_eq!(
            believed.last_known_inventory.get(&CommodityKind::Bread),
            Some(&Quantity(2))
        );
        assert!(believed.alive);
        assert_eq!(believed.observed_tick, Tick(3));
        assert_eq!(believed.source, PerceptionSource::DirectObservation);
        let claims = beliefs
            .get_entity_claims(&target)
            .expect("same-place witness should gain claim-backed entity memory");
        assert!(
            claims
                .iter()
                .any(|claim| claim.aspect == EntityBeliefAspect::Inventory(CommodityKind::Bread))
        );
        assert!(
            claims
                .iter()
                .any(|claim| claim.aspect == EntityBeliefAspect::Alive)
        );
    }

    #[test]
    fn passive_local_observation_emits_claims_and_derives_summary() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, target) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(target, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            let mut observer_profile = profile(1000);
            observer_profile.entity_memory_capacity = 16;
            observer_profile.entity_claim_capacity = 16;
            txn.set_component_perception_profile(observer, observer_profile)
                .unwrap();
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(2))
                .unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_possessor(bread, target).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, target)
        };

        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([0x44; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(2),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world.get_component_agent_belief_store(observer).unwrap();
        let believed = beliefs
            .get_entity(&target)
            .expect("passive local observation should still project a summary");
        assert_eq!(
            believed.last_known_inventory.get(&CommodityKind::Bread),
            Some(&Quantity(2))
        );
        assert!(believed.alive);
        let claims = beliefs
            .get_entity_claims(&target)
            .expect("passive local observation should emit entity claims");
        assert!(
            claims
                .iter()
                .any(|claim| claim.aspect == EntityBeliefAspect::Inventory(CommodityKind::Bread))
        );
        assert!(
            claims
                .iter()
                .any(|claim| claim.aspect == EntityBeliefAspect::Alive)
        );
    }

    #[test]
    fn trade_event_records_witnessed_cooperation() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, actor, counterparty) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let actor = txn.create_agent("Trader", ControlSource::Ai).unwrap();
            let counterparty = txn.create_agent("Counterparty", ControlSource::Ai).unwrap();
            for entity in [observer, actor, counterparty] {
                txn.set_ground_location(entity, place).unwrap();
            }
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, actor, counterparty)
        };
        let mut event_log = EventLog::new();
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(4),
            cause: CauseRef::Bootstrap,
            actor_id: Some(actor),
            action_name: None,
            target_ids: vec![counterparty],
            evidence: Vec::new(),
            place_id: Some(place),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData::default(),
            tags: BTreeSet::from([EventTag::Trade]),
        }));
        let mut rng = DeterministicRng::new(Seed([3; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(4),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        assert!(
            beliefs.iter_social_observations().any(|observation| {
                observation.kind() == SocialObservationKind::WitnessedCooperation
                    && observation.place == place
                    && observation.detail
                        == SocialObservationDetail::WitnessedCooperation {
                            actor,
                            counterpart: counterparty,
                        }
                    && observation.source == PerceptionSource::DirectObservation
                    && observation.observed_tick == Tick(4)
            }),
            "trade witness should record cooperation evidence"
        );
    }

    #[test]
    fn social_event_records_witnessed_telling() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, speaker, listener) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let speaker = txn.create_agent("Speaker", ControlSource::Ai).unwrap();
            let listener = txn.create_agent("Listener", ControlSource::Ai).unwrap();
            for entity in [observer, speaker, listener] {
                txn.set_ground_location(entity, place).unwrap();
            }
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, speaker, listener)
        };
        let mut event_log = EventLog::new();
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(4),
            cause: CauseRef::Bootstrap,
            actor_id: Some(speaker),
            action_name: None,
            target_ids: vec![listener],
            evidence: Vec::new(),
            place_id: Some(place),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData::default(),
            tags: BTreeSet::from([EventTag::Social]),
        }));
        let mut rng = DeterministicRng::new(Seed([5; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(4),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        assert!(
            beliefs.iter_social_observations().any(|observation| {
                observation.kind() == SocialObservationKind::WitnessedTelling
                    && observation.place == place
                    && observation.detail
                        == SocialObservationDetail::WitnessedTelling { speaker, listener }
                    && observation.source == PerceptionSource::DirectObservation
                    && observation.observed_tick == Tick(4)
            }),
            "social witness should record witnessed telling"
        );
    }

    #[test]
    fn social_transfer_event_records_witnessed_obligation() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, actor, target) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let actor = txn.create_agent("Actor", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            for entity in [observer, actor, target] {
                txn.set_ground_location(entity, place).unwrap();
            }
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, actor, target)
        };
        let mut event_log = EventLog::new();
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(4),
            cause: CauseRef::Bootstrap,
            actor_id: Some(actor),
            action_name: None,
            target_ids: vec![target],
            evidence: Vec::new(),
            place_id: Some(place),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData::default(),
            tags: BTreeSet::from([EventTag::Social, EventTag::Transfer]),
        }));
        let mut rng = DeterministicRng::new(Seed([6; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(4),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        assert!(
            beliefs.iter_social_observations().any(|observation| {
                observation.kind() == SocialObservationKind::WitnessedObligation
                    && observation.place == place
                    && observation.detail
                        == SocialObservationDetail::WitnessedObligation { actor, target }
                    && observation.source == PerceptionSource::DirectObservation
                    && observation.observed_tick == Tick(4)
            }),
            "social transfer witness should record obligation evidence"
        );
    }

    #[test]
    fn crime_transfer_item_event_records_suspected_theft() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, thief, stolen_lot) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let thief = txn.create_agent("Thief", ControlSource::Ai).unwrap();
            let owner = txn.create_agent("Owner", ControlSource::Ai).unwrap();
            let stolen_lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(2))
                .unwrap();
            for entity in [observer, thief, owner] {
                txn.set_ground_location(entity, place).unwrap();
            }
            txn.set_ground_location(stolen_lot, place).unwrap();
            txn.set_owner(stolen_lot, owner).unwrap();
            txn.set_possessor(stolen_lot, thief).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, thief, stolen_lot)
        };
        let mut event_log = EventLog::new();
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(4),
            cause: CauseRef::Bootstrap,
            actor_id: Some(thief),
            action_name: None,
            target_ids: vec![stolen_lot],
            evidence: Vec::new(),
            place_id: Some(place),
            state_deltas: Vec::new(),
            observed_entities: observed_from_world(&world, &[thief, stolen_lot]),
            visibility: VisibilitySpec::Hidden,
            witness_data: WitnessData::default(),
            tags: BTreeSet::from([EventTag::Crime, EventTag::Transfer]),
        }));
        let mut rng = DeterministicRng::new(Seed([0x41; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(4),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        assert!(
            beliefs.iter_social_observations().any(|observation| {
                observation.kind() == SocialObservationKind::SuspectedTheft
                    && observation.place == place
                    && observation.detail
                        == SocialObservationDetail::SuspectedTheft {
                            theft: TheftFacts {
                                missing_entity: stolen_lot,
                                expected_place: place,
                                commodity: CommodityKind::Bread,
                                quantity: Quantity(2),
                            },
                            suspect: Some(thief),
                        }
                    && observation.source == PerceptionSource::DirectObservation
                    && observation.observed_tick == Tick(4)
            }),
            "crime transfer witness should record typed theft evidence"
        );
    }

    #[test]
    fn political_event_records_witnessed_cooperation() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, actor, candidate, office) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let actor = txn.create_agent("Actor", ControlSource::Ai).unwrap();
            let candidate = txn.create_agent("Candidate", ControlSource::Ai).unwrap();
            let office = txn.create_office("Office").unwrap();
            for entity in [observer, actor, candidate] {
                txn.set_ground_location(entity, place).unwrap();
            }
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, actor, candidate, office)
        };
        let mut event_log = EventLog::new();
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(4),
            cause: CauseRef::Bootstrap,
            actor_id: Some(actor),
            action_name: None,
            target_ids: vec![office, candidate],
            evidence: Vec::new(),
            place_id: Some(place),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData::default(),
            tags: BTreeSet::from([EventTag::Political]),
        }));
        let mut rng = DeterministicRng::new(Seed([7; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(4),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        assert!(
            beliefs.iter_social_observations().any(|observation| {
                observation.kind() == SocialObservationKind::WitnessedCooperation
                    && observation.place == place
                    && observation.detail
                        == SocialObservationDetail::WitnessedCooperation {
                            actor,
                            counterpart: candidate,
                        }
                    && observation.source == PerceptionSource::DirectObservation
                    && observation.observed_tick == Tick(4)
            }),
            "political witness should record cooperation evidence for agent targets only"
        );
        assert!(
            beliefs.iter_social_observations().all(|observation| {
                observation.detail
                    != SocialObservationDetail::WitnessedCooperation {
                        actor,
                        counterpart: office,
                    }
            }),
            "non-agent office targets must not produce social observations"
        );
    }

    #[test]
    fn coercion_event_records_witnessed_conflict() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, actor, target) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let actor = txn.create_agent("Actor", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            for entity in [observer, actor, target] {
                txn.set_ground_location(entity, place).unwrap();
            }
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, actor, target)
        };
        let mut event_log = EventLog::new();
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(4),
            cause: CauseRef::Bootstrap,
            actor_id: Some(actor),
            action_name: None,
            target_ids: vec![target],
            evidence: Vec::new(),
            place_id: Some(place),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData::default(),
            tags: BTreeSet::from([EventTag::Social, EventTag::Coercion]),
        }));
        let mut rng = DeterministicRng::new(Seed([8; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(4),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        assert!(
            beliefs.iter_social_observations().any(|observation| {
                observation.kind() == SocialObservationKind::WitnessedConflict
                    && observation.place == place
                    && observation.detail
                        == SocialObservationDetail::WitnessedConflict { actor, target }
                    && observation.source == PerceptionSource::DirectObservation
                    && observation.observed_tick == Tick(4)
            }),
            "coercion witness should record conflict evidence"
        );
    }

    #[test]
    fn dispatch_table_installs_perception_system() {
        let handler = dispatch_table().get(SystemId::Perception);
        assert_eq!(handler as usize, perception_system as *const () as usize);
    }

    #[test]
    fn participants_only_event_uses_direct_witnesses() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (direct_witness, bystander, target) = {
            let mut txn = new_txn(&mut world, 1);
            let direct_witness = txn.create_agent("Witness", ControlSource::Ai).unwrap();
            let bystander = txn.create_agent("Bystander", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            for entity in [direct_witness, bystander, target] {
                txn.set_ground_location(entity, place).unwrap();
            }
            txn.set_component_perception_profile(direct_witness, profile(1000))
                .unwrap();
            txn.set_component_perception_profile(bystander, profile(0))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (direct_witness, bystander, target)
        };
        let mut event_log = EventLog::new();
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(5),
            cause: CauseRef::Bootstrap,
            actor_id: Some(target),
            action_name: None,
            target_ids: vec![target],
            evidence: Vec::new(),
            place_id: Some(place),
            state_deltas: Vec::new(),
            observed_entities: observed_from_world(&world, &[target]),
            visibility: VisibilitySpec::ParticipantsOnly,
            witness_data: WitnessData {
                direct_witnesses: BTreeSet::from([direct_witness]),
                potential_witnesses: BTreeSet::from([bystander, direct_witness]),
            },
            tags: BTreeSet::new(),
        }));
        let mut rng = DeterministicRng::new(Seed([9; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(5),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert!(
            world
                .get_component_agent_belief_store(direct_witness)
                .unwrap()
                .get_entity(&target)
                .is_some()
        );
        assert!(
            world
                .get_component_agent_belief_store(bystander)
                .unwrap()
                .get_entity(&target)
                .is_none()
        );
    }

    #[test]
    fn pending_event_satisfies_perception_eventview_helpers() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (direct_witness, bystander, speaker, listener) = {
            let mut txn = new_txn(&mut world, 1);
            let direct_witness = txn.create_agent("Witness", ControlSource::Ai).unwrap();
            let bystander = txn.create_agent("Bystander", ControlSource::Ai).unwrap();
            let speaker = txn.create_agent("Speaker", ControlSource::Ai).unwrap();
            let listener = txn.create_agent("Listener", ControlSource::Ai).unwrap();
            for entity in [direct_witness, bystander, speaker, listener] {
                txn.set_ground_location(entity, place).unwrap();
            }
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (direct_witness, bystander, speaker, listener)
        };

        let pending = PendingEvent::from_payload(EventPayload {
            tick: Tick(6),
            cause: CauseRef::Bootstrap,
            actor_id: Some(speaker),
            action_name: None,
            target_ids: vec![listener],
            evidence: Vec::new(),
            place_id: Some(place),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::ParticipantsOnly,
            witness_data: WitnessData {
                direct_witnesses: BTreeSet::from([direct_witness]),
                potential_witnesses: BTreeSet::from([direct_witness, bystander]),
            },
            tags: BTreeSet::from([EventTag::Social]),
        });

        assert_eq!(
            resolve_witnesses(&world, &pending),
            vec![direct_witness],
            "participants-only witness resolution should work for PendingEvent via EventView"
        );
        assert_eq!(
            social_kind(&pending),
            Some(SocialObservationKind::WitnessedTelling)
        );

        let obligation_pending = PendingEvent::from_payload(EventPayload {
            tick: Tick(6),
            cause: CauseRef::Bootstrap,
            actor_id: Some(speaker),
            action_name: None,
            target_ids: vec![listener],
            evidence: Vec::new(),
            place_id: Some(place),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::ParticipantsOnly,
            witness_data: WitnessData {
                direct_witnesses: BTreeSet::from([direct_witness]),
                potential_witnesses: BTreeSet::from([direct_witness, bystander]),
            },
            tags: BTreeSet::from([EventTag::Social, EventTag::Transfer]),
        });
        assert_eq!(
            social_kind(&obligation_pending),
            Some(SocialObservationKind::WitnessedObligation)
        );

        let coercion_pending = PendingEvent::from_payload(EventPayload {
            tick: Tick(6),
            cause: CauseRef::Bootstrap,
            actor_id: Some(speaker),
            action_name: None,
            target_ids: vec![listener],
            evidence: Vec::new(),
            place_id: Some(place),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::ParticipantsOnly,
            witness_data: WitnessData {
                direct_witnesses: BTreeSet::from([direct_witness]),
                potential_witnesses: BTreeSet::from([direct_witness, bystander]),
            },
            tags: BTreeSet::from([EventTag::Social, EventTag::Coercion]),
        });
        assert_eq!(
            social_kind(&coercion_pending),
            Some(SocialObservationKind::WitnessedConflict)
        );

        let observations = social_observations_for_event(&world, &pending, Tick(6));
        assert_eq!(observations.len(), 1);
        assert_eq!(
            observations[0].kind(),
            SocialObservationKind::WitnessedTelling
        );
        assert_eq!(
            observations[0].detail,
            SocialObservationDetail::WitnessedTelling { speaker, listener }
        );
        assert_eq!(observations[0].place, place);
        assert_eq!(observations[0].observed_tick, Tick(6));
    }

    #[test]
    fn adjacent_places_visibility_reaches_one_hop_only() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let origin = places[0];
        let adjacent = world.topology().neighbors(origin)[0];
        let remote = places
            .into_iter()
            .find(|place| *place != origin && *place != adjacent)
            .unwrap();
        let (origin_target, adjacent_witness, remote_witness) = {
            let mut txn = new_txn(&mut world, 1);
            let origin_target = txn.create_agent("Origin", ControlSource::Ai).unwrap();
            let adjacent_witness = txn.create_agent("Adjacent", ControlSource::Ai).unwrap();
            let remote_witness = txn.create_agent("Remote", ControlSource::Ai).unwrap();
            txn.set_ground_location(origin_target, origin).unwrap();
            txn.set_ground_location(adjacent_witness, adjacent).unwrap();
            txn.set_ground_location(remote_witness, remote).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (origin_target, adjacent_witness, remote_witness)
        };
        let mut event_log = EventLog::new();
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(6),
            cause: CauseRef::Bootstrap,
            actor_id: Some(origin_target),
            action_name: None,
            target_ids: vec![origin_target],
            evidence: Vec::new(),
            place_id: Some(origin),
            state_deltas: Vec::new(),
            observed_entities: observed_from_world(&world, &[origin_target]),
            visibility: VisibilitySpec::AdjacentPlaces { max_hops: 1 },
            witness_data: WitnessData::default(),
            tags: BTreeSet::new(),
        }));
        let mut rng = DeterministicRng::new(Seed([4; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(6),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert!(
            world
                .get_component_agent_belief_store(adjacent_witness)
                .unwrap()
                .get_entity(&origin_target)
                .is_some()
        );
        assert!(
            world
                .get_component_agent_belief_store(remote_witness)
                .unwrap()
                .get_entity(&origin_target)
                .is_none()
        );
    }

    #[test]
    fn memory_capacity_evicts_older_beliefs_after_new_observation() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, older_target, newer_target) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let older_target = txn.create_agent("Older", ControlSource::Ai).unwrap();
            let newer_target = txn.create_agent("Newer", ControlSource::Ai).unwrap();
            for entity in [observer, older_target, newer_target] {
                txn.set_ground_location(entity, place).unwrap();
            }
            let mut store = AgentBeliefStore::new();
            store.update_entity(
                older_target,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(place),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: None,
                    believed_contention: None,
                    believed_evidence: None,
                    observed_tick: Tick(1),
                    source: PerceptionSource::DirectObservation,
                },
            );
            txn.set_component_agent_belief_store(observer, store)
                .unwrap();
            txn.set_component_perception_profile(
                observer,
                PerceptionProfile {
                    entity_memory_capacity: 1,
                    entity_claim_capacity: 8,
                    memory_retention_ticks: 32,
                    observation_fidelity: Permille::new(1000).unwrap(),
                    confidence_policy: BeliefConfidencePolicy::default(),
                    institutional_memory_capacity: 20,
                    consultation_speed_factor: Permille::new(500).unwrap(),
                    contradiction_tolerance: Permille::new(300).unwrap(),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, older_target, newer_target)
        };
        let mut event_log = EventLog::new();
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(7),
            cause: CauseRef::Bootstrap,
            actor_id: Some(newer_target),
            action_name: None,
            target_ids: vec![newer_target],
            evidence: Vec::new(),
            place_id: Some(place),
            state_deltas: Vec::new(),
            observed_entities: observed_from_world(&world, &[newer_target]),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData::default(),
            tags: BTreeSet::new(),
        }));
        let mut rng = DeterministicRng::new(Seed([8; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(7),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world.get_component_agent_belief_store(observer).unwrap();
        assert!(beliefs.get_entity(&older_target).is_none());
        assert!(beliefs.get_entity(&newer_target).is_some());
    }

    #[test]
    fn passive_same_place_observation_updates_belief_without_event_reference() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let observer = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(target, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(2))
                .unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_possessor(bread, target).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            observer
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([11; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        let target_belief = beliefs
            .iter_known_entities()
            .find_map(|(_, belief)| {
                (belief.last_known_inventory.get(&CommodityKind::Bread) == Some(&Quantity(2)))
                    .then_some(belief)
            })
            .expect("passive same-place observation should capture already-present local entities");
        assert_eq!(target_belief.last_known_place, Some(place));
        assert_eq!(target_belief.observed_tick, Tick(3));
        assert_eq!(target_belief.source, PerceptionSource::DirectObservation);
    }

    #[test]
    fn passive_same_place_observation_projects_contention_state() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, target, grantee) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let target = txn.create_entity(EntityKind::Facility);
            let grantee = txn.create_agent("Grantee", ControlSource::Ai).unwrap();
            let waiter = txn.create_agent("Waiter", ControlSource::Ai).unwrap();
            for entity in [observer, target, grantee, waiter] {
                txn.set_ground_location(entity, place).unwrap();
            }
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            txn.set_component_contention_queue(
                target,
                ContentionQueue {
                    next_ordinal: 1,
                    waiting: BTreeMap::from([(
                        0,
                        ContentionWaiter {
                            actor: waiter,
                            intended_action: ActionDefId(9),
                            queued_at: Tick(2),
                        },
                    )]),
                    granted: Some(ContentionGrant {
                        actor: grantee,
                        intended_action: ActionDefId(8),
                        granted_at: Tick(2),
                        expires_at: Tick(6),
                    }),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, target, grantee)
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([0x21; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        let target_belief = beliefs
            .get_entity(&target)
            .expect("colocated target should be directly observed");
        assert_eq!(
            target_belief.believed_contention,
            Some(BelievedContentionState {
                grant_holder: Some(grantee),
                queue_length: 1,
                observed_tick: Tick(3),
            })
        );
    }

    #[test]
    fn passive_same_place_observation_respects_zero_fidelity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let observer = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(target, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(0))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            observer
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([12; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(2),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        assert!(
            beliefs.iter_known_entities().next().is_none(),
            "zero observation fidelity should block passive same-place observation"
        );
    }

    #[test]
    fn passive_observation_emits_discovery_for_alive_status_mismatch() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, target) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(target, place).unwrap();
            let mut beliefs = AgentBeliefStore::new();
            beliefs.update_entity(
                target,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(place),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: None,
                    believed_contention: None,
                    believed_evidence: None,
                    observed_tick: Tick(2),
                    source: PerceptionSource::DirectObservation,
                },
            );
            txn.set_component_agent_belief_store(observer, beliefs)
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            txn.set_component_dead_at(target, DeadAt(Tick(3))).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, target)
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([13; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let discoveries = discovery_records(&event_log);
        assert_eq!(discoveries.len(), 1);
        let discovery = discoveries[0];
        assert_eq!(discovery.actor_id(), Some(observer));
        assert_eq!(discovery.place_id(), Some(place));
        assert_eq!(discovery.visibility(), VisibilitySpec::ParticipantsOnly);
        assert!(discovery.tags().contains(&EventTag::Discovery));
        assert!(discovery.tags().contains(&EventTag::WorldMutation));
        assert_eq!(
            discovery.evidence(),
            vec![EvidenceRef::Mismatch {
                observer,
                subject: target,
                kind: MismatchKind::AliveStatusChanged,
            }]
        );
    }

    #[test]
    fn passive_observation_emits_discovery_for_inventory_mismatch() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, target) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(target, place).unwrap();
            let mut inventory = BTreeMap::new();
            inventory.insert(CommodityKind::Bread, Quantity(5));
            let mut beliefs = AgentBeliefStore::new();
            beliefs.update_entity(
                target,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(place),
                    last_known_inventory: inventory,
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: None,
                    believed_contention: None,
                    believed_evidence: None,
                    observed_tick: Tick(2),
                    source: PerceptionSource::DirectObservation,
                },
            );
            txn.set_component_agent_belief_store(observer, beliefs)
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(2))
                .unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_possessor(bread, target).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, target)
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([14; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let discoveries = discovery_records(&event_log);
        assert_eq!(discoveries.len(), 1);
        assert_eq!(
            discoveries[0].evidence(),
            vec![EvidenceRef::Mismatch {
                observer,
                subject: target,
                kind: MismatchKind::InventoryDiscrepancy {
                    commodity: CommodityKind::Bread,
                    believed: Quantity(5),
                    observed: Quantity(2),
                },
            }]
        );
    }

    #[test]
    fn passive_observation_emits_discovery_for_resource_source_mismatch() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, target) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let target = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(target, place).unwrap();
            let mut beliefs = AgentBeliefStore::new();
            beliefs.update_entity(
                target,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(place),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: Some(WorkstationTag::OrchardRow),
                    resource_source: Some(ResourceSource {
                        commodity: CommodityKind::Apple,
                        available_quantity: Quantity(5),
                        max_quantity: Quantity(10),
                        regeneration_ticks_per_unit: None,
                        last_regeneration_tick: None,
                    }),
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: None,
                    believed_contention: None,
                    believed_evidence: None,
                    observed_tick: Tick(2),
                    source: PerceptionSource::DirectObservation,
                },
            );
            txn.set_component_agent_belief_store(observer, beliefs)
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            txn.set_component_workstation_marker(
                target,
                WorkstationMarker(WorkstationTag::OrchardRow),
            )
            .unwrap();
            txn.set_component_resource_source(
                target,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(2),
                    max_quantity: Quantity(10),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                },
            )
            .unwrap();
            txn.set_component_production_output_ownership_policy(
                target,
                ProductionOutputOwnershipPolicy {
                    output_owner: ProductionOutputOwner::Actor,
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, target)
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([19; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let discoveries = discovery_records(&event_log);
        assert_eq!(discoveries.len(), 1);
        assert_eq!(
            discoveries[0].evidence(),
            vec![EvidenceRef::Mismatch {
                observer,
                subject: target,
                kind: MismatchKind::ResourceSourceDiscrepancy {
                    commodity: CommodityKind::Apple,
                    believed: Quantity(5),
                    observed: Quantity(2),
                },
            }]
        );
    }

    #[test]
    fn passive_observation_without_prior_belief_emits_no_discovery() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(target, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([15; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert!(discovery_records(&event_log).is_empty());
    }

    #[test]
    fn passive_bandit_camp_observation_projects_rally_claim_for_colocated_member() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let camp_place = prototype_place_entity(PrototypePlace::BanditCamp);
        let rally_place = prototype_place_entity(PrototypePlace::ForestPath);
        let observer = {
            let mut txn = new_txn(&mut world, 1);
            let faction = txn.create_faction("Forest Bandits").unwrap();
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.add_member(observer, faction).unwrap();
            txn.set_ground_location(observer, camp_place).unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let supplies = txn
                .create_container(Container {
                    capacity: LoadUnits(2),
                    allowed_commodities: None,
                    allows_unique_items: false,
                    allows_nested_containers: false,
                })
                .unwrap();
            txn.set_ground_location(supplies, camp_place).unwrap();
            txn.set_component_bandit_faction_policy(
                faction,
                BanditFactionPolicy {
                    min_regroup_count: 2,
                    establishment_duration_ticks: NonZeroU32::new(3).unwrap(),
                    abandonment_grace_ticks: NonZeroU32::new(2).unwrap(),
                    flee_wound_threshold: Permille::new(650).unwrap(),
                    rally_place: Some(rally_place),
                },
            )
            .unwrap();
            txn.set_component_bandit_camp(
                camp_place,
                BanditCamp {
                    faction,
                    supplies,
                    empty_since_tick: None,
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            observer
        };

        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([0x33; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(2),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let faction = world.factions_of(observer)[0];
        let beliefs = world.get_component_agent_belief_store(observer).unwrap();
        assert_eq!(
            beliefs.believed_faction_rally_point(faction),
            worldwake_core::InstitutionalBeliefRead::Certain(Some(rally_place))
        );
    }

    #[test]
    fn passive_bandit_camp_observation_skips_remote_and_non_member_agents() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let camp_place = prototype_place_entity(PrototypePlace::BanditCamp);
        let remote_place = prototype_place_entity(PrototypePlace::VillageSquare);
        let rally_place = prototype_place_entity(PrototypePlace::ForestPath);
        let (remote_observer, non_member_observer, faction) = {
            let mut txn = new_txn(&mut world, 1);
            let faction = txn.create_faction("Forest Bandits").unwrap();
            let remote = txn.create_agent("Remote", ControlSource::Ai).unwrap();
            let non_member = txn.create_agent("Outsider", ControlSource::Ai).unwrap();
            let member = txn.create_agent("Member", ControlSource::Ai).unwrap();
            for agent in [remote, non_member, member] {
                txn.set_component_perception_profile(agent, profile(1000))
                    .unwrap();
            }
            txn.add_member(remote, faction).unwrap();
            txn.add_member(member, faction).unwrap();
            txn.set_ground_location(remote, remote_place).unwrap();
            txn.set_ground_location(non_member, camp_place).unwrap();
            txn.set_ground_location(member, camp_place).unwrap();
            let supplies = txn
                .create_container(Container {
                    capacity: LoadUnits(2),
                    allowed_commodities: None,
                    allows_unique_items: false,
                    allows_nested_containers: false,
                })
                .unwrap();
            txn.set_ground_location(supplies, camp_place).unwrap();
            txn.set_component_bandit_faction_policy(
                faction,
                BanditFactionPolicy {
                    min_regroup_count: 2,
                    establishment_duration_ticks: NonZeroU32::new(3).unwrap(),
                    abandonment_grace_ticks: NonZeroU32::new(2).unwrap(),
                    flee_wound_threshold: Permille::new(650).unwrap(),
                    rally_place: Some(rally_place),
                },
            )
            .unwrap();
            txn.set_component_bandit_camp(
                camp_place,
                BanditCamp {
                    faction,
                    supplies,
                    empty_since_tick: None,
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (remote, non_member, faction)
        };

        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([0x34; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(2),
            system_id: SystemId::Perception,
        })
        .unwrap();

        for agent in [remote_observer, non_member_observer] {
            let beliefs = world.get_component_agent_belief_store(agent).unwrap();
            assert_eq!(
                beliefs.believed_faction_rally_point(faction),
                worldwake_core::InstitutionalBeliefRead::Unknown
            );
        }
    }

    #[test]
    fn political_event_projects_office_installation_claim_for_witness() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, holder, office) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let holder = txn.create_agent("Holder", ControlSource::Ai).unwrap();
            let office = txn.create_office("Steward").unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(holder, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, holder, office)
        };
        let mut event_log = EventLog::new();
        emit_political_relation_event(
            &mut event_log,
            Tick(3),
            place,
            None,
            vec![office, holder],
            vec![StateDelta::Relation(RelationDelta::Added {
                relation_kind: RelationKind::OfficeHolder,
                relation: RelationValue::OfficeHolder { office, holder },
            })],
        );
        let mut rng = DeterministicRng::new(Seed([31; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let store = world.get_component_agent_belief_store(observer).unwrap();
        let beliefs = store
            .get_institutional_beliefs(&InstitutionalBeliefKey::OfficeHolderOf { office })
            .expect("office-holder belief should be projected for the witness");
        assert_eq!(beliefs.len(), 1);
        assert_eq!(
            beliefs[0].claim,
            InstitutionalClaim::OfficeHolder {
                office,
                holder: Some(holder),
                effective_tick: Tick(3),
            }
        );
        assert_eq!(
            beliefs[0].source,
            InstitutionalKnowledgeSource::WitnessedEvent
        );
        assert_eq!(beliefs[0].learned_tick, Tick(3));
        assert_eq!(beliefs[0].learned_at, Some(place));
    }

    #[test]
    fn political_event_projects_office_vacancy_claim_for_witness() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, holder, office) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let holder = txn.create_agent("FormerHolder", ControlSource::Ai).unwrap();
            let office = txn.create_office("Steward").unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(holder, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, holder, office)
        };
        let mut event_log = EventLog::new();
        emit_political_relation_event(
            &mut event_log,
            Tick(4),
            place,
            None,
            vec![office],
            vec![StateDelta::Relation(RelationDelta::Removed {
                relation_kind: RelationKind::OfficeHolder,
                relation: RelationValue::OfficeHolder { office, holder },
            })],
        );
        let mut rng = DeterministicRng::new(Seed([32; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(4),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let store = world.get_component_agent_belief_store(observer).unwrap();
        let beliefs = store
            .get_institutional_beliefs(&InstitutionalBeliefKey::OfficeHolderOf { office })
            .expect("vacancy belief should be projected for the witness");
        assert_eq!(beliefs.len(), 1);
        assert_eq!(
            beliefs[0].claim,
            InstitutionalClaim::OfficeHolder {
                office,
                holder: None,
                effective_tick: Tick(4),
            }
        );
        assert_eq!(
            beliefs[0].source,
            InstitutionalKnowledgeSource::WitnessedEvent
        );
        assert_eq!(beliefs[0].learned_tick, Tick(4));
        assert_eq!(beliefs[0].learned_at, Some(place));
    }

    #[test]
    fn political_event_projects_force_control_claim_for_witness() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, controller, office) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let controller = txn.create_agent("Controller", ControlSource::Ai).unwrap();
            let office = txn.create_office("Steward").unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(controller, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, controller, office)
        };
        let mut event_log = EventLog::new();
        emit_political_relation_event(
            &mut event_log,
            Tick(5),
            place,
            None,
            vec![office, controller],
            vec![
                StateDelta::Relation(RelationDelta::Added {
                    relation_kind: RelationKind::OfficeController,
                    relation: RelationValue::OfficeController { office, controller },
                }),
                StateDelta::Component(ComponentDelta::Set {
                    entity: office,
                    component_kind: ComponentKind::OfficeForceState,
                    before: Some(ComponentValue::OfficeForceState(OfficeForceState {
                        control_since: None,
                        challenged_since: Some(Tick(4)),
                        contested_since: Some(Tick(4)),
                        last_uncontested_tick: None,
                    })),
                    after: ComponentValue::OfficeForceState(OfficeForceState {
                        control_since: Some(Tick(5)),
                        challenged_since: None,
                        contested_since: None,
                        last_uncontested_tick: Some(Tick(5)),
                    }),
                }),
            ],
        );
        let mut rng = DeterministicRng::new(Seed([33; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(5),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let store = world.get_component_agent_belief_store(observer).unwrap();
        let beliefs = store
            .get_institutional_beliefs(&InstitutionalBeliefKey::ForceControllerOf { office })
            .unwrap();
        assert_eq!(beliefs.len(), 1);
        assert_eq!(
            beliefs[0].claim,
            InstitutionalClaim::ForceControl {
                office,
                controller: Some(controller),
                contested: false,
                effective_tick: Tick(5),
            }
        );
        assert_eq!(
            beliefs[0].source,
            InstitutionalKnowledgeSource::WitnessedEvent
        );
    }

    #[test]
    fn political_event_support_overwrite_projects_only_final_claim() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, supporter, old_candidate, new_candidate, office) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let supporter = txn.create_agent("Supporter", ControlSource::Ai).unwrap();
            let old_candidate = txn.create_agent("OldCandidate", ControlSource::Ai).unwrap();
            let new_candidate = txn.create_agent("NewCandidate", ControlSource::Ai).unwrap();
            let office = txn.create_office("Steward").unwrap();
            for entity in [observer, supporter, old_candidate, new_candidate] {
                txn.set_ground_location(entity, place).unwrap();
            }
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, supporter, old_candidate, new_candidate, office)
        };
        let mut event_log = EventLog::new();
        emit_political_relation_event(
            &mut event_log,
            Tick(5),
            place,
            Some(supporter),
            vec![office, new_candidate],
            vec![
                StateDelta::Relation(RelationDelta::Removed {
                    relation_kind: RelationKind::SupportDeclaration,
                    relation: RelationValue::SupportDeclaration {
                        supporter,
                        office,
                        candidate: old_candidate,
                    },
                }),
                StateDelta::Relation(RelationDelta::Added {
                    relation_kind: RelationKind::SupportDeclaration,
                    relation: RelationValue::SupportDeclaration {
                        supporter,
                        office,
                        candidate: new_candidate,
                    },
                }),
            ],
        );
        let mut rng = DeterministicRng::new(Seed([33; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(5),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let store = world.get_component_agent_belief_store(observer).unwrap();
        let beliefs = store
            .get_institutional_beliefs(&InstitutionalBeliefKey::SupportFor { supporter, office })
            .expect("support belief should be projected for the witness");
        assert_eq!(beliefs.len(), 1);
        assert_eq!(
            beliefs[0].claim,
            InstitutionalClaim::SupportDeclaration {
                office,
                supporter,
                candidate: Some(new_candidate),
                effective_tick: Tick(5),
            }
        );
    }

    #[test]
    fn political_event_does_not_project_institutional_claim_to_remote_agent() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let place = places[0];
        let remote_place = places.get(1).copied().unwrap_or(place);
        let (observer, remote, holder, office) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let remote = txn.create_agent("Remote", ControlSource::Ai).unwrap();
            let holder = txn.create_agent("Holder", ControlSource::Ai).unwrap();
            let office = txn.create_office("Steward").unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(remote, remote_place).unwrap();
            txn.set_ground_location(holder, place).unwrap();
            for agent in [observer, remote] {
                txn.set_component_agent_belief_store(agent, AgentBeliefStore::new())
                    .unwrap();
                txn.set_component_perception_profile(agent, profile(1000))
                    .unwrap();
            }
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, remote, holder, office)
        };
        let mut event_log = EventLog::new();
        emit_political_relation_event(
            &mut event_log,
            Tick(6),
            place,
            None,
            vec![office, holder],
            vec![StateDelta::Relation(RelationDelta::Added {
                relation_kind: RelationKind::OfficeHolder,
                relation: RelationValue::OfficeHolder { office, holder },
            })],
        );
        let mut rng = DeterministicRng::new(Seed([34; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(6),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let local_store = world.get_component_agent_belief_store(observer).unwrap();
        assert!(
            local_store
                .has_institutional_belief(&InstitutionalBeliefKey::OfficeHolderOf { office })
        );
        let remote_store = world.get_component_agent_belief_store(remote).unwrap();
        assert!(
            !remote_store
                .has_institutional_belief(&InstitutionalBeliefKey::OfficeHolderOf { office })
        );
    }

    #[test]
    fn passive_observation_with_matching_prior_belief_emits_no_discovery() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(target, place).unwrap();
            let mut inventory = BTreeMap::new();
            inventory.insert(CommodityKind::Bread, Quantity(2));
            let mut beliefs = AgentBeliefStore::new();
            beliefs.update_entity(
                target,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(place),
                    last_known_inventory: inventory,
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: None,
                    believed_contention: None,
                    believed_evidence: None,
                    observed_tick: Tick(2),
                    source: PerceptionSource::DirectObservation,
                },
            );
            txn.set_component_agent_belief_store(observer, beliefs)
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(2))
                .unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_possessor(bread, target).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([16; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert!(discovery_records(&event_log).is_empty());
    }

    #[test]
    fn passive_observation_emits_discovery_for_missing_entity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let place = places[0];
        let other_place = places
            .iter()
            .copied()
            .find(|candidate| *candidate != place)
            .unwrap();
        let (observer, target) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(target, other_place).unwrap();
            let mut beliefs = AgentBeliefStore::new();
            beliefs.update_entity(
                target,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(place),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: None,
                    believed_contention: None,
                    believed_evidence: None,
                    observed_tick: Tick(2),
                    source: PerceptionSource::DirectObservation,
                },
            );
            txn.set_component_agent_belief_store(observer, beliefs)
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, target)
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([17; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert_eq!(
            discovery_records(&event_log)[0].evidence(),
            vec![EvidenceRef::Mismatch {
                observer,
                subject: target,
                kind: MismatchKind::EntityMissing,
            }]
        );
    }

    #[test]
    fn passive_observation_does_not_emit_missing_without_prior_belief() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([18; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert!(discovery_records(&event_log).is_empty());
    }

    #[test]
    fn passive_observation_does_not_emit_missing_when_entity_is_still_present() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(target, place).unwrap();
            let mut beliefs = AgentBeliefStore::new();
            beliefs.update_entity(
                target,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(place),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: None,
                    believed_contention: None,
                    believed_evidence: None,
                    observed_tick: Tick(2),
                    source: PerceptionSource::DirectObservation,
                },
            );
            txn.set_component_agent_belief_store(observer, beliefs)
                .unwrap();
            txn.set_component_perception_profile(observer, profile(0))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([19; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert!(discovery_records(&event_log).is_empty());
    }

    #[test]
    fn adjacent_event_observation_emits_discovery_for_alive_status_mismatch() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let origin = places[0];
        let adjacent = world.topology().neighbors(origin)[0];
        let (observer, target) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, adjacent).unwrap();
            txn.set_ground_location(target, origin).unwrap();
            let mut beliefs = AgentBeliefStore::new();
            beliefs.update_entity(
                target,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(origin),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: None,
                    believed_contention: None,
                    believed_evidence: None,
                    observed_tick: Tick(2),
                    source: PerceptionSource::DirectObservation,
                },
            );
            txn.set_component_agent_belief_store(observer, beliefs)
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            txn.set_component_dead_at(target, DeadAt(Tick(3))).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, target)
        };
        let mut event_log = EventLog::new();
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(3),
            cause: CauseRef::Bootstrap,
            actor_id: Some(target),
            action_name: None,
            target_ids: vec![target],
            evidence: Vec::new(),
            place_id: Some(origin),
            state_deltas: Vec::new(),
            observed_entities: observed_from_world(&world, &[target]),
            visibility: VisibilitySpec::AdjacentPlaces { max_hops: 1 },
            witness_data: WitnessData::default(),
            tags: BTreeSet::new(),
        }));
        let mut rng = DeterministicRng::new(Seed([20; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert!(
            discovery_records(&event_log).iter().any(|record| {
                record.evidence()
                    == vec![EvidenceRef::Mismatch {
                        observer,
                        subject: target,
                        kind: MismatchKind::AliveStatusChanged,
                    }]
            }),
            "adjacent event witness should record alive-status mismatch"
        );
    }

    #[test]
    fn adjacent_event_observation_emits_discovery_for_inventory_mismatch() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let origin = places[0];
        let adjacent = world.topology().neighbors(origin)[0];
        let (observer, target) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, adjacent).unwrap();
            txn.set_ground_location(target, origin).unwrap();
            let mut prior_inventory = BTreeMap::new();
            prior_inventory.insert(CommodityKind::Bread, Quantity(5));
            let mut beliefs = AgentBeliefStore::new();
            beliefs.update_entity(
                target,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(origin),
                    last_known_inventory: prior_inventory,
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: None,
                    believed_contention: None,
                    believed_evidence: None,
                    observed_tick: Tick(2),
                    source: PerceptionSource::DirectObservation,
                },
            );
            txn.set_component_agent_belief_store(observer, beliefs)
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(2))
                .unwrap();
            txn.set_ground_location(bread, origin).unwrap();
            txn.set_possessor(bread, target).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, target)
        };
        let mut event_log = EventLog::new();
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(3),
            cause: CauseRef::Bootstrap,
            actor_id: Some(target),
            action_name: None,
            target_ids: vec![target],
            evidence: Vec::new(),
            place_id: Some(origin),
            state_deltas: Vec::new(),
            observed_entities: observed_from_world(&world, &[target]),
            visibility: VisibilitySpec::AdjacentPlaces { max_hops: 1 },
            witness_data: WitnessData::default(),
            tags: BTreeSet::new(),
        }));
        let mut rng = DeterministicRng::new(Seed([21; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert!(
            discovery_records(&event_log).iter().any(|record| {
                record.evidence()
                    == vec![EvidenceRef::Mismatch {
                        observer,
                        subject: target,
                        kind: MismatchKind::InventoryDiscrepancy {
                            commodity: CommodityKind::Bread,
                            believed: Quantity(5),
                            observed: Quantity(2),
                        },
                    }]
            }),
            "adjacent event witness should record inventory mismatch"
        );
    }

    #[test]
    fn adjacent_event_observation_emits_discovery_for_place_changed() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let origin = places[0];
        let adjacent = world.topology().neighbors(origin)[0];
        let remote = places
            .iter()
            .copied()
            .find(|candidate| *candidate != origin && *candidate != adjacent)
            .unwrap();
        let (observer, target) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, adjacent).unwrap();
            txn.set_ground_location(target, remote).unwrap();
            let mut beliefs = AgentBeliefStore::new();
            beliefs.update_entity(
                target,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(origin),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: None,
                    believed_contention: None,
                    believed_evidence: None,
                    observed_tick: Tick(2),
                    source: PerceptionSource::DirectObservation,
                },
            );
            txn.set_component_agent_belief_store(observer, beliefs)
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, target)
        };
        let mut event_log = EventLog::new();
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(3),
            cause: CauseRef::Bootstrap,
            actor_id: Some(target),
            action_name: None,
            target_ids: vec![target],
            evidence: Vec::new(),
            place_id: Some(origin),
            state_deltas: Vec::new(),
            observed_entities: observed_from_world(&world, &[target]),
            visibility: VisibilitySpec::AdjacentPlaces { max_hops: 1 },
            witness_data: WitnessData::default(),
            tags: BTreeSet::new(),
        }));
        let mut rng = DeterministicRng::new(Seed([22; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert!(
            discovery_records(&event_log).iter().any(|record| {
                record.evidence()
                    == vec![EvidenceRef::Mismatch {
                        observer,
                        subject: target,
                        kind: MismatchKind::PlaceChanged {
                            believed_place: origin,
                            observed_place: remote,
                        },
                    }]
            }),
            "adjacent event witness should record place mismatch"
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn same_tick_events_use_distinct_event_local_snapshots_in_sequence() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let origin = places[0];
        let adjacent = world.topology().neighbors(origin)[0];
        let remote = places
            .iter()
            .copied()
            .find(|candidate| *candidate != origin && *candidate != adjacent)
            .unwrap();
        let (observer, target) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, adjacent).unwrap();
            txn.set_ground_location(target, remote).unwrap();
            let mut beliefs = AgentBeliefStore::new();
            let mut prior_inventory = BTreeMap::new();
            prior_inventory.insert(CommodityKind::Bread, Quantity(5));
            beliefs.update_entity(
                target,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(origin),
                    last_known_inventory: prior_inventory,
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: None,
                    believed_contention: None,
                    believed_evidence: None,
                    observed_tick: Tick(2),
                    source: PerceptionSource::DirectObservation,
                },
            );
            txn.set_component_agent_belief_store(observer, beliefs)
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, target)
        };
        let mut event_log = EventLog::new();
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(3),
            cause: CauseRef::Bootstrap,
            actor_id: Some(target),
            action_name: None,
            target_ids: vec![target],
            evidence: Vec::new(),
            place_id: Some(origin),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::from([(target, observed_snapshot(Some(origin), 4))]),
            visibility: VisibilitySpec::AdjacentPlaces { max_hops: 1 },
            witness_data: WitnessData::default(),
            tags: BTreeSet::new(),
        }));
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(3),
            cause: CauseRef::Bootstrap,
            actor_id: Some(target),
            action_name: None,
            target_ids: vec![target],
            evidence: Vec::new(),
            place_id: Some(origin),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::from([(target, observed_snapshot(Some(remote), 2))]),
            visibility: VisibilitySpec::AdjacentPlaces { max_hops: 1 },
            witness_data: WitnessData::default(),
            tags: BTreeSet::new(),
        }));
        let mut rng = DeterministicRng::new(Seed([24; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let mismatches = discovery_records(&event_log)
            .iter()
            .flat_map(|record| record.evidence().iter())
            .filter_map(|evidence| match evidence {
                EvidenceRef::Mismatch {
                    observer: seen_by,
                    subject,
                    kind,
                } if *seen_by == observer && *subject == target => Some(*kind),
                EvidenceRef::Wound { .. } | EvidenceRef::Mismatch { .. } => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            mismatches,
            vec![
                MismatchKind::InventoryDiscrepancy {
                    commodity: CommodityKind::Bread,
                    believed: Quantity(5),
                    observed: Quantity(4),
                },
                MismatchKind::InventoryDiscrepancy {
                    commodity: CommodityKind::Bread,
                    believed: Quantity(4),
                    observed: Quantity(2),
                },
                MismatchKind::PlaceChanged {
                    believed_place: origin,
                    observed_place: remote,
                },
            ]
        );

        let final_belief = world
            .get_component_agent_belief_store(observer)
            .unwrap()
            .get_entity(&target)
            .unwrap();
        assert_eq!(final_belief.last_known_place, Some(remote));
        assert_eq!(
            final_belief.last_known_inventory,
            BTreeMap::from([(CommodityKind::Bread, Quantity(2))])
        );
    }

    #[test]
    fn adjacent_event_observation_with_matching_belief_emits_no_discovery() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let origin = places[0];
        let adjacent = world.topology().neighbors(origin)[0];
        {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let target = txn.create_agent("Target", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, adjacent).unwrap();
            txn.set_ground_location(target, origin).unwrap();
            let mut beliefs = AgentBeliefStore::new();
            beliefs.update_entity(
                target,
                BelievedEntityState {
                    believed_kind: None,
                    last_known_place: Some(origin),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: None,
                    believed_contention: None,
                    believed_evidence: None,
                    observed_tick: Tick(2),
                    source: PerceptionSource::DirectObservation,
                },
            );
            txn.set_component_agent_belief_store(observer, beliefs)
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let mut event_log = EventLog::new();
        let target = world
            .query_agent_data()
            .find(|(entity, _)| {
                world.effective_place(*entity) == Some(origin)
                    && world.get_component_dead_at(*entity).is_none()
            })
            .unwrap()
            .0;
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(3),
            cause: CauseRef::Bootstrap,
            actor_id: Some(target),
            action_name: None,
            target_ids: vec![target],
            evidence: Vec::new(),
            place_id: Some(origin),
            state_deltas: Vec::new(),
            observed_entities: observed_from_world(&world, &[target]),
            visibility: VisibilitySpec::AdjacentPlaces { max_hops: 1 },
            witness_data: WitnessData::default(),
            tags: BTreeSet::new(),
        }));
        let mut rng = DeterministicRng::new(Seed([23; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert!(discovery_records(&event_log).is_empty());
    }

    #[test]
    fn trace_records_institutional_claims() {
        use worldwake_sim::PerceptionTraceSink;

        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();

        let (observer, office) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let office = txn.create_office("Council").unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, office)
        };

        let mut event_log = EventLog::new();
        emit_political_relation_event(
            &mut event_log,
            Tick(3),
            place,
            None,
            vec![office, observer],
            vec![StateDelta::Relation(RelationDelta::Added {
                relation_kind: RelationKind::OfficeHolder,
                relation: RelationValue::OfficeHolder {
                    office,
                    holder: observer,
                },
            })],
        );

        let mut rng = DeterministicRng::new(Seed([7; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();
        let mut trace_sink = PerceptionTraceSink::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: Some(&mut trace_sink),
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let events = trace_sink.events_for(observer);
        assert!(
            !events.is_empty(),
            "trace should record at least one event for the observer"
        );
        let event = events[0];
        assert!(event.observation_passed);
        assert!(
            !event.institutional_claims.is_empty(),
            "trace should record institutional claims from political event"
        );
        assert_eq!(
            event.institutional_claims[0].0,
            InstitutionalBeliefKey::OfficeHolderOf { office }
        );
    }

    #[test]
    fn passive_observation_records_bounty_artifact_belief() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();

        let (observer, issuer, artifact) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let issuer = txn.create_agent("Issuer", ControlSource::Ai).unwrap();
            let artifact = txn.create_entity(EntityKind::SocialArtifact);
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(issuer, place).unwrap();
            txn.set_ground_location(artifact, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            txn.set_component_artifact_header(
                artifact,
                ArtifactHeader {
                    kind: ArtifactKind::Bounty,
                    issuer,
                    issuing_authority: None,
                    created_at: Tick(1),
                    expires_at: Some(Tick(8)),
                    state: ArtifactState::Active,
                    jurisdiction: None,
                },
            )
            .unwrap();
            txn.set_component_bounty_terms(
                artifact,
                BountyTerms {
                    target: BountyTarget::EliminateEntity { target: issuer },
                    proof_requirement: ProofRequirement::SelfReport,
                    reward_commodity: CommodityKind::Coin,
                    reward_quantity: Quantity(7),
                    reward_source: RewardSource::PersonalFunds { issuer },
                    claim_place: place,
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, issuer, artifact)
        };

        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([9; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(2),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let belief = world
            .get_component_agent_belief_store(observer)
            .unwrap()
            .get_entity(&artifact)
            .unwrap()
            .believed_artifact
            .clone()
            .unwrap();
        assert_eq!(belief.kind, ArtifactKind::Bounty);
        assert_eq!(belief.state, ArtifactState::Active);
        assert_eq!(belief.issuer, issuer);
        assert_eq!(belief.expires_at, Some(Tick(8)));
        assert_eq!(
            belief.bounty_terms,
            Some(worldwake_core::BelievedBountyTerms {
                target: BountyTarget::EliminateEntity { target: issuer },
                reward_commodity: CommodityKind::Coin,
                reward_quantity: Quantity(7),
                claim_place: place,
            })
        );
    }

    #[test]
    fn passive_observation_internalizes_office_vacancy_notice() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();

        let (observer, office, artifact) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let issuer = txn.create_agent("Issuer", ControlSource::Ai).unwrap();
            let office = txn.create_office("Council").unwrap();
            let artifact = txn.create_entity(EntityKind::SocialArtifact);
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(issuer, place).unwrap();
            txn.set_ground_location(office, place).unwrap();
            txn.set_ground_location(artifact, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            txn.set_component_artifact_header(
                artifact,
                ArtifactHeader {
                    kind: ArtifactKind::Notice,
                    issuer,
                    issuing_authority: None,
                    created_at: Tick(1),
                    expires_at: None,
                    state: ArtifactState::Active,
                    jurisdiction: None,
                },
            )
            .unwrap();
            txn.set_component_notice_content(
                artifact,
                NoticeContent {
                    topic: NoticeTopic::OfficeVacancy { office },
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, office, artifact)
        };

        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([11; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(2),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let store = world.get_component_agent_belief_store(observer).unwrap();
        let beliefs = store
            .get_institutional_beliefs(&InstitutionalBeliefKey::OfficeHolderOf { office })
            .unwrap();
        assert!(beliefs.iter().any(|belief| {
            belief.claim
                == InstitutionalClaim::OfficeHolder {
                    office,
                    holder: None,
                    effective_tick: Tick(2),
                }
                && belief.source == InstitutionalKnowledgeSource::DirectObservation
                && belief.learned_at == Some(place)
        }));

        assert_eq!(
            store
                .get_entity(&artifact)
                .unwrap()
                .believed_artifact
                .as_ref()
                .unwrap()
                .notice_topic,
            Some(NoticeTopic::OfficeVacancy { office })
        );
    }

    #[test]
    fn passive_observation_internalizes_institutional_notice_claim() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();

        let (observer, office, holder) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let issuer = txn.create_agent("Issuer", ControlSource::Ai).unwrap();
            let holder = txn.create_agent("Holder", ControlSource::Ai).unwrap();
            let office = txn.create_office("Council").unwrap();
            let artifact = txn.create_entity(EntityKind::SocialArtifact);
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(issuer, place).unwrap();
            txn.set_ground_location(holder, place).unwrap();
            txn.set_ground_location(office, place).unwrap();
            txn.set_ground_location(artifact, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            txn.set_component_artifact_header(
                artifact,
                ArtifactHeader {
                    kind: ArtifactKind::Notice,
                    issuer,
                    issuing_authority: None,
                    created_at: Tick(1),
                    expires_at: None,
                    state: ArtifactState::Active,
                    jurisdiction: None,
                },
            )
            .unwrap();
            txn.set_component_notice_content(
                artifact,
                NoticeContent {
                    topic: NoticeTopic::Institutional {
                        claim: InstitutionalClaim::OfficeHolder {
                            office,
                            holder: Some(holder),
                            effective_tick: Tick(1),
                        },
                    },
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, office, holder)
        };

        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([12; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(2),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let store = world.get_component_agent_belief_store(observer).unwrap();
        let beliefs = store
            .get_institutional_beliefs(&InstitutionalBeliefKey::OfficeHolderOf { office })
            .unwrap();
        assert!(beliefs.iter().any(|belief| {
            belief.claim
                == InstitutionalClaim::OfficeHolder {
                    office,
                    holder: Some(holder),
                    effective_tick: Tick(1),
                }
                && belief.source == InstitutionalKnowledgeSource::DirectObservation
        }));
    }

    #[test]
    fn trace_records_failed_observation_check() {
        use worldwake_sim::PerceptionTraceSink;

        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();

        let (observer, office) = {
            let mut txn = new_txn(&mut world, 1);
            // fidelity=0 means observation check always fails
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(0))
                .unwrap();
            let office = txn.create_office("Council").unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, office)
        };

        let mut event_log = EventLog::new();
        emit_political_relation_event(
            &mut event_log,
            Tick(3),
            place,
            None,
            vec![office, observer],
            vec![StateDelta::Relation(RelationDelta::Added {
                relation_kind: RelationKind::OfficeHolder,
                relation: RelationValue::OfficeHolder {
                    office,
                    holder: observer,
                },
            })],
        );

        let mut rng = DeterministicRng::new(Seed([7; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();
        let mut trace_sink = PerceptionTraceSink::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: Some(&mut trace_sink),
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let events = trace_sink.events_for(observer);
        assert!(
            !events.is_empty(),
            "trace should record failed observation check"
        );
        assert!(!events[0].observation_passed);
        assert_eq!(events[0].effective_fidelity, 0);
        assert!(events[0].institutional_claims.is_empty());
    }

    #[test]
    fn trace_records_modulated_effective_fidelity_for_witness_events() {
        use worldwake_sim::PerceptionTraceSink;

        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();

        let (observer, office) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            txn.set_component_homeostatic_needs(observer, HomeostaticNeeds::new_sated())
                .unwrap();
            txn.set_component_place_visibility_profile(
                place,
                PlaceVisibilityProfile {
                    base_concealment: Permille::new(1000).unwrap(),
                },
            )
            .unwrap();
            let office = txn.create_office("Council").unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, office)
        };

        let mut event_log = EventLog::new();
        emit_political_relation_event(
            &mut event_log,
            Tick(3),
            place,
            None,
            vec![office, observer],
            vec![StateDelta::Relation(RelationDelta::Added {
                relation_kind: RelationKind::OfficeHolder,
                relation: RelationValue::OfficeHolder {
                    office,
                    holder: observer,
                },
            })],
        );

        let mut rng = DeterministicRng::new(Seed([13; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();
        let mut trace_sink = PerceptionTraceSink::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: Some(&mut trace_sink),
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let event = trace_sink.events_for(observer)[0];
        assert_eq!(event.effective_fidelity, 0);
        assert!(!event.observation_passed);
    }

    #[test]
    fn trace_absent_when_disabled() {
        // This test verifies that perception_system works fine with None trace.
        // The zero-cost guarantee comes from Option::is_some() checks at compile time.
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();

        {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        };

        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([7; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        // No trace sink — should still succeed without any allocation.
        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();
    }
}
