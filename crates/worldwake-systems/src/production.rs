use worldwake_core::{
    CauseRef, EventTag, Quantity, ResourceSource, Tick, VisibilitySpec, WitnessData, World,
    WorldTxn,
};
use worldwake_sim::{SystemError, SystemExecutionContext};

pub fn resource_regeneration_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let SystemExecutionContext {
        world,
        event_log,
        rng: _rng,
        active_actions: _active_actions,
        action_defs: _action_defs,
        tick,
        system_id: _system_id,
    } = ctx;

    let updates = collect_updates(world, tick);
    for update in updates {
        let mut txn = WorldTxn::new(
            world,
            tick,
            CauseRef::SystemTick(tick),
            None,
            update.place_id,
            VisibilitySpec::Hidden,
            WitnessData::default(),
        );
        txn.add_tag(EventTag::System)
            .add_tag(EventTag::WorldMutation)
            .add_target(update.entity);
        txn.set_component_resource_source(update.entity, update.next)
            .map_err(|error| SystemError::new(error.to_string()))?;
        let _ = txn.commit(event_log);
    }

    Ok(())
}

#[derive(Clone)]
struct PendingUpdate {
    entity: worldwake_core::EntityId,
    place_id: Option<worldwake_core::EntityId>,
    next: ResourceSource,
}

fn collect_updates(world: &World, tick: Tick) -> Vec<PendingUpdate> {
    let mut updates = Vec::new();

    for (entity, source) in world.query_resource_source() {
        let Some(next) = next_state(source.clone(), tick) else {
            continue;
        };
        updates.push(PendingUpdate {
            entity,
            place_id: resource_place(world, entity),
            next,
        });
    }

    updates
}

fn next_state(source: ResourceSource, tick: Tick) -> Option<ResourceSource> {
    let interval = u64::from(source.regeneration_ticks_per_unit?.get());
    if source.available_quantity == source.max_quantity {
        return None;
    }

    let Some(last_tick) = source.last_regeneration_tick else {
        let mut next = source;
        next.last_regeneration_tick = Some(tick);
        return Some(next);
    };

    if tick.0.saturating_sub(last_tick.0) < interval {
        return None;
    }

    let mut next = source;
    next.available_quantity = Quantity(next.available_quantity.0.saturating_add(1));
    if next.available_quantity > next.max_quantity {
        next.available_quantity = next.max_quantity;
    }
    next.last_regeneration_tick = Some(tick);
    Some(next)
}

fn resource_place(
    world: &World,
    entity: worldwake_core::EntityId,
) -> Option<worldwake_core::EntityId> {
    if world.topology().place(entity).is_some() {
        Some(entity)
    } else {
        world.effective_place(entity)
    }
}

#[cfg(test)]
mod tests {
    use super::{next_state, resource_regeneration_system};
    use crate::dispatch_table;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, CauseRef, CommodityKind, ComponentDelta, ComponentKind,
        ComponentValue, DemandMemory, DemandObservation, DemandObservationReason, EventLog,
        EventTag, Permille, Quantity, ResourceSource, Seed, StateDelta, Tick,
        TradeDispositionProfile, VisibilitySpec, WitnessData, World, WorldTxn,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionInstance, ActionInstanceId, DeterministicRng,
        SystemExecutionContext, SystemId,
    };

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    fn source(
        available_quantity: u32,
        max_quantity: u32,
        regeneration_ticks_per_unit: Option<u32>,
        last_regeneration_tick: Option<u64>,
    ) -> ResourceSource {
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: Quantity(available_quantity),
            max_quantity: Quantity(max_quantity),
            regeneration_ticks_per_unit: regeneration_ticks_per_unit.map(nz),
            last_regeneration_tick: last_regeneration_tick.map(Tick),
        }
    }

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

    fn seed_source_on_first_place(
        world: &mut World,
        source: ResourceSource,
    ) -> worldwake_core::EntityId {
        let place = world.topology().place_ids().next().unwrap();
        let mut txn = new_txn(world, 1);
        txn.set_component_resource_source(place, source).unwrap();
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
        place
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn seed_trade_memory(world: &mut World) -> worldwake_core::EntityId {
        let place = world.topology().place_ids().next().unwrap();
        let mut txn = new_txn(world, 1);
        let agent = txn.create_agent("Trader", worldwake_core::ControlSource::Ai).unwrap();
        txn.set_component_demand_memory(
            agent,
            DemandMemory {
                observations: vec![DemandObservation {
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(1),
                    place,
                    tick: Tick(1),
                    counterparty: None,
                    reason: DemandObservationReason::WantedToBuyButNoSeller,
                }],
            },
        )
        .unwrap();
        txn.set_component_trade_disposition_profile(
            agent,
            TradeDispositionProfile {
                negotiation_round_ticks: nz(1),
                initial_offer_bias: pm(500),
                concession_rate: pm(100),
                demand_memory_retention_ticks: 1,
            },
        )
        .unwrap();
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
        agent
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
            tick: Tick(tick),
            system_id: SystemId::Production,
        }
    }

    #[test]
    fn next_state_initializes_regeneration_baseline_before_first_tick_interval() {
        let initial = source(1, 3, Some(5), None);

        assert_eq!(
            next_state(initial.clone(), Tick(10)),
            Some(source(1, 3, Some(5), Some(10)))
        );
        assert_eq!(next_state(source(1, 3, Some(5), Some(10)), Tick(14)), None);
        assert_eq!(
            next_state(source(1, 3, Some(5), Some(10)), Tick(15)),
            Some(source(2, 3, Some(5), Some(15)))
        );
    }

    #[test]
    fn resource_regeneration_system_grows_one_unit_at_interval_and_records_event() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = seed_source_on_first_place(&mut world, source(1, 3, Some(5), Some(2)));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([1; 32]));

        resource_regeneration_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            7,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_resource_source(place),
            Some(&source(2, 3, Some(5), Some(7)))
        );
        assert_eq!(event_log.len(), 1);
        let record = event_log.get(worldwake_core::EventId(0)).unwrap();
        assert_eq!(record.cause, CauseRef::SystemTick(Tick(7)));
        assert_eq!(record.place_id, Some(place));
        assert_eq!(record.target_ids, vec![place]);
        assert!(record.tags.contains(&EventTag::System));
        assert!(record.tags.contains(&EventTag::WorldMutation));
        assert_eq!(
            record.state_deltas,
            vec![StateDelta::Component(ComponentDelta::Set {
                entity: place,
                component_kind: ComponentKind::ResourceSource,
                before: Some(ComponentValue::ResourceSource(source(
                    1,
                    3,
                    Some(5),
                    Some(2)
                ))),
                after: ComponentValue::ResourceSource(source(2, 3, Some(5), Some(7))),
            })]
        );
    }

    #[test]
    fn resource_regeneration_system_respects_cap_and_skips_full_or_disabled_sources() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let second_place = world.topology().place_ids().nth(1).unwrap();
        let mut txn = new_txn(&mut world, 1);
        txn.set_component_resource_source(place, source(3, 3, Some(5), Some(0)))
            .unwrap();
        txn.set_component_resource_source(second_place, source(1, 3, None, None))
            .unwrap();
        let mut setup_log = EventLog::new();
        let _ = txn.commit(&mut setup_log);

        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([2; 32]));

        resource_regeneration_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            10,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_resource_source(place),
            Some(&source(3, 3, Some(5), Some(0)))
        );
        assert_eq!(
            world.get_component_resource_source(second_place),
            Some(&source(1, 3, None, None))
        );
        assert!(event_log.is_empty());
    }

    #[test]
    fn resource_regeneration_system_updates_multiple_sources_independently() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let first_place = world.topology().place_ids().next().unwrap();
        let second_place = world.topology().place_ids().nth(1).unwrap();
        let mut txn = new_txn(&mut world, 1);
        txn.set_component_resource_source(first_place, source(0, 2, Some(3), Some(2)))
            .unwrap();
        txn.set_component_resource_source(second_place, source(1, 2, Some(5), Some(1)))
            .unwrap();
        let mut setup_log = EventLog::new();
        let _ = txn.commit(&mut setup_log);

        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([3; 32]));

        resource_regeneration_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            6,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_resource_source(first_place),
            Some(&source(1, 2, Some(3), Some(6)))
        );
        assert_eq!(
            world.get_component_resource_source(second_place),
            Some(&source(2, 2, Some(5), Some(6)))
        );
        assert_eq!(event_log.len(), 2);
    }

    #[test]
    fn dispatch_table_registers_production_system_and_trade_slot_ages_demand_memory() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = seed_source_on_first_place(&mut world, source(1, 3, Some(5), Some(2)));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([4; 32]));
        let systems = dispatch_table();

        systems.get(SystemId::Production)(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            7,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_resource_source(place),
            Some(&source(2, 3, Some(5), Some(7)))
        );

        let mut noop_world = World::new(build_prototype_world()).unwrap();
        let mut noop_log = EventLog::new();
        let mut noop_rng = DeterministicRng::new(Seed([5; 32]));
        let trader = seed_trade_memory(&mut noop_world);
        systems.get(SystemId::Trade)(SystemExecutionContext {
            world: &mut noop_world,
            event_log: &mut noop_log,
            rng: &mut noop_rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            tick: Tick(8),
            system_id: SystemId::Trade,
        })
        .unwrap();

        assert!(noop_world
            .get_component_demand_memory(trader)
            .unwrap()
            .observations
            .is_empty());
        assert_eq!(noop_log.len(), 1);
    }
}
