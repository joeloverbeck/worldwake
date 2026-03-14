use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    build_believed_entity_state, AgentBeliefStore, CauseRef, ComponentDelta, EntityDelta, EntityId,
    EntityKind, EventRecord, EventTag, EvidenceRef, PerceptionSource, SocialObservation,
    SocialObservationKind, StateDelta, VisibilitySpec, WitnessData, World, WorldTxn,
};
use worldwake_sim::{SystemError, SystemExecutionContext};

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

    observe_passive_local_entities(world, tick, rng, &mut updated_stores);

    for event_id in event_ids {
        let Some(record) = event_log.get(event_id).cloned() else {
            continue;
        };
        let observed_entities = observed_entities(&record);
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

            for entity in &observed_entities {
                if let Some(snapshot) = build_believed_entity_state(
                    world,
                    *entity,
                    tick,
                    PerceptionSource::DirectObservation,
                ) {
                    store.update_entity(*entity, snapshot);
                }
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
                store.update_entity(entity, snapshot);
                observed_any = true;
            }
        }

        if observed_any {
            store.enforce_capacity(&profile, tick);
        } else {
            updated_stores.remove(&agent);
        }
    }
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

fn observed_entities(record: &EventRecord) -> BTreeSet<EntityId> {
    let mut entities = BTreeSet::new();
    if let Some(actor) = record.actor_id {
        entities.insert(actor);
    }
    entities.extend(record.target_ids.iter().copied());
    for evidence in &record.evidence {
        match evidence {
            EvidenceRef::Wound { entity, .. } => {
                entities.insert(*entity);
            }
            EvidenceRef::Mismatch {
                observer, subject, ..
            } => {
                entities.insert(*observer);
                entities.insert(*subject);
            }
        }
    }
    for delta in &record.state_deltas {
        match delta {
            StateDelta::Entity(entity_delta) => match entity_delta {
                EntityDelta::Created { entity, .. } | EntityDelta::Archived { entity, .. } => {
                    entities.insert(*entity);
                }
            },
            StateDelta::Component(component_delta) => match component_delta {
                ComponentDelta::Set { entity, .. } | ComponentDelta::Removed { entity, .. } => {
                    entities.insert(*entity);
                }
            },
            StateDelta::Relation(relation_delta) => {
                entities.extend(relation_entities(relation_delta));
            }
            StateDelta::Quantity(quantity_delta) => match quantity_delta {
                worldwake_core::QuantityDelta::Changed { entity, .. } => {
                    entities.insert(*entity);
                }
            },
            StateDelta::Reservation(reservation_delta) => match reservation_delta {
                worldwake_core::ReservationDelta::Created { reservation }
                | worldwake_core::ReservationDelta::Released { reservation } => {
                    entities.insert(reservation.entity);
                    entities.insert(reservation.reserver);
                }
            },
        }
    }
    entities
}

fn relation_entities(relation_delta: &worldwake_core::RelationDelta) -> BTreeSet<EntityId> {
    use worldwake_core::RelationDelta::{Added, Removed};
    use worldwake_core::RelationValue;

    let relation = match relation_delta {
        Added { relation, .. } | Removed { relation, .. } => relation,
    };

    match relation {
        RelationValue::LocatedIn { entity, place } => BTreeSet::from([*entity, *place]),
        RelationValue::InTransit { entity } => BTreeSet::from([*entity]),
        RelationValue::ContainedBy { entity, container } => BTreeSet::from([*entity, *container]),
        RelationValue::PossessedBy { entity, holder } => BTreeSet::from([*entity, *holder]),
        RelationValue::OwnedBy { entity, owner } => BTreeSet::from([*entity, *owner]),
        RelationValue::MemberOf { member, faction } => BTreeSet::from([*member, *faction]),
        RelationValue::LoyalTo {
            subject, target, ..
        }
        | RelationValue::HostileTo { subject, target } => BTreeSet::from([*subject, *target]),
        RelationValue::OfficeHolder { office, holder } => BTreeSet::from([*office, *holder]),
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
        build_prototype_world, AgentBeliefStore, BelievedEntityState, CauseRef, CommodityKind,
        ControlSource, EventLog, EventTag, PendingEvent, PerceptionProfile, PerceptionSource,
        Permille, Quantity, Seed, SocialObservationKind, Tick, VisibilitySpec, WitnessData, World,
        WorldTxn,
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
        let _ = event_log.emit(PendingEvent::new(
            Tick(3),
            CauseRef::Bootstrap,
            Some(target),
            vec![target],
            Some(place),
            Vec::new(),
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
        let _ = event_log.emit(PendingEvent::new(
            Tick(5),
            CauseRef::Bootstrap,
            Some(target),
            vec![target],
            Some(place),
            Vec::new(),
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
        let _ = event_log.emit(PendingEvent::new(
            Tick(6),
            CauseRef::Bootstrap,
            Some(origin_target),
            vec![origin_target],
            Some(origin),
            Vec::new(),
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
                },
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
            (observer, older_target, newer_target)
        };
        let mut event_log = EventLog::new();
        let _ = event_log.emit(PendingEvent::new(
            Tick(7),
            CauseRef::Bootstrap,
            Some(newer_target),
            vec![newer_target],
            Some(place),
            Vec::new(),
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
}
