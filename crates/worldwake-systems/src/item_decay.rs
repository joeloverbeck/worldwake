use worldwake_core::{
    CauseRef, EventLog, EventTag, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
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
        CauseRef, CommodityKind, EntityId, EventLog, EventTag, EventView, PrototypePlace,
        Quantity, Seed, Tick, VisibilitySpec, WitnessData, World, WorldTxn, build_prototype_world,
        prototype_place_entity,
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
}
