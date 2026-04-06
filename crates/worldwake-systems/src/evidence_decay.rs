use worldwake_core::{
    CauseRef, EventLog, EventTag, EvidenceEntry, SceneEvidence, Tick, VisibilitySpec, WitnessData,
    World, WorldTxn,
};
use worldwake_sim::{SystemError, SystemExecutionContext};

pub fn evidence_decay_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let SystemExecutionContext {
        world,
        event_log,
        rng: _rng,
        active_actions: _active_actions,
        action_defs: _action_defs,
        politics_trace: _,
        perception_trace: _,
        tick,
        system_id: _system_id,
    } = ctx;

    let updates = collect_updates(world, tick);
    for update in updates {
        apply_update(world, event_log, tick, update)?;
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingUpdate {
    place: worldwake_core::EntityId,
    next: Option<SceneEvidence>,
}

fn collect_updates(world: &World, tick: Tick) -> Vec<PendingUpdate> {
    world
        .query_scene_evidence()
        .filter_map(|(place, scene)| {
            let mut next = scene.clone();
            next.evidence.retain(|entry| !is_expired(entry, tick));

            if next.evidence.len() == scene.evidence.len() {
                return None;
            }

            (!next.evidence.is_empty())
                .then_some(PendingUpdate {
                    place,
                    next: Some(next),
                })
                .or(Some(PendingUpdate { place, next: None }))
        })
        .collect()
}

fn is_expired(entry: &EvidenceEntry, tick: Tick) -> bool {
    tick.0.saturating_sub(entry.created_at.0) >= u64::from(entry.decay_ticks)
}

fn apply_update(
    world: &mut World,
    event_log: &mut EventLog,
    tick: Tick,
    update: PendingUpdate,
) -> Result<(), SystemError> {
    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        None,
        Some(update.place),
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.add_tag(EventTag::System)
        .add_tag(EventTag::WorldMutation)
        .add_target(update.place);

    match update.next {
        Some(scene) => txn
            .set_component_scene_evidence(update.place, scene)
            .map_err(|error| SystemError::new(error.to_string()))?,
        None => txn
            .clear_component_scene_evidence(update.place)
            .map_err(|error| SystemError::new(error.to_string()))?,
    }

    let _ = txn.commit(event_log);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{evidence_decay_system, is_expired};
    use crate::dispatch_table;
    use std::collections::BTreeMap;
    use worldwake_core::{
        CauseRef, DisturbanceKind, EventLog, EventTag, EventView, EvidenceEntry, EvidenceEntryId,
        EvidenceKind, PrototypePlace, SceneEvidence, Seed, Tick, VisibilitySpec, WitnessData,
        World, WorldTxn, build_prototype_world, prototype_place_entity,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionInstance, ActionInstanceId, DeterministicRng,
        SystemExecutionContext, SystemId,
    };

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::Hidden,
            WitnessData::default(),
        )
    }

    fn seed_scene(world: &mut World, place: worldwake_core::EntityId, scene: SceneEvidence) {
        let mut txn = new_txn(world, 1);
        txn.set_component_scene_evidence(place, scene).unwrap();
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    fn system_context<'a>(
        world: &'a mut World,
        event_log: &'a mut EventLog,
        rng: &'a mut DeterministicRng,
        active_actions: &'a BTreeMap<ActionInstanceId, ActionInstance>,
        action_defs: &'a ActionDefRegistry,
        tick: u64,
    ) -> SystemExecutionContext<'a> {
        SystemExecutionContext {
            world,
            event_log,
            rng,
            active_actions,
            action_defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(tick),
            system_id: SystemId::EvidenceDecay,
        }
    }

    fn movement_trace(
        id: u64,
        created_at: u64,
        decay_ticks: u32,
        place: worldwake_core::EntityId,
        direction: worldwake_core::EntityId,
    ) -> EvidenceEntry {
        EvidenceEntry {
            id: EvidenceEntryId(id),
            kind: EvidenceKind::MovementTrace {
                entity: worldwake_core::EntityId {
                    slot: 90 + id as u32,
                    generation: 0,
                },
                departed_from: place,
                direction,
                observed_at: Tick(created_at),
            },
            created_at: Tick(created_at),
            decay_ticks,
        }
    }

    fn disturbance(
        id: u64,
        created_at: u64,
        decay_ticks: u32,
        place: worldwake_core::EntityId,
    ) -> EvidenceEntry {
        EvidenceEntry {
            id: EvidenceEntryId(id),
            kind: EvidenceKind::DisturbanceMarker {
                place,
                kind: DisturbanceKind::WildernessRelief,
                created_at: Tick(created_at),
            },
            created_at: Tick(created_at),
            decay_ticks,
        }
    }

    #[test]
    fn evidence_entries_expire_at_boundary_tick() {
        let place = prototype_place_entity(PrototypePlace::VillageSquare);
        let direction = prototype_place_entity(PrototypePlace::ForestPath);
        let entry = movement_trace(0, 3, 5, place, direction);

        assert!(!is_expired(&entry, Tick(7)));
        assert!(is_expired(&entry, Tick(8)));
    }

    #[test]
    fn evidence_decay_system_keeps_unexpired_entries() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::VillageSquare);
        let direction = prototype_place_entity(PrototypePlace::ForestPath);
        seed_scene(
            &mut world,
            place,
            SceneEvidence {
                evidence: vec![movement_trace(0, 5, 4, place, direction)],
                next_entry_id: 1,
            },
        );
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([1; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        evidence_decay_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            8,
        ))
        .unwrap();

        let scene = world.get_component_scene_evidence(place).unwrap();
        assert_eq!(scene.evidence.len(), 1);
        assert_eq!(scene.next_entry_id, 1);
        assert!(event_log.is_empty());
    }

    #[test]
    fn evidence_decay_system_removes_only_expired_entries() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::VillageSquare);
        let direction = prototype_place_entity(PrototypePlace::ForestPath);
        seed_scene(
            &mut world,
            place,
            SceneEvidence {
                evidence: vec![
                    movement_trace(0, 2, 5, place, direction),
                    disturbance(1, 6, 5, place),
                ],
                next_entry_id: 2,
            },
        );
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([2; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        evidence_decay_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            7,
        ))
        .unwrap();

        let scene = world.get_component_scene_evidence(place).unwrap();
        assert_eq!(scene.evidence, vec![disturbance(1, 6, 5, place)]);
        assert_eq!(scene.next_entry_id, 2);
        assert_eq!(event_log.events_by_tag(EventTag::WorldMutation).len(), 1);
    }

    #[test]
    fn evidence_decay_system_clears_component_when_last_entry_expires() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::VillageSquare);
        let direction = prototype_place_entity(PrototypePlace::ForestPath);
        seed_scene(
            &mut world,
            place,
            SceneEvidence {
                evidence: vec![movement_trace(0, 2, 5, place, direction)],
                next_entry_id: 1,
            },
        );
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([3; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        evidence_decay_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            7,
        ))
        .unwrap();

        assert!(world.get_component_scene_evidence(place).is_none());
        let world_mutations = event_log.events_by_tag(EventTag::WorldMutation);
        assert_eq!(world_mutations.len(), 1);
        let record = event_log.get(world_mutations[0]).unwrap();
        assert!(record.state_deltas().iter().any(|delta| matches!(
            delta,
            worldwake_core::StateDelta::Component(worldwake_core::ComponentDelta::Removed {
                entity,
                component_kind: worldwake_core::ComponentKind::SceneEvidence,
                ..
            }) if *entity == place
        )));
    }

    #[test]
    fn dispatch_table_routes_evidence_decay_system() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::VillageSquare);
        let direction = prototype_place_entity(PrototypePlace::ForestPath);
        seed_scene(
            &mut world,
            place,
            SceneEvidence {
                evidence: vec![movement_trace(0, 2, 5, place, direction)],
                next_entry_id: 1,
            },
        );
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([4; 32]));
        let active_actions = BTreeMap::<ActionInstanceId, ActionInstance>::new();
        let action_defs = ActionDefRegistry::new();

        dispatch_table().get(SystemId::EvidenceDecay)(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            7,
        ))
        .unwrap();

        assert!(world.get_component_scene_evidence(place).is_none());
    }
}
