use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    AgentBeliefStore, BelievedActivity, BelievedInstitutionalClaim, BelievedOfficeDataSnapshot,
    BelievedRecordDataSnapshot, CauseRef, CommodityKind, ComponentDelta, ComponentKind,
    ComponentValue, EntityBeliefAspect, EntityBeliefClaim, EntityId, EntityKind, EventLog,
    EventPayload, EventTag, EventView, EvidenceRef, GoalKind, HypothesisKind,
    InstitutionalBeliefKey, InstitutionalClaim, InstitutionalKnowledgeSource,
    InstitutionalSnapshotSource, MismatchKind, NoticeTopic, ObservationContext,
    ObservationOmission, OmissionReason, PendingEvent, PerceptionSource, Permille, Quantity,
    RelationDelta, RelationValue, ReliabilityRecord, SocialObservation, SocialObservationDetail,
    SocialObservationKind, SourceKey, SourceReliability, StateDelta, SurveyRecord, TheftFacts,
    VisibilitySpec, WitnessData, World, WorldTxn, build_believed_entity_state,
};
use worldwake_core::{DecisionEventPayload, SurveyRecordedPayload};
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
    observed_record_snapshots: BTreeMap<EntityId, BelievedRecordDataSnapshot>,
    observed_office_snapshots: BTreeMap<EntityId, BelievedOfficeDataSnapshot>,
    observed_holders: BTreeMap<EntityId, Option<EntityId>>,
    noticed_missing_subjects: BTreeSet<EntityId>,
    omitted_observations: Vec<ObservationOmission>,
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
    let source_reliability_event = event_log.next_id();
    let updated_source_reliabilities = capacity_observations_for_direct_local_batches(
        world,
        tick,
        source_reliability_event,
        &direct_local_batches,
    );
    observe_active_actions(
        world,
        tick,
        active_actions,
        action_defs,
        &direct_local_batches,
        &mut updated_stores,
    );
    let survey_records = survey_records_for_arrivals(world, tick, &direct_local_batches);

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

    if updated_stores.is_empty()
        && updated_source_reliabilities.is_empty()
        && survey_records.is_empty()
    {
        return Ok(());
    }

    if !updated_stores.is_empty() || !updated_source_reliabilities.is_empty() {
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
        for (agent, reliability) in updated_source_reliabilities {
            txn.set_component_source_reliability(agent, reliability)
                .map_err(|error| SystemError::new(error.to_string()))?;
        }
        let _ = txn.commit(event_log);
    }

    for survey in survey_records {
        record_survey(world, event_log, tick, survey)?;
    }
    Ok(())
}

fn capacity_observations_for_direct_local_batches(
    world: &World,
    tick: worldwake_core::Tick,
    provenance_event: worldwake_core::EventId,
    direct_local_batches: &BTreeMap<EntityId, DirectLocalObservationBatch>,
) -> BTreeMap<EntityId, SourceReliability> {
    let mut updates = BTreeMap::new();

    for (agent, batch) in direct_local_batches {
        let mut reliability = world
            .get_component_source_reliability(*agent)
            .cloned()
            .unwrap_or_default();
        let mut changed = false;

        for (source_entity, snapshot) in &batch.observed_snapshots {
            let Some(source) = snapshot.resource_source.as_ref() else {
                continue;
            };
            let observed_capacity = u16::try_from(source.available_quantity.0).unwrap_or(u16::MAX);
            let key = SourceKey {
                entity: *source_entity,
                commodity: source.commodity,
            };
            reliability
                .sources
                .entry(key)
                .or_insert_with(|| ReliabilityRecord::new(tick))
                .observe_capacity(observed_capacity, tick);
            if let Some(record) = reliability.sources.get_mut(&key) {
                record.push_provenance(provenance_event);
            }
            changed = true;
        }

        if changed {
            updates.insert(*agent, reliability);
        }
    }

    updates
}

#[derive(Clone, Copy)]
struct PendingSurveyRecord {
    agent: EntityId,
    place: EntityId,
    record: SurveyRecord,
}

fn survey_records_for_arrivals(
    world: &World,
    tick: worldwake_core::Tick,
    direct_local_batches: &BTreeMap<EntityId, DirectLocalObservationBatch>,
) -> Vec<PendingSurveyRecord> {
    direct_local_batches
        .iter()
        .filter_map(|(agent, batch)| pending_survey_record(world, tick, *agent, batch.place))
        .collect()
}

fn pending_survey_record(
    world: &World,
    tick: worldwake_core::Tick,
    agent: EntityId,
    place: EntityId,
) -> Option<PendingSurveyRecord> {
    if world.effective_place(agent) != Some(place) {
        return None;
    }
    let frame = world.get_component_intention_frame(agent)?;
    let GoalKind::ExploreLocation {
        target_place,
        hypothesis,
        ..
    } = frame.goal.kind
    else {
        return None;
    };
    if target_place != place {
        return None;
    }
    if world
        .get_component_survey_memory(agent)
        .and_then(|memory| memory.find(place, hypothesis))
        .is_some_and(|record| record.recorded_tick == tick)
    {
        return None;
    }
    let confidence = world
        .get_component_perception_profile(agent)?
        .observation_fidelity;
    let found = evaluate_hypothesis(world, place, hypothesis);
    Some(PendingSurveyRecord {
        agent,
        place,
        record: SurveyRecord {
            place,
            hypothesis,
            found,
            confidence,
            recorded_tick: tick,
        },
    })
}

fn evaluate_hypothesis(world: &World, place: EntityId, hypothesis: HypothesisKind) -> bool {
    match hypothesis {
        HypothesisKind::MayContainCommodity { commodity } => {
            world.query_resource_source().any(|(entity, source)| {
                world.effective_place(entity) == Some(place)
                    && source.commodity == commodity
                    && source.available_quantity > Quantity(0)
            }) || world.query_item_lot().any(|(entity, lot)| {
                world.effective_place(entity) == Some(place) && lot.commodity == commodity
            })
        }
        HypothesisKind::MayContainLatrine => {
            world.place_has_tag(place, worldwake_core::PlaceTag::Latrine)
        }
        HypothesisKind::MayContainWashBasin => {
            world.query_workstation_marker().any(|(entity, marker)| {
                world.effective_place(entity) == Some(place)
                    && marker.0 == worldwake_core::WorkstationTag::WashBasin
            })
        }
        HypothesisKind::MayContainSleepSite => world
            .get_component_sleep_quality_profile(place)
            .is_some_and(|profile| {
                profile.recovery_modifier > worldwake_core::SleepRecoveryModifier::IDENTITY
            }),
        HypothesisKind::Proactive => true,
    }
}

fn record_survey(
    world: &mut World,
    event_log: &mut EventLog,
    tick: worldwake_core::Tick,
    survey: PendingSurveyRecord,
) -> Result<(), SystemError> {
    let capacity = world
        .get_component_cognitive_profile(survey.agent)
        .map(|profile| profile.survey_memory_capacity)
        .unwrap_or_default();
    let mut memory = world
        .get_component_survey_memory(survey.agent)
        .cloned()
        .unwrap_or_default();
    memory.record(survey.record, capacity);

    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        Some(survey.agent),
        Some(survey.place),
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.set_component_survey_memory(survey.agent, memory)
        .map_err(|error| SystemError::new(error.to_string()))?;
    txn.add_tag(EventTag::SurveyRecorded).set_decision_payload(
        DecisionEventPayload::SurveyRecorded(SurveyRecordedPayload {
            surveyor: survey.agent,
            place: survey.place,
            hypothesis: survey.record.hypothesis,
            found: survey.record.found,
            confidence: survey.record.confidence,
        }),
    );
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
    for ((subject, aspect), value) in authority_claims_for_event(record) {
        record_direct_authority_claim(store, subject, aspect, value, record.tick());
    }

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

    let needs = world
        .get_component_homeostatic_needs(witness)
        .copied()
        .unwrap_or_default();
    store.prune_decayed_beliefs(&profile, tick, &needs);
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
        let needs = world
            .get_component_homeostatic_needs(agent)
            .copied()
            .unwrap_or_default();
        let mut store = base_store;
        let mut store_changed = store.record_place_visit(place, tick);

        if let Some(batch) = collect_direct_local_observation_batch(
            world,
            agent,
            place,
            colocated_entities,
            tick,
            effective_fidelity,
            rng,
            &store,
            needs,
            &profile,
        ) {
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
                needs,
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
                store_changed = true;
            }
            if !batch.omitted_observations.is_empty() {
                store_changed = true;
            }
            batches.insert(agent, batch);
        }

        if store_changed {
            updated_stores.insert(agent, store);
        }
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
        let Some(profile) = world.get_component_perception_profile(*agent).copied() else {
            continue;
        };
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

            if store.update_believed_activity(
                subject,
                next_activity,
                tick,
                &profile.confidence_policy,
            ) {
                changed = true;
            }
        }

        for subject in &batch.noticed_missing_subjects {
            if store.clear_believed_activity(subject, tick, &profile.confidence_policy) {
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
                    && store.update_departure_projection(
                        subject,
                        destination,
                        tick,
                        &profile.confidence_policy,
                    )
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
    needs: worldwake_core::HomeostaticNeeds,
    profile: &worldwake_core::PerceptionProfile,
) -> Option<DirectLocalObservationBatch> {
    let mut observed_snapshots = BTreeMap::new();
    let mut observed_record_snapshots = BTreeMap::new();
    let mut observed_office_snapshots = BTreeMap::new();
    let mut observed_holders = BTreeMap::new();
    let mut prioritized_entities = colocated_entities
        .iter()
        .copied()
        .filter(|entity| *entity != observer)
        .map(|entity| {
            (
                compute_observation_priority(world, entity, &needs, profile),
                entity,
            )
        })
        .collect::<Vec<_>>();
    prioritized_entities
        .sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let candidate_count = prioritized_entities.len();
    let budget = usize::from(profile.observation_budget);
    let candidates_seen = u16::try_from(candidate_count).unwrap_or(u16::MAX);
    let omitted_observations = prioritized_entities
        .iter()
        .skip(budget)
        .map(|(_, entity)| ObservationOmission {
            omitted_entity: *entity,
            reason: OmissionReason::OverBudget {
                budget: profile.observation_budget,
                candidates_seen,
            },
            observed_tick: tick,
        })
        .collect::<Vec<_>>();
    prioritized_entities.truncate(budget);

    for (_, entity) in prioritized_entities {
        if !passes_observation_check(observation_fidelity, rng) {
            continue;
        }
        if let Some(snapshot) =
            build_believed_entity_state(world, entity, tick, PerceptionSource::DirectObservation)
        {
            let learned_at = Some(place);
            match world.entity_kind(entity) {
                Some(EntityKind::Record) => {
                    if let Some(data) = world.get_component_record_data(entity).cloned() {
                        observed_record_snapshots.insert(
                            entity,
                            BelievedRecordDataSnapshot {
                                data,
                                source: InstitutionalSnapshotSource::DirectObservation,
                                learned_tick: tick,
                                learned_at,
                            },
                        );
                    }
                }
                Some(EntityKind::Office) => {
                    if let Some(data) = world.get_component_office_data(entity).cloned() {
                        observed_office_snapshots.insert(
                            entity,
                            BelievedOfficeDataSnapshot {
                                data,
                                source: InstitutionalSnapshotSource::DirectObservation,
                                learned_tick: tick,
                                learned_at,
                            },
                        );
                    }
                }
                _ => {}
            }
            if matches!(
                world.entity_kind(entity),
                Some(EntityKind::ItemLot | EntityKind::UniqueItem | EntityKind::Container)
            ) {
                observed_holders.insert(entity, world.possessor_of(entity));
            }
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

    if observed_snapshots.is_empty()
        && observed_record_snapshots.is_empty()
        && observed_office_snapshots.is_empty()
        && noticed_missing_subjects.is_empty()
        && omitted_observations.is_empty()
    {
        return None;
    }

    Some(DirectLocalObservationBatch {
        place,
        observed_snapshots,
        observed_record_snapshots,
        observed_office_snapshots,
        observed_holders,
        noticed_missing_subjects,
        omitted_observations,
    })
}

fn compute_observation_priority(
    world: &World,
    entity: EntityId,
    needs: &worldwake_core::HomeostaticNeeds,
    profile: &worldwake_core::PerceptionProfile,
) -> u16 {
    let item_need_boost = || -> u16 {
        if needs.max_value() < profile.need_salience_urgency_threshold.value() {
            return 0;
        }
        (u32::from(needs.max_value()) * u32::from(profile.need_salience_boost.value()) / 1000)
            as u16
    };

    match world.entity_kind(entity) {
        Some(EntityKind::Agent) => 900,
        Some(EntityKind::Place) => 800,
        Some(EntityKind::Facility) => 700,
        Some(EntityKind::Office | EntityKind::Record) => 850,
        Some(EntityKind::UniqueItem) => 600,
        Some(EntityKind::Container) => 500,
        Some(EntityKind::Faction) => 450,
        Some(EntityKind::SocialArtifact) => 400,
        Some(EntityKind::ItemLot) => match world.get_component_item_lot(entity) {
            Some(lot) if lot.commodity == CommodityKind::Waste => 100,
            Some(_) => 300 + item_need_boost(),
            None => 300,
        },
        None => 0,
    }
}

fn apply_direct_local_observation_batch(
    event_log: &mut EventLog,
    context: DiscoveryContext,
    store: &mut AgentBeliefStore,
    batch: &DirectLocalObservationBatch,
    profile: &worldwake_core::PerceptionProfile,
    needs: worldwake_core::HomeostaticNeeds,
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
        if let Some(holder) = batch.observed_holders.get(subject) {
            record_direct_authority_claim(
                store,
                *subject,
                EntityBeliefAspect::Holder,
                *holder,
                context.tick,
            );
        }
    }
    for (record, snapshot) in &batch.observed_record_snapshots {
        store.record_believed_record_data(*record, snapshot.clone());
    }
    for (office, snapshot) in &batch.observed_office_snapshots {
        store.record_believed_office_data(*office, snapshot.clone());
    }

    for subject in &batch.noticed_missing_subjects {
        emit_discovery_event(event_log, context, *subject, MismatchKind::EntityMissing);
    }

    for omission in &batch.omitted_observations {
        store.observation_omission_log.entries.push_back(*omission);
    }
    while store.observation_omission_log.entries.len() > usize::from(profile.omission_log_capacity)
    {
        store.observation_omission_log.entries.pop_front();
    }

    if !batch.observed_snapshots.is_empty() {
        store.prune_decayed_beliefs(profile, context.tick, &needs);
    }
}

fn record_direct_authority_claim(
    store: &mut AgentBeliefStore,
    subject: EntityId,
    aspect: EntityBeliefAspect,
    value: Option<EntityId>,
    tick: worldwake_core::Tick,
) {
    store.record_entity_claim(EntityBeliefClaim {
        claim_id: store.next_claim_id,
        subject,
        aspect,
        value: worldwake_core::ClaimValue::Entity(value),
        source: PerceptionSource::DirectObservation,
        acquired_tick: tick,
        claimed_event_tick: Some(tick),
        confidence: Permille::new(1000).expect("1000 permille is valid"),
        refuted_at_tick: None,
    });
}

fn authority_claims_for_event(
    record: &worldwake_core::EventRecord,
) -> BTreeMap<(EntityId, EntityBeliefAspect), Option<EntityId>> {
    let mut claims = BTreeMap::new();
    for delta in record.state_deltas() {
        let StateDelta::Relation(relation_delta) = delta else {
            continue;
        };
        match relation_delta {
            RelationDelta::Added {
                relation: RelationValue::PossessedBy { entity, holder },
                ..
            } => {
                claims.insert((*entity, EntityBeliefAspect::Holder), Some(*holder));
            }
            RelationDelta::Removed {
                relation: RelationValue::PossessedBy { entity, .. },
                ..
            } => {
                claims.insert((*entity, EntityBeliefAspect::Holder), None);
            }
            RelationDelta::Added {
                relation: RelationValue::OwnedBy { entity, owner },
                ..
            } => {
                claims.insert((*entity, EntityBeliefAspect::Owner), Some(*owner));
            }
            RelationDelta::Removed {
                relation: RelationValue::OwnedBy { entity, .. },
                ..
            } => {
                claims.insert((*entity, EntityBeliefAspect::Owner), None);
            }
            _ => {}
        }
    }
    claims
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
        snapshot.last_observed_tick(),
        notice_context.profile.observation_buffer_capacity,
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
    if artifact.actionability != worldwake_core::ArtifactActionability::Actionable {
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
        InstitutionalClaim::ArtifactCredibilityRefutation { artifact, .. } => {
            InstitutionalBeliefKey::ArtifactCredibilityOf { artifact }
        }
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
        contention_event_payload: None,
        decision_payload: None,
        artifact_transition_payload: None,

        personality_assigned_payload: None,
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
    use std::num::{NonZeroU8, NonZeroU32};
    use worldwake_core::{
        ActionDefId, ActionDomain, AgentBeliefStore, ArtifactActionability, ArtifactHeader,
        ArtifactKind, ArtifactLegalEffect, BanditCamp, BanditFactionPolicy, BeliefConfidencePolicy,
        BelievedActivity, BelievedContentionState, BelievedEntityState, BelievedEvidenceEntry,
        BelievedEvidenceState, BountyTarget, BountyTerms, CauseRef, CognitiveProfile,
        CommodityKind, ComponentDelta, ComponentKind, ComponentValue, Container, ContentionGrant,
        ContentionQueue, ContentionWaiter, ControlSource, DeadAt, DecisionEventPayload,
        DisturbanceKind, EntityBeliefAspect, EntityKind, EventId, EventLog, EventPayload, EventTag,
        EventView, EvidenceKind, EvidenceRef, ExplorationMotivation, FrameState, GoalKey, GoalKind,
        GroundComfortTag, HomeostaticNeeds, HypothesisKind, InstitutionalBeliefKey,
        InstitutionalClaim, InstitutionalKnowledgeSource, InstitutionalSnapshotSource,
        IntentionDomain, IntentionFrame, LoadUnits, MismatchKind, NoticeContent, NoticeTopic,
        ObservedEntitySnapshot, OfficeData, OfficeForceState, OmissionReason, PendingEvent,
        PerceptionProfile, PerceptionSource, Permille, PlaceVisibilityProfile,
        ProductionOutputOwner, ProductionOutputOwnershipPolicy, ProofRequirement, PrototypePlace,
        Quantity, RecordData, RecordKind, RelationDelta, RelationKind, RelationValue,
        ReliabilityRecord, ResourceSource, RewardSource, SaleListing, SceneEvidence, Seed,
        ShelterTag, SleepQualityProfile, SleepRecoveryModifier, SocialObservationDetail,
        SocialObservationKind, SourceKey, StateDelta, StockAssignment, StockAssignmentKind,
        SuccessionLaw, SurveyRecordedPayload, TheftFacts, Tick, VisibilitySpec, WitnessData,
        WorkstationMarker, WorkstationTag, World, WorldTxn, build_observed_entity_snapshot,
        build_prototype_world, prototype_place_entity,
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
            observation_fidelity: Permille::new(fidelity).unwrap(),
            confidence_policy: BeliefConfidencePolicy::default(),
            institutional_memory_capacity: 20,
            consultation_speed_factor: Permille::new(500).unwrap(),
            contradiction_tolerance: Permille::new(300).unwrap(),
            entity_activation_threshold: Permille::new(100).unwrap(),
            claim_confidence_threshold: Permille::new(50).unwrap(),
            observation_buffer_capacity: 5,
            observation_budget: 24,
            salience_policy: worldwake_core::SaliencePolicy::default(),
            omission_log_capacity: worldwake_core::default_omission_log_capacity(),
            opportunity_floor_permille: worldwake_core::default_opportunity_floor_permille(),
            need_salience_boost: Permille::new(500).unwrap(),
            need_salience_urgency_threshold: Permille::new(500).unwrap(),
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
            wash_basin_state: None,
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        })
    }

    fn survey_goal(place: worldwake_core::EntityId, hypothesis: HypothesisKind) -> GoalKind {
        GoalKind::ExploreLocation {
            target_place: place,
            motivating_need: ExplorationMotivation::Proactive,
            hypothesis,
        }
    }

    fn active_explore_frame(
        place: worldwake_core::EntityId,
        hypothesis: HypothesisKind,
    ) -> IntentionFrame {
        IntentionFrame {
            goal: GoalKey::from(survey_goal(place, hypothesis)),
            domain: IntentionDomain::Travel { destination: place },
            assumptions: vec![],
            state: FrameState::Active,
            established_at: Tick(1),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 30,
            motive_refs: Vec::new(),
            resume_conditions: Vec::new(),
            abandon_conditions: Vec::new(),
            explicit_claims: Vec::new(),
            causal_links: Vec::new(),
        }
    }

    fn run_perception(world: &mut World, event_log: &mut EventLog, tick: u64) {
        let mut rng = DeterministicRng::new(Seed([0x77; 32]));
        perception_system(SystemExecutionContext {
            world,
            event_log,
            rng: &mut rng,
            active_actions: &BTreeMap::new(),
            action_defs: &ActionDefRegistry::new(),
            politics_trace: None,
            perception_trace: None,
            tick: Tick(tick),
            system_id: SystemId::Perception,
        })
        .unwrap();
    }

    fn survey_payloads(event_log: &EventLog) -> Vec<SurveyRecordedPayload> {
        event_log
            .events_by_tag(EventTag::SurveyRecorded)
            .iter()
            .filter_map(|event_id| event_log.get(*event_id))
            .filter_map(|event| match event.decision_payload() {
                Some(DecisionEventPayload::SurveyRecorded(payload)) => Some(payload.clone()),
                _ => None,
            })
            .collect()
    }

    fn setup_survey_agent(
        world: &mut World,
        place: worldwake_core::EntityId,
        hypothesis: HypothesisKind,
    ) -> worldwake_core::EntityId {
        let mut txn = new_txn(world, 1);
        let agent = txn.create_agent("Surveyor", ControlSource::Ai).unwrap();
        txn.set_ground_location(agent, place).unwrap();
        txn.set_component_agent_belief_store(agent, AgentBeliefStore::new())
            .unwrap();
        txn.set_component_perception_profile(agent, profile(1000))
            .unwrap();
        txn.set_component_cognitive_profile(
            agent,
            CognitiveProfile {
                survey_memory_capacity: 8,
                ..CognitiveProfile::default()
            },
        )
        .unwrap();
        txn.set_component_intention_frame(agent, active_explore_frame(place, hypothesis))
            .unwrap();
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
        agent
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
    fn arrival_with_negative_commodity_hypothesis_writes_negative_survey() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let hypothesis = HypothesisKind::MayContainCommodity {
            commodity: CommodityKind::Apple,
        };
        let agent = setup_survey_agent(&mut world, place, hypothesis);
        let mut event_log = EventLog::new();

        run_perception(&mut world, &mut event_log, 3);

        let record = world
            .get_component_survey_memory(agent)
            .and_then(|memory| memory.find(place, hypothesis))
            .expect("arrival should record survey");
        assert!(!record.found);
        assert_eq!(record.confidence, Permille::new(1000).unwrap());
        assert_eq!(record.recorded_tick, Tick(3));
        let payloads = survey_payloads(&event_log);
        assert_eq!(payloads.len(), 1);
        assert_eq!(
            payloads[0],
            SurveyRecordedPayload {
                surveyor: agent,
                place,
                hypothesis,
                found: false,
                confidence: Permille::new(1000).unwrap(),
            }
        );
    }

    #[test]
    fn arrival_with_positive_commodity_hypothesis_writes_positive_survey() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let hypothesis = HypothesisKind::MayContainCommodity {
            commodity: CommodityKind::Apple,
        };
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let source = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(source, place).unwrap();
            txn.set_component_resource_source(
                source,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(3),
                    max_quantity: Quantity(10),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                    extraction_slots: NonZeroU8::new(1).unwrap(),
                    extraction_duration_ticks: NonZeroU32::new(1).unwrap(),
                    quality: None,
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            setup_survey_agent(&mut world, place, hypothesis)
        };
        let mut event_log = EventLog::new();

        run_perception(&mut world, &mut event_log, 3);

        let record = world
            .get_component_survey_memory(agent)
            .and_then(|memory| memory.find(place, hypothesis))
            .expect("arrival should record survey");
        assert!(record.found);
        assert!(survey_payloads(&event_log)[0].found);
    }

    #[test]
    fn arrival_with_item_lot_satisfies_commodity_hypothesis() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let hypothesis = HypothesisKind::MayContainCommodity {
            commodity: CommodityKind::Apple,
        };
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let lot = txn
                .create_item_lot(CommodityKind::Apple, Quantity(2))
                .unwrap();
            txn.set_ground_location(lot, place).unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            setup_survey_agent(&mut world, place, hypothesis)
        };
        let mut event_log = EventLog::new();

        run_perception(&mut world, &mut event_log, 3);

        assert!(
            world
                .get_component_survey_memory(agent)
                .and_then(|memory| memory.find(place, hypothesis))
                .is_some_and(|record| record.found)
        );
    }

    #[test]
    fn arrival_with_latrine_hypothesis_uses_place_tag() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::PublicLatrine);
        let hypothesis = HypothesisKind::MayContainLatrine;
        let agent = setup_survey_agent(&mut world, place, hypothesis);
        let mut event_log = EventLog::new();

        run_perception(&mut world, &mut event_log, 3);

        assert!(
            world
                .get_component_survey_memory(agent)
                .and_then(|memory| memory.find(place, hypothesis))
                .is_some_and(|record| record.found)
        );
    }

    #[test]
    fn arrival_with_wash_basin_hypothesis_uses_workstation_tag() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let hypothesis = HypothesisKind::MayContainWashBasin;
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let basin = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(basin, place).unwrap();
            txn.set_component_workstation_marker(
                basin,
                WorkstationMarker(WorkstationTag::WashBasin),
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            setup_survey_agent(&mut world, place, hypothesis)
        };
        let mut event_log = EventLog::new();

        run_perception(&mut world, &mut event_log, 3);

        assert!(
            world
                .get_component_survey_memory(agent)
                .and_then(|memory| memory.find(place, hypothesis))
                .is_some_and(|record| record.found)
        );
    }

    #[test]
    fn arrival_with_sleep_site_hypothesis_requires_recovery_modifier_above_universal_default() {
        let mut default_world = World::new(build_prototype_world()).unwrap();
        let default_place = default_world.topology().place_ids().next().unwrap();
        let hypothesis = HypothesisKind::MayContainSleepSite;
        let default_agent = setup_survey_agent(&mut default_world, default_place, hypothesis);
        let mut default_log = EventLog::new();

        run_perception(&mut default_world, &mut default_log, 3);

        assert!(
            default_world
                .get_component_survey_memory(default_agent)
                .and_then(|memory| memory.find(default_place, hypothesis))
                .is_some_and(|record| !record.found)
        );

        let mut better_world = World::new(build_prototype_world()).unwrap();
        let better_place = better_world.topology().place_ids().next().unwrap();
        {
            let mut txn = new_txn(&mut better_world, 1);
            txn.set_component_sleep_quality_profile(
                better_place,
                SleepQualityProfile {
                    shelter: ShelterTag::Shelter,
                    ground_comfort: GroundComfortTag::Soft,
                    recovery_modifier: SleepRecoveryModifier::new(1250),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }
        let better_agent = setup_survey_agent(&mut better_world, better_place, hypothesis);
        let mut better_log = EventLog::new();

        run_perception(&mut better_world, &mut better_log, 3);

        assert!(
            better_world
                .get_component_survey_memory(better_agent)
                .and_then(|memory| memory.find(better_place, hypothesis))
                .is_some_and(|record| record.found)
        );
    }

    #[test]
    fn arrival_with_proactive_hypothesis_always_writes_positive_survey() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let hypothesis = HypothesisKind::Proactive;
        let agent = setup_survey_agent(&mut world, place, hypothesis);
        let mut event_log = EventLog::new();

        run_perception(&mut world, &mut event_log, 3);

        assert!(
            world
                .get_component_survey_memory(agent)
                .and_then(|memory| memory.find(place, hypothesis))
                .is_some_and(|record| record.found)
        );
    }

    #[test]
    fn arrival_without_active_explore_location_writes_no_survey() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let mut txn = new_txn(&mut world, 1);
        let agent = txn.create_agent("Surveyor", ControlSource::Ai).unwrap();
        txn.set_ground_location(agent, place).unwrap();
        txn.set_component_agent_belief_store(agent, AgentBeliefStore::new())
            .unwrap();
        txn.set_component_perception_profile(agent, profile(1000))
            .unwrap();
        txn.set_component_intention_frame(
            agent,
            IntentionFrame {
                goal: GoalKey::from(GoalKind::Sleep),
                domain: IntentionDomain::Generic,
                assumptions: vec![],
                state: FrameState::Active,
                established_at: Tick(1),
                last_progress_tick: None,
                stalled_ticks: 0,
                patience_limit: 30,
                motive_refs: Vec::new(),
                resume_conditions: Vec::new(),
                abandon_conditions: Vec::new(),
                explicit_claims: Vec::new(),
                causal_links: Vec::new(),
            },
        )
        .unwrap();
        let mut bootstrap_log = EventLog::new();
        let _ = txn.commit(&mut bootstrap_log);
        let mut event_log = EventLog::new();

        run_perception(&mut world, &mut event_log, 3);

        assert!(
            world
                .get_component_survey_memory(agent)
                .is_none_or(|memory| memory.entries.is_empty())
        );
        assert!(survey_payloads(&event_log).is_empty());
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
        assert_eq!(belief.last_observed_tick(), Some(Tick(3)));
        assert_eq!(belief.source, PerceptionSource::DirectObservation);
    }

    #[test]
    fn passive_perception_projects_local_record_and_office_snapshots() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::VillageSquare);
        let (observer, record, record_data, office, office_data) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();

            let office = txn.create_office("Market Warden").unwrap();
            txn.set_ground_location(office, place).unwrap();
            let office_data = OfficeData {
                title: "Market Warden".to_string(),
                seat: place,
                jurisdiction: BTreeSet::from([place]),
                succession_law: SuccessionLaw::Force,
                eligibility_rules: Vec::new(),
                succession_period_ticks: 4,
                vacancy_since: Some(Tick(0)),
            };
            txn.set_component_office_data(office, office_data.clone())
                .unwrap();

            let record_data = RecordData {
                record_kind: RecordKind::OfficeRegister,
                home_place: place,
                issuer: office,
                consultation_ticks: 4,
                max_entries_per_consult: 6,
                entries: Vec::new(),
                next_entry_id: 0,
            };
            let record = txn.create_record(record_data.clone()).unwrap();
            txn.set_ground_location(record, place).unwrap();

            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, record, record_data, office, office_data)
        };
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([0x25; 32]));

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

        let store = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        let believed_record = store
            .believed_record_data(record)
            .expect("local record observation should project RecordData");
        assert_eq!(believed_record.data, record_data);
        assert_eq!(
            believed_record.source,
            InstitutionalSnapshotSource::DirectObservation
        );
        assert_eq!(believed_record.learned_tick, Tick(3));
        assert_eq!(believed_record.learned_at, Some(place));

        let believed_office = store
            .believed_office_data(office)
            .expect("local office observation should project OfficeData");
        assert_eq!(believed_office.data, office_data);
        assert_eq!(
            believed_office.source,
            InstitutionalSnapshotSource::DirectObservation
        );
        assert_eq!(believed_office.learned_tick, Tick(3));
        assert_eq!(believed_office.learned_at, Some(place));
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
        assert_eq!(belief.last_observed_tick(), Some(Tick(5)));
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
        assert_eq!(believed.last_observed_tick(), Some(Tick(3)));
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
            observer_profile.entity_activation_threshold = Permille::new(50).unwrap();
            observer_profile.observation_buffer_capacity = 8;
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
        assert_eq!(believed.last_observed_tick(), Some(Tick(3)));
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
            observer_profile.entity_activation_threshold = Permille::new(50).unwrap();
            observer_profile.observation_buffer_capacity = 8;
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
    fn passive_local_observation_applies_budget_priority_to_non_place_entities() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, other_agent, facilities, waste_lots) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let other_agent = txn.create_agent("Other Agent", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(other_agent, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_homeostatic_needs(observer, HomeostaticNeeds::new_sated())
                .unwrap();
            let mut observer_profile = profile(1000);
            observer_profile.observation_budget = 10;
            txn.set_component_perception_profile(observer, observer_profile)
                .unwrap();

            let facilities = (0..2)
                .map(|_| {
                    let facility = txn.create_entity(EntityKind::Facility);
                    txn.set_ground_location(facility, place).unwrap();
                    facility
                })
                .collect::<Vec<_>>();
            let waste_lots = (0..30)
                .map(|_| {
                    let lot = txn
                        .create_item_lot(CommodityKind::Waste, Quantity(1))
                        .unwrap();
                    txn.set_ground_location(lot, place).unwrap();
                    lot
                })
                .collect::<Vec<_>>();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, other_agent, facilities, waste_lots)
        };

        let observer_profile = world
            .get_component_perception_profile(observer)
            .copied()
            .unwrap();
        let needs = world
            .get_component_homeostatic_needs(observer)
            .copied()
            .unwrap();
        let base_store = world
            .get_component_agent_belief_store(observer)
            .cloned()
            .unwrap_or_default();
        let colocated_entities = world.entities_effectively_at(place);
        let mut rng = DeterministicRng::new(Seed([0x61; 32]));

        let batch = super::collect_direct_local_observation_batch(
            &world,
            observer,
            place,
            &colocated_entities,
            Tick(2),
            1000,
            &mut rng,
            &base_store,
            needs,
            &observer_profile,
        )
        .expect("budgeted same-place observation should produce a batch");

        assert!(
            batch.observed_snapshots.contains_key(&place),
            "place observation should remain separate from the budgeted entity set"
        );
        let observed_non_place_entities = batch
            .observed_snapshots
            .keys()
            .copied()
            .filter(|entity| *entity != place)
            .collect::<Vec<_>>();
        assert_eq!(
            observed_non_place_entities.len(),
            usize::from(observer_profile.observation_budget)
        );
        assert!(
            batch.observed_snapshots.contains_key(&other_agent),
            "other colocated agents should outrank waste"
        );
        for facility in &facilities {
            assert!(
                batch.observed_snapshots.contains_key(facility),
                "facilities should outrank waste under the observation budget"
            );
        }

        let retained_waste = waste_lots
            .iter()
            .copied()
            .filter(|lot| batch.observed_snapshots.contains_key(lot))
            .collect::<Vec<_>>();
        assert_eq!(retained_waste.len(), 7);
        assert_eq!(
            retained_waste,
            waste_lots[..7].to_vec(),
            "same-priority waste lots should be selected by lowest EntityId"
        );
    }

    #[test]
    fn passive_local_observation_records_overbudget_omissions() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, waste_lots) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_homeostatic_needs(observer, HomeostaticNeeds::new_sated())
                .unwrap();
            let mut observer_profile = profile(1000);
            observer_profile.observation_budget = 12;
            observer_profile.omission_log_capacity = 30;
            txn.set_component_perception_profile(observer, observer_profile)
                .unwrap();

            let waste_lots = (0..30)
                .map(|_| {
                    let lot = txn
                        .create_item_lot(CommodityKind::Waste, Quantity(1))
                        .unwrap();
                    txn.set_ground_location(lot, place).unwrap();
                    lot
                })
                .collect::<Vec<_>>();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, waste_lots)
        };

        let observer_profile = world
            .get_component_perception_profile(observer)
            .copied()
            .unwrap();
        let needs = world
            .get_component_homeostatic_needs(observer)
            .copied()
            .unwrap();
        let mut store = world
            .get_component_agent_belief_store(observer)
            .cloned()
            .unwrap_or_default();
        let colocated_entities = world.entities_effectively_at(place);
        let mut rng = DeterministicRng::new(Seed([0x63; 32]));
        let batch = super::collect_direct_local_observation_batch(
            &world,
            observer,
            place,
            &colocated_entities,
            Tick(2),
            1000,
            &mut rng,
            &store,
            needs,
            &observer_profile,
        )
        .expect("budget truncation should produce a batch");
        let mut event_log = EventLog::new();
        super::apply_direct_local_observation_batch(
            &mut event_log,
            super::DiscoveryContext {
                tick: Tick(2),
                observer,
                place: Some(place),
            },
            &mut store,
            &batch,
            &observer_profile,
            needs,
        );

        let observed_non_place_count = store
            .known_entities
            .keys()
            .filter(|entity| **entity != place)
            .count();
        assert_eq!(observed_non_place_count, 12);
        let omitted = store
            .observation_omission_log
            .entries
            .iter()
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(omitted.len(), 18);
        assert_eq!(
            omitted
                .iter()
                .map(|entry| entry.omitted_entity)
                .collect::<Vec<_>>(),
            waste_lots[12..].to_vec()
        );
        assert!(omitted.iter().all(|entry| {
            entry.reason
                == OmissionReason::OverBudget {
                    budget: 12,
                    candidates_seen: 30,
                }
                && entry.observed_tick == Tick(2)
        }));
    }

    #[test]
    fn passive_local_observation_omission_log_evicts_fifo() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, waste_lots) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_homeostatic_needs(observer, HomeostaticNeeds::new_sated())
                .unwrap();
            let mut observer_profile = profile(1000);
            observer_profile.observation_budget = 10;
            observer_profile.omission_log_capacity = 5;
            txn.set_component_perception_profile(observer, observer_profile)
                .unwrap();

            let waste_lots = (0..30)
                .map(|_| {
                    let lot = txn
                        .create_item_lot(CommodityKind::Waste, Quantity(1))
                        .unwrap();
                    txn.set_ground_location(lot, place).unwrap();
                    lot
                })
                .collect::<Vec<_>>();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, waste_lots)
        };

        let observer_profile = world
            .get_component_perception_profile(observer)
            .copied()
            .unwrap();
        let needs = world
            .get_component_homeostatic_needs(observer)
            .copied()
            .unwrap();
        let mut store = world
            .get_component_agent_belief_store(observer)
            .cloned()
            .unwrap_or_default();
        let colocated_entities = world.entities_effectively_at(place);
        let mut rng = DeterministicRng::new(Seed([0x64; 32]));
        let batch = super::collect_direct_local_observation_batch(
            &world,
            observer,
            place,
            &colocated_entities,
            Tick(2),
            1000,
            &mut rng,
            &store,
            needs,
            &observer_profile,
        )
        .expect("budget truncation should produce a batch");
        let mut event_log = EventLog::new();
        super::apply_direct_local_observation_batch(
            &mut event_log,
            super::DiscoveryContext {
                tick: Tick(2),
                observer,
                place: Some(place),
            },
            &mut store,
            &batch,
            &observer_profile,
            needs,
        );

        let retained = store
            .observation_omission_log
            .entries
            .iter()
            .map(|entry| entry.omitted_entity)
            .collect::<Vec<_>>();
        assert_eq!(retained, waste_lots[25..].to_vec());
    }

    #[test]
    fn passive_local_observation_omissions_keep_stable_order_across_ticks() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, waste_lots) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_homeostatic_needs(observer, HomeostaticNeeds::new_sated())
                .unwrap();
            let mut observer_profile = profile(1000);
            observer_profile.observation_budget = 12;
            observer_profile.omission_log_capacity = 100;
            txn.set_component_perception_profile(observer, observer_profile)
                .unwrap();

            let waste_lots = (0..30)
                .map(|_| {
                    let lot = txn
                        .create_item_lot(CommodityKind::Waste, Quantity(1))
                        .unwrap();
                    txn.set_ground_location(lot, place).unwrap();
                    lot
                })
                .collect::<Vec<_>>();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, waste_lots)
        };

        let observer_profile = world
            .get_component_perception_profile(observer)
            .copied()
            .unwrap();
        let needs = world
            .get_component_homeostatic_needs(observer)
            .copied()
            .unwrap();
        let mut store = world
            .get_component_agent_belief_store(observer)
            .cloned()
            .unwrap_or_default();
        let colocated_entities = world.entities_effectively_at(place);
        let mut event_log = EventLog::new();
        for tick in 2..5 {
            let mut rng = DeterministicRng::new(Seed([tick as u8; 32]));
            let batch = super::collect_direct_local_observation_batch(
                &world,
                observer,
                place,
                &colocated_entities,
                Tick(tick),
                1000,
                &mut rng,
                &store,
                needs,
                &observer_profile,
            )
            .expect("budget truncation should produce a batch");
            super::apply_direct_local_observation_batch(
                &mut event_log,
                super::DiscoveryContext {
                    tick: Tick(tick),
                    observer,
                    place: Some(place),
                },
                &mut store,
                &batch,
                &observer_profile,
                needs,
            );
        }

        for tick in 2..5 {
            let omitted_for_tick = store
                .observation_omission_log
                .entries
                .iter()
                .filter(|entry| entry.observed_tick == Tick(tick))
                .map(|entry| entry.omitted_entity)
                .collect::<Vec<_>>();
            assert_eq!(omitted_for_tick, waste_lots[12..].to_vec());
        }
    }

    #[test]
    fn passive_local_observation_omissions_are_disjoint_from_known_entities() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let observer = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_homeostatic_needs(observer, HomeostaticNeeds::new_sated())
                .unwrap();
            let mut observer_profile = profile(1000);
            observer_profile.observation_budget = 12;
            observer_profile.omission_log_capacity = 100;
            txn.set_component_perception_profile(observer, observer_profile)
                .unwrap();

            for _ in 0..30 {
                let lot = txn
                    .create_item_lot(CommodityKind::Waste, Quantity(1))
                    .unwrap();
                txn.set_ground_location(lot, place).unwrap();
            }
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            observer
        };

        let observer_profile = world
            .get_component_perception_profile(observer)
            .copied()
            .unwrap();
        let needs = world
            .get_component_homeostatic_needs(observer)
            .copied()
            .unwrap();
        let mut store = world
            .get_component_agent_belief_store(observer)
            .cloned()
            .unwrap_or_default();
        let colocated_entities = world.entities_effectively_at(place);
        let mut event_log = EventLog::new();
        for tick in 2..7 {
            let mut rng = DeterministicRng::new(Seed([tick as u8; 32]));
            let batch = super::collect_direct_local_observation_batch(
                &world,
                observer,
                place,
                &colocated_entities,
                Tick(tick),
                1000,
                &mut rng,
                &store,
                needs,
                &observer_profile,
            )
            .expect("budget truncation should produce a batch");
            super::apply_direct_local_observation_batch(
                &mut event_log,
                super::DiscoveryContext {
                    tick: Tick(tick),
                    observer,
                    place: Some(place),
                },
                &mut store,
                &batch,
                &observer_profile,
                needs,
            );
        }

        let known = store
            .known_entities
            .keys()
            .copied()
            .collect::<BTreeSet<_>>();
        let omitted = store
            .observation_omission_log
            .entries
            .iter()
            .map(|entry| entry.omitted_entity)
            .collect::<BTreeSet<_>>();
        assert!(known.is_disjoint(&omitted));
    }

    #[test]
    fn passive_local_observation_boosts_non_waste_item_lots_when_needs_are_urgent() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let urgent_needs = HomeostaticNeeds::new(
            Permille::new(800).unwrap(),
            Permille::ZERO,
            Permille::ZERO,
            Permille::ZERO,
            Permille::ZERO,
        );
        let (observer, apple_lots, waste_lots) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_homeostatic_needs(observer, urgent_needs)
                .unwrap();
            let mut observer_profile = profile(1000);
            observer_profile.observation_budget = 8;
            observer_profile.need_salience_urgency_threshold = Permille::new(400).unwrap();
            observer_profile.need_salience_boost = Permille::new(500).unwrap();
            txn.set_component_perception_profile(observer, observer_profile)
                .unwrap();

            let apple_lots = (0..5)
                .map(|_| {
                    let lot = txn
                        .create_item_lot(CommodityKind::Apple, Quantity(1))
                        .unwrap();
                    txn.set_ground_location(lot, place).unwrap();
                    lot
                })
                .collect::<Vec<_>>();
            let waste_lots = (0..10)
                .map(|_| {
                    let lot = txn
                        .create_item_lot(CommodityKind::Waste, Quantity(1))
                        .unwrap();
                    txn.set_ground_location(lot, place).unwrap();
                    lot
                })
                .collect::<Vec<_>>();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, apple_lots, waste_lots)
        };

        let observer_profile = world
            .get_component_perception_profile(observer)
            .copied()
            .unwrap();
        let base_store = world
            .get_component_agent_belief_store(observer)
            .cloned()
            .unwrap_or_default();
        let colocated_entities = world.entities_effectively_at(place);
        let mut rng = DeterministicRng::new(Seed([0x62; 32]));

        let batch = super::collect_direct_local_observation_batch(
            &world,
            observer,
            place,
            &colocated_entities,
            Tick(2),
            1000,
            &mut rng,
            &base_store,
            urgent_needs,
            &observer_profile,
        )
        .expect("urgent-need observation should produce a batch");

        let retained_apples = apple_lots
            .iter()
            .copied()
            .filter(|lot| batch.observed_snapshots.contains_key(lot))
            .collect::<Vec<_>>();
        let retained_waste = waste_lots
            .iter()
            .copied()
            .filter(|lot| batch.observed_snapshots.contains_key(lot))
            .collect::<Vec<_>>();

        assert_eq!(
            retained_apples, apple_lots,
            "urgent non-waste item lots should fill the budget before waste"
        );
        assert_eq!(retained_waste.len(), 3);
        assert_eq!(
            retained_waste,
            waste_lots[..3].to_vec(),
            "remaining budget should admit only the lowest-EntityId waste lots"
        );
    }

    #[test]
    fn passive_local_observation_keeps_institutional_records_under_need_pressure() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let urgent_needs = HomeostaticNeeds::new(
            Permille::new(1000).unwrap(),
            Permille::ZERO,
            Permille::ZERO,
            Permille::ZERO,
            Permille::ZERO,
        );
        let (observer, office, record, apple_lots) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_homeostatic_needs(observer, urgent_needs)
                .unwrap();
            let mut observer_profile = profile(1000);
            observer_profile.observation_budget = 3;
            observer_profile.need_salience_urgency_threshold = Permille::new(400).unwrap();
            observer_profile.need_salience_boost = Permille::new(500).unwrap();
            txn.set_component_perception_profile(observer, observer_profile)
                .unwrap();

            let office = txn.create_office("Market Warden").unwrap();
            txn.set_ground_location(office, place).unwrap();
            txn.set_component_office_data(
                office,
                OfficeData {
                    title: "Market Warden".to_string(),
                    seat: place,
                    jurisdiction: BTreeSet::from([place]),
                    succession_law: SuccessionLaw::Force,
                    eligibility_rules: Vec::new(),
                    succession_period_ticks: 4,
                    vacancy_since: Some(Tick(0)),
                },
            )
            .unwrap();

            let record = txn
                .create_record(RecordData {
                    record_kind: RecordKind::CrimeRegister,
                    home_place: place,
                    issuer: office,
                    consultation_ticks: 4,
                    max_entries_per_consult: 6,
                    entries: Vec::new(),
                    next_entry_id: 0,
                })
                .unwrap();
            txn.set_ground_location(record, place).unwrap();

            let apple_lots = (0..6)
                .map(|_| {
                    let lot = txn
                        .create_item_lot(CommodityKind::Apple, Quantity(1))
                        .unwrap();
                    txn.set_ground_location(lot, place).unwrap();
                    lot
                })
                .collect::<Vec<_>>();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, office, record, apple_lots)
        };

        let observer_profile = world
            .get_component_perception_profile(observer)
            .copied()
            .unwrap();
        let base_store = world
            .get_component_agent_belief_store(observer)
            .cloned()
            .unwrap_or_default();
        let colocated_entities = world.entities_effectively_at(place);
        let mut rng = DeterministicRng::new(Seed([0x64; 32]));

        let batch = super::collect_direct_local_observation_batch(
            &world,
            observer,
            place,
            &colocated_entities,
            Tick(2),
            1000,
            &mut rng,
            &base_store,
            urgent_needs,
            &observer_profile,
        )
        .expect("urgent-need observation should produce a batch");

        assert!(batch.observed_snapshots.contains_key(&office));
        assert!(batch.observed_snapshots.contains_key(&record));
        assert!(batch.observed_record_snapshots.contains_key(&record));
        assert!(batch.observed_office_snapshots.contains_key(&office));
        let retained_apples = apple_lots
            .iter()
            .filter(|lot| batch.observed_snapshots.contains_key(lot))
            .count();
        assert_eq!(
            retained_apples, 1,
            "record and office carriers should outrank need-boosted item lots"
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
    fn new_observation_does_not_hard_cap_existing_entity_beliefs() {
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
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(1),
                        PerceptionSource::DirectObservation,
                    )
                },
            );
            txn.set_component_agent_belief_store(observer, store)
                .unwrap();
            txn.set_component_perception_profile(
                observer,
                PerceptionProfile {
                    observation_fidelity: Permille::new(1000).unwrap(),
                    confidence_policy: BeliefConfidencePolicy::default(),
                    institutional_memory_capacity: 20,
                    consultation_speed_factor: Permille::new(500).unwrap(),
                    contradiction_tolerance: Permille::new(300).unwrap(),
                    entity_activation_threshold: Permille::new(100).unwrap(),
                    claim_confidence_threshold: Permille::new(50).unwrap(),
                    observation_buffer_capacity: 5,
                    observation_budget: 24,
                    salience_policy: worldwake_core::SaliencePolicy::default(),
                    omission_log_capacity: worldwake_core::default_omission_log_capacity(),
                    opportunity_floor_permille: worldwake_core::default_opportunity_floor_permille(
                    ),
                    need_salience_boost: Permille::new(500).unwrap(),
                    need_salience_urgency_threshold: Permille::new(500).unwrap(),
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
        assert!(beliefs.get_entity(&older_target).is_some());
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
        assert_eq!(target_belief.last_observed_tick(), Some(Tick(3)));
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
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(2),
                        PerceptionSource::DirectObservation,
                    )
                },
            );
            txn.set_component_agent_belief_store(observer, beliefs)
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            txn.set_component_dead_at(
                target,
                DeadAt {
                    tick: Tick(3),
                    cause: worldwake_core::DeathCause::CombatWounds,
                },
            )
            .unwrap();
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
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(2),
                        PerceptionSource::DirectObservation,
                    )
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
                        extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                        extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                        quality: None,
                    }),
                    alive: true,
                    wounds: Vec::new(),
                    last_known_courage: None,
                    believed_activity: None,
                    believed_artifact: None,
                    believed_contention: None,
                    believed_evidence: None,
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(2),
                        PerceptionSource::DirectObservation,
                    )
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
                    extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                    extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
                    quality: None,
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
    fn perception_writes_capacity_observation_for_co_located_resource_source() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, source) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let source = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(source, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            txn.set_component_resource_source(
                source,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(18),
                    max_quantity: Quantity(20),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                    extraction_slots: NonZeroU8::new(1).unwrap(),
                    extraction_duration_ticks: NonZeroU32::new(1).unwrap(),
                    quality: None,
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, source)
        };
        let mut event_log = EventLog::new();

        run_perception(&mut world, &mut event_log, 100);

        let key = SourceKey {
            entity: source,
            commodity: CommodityKind::Apple,
        };
        let record = world
            .get_component_source_reliability(observer)
            .and_then(|reliability| reliability.sources.get(&key))
            .copied()
            .expect("perception should record observed capacity");
        assert_eq!(
            record,
            ReliabilityRecord {
                last_observed_capacity: 18,
                last_observed_capacity_tick: Tick(100),
                provenance_events: [Some(EventId(0)), None, None, None, None, None, None, None,],
                ..ReliabilityRecord::new(Tick(100))
            }
        );
    }

    #[test]
    fn perception_overwrites_capacity_observation_on_subsequent_tick() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (observer, source) = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            let source = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(observer, place).unwrap();
            txn.set_ground_location(source, place).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            txn.set_component_resource_source(
                source,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(18),
                    max_quantity: Quantity(20),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                    extraction_slots: NonZeroU8::new(1).unwrap(),
                    extraction_duration_ticks: NonZeroU32::new(1).unwrap(),
                    quality: None,
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, source)
        };
        let mut event_log = EventLog::new();

        let first_capacity_event = event_log.next_id();
        run_perception(&mut world, &mut event_log, 100);
        {
            let mut txn = new_txn(&mut world, 150);
            txn.set_component_resource_source(
                source,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(5),
                    max_quantity: Quantity(20),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                    extraction_slots: NonZeroU8::new(1).unwrap(),
                    extraction_duration_ticks: NonZeroU32::new(1).unwrap(),
                    quality: None,
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        run_perception(&mut world, &mut event_log, 200);

        let key = SourceKey {
            entity: source,
            commodity: CommodityKind::Apple,
        };
        let record = world
            .get_component_source_reliability(observer)
            .and_then(|reliability| reliability.sources.get(&key))
            .copied()
            .expect("perception should refresh observed capacity");
        assert_eq!(record.last_observed_capacity, 5);
        assert_eq!(record.last_observed_capacity_tick, Tick(200));
        assert_eq!(record.successful_acquisitions, 0);
        assert_eq!(record.failed_attempts, 0);
        assert_eq!(record.last_attempt_tick, Tick(100));
        assert_eq!(
            record.provenance_events,
            [
                Some(first_capacity_event),
                Some(EventId(2)),
                None,
                None,
                None,
                None,
                None,
                None,
            ]
        );
        assert_eq!(record.average_wait_ticks, 0);
        assert_eq!(record.wait_observation_count, 0);
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
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(2),
                        PerceptionSource::DirectObservation,
                    )
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
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(2),
                        PerceptionSource::DirectObservation,
                    )
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
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(2),
                        PerceptionSource::DirectObservation,
                    )
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
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(2),
                        PerceptionSource::DirectObservation,
                    )
                },
            );
            txn.set_component_agent_belief_store(observer, beliefs)
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            txn.set_component_dead_at(
                target,
                DeadAt {
                    tick: Tick(3),
                    cause: worldwake_core::DeathCause::CombatWounds,
                },
            )
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(2),
                        PerceptionSource::DirectObservation,
                    )
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(2),
                        PerceptionSource::DirectObservation,
                    )
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(2),
                        PerceptionSource::DirectObservation,
                    )
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
                    ..BelievedEntityState::single_observation_defaults(
                        Tick(2),
                        PerceptionSource::DirectObservation,
                    )
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
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,

            personality_assigned_payload: None,
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
                ArtifactHeader::posted_active(
                    ArtifactKind::Bounty,
                    issuer,
                    None,
                    Tick(1),
                    Some(Tick(8)),
                    None,
                    place,
                ),
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
        assert_eq!(belief.actionability, ArtifactActionability::Actionable);
        assert_eq!(
            belief.legal_effect,
            ArtifactLegalEffect::Active {
                expires_at: Some(Tick(8))
            }
        );
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
                ArtifactHeader::posted_active(
                    ArtifactKind::Notice,
                    issuer,
                    None,
                    Tick(1),
                    None,
                    None,
                    place,
                ),
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
                ArtifactHeader::posted_active(
                    ArtifactKind::Notice,
                    issuer,
                    None,
                    Tick(1),
                    None,
                    None,
                    place,
                ),
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
    fn passive_perception_updates_place_visits_across_return_cycle() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let home = places[0];
        let away = world.topology().neighbors(home)[0];

        let observer = {
            let mut txn = new_txn(&mut world, 1);
            let observer = txn.create_agent("Observer", ControlSource::Ai).unwrap();
            txn.set_ground_location(observer, home).unwrap();
            txn.set_component_agent_belief_store(observer, AgentBeliefStore::new())
                .unwrap();
            txn.set_component_perception_profile(observer, profile(1000))
                .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            observer
        };

        let mut event_log = EventLog::new();
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        let run_tick = |tick: u64, world: &mut World, event_log: &mut EventLog| {
            let mut rng = DeterministicRng::new(Seed([tick as u8; 32]));
            perception_system(SystemExecutionContext {
                world,
                event_log,
                rng: &mut rng,
                active_actions: &active_actions,
                action_defs: &action_defs,
                politics_trace: None,
                perception_trace: None,
                tick: Tick(tick),
                system_id: SystemId::Perception,
            })
            .unwrap();
        };

        run_tick(2, &mut world, &mut event_log);
        run_tick(3, &mut world, &mut event_log);

        {
            let mut txn = new_txn(&mut world, 4);
            txn.set_ground_location(observer, away).unwrap();
            let _ = txn.commit(&mut event_log);
        }
        run_tick(4, &mut world, &mut event_log);
        run_tick(5, &mut world, &mut event_log);

        {
            let mut txn = new_txn(&mut world, 6);
            txn.set_ground_location(observer, home).unwrap();
            let _ = txn.commit(&mut event_log);
        }
        run_tick(6, &mut world, &mut event_log);

        let visits = &world
            .get_component_agent_belief_store(observer)
            .unwrap()
            .place_visits;
        assert_eq!(
            visits.get(&home),
            Some(&worldwake_core::PlaceVisitRecord {
                ticks_present: 0,
                last_arrival_tick: Tick(6),
                visit_count: 2,
            })
        );
        assert_eq!(
            visits.get(&away),
            Some(&worldwake_core::PlaceVisitRecord {
                ticks_present: 1,
                last_arrival_tick: Tick(4),
                visit_count: 1,
            })
        );
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
