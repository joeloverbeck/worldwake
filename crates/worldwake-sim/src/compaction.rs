//! Epoch compaction: periodic World checkpoints and `state_deltas` stripping.

use crate::system_dispatch::{SystemError, SystemExecutionContext};
use worldwake_core::CheckpointData;

/// Periodically creates World checkpoints and strips old `state_deltas` payloads
/// from events before the checkpoint tick. Runs as the last system in each tick.
///
/// Does nothing when `compaction_interval` is 0 or the current tick is not a
/// compaction boundary.
#[allow(clippy::needless_pass_by_value)]
pub fn compact_event_log(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let interval = ctx.event_log.compaction_interval();
    if interval == 0 || !ctx.tick.0.is_multiple_of(u64::from(interval)) {
        return Ok(());
    }

    let snapshot =
        bincode::serialize(ctx.world).expect("World serialization must not fail");
    ctx.event_log
        .add_checkpoint(ctx.tick, CheckpointData::new(snapshot));

    ctx.event_log.strip_deltas_before(ctx.tick);

    ctx.event_log.prune_old_checkpoints(2);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::compact_event_log;
    use crate::system_dispatch::SystemExecutionContext;
    use crate::{ActionDefRegistry, DeterministicRng, SystemId};
    use std::collections::BTreeMap;
    use worldwake_core::{
        CauseRef, EntityDelta, EntityId, EntityKind, EventLog, EventPayload, EventTag, EventView,
        PendingEvent, Seed, StateDelta, Tick, VisibilitySpec, WitnessData, World,
        build_prototype_world,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn pending_with_deltas(tick: Tick, deltas: Vec<StateDelta>) -> PendingEvent {
        PendingEvent::from_payload(EventPayload {
            tick,
            cause: CauseRef::Bootstrap,
            actor_id: Some(entity(1)),
            action_name: None,
            target_ids: vec![],
            evidence: Vec::new(),
            place_id: Some(entity(4)),
            state_deltas: deltas,
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData::default(),
            tags: std::collections::BTreeSet::from([EventTag::WorldMutation]),
        })
    }

    fn sample_delta() -> StateDelta {
        StateDelta::Entity(EntityDelta::Created {
            entity: entity(99),
            kind: EntityKind::Agent,
        })
    }

    fn run_compaction(
        world: &mut World,
        event_log: &mut EventLog,
        tick: Tick,
    ) {
        let mut rng = DeterministicRng::new(Seed([7; 32]));
        let action_defs = ActionDefRegistry::new();
        let active_actions = BTreeMap::new();

        compact_event_log(SystemExecutionContext {
            world,
            event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            politics_trace: None,
            perception_trace: None,
            tick,
            system_id: SystemId::Compaction,
        })
        .unwrap();
    }

    #[test]
    fn compaction_triggers_at_interval_boundary() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();
        event_log.set_compaction_interval(50);

        // Emit events at ticks 1..50
        for t in 1..50 {
            event_log.emit(pending_with_deltas(Tick(t), vec![sample_delta()]));
        }

        run_compaction(&mut world, &mut event_log, Tick(50));

        // Checkpoint should exist at tick 50
        let (tick, _) = event_log.latest_checkpoint().unwrap();
        assert_eq!(*tick, Tick(50));

        // Events before tick 50 should be stripped
        for t in 1..50 {
            for id in event_log.events_at_tick(Tick(t)) {
                let record = event_log.get(*id).unwrap();
                assert!(
                    record.state_deltas().is_empty(),
                    "event at tick {t} should be stripped",
                );
            }
        }
    }

    #[test]
    fn compaction_is_noop_at_non_interval_tick() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();
        event_log.set_compaction_interval(50);

        event_log.emit(pending_with_deltas(Tick(1), vec![sample_delta()]));

        run_compaction(&mut world, &mut event_log, Tick(49));

        assert!(event_log.latest_checkpoint().is_none());
        let record = event_log.get(worldwake_core::EventId(0)).unwrap();
        assert_eq!(record.state_deltas().len(), 1);
    }

    #[test]
    fn compaction_disabled_when_interval_is_zero() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();
        // interval defaults to 0

        event_log.emit(pending_with_deltas(Tick(1), vec![sample_delta()]));

        run_compaction(&mut world, &mut event_log, Tick(50));

        assert!(event_log.latest_checkpoint().is_none());
        let record = event_log.get(worldwake_core::EventId(0)).unwrap();
        assert_eq!(record.state_deltas().len(), 1);
    }

    #[test]
    fn compaction_is_deterministic() {
        let mut world1 = World::new(build_prototype_world()).unwrap();
        let mut log1 = EventLog::new();
        log1.set_compaction_interval(50);
        log1.emit(pending_with_deltas(Tick(1), vec![sample_delta()]));
        run_compaction(&mut world1, &mut log1, Tick(50));

        let mut world2 = World::new(build_prototype_world()).unwrap();
        let mut log2 = EventLog::new();
        log2.set_compaction_interval(50);
        log2.emit(pending_with_deltas(Tick(1), vec![sample_delta()]));
        run_compaction(&mut world2, &mut log2, Tick(50));

        let (_, cp1) = log1.latest_checkpoint().unwrap();
        let (_, cp2) = log2.latest_checkpoint().unwrap();
        assert_eq!(cp1.world_snapshot(), cp2.world_snapshot());
    }
}
