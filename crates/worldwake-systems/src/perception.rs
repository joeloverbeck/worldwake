use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    build_believed_entity_state, AgentBeliefStore, CauseRef, EntityId, EntityKind, EventLog,
    EventRecord, EventTag, EvidenceRef, MismatchKind, PendingEvent, PerceptionSource,
    SocialObservation, SocialObservationKind, VisibilitySpec, WitnessData, World, WorldTxn,
};
use worldwake_sim::{SystemError, SystemExecutionContext};

#[derive(Copy, Clone)]
struct DiscoveryContext {
    tick: worldwake_core::Tick,
    observer: EntityId,
    place: Option<EntityId>,
}

pub fn perception_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let SystemExecutionContext {
        world,
        event_log,
        rng,
        active_actions: _active_actions,
        action_defs: _action_defs,
        tick,
        system_id: _system_id,
    } = ctx;
    let event_ids = event_log.events_at_tick(tick).to_vec();
    let mut updated_stores = BTreeMap::<EntityId, AgentBeliefStore>::new();

    observe_passive_local_entities(world, event_log, tick, rng, &mut updated_stores);

    for event_id in event_ids {
        let Some(record) = event_log.get(event_id).cloned() else {
            continue;
        };
        let social_observations = social_observations_for_event(world, &record, tick);

        for witness in resolve_witnesses(world, &record) {
            let Some(profile) = world.get_component_perception_profile(witness).copied() else {
                continue;
            };
            if !passes_observation_check(profile.observation_fidelity.value(), rng) {
                continue;
            }

            let store = updated_stores.entry(witness).or_insert_with(|| {
                world
                    .get_component_agent_belief_store(witness)
                    .cloned()
                    .unwrap_or_default()
            });

            for (entity, observed) in &record.observed_entities {
                let snapshot = observed
                    .to_believed_entity_state(record.tick, PerceptionSource::DirectObservation);
                record_observed_snapshot(
                    event_log,
                    DiscoveryContext {
                        tick,
                        observer: witness,
                        place: record.place_id.or(snapshot.last_known_place),
                    },
                    store,
                    *entity,
                    snapshot,
                    true,
                );
            }

            for observation in &social_observations {
                store.record_social_observation(observation.clone());
            }

            store.enforce_capacity(&profile, tick);
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

fn observe_passive_local_entities(
    world: &World,
    event_log: &mut EventLog,
    tick: worldwake_core::Tick,
    rng: &mut worldwake_sim::DeterministicRng,
    updated_stores: &mut BTreeMap<EntityId, AgentBeliefStore>,
) {
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

        let store = updated_stores.entry(agent).or_insert_with(|| {
            world
                .get_component_agent_belief_store(agent)
                .cloned()
                .unwrap_or_default()
        });

        let mut observed_any = false;
        let mut observed_entities = BTreeSet::new();
        for entity in world.entities_effectively_at(place) {
            if entity == agent {
                continue;
            }
            if !passes_observation_check(profile.observation_fidelity.value(), rng) {
                continue;
            }
            if let Some(snapshot) = build_believed_entity_state(
                world,
                entity,
                tick,
                PerceptionSource::DirectObservation,
            ) {
                record_observed_snapshot(
                    event_log,
                    DiscoveryContext {
                        tick,
                        observer: agent,
                        place: Some(place),
                    },
                    store,
                    entity,
                    snapshot,
                    false,
                );
                observed_entities.insert(entity);
                observed_any = true;
            }
        }

        emit_entity_missing_discoveries(
            world,
            event_log,
            rng,
            DiscoveryContext {
                tick,
                observer: agent,
                place: Some(place),
            },
            profile.observation_fidelity.value(),
            store,
            &observed_entities,
        );

        if observed_any {
            store.enforce_capacity(&profile, tick);
        } else {
            updated_stores.remove(&agent);
        }
    }
}

fn record_observed_snapshot(
    event_log: &mut EventLog,
    context: DiscoveryContext,
    store: &mut AgentBeliefStore,
    subject: EntityId,
    snapshot: worldwake_core::BelievedEntityState,
    include_place_change: bool,
) {
    if let Some(prior) = store.get_entity(&subject) {
        for mismatch in detect_observation_mismatches(prior, &snapshot, include_place_change) {
            emit_discovery_event(event_log, context, subject, mismatch);
        }
    }
    store.update_entity(subject, snapshot);
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

    if include_place_change {
        if let (Some(believed_place), Some(observed_place)) =
            (prior.last_known_place, observed.last_known_place)
        {
            if believed_place != observed_place {
                mismatches.push(MismatchKind::PlaceChanged {
                    believed_place,
                    observed_place,
                });
            }
        }
    }

    mismatches
}

fn emit_entity_missing_discoveries(
    world: &World,
    event_log: &mut EventLog,
    rng: &mut worldwake_sim::DeterministicRng,
    context: DiscoveryContext,
    observation_fidelity: u16,
    store: &AgentBeliefStore,
    observed_entities: &BTreeSet<EntityId>,
) {
    let Some(place) = context.place else {
        return;
    };
    for (subject, belief) in &store.known_entities {
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

        emit_discovery_event(event_log, context, *subject, MismatchKind::EntityMissing);
    }
}

fn emit_discovery_event(
    event_log: &mut EventLog,
    context: DiscoveryContext,
    subject: EntityId,
    mismatch: MismatchKind,
) {
    let _ = event_log.emit(PendingEvent::new_complete(
        context.tick,
        CauseRef::SystemTick(context.tick),
        Some(context.observer),
        vec![subject],
        vec![EvidenceRef::Mismatch {
            observer: context.observer,
            subject,
            kind: mismatch,
        }],
        context.place,
        Vec::new(),
        BTreeMap::new(),
        VisibilitySpec::ParticipantsOnly,
        WitnessData {
            direct_witnesses: BTreeSet::from([context.observer]),
            potential_witnesses: BTreeSet::from([context.observer]),
        },
        BTreeSet::from([EventTag::Discovery, EventTag::WorldMutation]),
    ));
}

fn resolve_witnesses(world: &World, record: &EventRecord) -> Vec<EntityId> {
    let candidates = match record.visibility {
        VisibilitySpec::ParticipantsOnly => record.witness_data.direct_witnesses.clone(),
        VisibilitySpec::SamePlace => place_witnesses(world, record.place_id),
        VisibilitySpec::AdjacentPlaces { max_hops } => {
            adjacent_place_witnesses(world, record.place_id, max_hops)
        }
        VisibilitySpec::PublicRecord | VisibilitySpec::Hidden => BTreeSet::new(),
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

fn social_observations_for_event(
    world: &World,
    record: &EventRecord,
    tick: worldwake_core::Tick,
) -> Vec<SocialObservation> {
    let Some(place) = record.place_id else {
        return Vec::new();
    };
    let Some(actor) = record
        .actor_id
        .filter(|actor| world.entity_kind(*actor) == Some(EntityKind::Agent))
    else {
        return Vec::new();
    };
    let targets = record
        .target_ids
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
            kind,
            subjects: (actor, target),
            place,
            observed_tick: tick,
            source: PerceptionSource::DirectObservation,
        })
        .collect()
}

fn social_kind(record: &EventRecord) -> Option<SocialObservationKind> {
    if record.tags.contains(&EventTag::Social) {
        return Some(SocialObservationKind::WitnessedTelling);
    }
    if record.tags.contains(&EventTag::Trade) {
        return Some(SocialObservationKind::WitnessedCooperation);
    }
    if record.tags.contains(&EventTag::Combat) {
        return Some(SocialObservationKind::WitnessedConflict);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::perception_system;
    use crate::dispatch_table;
    use std::collections::{BTreeMap, BTreeSet};
    use worldwake_core::{
        build_observed_entity_snapshot, build_prototype_world, AgentBeliefStore,
        BeliefConfidencePolicy, BelievedEntityState, CauseRef, CommodityKind, ControlSource,
        DeadAt, EventLog, EventTag, EvidenceRef, MismatchKind, ObservedEntitySnapshot,
        PendingEvent, PerceptionProfile, PerceptionSource, Permille, Quantity, Seed,
        SocialObservationKind, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
    };
    use worldwake_sim::{ActionDefRegistry, DeterministicRng, SystemExecutionContext, SystemId};

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
            memory_capacity: 8,
            memory_retention_ticks: 32,
            observation_fidelity: Permille::new(fidelity).unwrap(),
            confidence_policy: BeliefConfidencePolicy::default(),
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
            last_known_place: place,
            last_known_inventory: inventory,
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
        }
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
        let _ = event_log.emit(PendingEvent::new_complete(
            Tick(3),
            CauseRef::Bootstrap,
            Some(target),
            vec![target],
            Vec::new(),
            Some(place),
            Vec::new(),
            observed_from_world(&world, &[target]),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
            BTreeSet::new(),
        ));
        let mut rng = DeterministicRng::new(Seed([7; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
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
        assert_eq!(believed.last_known_place, Some(place));
        assert_eq!(
            believed.last_known_inventory.get(&CommodityKind::Bread),
            Some(&Quantity(2))
        );
        assert!(believed.alive);
        assert_eq!(believed.observed_tick, Tick(3));
        assert_eq!(believed.source, PerceptionSource::DirectObservation);
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
        let _ = event_log.emit(PendingEvent::new(
            Tick(4),
            CauseRef::Bootstrap,
            Some(actor),
            vec![counterparty],
            Some(place),
            Vec::new(),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
            BTreeSet::from([EventTag::Trade]),
        ));
        let mut rng = DeterministicRng::new(Seed([3; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            tick: Tick(4),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        assert!(
            beliefs.social_observations.iter().any(|observation| {
                observation.kind == SocialObservationKind::WitnessedCooperation
                    && observation.place == place
                    && observation.subjects == (actor, counterparty)
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
        let _ = event_log.emit(PendingEvent::new(
            Tick(4),
            CauseRef::Bootstrap,
            Some(speaker),
            vec![listener],
            Some(place),
            Vec::new(),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
            BTreeSet::from([EventTag::Social]),
        ));
        let mut rng = DeterministicRng::new(Seed([5; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            tick: Tick(4),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        assert!(
            beliefs.social_observations.iter().any(|observation| {
                observation.kind == SocialObservationKind::WitnessedTelling
                    && observation.place == place
                    && observation.subjects == (speaker, listener)
                    && observation.source == PerceptionSource::DirectObservation
                    && observation.observed_tick == Tick(4)
            }),
            "social witness should record witnessed telling"
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
        let _ = event_log.emit(PendingEvent::new_complete(
            Tick(5),
            CauseRef::Bootstrap,
            Some(target),
            vec![target],
            Vec::new(),
            Some(place),
            Vec::new(),
            observed_from_world(&world, &[target]),
            VisibilitySpec::ParticipantsOnly,
            WitnessData {
                direct_witnesses: BTreeSet::from([direct_witness]),
                potential_witnesses: BTreeSet::from([bystander, direct_witness]),
            },
            BTreeSet::new(),
        ));
        let mut rng = DeterministicRng::new(Seed([9; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            tick: Tick(5),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert!(world
            .get_component_agent_belief_store(direct_witness)
            .unwrap()
            .get_entity(&target)
            .is_some());
        assert!(world
            .get_component_agent_belief_store(bystander)
            .unwrap()
            .get_entity(&target)
            .is_none());
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
        let _ = event_log.emit(PendingEvent::new_complete(
            Tick(6),
            CauseRef::Bootstrap,
            Some(origin_target),
            vec![origin_target],
            Vec::new(),
            Some(origin),
            Vec::new(),
            observed_from_world(&world, &[origin_target]),
            VisibilitySpec::AdjacentPlaces { max_hops: 1 },
            WitnessData::default(),
            BTreeSet::new(),
        ));
        let mut rng = DeterministicRng::new(Seed([4; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            tick: Tick(6),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert!(world
            .get_component_agent_belief_store(adjacent_witness)
            .unwrap()
            .get_entity(&origin_target)
            .is_some());
        assert!(world
            .get_component_agent_belief_store(remote_witness)
            .unwrap()
            .get_entity(&origin_target)
            .is_none());
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
                    last_known_place: Some(place),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
                    observed_tick: Tick(1),
                    source: PerceptionSource::DirectObservation,
                },
            );
            txn.set_component_agent_belief_store(observer, store)
                .unwrap();
            txn.set_component_perception_profile(
                observer,
                PerceptionProfile {
                    memory_capacity: 1,
                    memory_retention_ticks: 32,
                    observation_fidelity: Permille::new(1000).unwrap(),
                    confidence_policy: BeliefConfidencePolicy::default(),
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, older_target, newer_target)
        };
        let mut event_log = EventLog::new();
        let _ = event_log.emit(PendingEvent::new_complete(
            Tick(7),
            CauseRef::Bootstrap,
            Some(newer_target),
            vec![newer_target],
            Vec::new(),
            Some(place),
            Vec::new(),
            observed_from_world(&world, &[newer_target]),
            VisibilitySpec::SamePlace,
            WitnessData::default(),
            BTreeSet::new(),
        ));
        let mut rng = DeterministicRng::new(Seed([8; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
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
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        let target_belief = beliefs
            .known_entities
            .values()
            .find(|belief| {
                belief.last_known_inventory.get(&CommodityKind::Bread) == Some(&Quantity(2))
            })
            .expect("passive same-place observation should capture already-present local entities");
        assert_eq!(target_belief.last_known_place, Some(place));
        assert_eq!(target_belief.observed_tick, Tick(3));
        assert_eq!(target_belief.source, PerceptionSource::DirectObservation);
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
            tick: Tick(2),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let beliefs = world
            .get_component_agent_belief_store(observer)
            .expect("observer should have a belief store");
        assert!(
            beliefs.known_entities.is_empty(),
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
                    last_known_place: Some(place),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
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
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let discoveries = discovery_records(&event_log);
        assert_eq!(discoveries.len(), 1);
        let discovery = discoveries[0];
        assert_eq!(discovery.actor_id, Some(observer));
        assert_eq!(discovery.place_id, Some(place));
        assert_eq!(discovery.visibility, VisibilitySpec::ParticipantsOnly);
        assert!(discovery.tags.contains(&EventTag::Discovery));
        assert!(discovery.tags.contains(&EventTag::WorldMutation));
        assert_eq!(
            discovery.evidence,
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
                    last_known_place: Some(place),
                    last_known_inventory: inventory,
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
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
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let discoveries = discovery_records(&event_log);
        assert_eq!(discoveries.len(), 1);
        assert_eq!(
            discoveries[0].evidence,
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
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert!(discovery_records(&event_log).is_empty());
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
                    last_known_place: Some(place),
                    last_known_inventory: inventory,
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
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
                    last_known_place: Some(place),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
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
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert_eq!(
            discovery_records(&event_log)[0].evidence,
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
                    last_known_place: Some(place),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
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
                    last_known_place: Some(origin),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
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
        let _ = event_log.emit(PendingEvent::new_complete(
            Tick(3),
            CauseRef::Bootstrap,
            Some(target),
            vec![target],
            Vec::new(),
            Some(origin),
            Vec::new(),
            observed_from_world(&world, &[target]),
            VisibilitySpec::AdjacentPlaces { max_hops: 1 },
            WitnessData::default(),
            BTreeSet::new(),
        ));
        let mut rng = DeterministicRng::new(Seed([20; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert!(
            discovery_records(&event_log).iter().any(|record| {
                record.evidence
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
                    last_known_place: Some(origin),
                    last_known_inventory: prior_inventory,
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
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
        let _ = event_log.emit(PendingEvent::new_complete(
            Tick(3),
            CauseRef::Bootstrap,
            Some(target),
            vec![target],
            Vec::new(),
            Some(origin),
            Vec::new(),
            observed_from_world(&world, &[target]),
            VisibilitySpec::AdjacentPlaces { max_hops: 1 },
            WitnessData::default(),
            BTreeSet::new(),
        ));
        let mut rng = DeterministicRng::new(Seed([21; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert!(
            discovery_records(&event_log).iter().any(|record| {
                record.evidence
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
                    last_known_place: Some(origin),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
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
        let _ = event_log.emit(PendingEvent::new_complete(
            Tick(3),
            CauseRef::Bootstrap,
            Some(target),
            vec![target],
            Vec::new(),
            Some(origin),
            Vec::new(),
            observed_from_world(&world, &[target]),
            VisibilitySpec::AdjacentPlaces { max_hops: 1 },
            WitnessData::default(),
            BTreeSet::new(),
        ));
        let mut rng = DeterministicRng::new(Seed([22; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert!(
            discovery_records(&event_log).iter().any(|record| {
                record.evidence
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
                    last_known_place: Some(origin),
                    last_known_inventory: prior_inventory,
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
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
        let _ = event_log.emit(PendingEvent::new_complete(
            Tick(3),
            CauseRef::Bootstrap,
            Some(target),
            vec![target],
            Vec::new(),
            Some(origin),
            Vec::new(),
            BTreeMap::from([(target, observed_snapshot(Some(origin), 4))]),
            VisibilitySpec::AdjacentPlaces { max_hops: 1 },
            WitnessData::default(),
            BTreeSet::new(),
        ));
        let _ = event_log.emit(PendingEvent::new_complete(
            Tick(3),
            CauseRef::Bootstrap,
            Some(target),
            vec![target],
            Vec::new(),
            Some(origin),
            Vec::new(),
            BTreeMap::from([(target, observed_snapshot(Some(remote), 2))]),
            VisibilitySpec::AdjacentPlaces { max_hops: 1 },
            WitnessData::default(),
            BTreeSet::new(),
        ));
        let mut rng = DeterministicRng::new(Seed([24; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        let mismatches = discovery_records(&event_log)
            .iter()
            .flat_map(|record| record.evidence.iter())
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
                    last_known_place: Some(origin),
                    last_known_inventory: BTreeMap::new(),
                    workstation_tag: None,
                    resource_source: None,
                    alive: true,
                    wounds: Vec::new(),
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
        let _ = event_log.emit(PendingEvent::new_complete(
            Tick(3),
            CauseRef::Bootstrap,
            Some(target),
            vec![target],
            Vec::new(),
            Some(origin),
            Vec::new(),
            observed_from_world(&world, &[target]),
            VisibilitySpec::AdjacentPlaces { max_hops: 1 },
            WitnessData::default(),
            BTreeSet::new(),
        ));
        let mut rng = DeterministicRng::new(Seed([23; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        perception_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            tick: Tick(3),
            system_id: SystemId::Perception,
        })
        .unwrap();

        assert!(discovery_records(&event_log).is_empty());
    }
}
