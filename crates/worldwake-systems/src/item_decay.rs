use worldwake_core::{
    CauseRef, EntityId, EventLog, EventTag, LastHarvestTrace, Tick, VisibilitySpec, WitnessData,
    World, WorldTxn,
};
use worldwake_sim::{SystemError, SystemExecutionContext};

pub fn item_decay_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
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

    let to_archive = collect_decay_targets(world, tick);
    for entity in to_archive {
        apply_decay(world, event_log, tick, entity);
    }

    let trace_updates = collect_harvest_trace_updates(world, tick);
    for (entity, pruned) in trace_updates {
        apply_harvest_trace_pruning(world, event_log, tick, entity, pruned);
    }

    Ok(())
}

fn collect_decay_targets(world: &World, tick: Tick) -> Vec<worldwake_core::EntityId> {
    world
        .query_ground_since()
        .filter_map(|(entity, ground_since)| {
            let item_lot = world.get_component_item_lot(entity)?;
            let decay_ticks = world.commodity_decay().get(&item_lot.commodity)?;
            let elapsed = tick.0.saturating_sub(ground_since.0.0);
            (elapsed >= u64::from(decay_ticks.get())).then_some(entity)
        })
        .collect()
}

/// Returns `(entity, pruned_trace)` pairs for sources whose
/// `LastHarvestTrace` shed at least one entry past the configured retention
/// window. Sources with no eligible eviction are skipped to preserve the
/// no-op-event invariant of the existing decay pass.
fn collect_harvest_trace_updates(world: &World, tick: Tick) -> Vec<(EntityId, LastHarvestTrace)> {
    let retention_ticks = u64::from(world.harvest_trace_retention_ticks());
    let cutoff = tick.0.saturating_sub(retention_ticks);
    world
        .query_last_harvest_trace()
        .filter_map(|(entity, trace)| {
            if trace.entries.iter().all(|entry| entry.tick.0 >= cutoff) {
                return None;
            }
            let pruned = LastHarvestTrace {
                entries: trace
                    .entries
                    .iter()
                    .filter(|entry| entry.tick.0 >= cutoff)
                    .copied()
                    .collect(),
            };
            Some((entity, pruned))
        })
        .collect()
}

fn apply_harvest_trace_pruning(
    world: &mut World,
    event_log: &mut EventLog,
    tick: Tick,
    entity: EntityId,
    pruned: LastHarvestTrace,
) {
    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        None,
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.add_tag(EventTag::ItemDecay)
        .add_tag(EventTag::WorldMutation)
        .add_target(entity);

    if txn
        .set_component_last_harvest_trace(entity, pruned)
        .is_err()
    {
        return;
    }

    let _ = txn.commit(event_log);
}

fn apply_decay(
    world: &mut World,
    event_log: &mut EventLog,
    tick: Tick,
    entity: worldwake_core::EntityId,
) {
    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        None,
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.add_tag(EventTag::ItemDecay)
        .add_tag(EventTag::WorldMutation)
        .add_target(entity);

    if txn.archive_entity(entity).is_err() {
        return;
    }

    let _ = txn.commit(event_log);
}

#[cfg(test)]
mod tests {
    use super::item_decay_system;
    use crate::dispatch_table;
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        CauseRef, CommodityKind, EntityId, EntityKind, EventLog, EventTag, EventView,
        HARVEST_TRACE_RETENTION_TICKS, HarvestTraceEntry, LastHarvestTrace, PrototypePlace,
        Quantity, ResourceSource, Seed, Tick, VisibilitySpec, WitnessData, WorkstationMarker,
        WorkstationTag, World, WorldTxn, build_prototype_world, prototype_place_entity,
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
            system_id: SystemId::ItemDecay,
        }
    }

    fn seed_ground_item(
        world: &mut World,
        tick: u64,
        commodity: CommodityKind,
        quantity: u32,
    ) -> EntityId {
        let place = prototype_place_entity(PrototypePlace::VillageSquare);
        let mut txn = new_txn(world, tick);
        let item = txn
            .create_item_lot_with_owner(commodity, Quantity(quantity), place, None)
            .unwrap();
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
        item
    }

    #[test]
    fn waste_decays_at_threshold_tick() {
        let mut world = World::new(build_prototype_world()).unwrap();
        world.set_commodity_decay(BTreeMap::from([(
            CommodityKind::Waste,
            NonZeroU32::new(50).unwrap(),
        )]));
        let waste = seed_ground_item(&mut world, 10, CommodityKind::Waste, 1);

        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([1; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        item_decay_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            59,
        ))
        .unwrap();
        assert_eq!(
            world
                .query_ground_since()
                .map(|(entity, _)| entity)
                .collect::<Vec<_>>(),
            vec![waste]
        );
        assert!(event_log.is_empty());

        item_decay_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            60,
        ))
        .unwrap();
        assert!(
            !world
                .query_ground_since()
                .any(|(entity, _)| entity == waste)
        );
        assert_eq!(world.get_component_ground_since(waste), None);
        assert_eq!(event_log.events_by_tag(EventTag::ItemDecay).len(), 1);
    }

    #[test]
    fn multi_commodity_selective_decay() {
        let mut world = World::new(build_prototype_world()).unwrap();
        world.set_commodity_decay(BTreeMap::from([
            (CommodityKind::Waste, NonZeroU32::new(50).unwrap()),
            (CommodityKind::Apple, NonZeroU32::new(100).unwrap()),
        ]));
        let waste = seed_ground_item(&mut world, 10, CommodityKind::Waste, 1);
        let apple = seed_ground_item(&mut world, 10, CommodityKind::Apple, 1);
        let sword = seed_ground_item(&mut world, 10, CommodityKind::Sword, 1);

        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([2; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        item_decay_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            60,
        ))
        .unwrap();

        let after_first = world
            .query_ground_since()
            .map(|(entity, _)| entity)
            .collect::<BTreeSet<_>>();
        assert!(!after_first.contains(&waste));
        assert!(after_first.contains(&apple));
        assert!(after_first.contains(&sword));

        item_decay_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            110,
        ))
        .unwrap();

        let after_second = world
            .query_ground_since()
            .map(|(entity, _)| entity)
            .collect::<BTreeSet<_>>();
        assert!(!after_second.contains(&waste));
        assert!(!after_second.contains(&apple));
        assert!(after_second.contains(&sword));
    }

    #[test]
    fn no_decay_for_missing_commodity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        world.set_commodity_decay(BTreeMap::from([(
            CommodityKind::Waste,
            NonZeroU32::new(50).unwrap(),
        )]));
        let water = seed_ground_item(&mut world, 10, CommodityKind::Water, 1);

        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([3; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        item_decay_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            10_000,
        ))
        .unwrap();

        assert!(
            world
                .query_ground_since()
                .any(|(entity, _)| entity == water)
        );
        assert!(event_log.events_by_tag(EventTag::ItemDecay).is_empty());
    }

    #[test]
    fn decay_event_has_correct_tags() {
        let mut world = World::new(build_prototype_world()).unwrap();
        world.set_commodity_decay(BTreeMap::from([(
            CommodityKind::Waste,
            NonZeroU32::new(1).unwrap(),
        )]));
        let waste = seed_ground_item(&mut world, 10, CommodityKind::Waste, 1);

        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([4; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        item_decay_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            11,
        ))
        .unwrap();

        let decay_events = event_log.events_by_tag(EventTag::ItemDecay);
        let world_mutations = event_log.events_by_tag(EventTag::WorldMutation);
        assert_eq!(decay_events.len(), 1);
        assert_eq!(world_mutations, decay_events);

        let record = event_log.get(decay_events[0]).unwrap();
        assert!(record.tags().contains(&EventTag::ItemDecay));
        assert!(record.tags().contains(&EventTag::WorldMutation));
        assert_eq!(record.target_ids(), vec![waste]);
    }

    #[test]
    fn dispatch_table_routes_item_decay_system() {
        let mut world = World::new(build_prototype_world()).unwrap();
        world.set_commodity_decay(BTreeMap::from([(
            CommodityKind::Waste,
            NonZeroU32::new(1).unwrap(),
        )]));
        let waste = seed_ground_item(&mut world, 10, CommodityKind::Waste, 1);

        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([5; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        dispatch_table().get(SystemId::ItemDecay)(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            11,
        ))
        .unwrap();

        assert!(
            !world
                .query_ground_since()
                .any(|(entity, _)| entity == waste)
        );
        assert_eq!(event_log.events_by_tag(EventTag::ItemDecay).len(), 1);
    }

    fn seed_harvest_source(
        world: &mut World,
        tick: u64,
        place: EntityId,
        commodity: CommodityKind,
        trace: LastHarvestTrace,
    ) -> EntityId {
        let mut txn = new_txn(world, tick);
        let source = txn.create_entity(EntityKind::Facility);
        txn.set_ground_location(source, place).unwrap();
        txn.set_component_workstation_marker(source, WorkstationMarker(WorkstationTag::OrchardRow))
            .unwrap();
        txn.set_component_resource_source(
            source,
            ResourceSource {
                commodity,
                available_quantity: Quantity(8),
                max_quantity: Quantity(12),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: NonZeroU32::new(1).unwrap(),
            },
        )
        .unwrap();
        txn.set_component_last_harvest_trace(source, trace).unwrap();
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
        source
    }

    fn entry(slot: u32, tick: u64) -> HarvestTraceEntry {
        HarvestTraceEntry {
            harvester: EntityId {
                slot,
                generation: 0,
            },
            tick: Tick(tick),
            quantity: 1,
            partial: false,
        }
    }

    #[test]
    fn item_decay_prunes_stale_harvest_trace_entries() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::VillageSquare);

        let stale_tick = 5;
        let fresh_tick = 250;
        let trace = LastHarvestTrace {
            entries: vec![
                entry(1, stale_tick),
                entry(2, stale_tick + 1),
                entry(3, fresh_tick),
            ],
        };
        let source = seed_harvest_source(&mut world, 1, place, CommodityKind::Apple, trace);

        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([6; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        // Tick `stale_tick + retention - 1`: not yet stale, no pruning event.
        let safe_tick = u64::from(HARVEST_TRACE_RETENTION_TICKS) + stale_tick - 1;
        item_decay_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            safe_tick,
        ))
        .unwrap();
        assert_eq!(
            world
                .get_component_last_harvest_trace(source)
                .map(|t| t.entries.len()),
            Some(3),
            "no entries should be pruned before retention window elapses"
        );

        // Advance past retention; the stale entries (Tick 5, Tick 6) leave
        // the window but Tick 250 stays.
        let prune_tick = u64::from(HARVEST_TRACE_RETENTION_TICKS) + stale_tick + 5;
        item_decay_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            prune_tick,
        ))
        .unwrap();

        let pruned = world
            .get_component_last_harvest_trace(source)
            .expect("trace component must remain present after pruning");
        assert_eq!(
            pruned.entries.len(),
            1,
            "only the fresh entry should survive"
        );
        assert_eq!(pruned.entries[0].tick, Tick(fresh_tick));
    }

    #[test]
    fn item_decay_skips_traces_without_stale_entries() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::VillageSquare);

        let trace = LastHarvestTrace {
            entries: vec![entry(1, 100), entry(2, 110)],
        };
        let source = seed_harvest_source(&mut world, 1, place, CommodityKind::Apple, trace);

        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([7; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        item_decay_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            150,
        ))
        .unwrap();

        assert_eq!(
            world
                .get_component_last_harvest_trace(source)
                .map(|t| t.entries.len()),
            Some(2)
        );
        assert!(
            event_log.events_by_tag(EventTag::ItemDecay).is_empty(),
            "no decay event should fire when no entries are pruned"
        );
    }

    #[test]
    fn item_decay_honors_world_level_retention_override() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::VillageSquare);

        let trace = LastHarvestTrace {
            entries: vec![entry(1, 1), entry(2, 50)],
        };
        let source = seed_harvest_source(&mut world, 1, place, CommodityKind::Apple, trace);
        // Tighten retention to 10 ticks so the entry at Tick 1 ages out
        // sooner than the default 200-tick window would allow.
        world.set_harvest_trace_retention_ticks(10);

        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([8; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        item_decay_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            55,
        ))
        .unwrap();

        let pruned = world
            .get_component_last_harvest_trace(source)
            .expect("trace remains");
        assert_eq!(pruned.entries.len(), 1);
        assert_eq!(pruned.entries[0].tick, Tick(50));
    }
}
