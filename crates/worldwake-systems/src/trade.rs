use worldwake_core::{
    CauseRef, CommodityKind, DemandMemory, EntityId, EventTag, Quantity, Tick, VisibilitySpec,
    WitnessData, World, WorldTxn,
};
use worldwake_sim::{SystemError, SystemExecutionContext};

pub fn trade_system_tick(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let SystemExecutionContext {
        world,
        event_log,
        rng: _rng,
        active_actions: _active_actions,
        action_defs: _action_defs,
        tick,
        system_id: _system_id,
    } = ctx;

    let updates = collect_aging_updates(world, tick);
    if updates.is_empty() {
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

    for (entity, memory) in updates {
        txn.add_target(entity);
        txn.set_component_demand_memory(entity, memory)
            .map_err(|error| SystemError::new(error.to_string()))?;
    }

    let _ = txn.commit(event_log);
    Ok(())
}

#[must_use]
pub fn restock_candidates(agent: EntityId, world: &World) -> Vec<CommodityKind> {
    let Some(profile) = world.get_component_merchandise_profile(agent) else {
        return Vec::new();
    };

    profile
        .sale_kinds
        .iter()
        .copied()
        .filter(|kind| world.controlled_commodity_quantity(agent, *kind) == Quantity(0))
        .collect()
}

fn collect_aging_updates(world: &World, tick: Tick) -> Vec<(worldwake_core::EntityId, DemandMemory)> {
    let mut updates = Vec::new();

    for (entity, memory) in world.query_demand_memory() {
        let Some(profile) = world.get_component_trade_disposition_profile(entity) else {
            continue;
        };
        let retention = u64::from(profile.demand_memory_retention_ticks);
        let observations = memory
            .observations
            .iter()
            .filter(|observation| tick.0.saturating_sub(observation.tick.0) <= retention)
            .cloned()
            .collect::<Vec<_>>();
        if observations.len() == memory.observations.len() {
            continue;
        }
        updates.push((entity, DemandMemory { observations }));
    }

    updates
}

#[cfg(test)]
mod tests {
    use super::{restock_candidates, trade_system_tick};
    use crate::dispatch_table;
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, CauseRef, CommodityKind, ControlSource, DemandMemory,
        DemandObservation, DemandObservationReason, EventLog, EventTag, MerchandiseProfile,
        Permille, Quantity, Seed, Tick, TradeDispositionProfile, VisibilitySpec, WitnessData,
        World, WorldTxn,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionInstance, ActionInstanceId, DeterministicRng,
        SystemExecutionContext, SystemId,
    };

    fn entity(slot: u32) -> worldwake_core::EntityId {
        worldwake_core::EntityId {
            slot,
            generation: 1,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn nz(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
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

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    fn observation(tick: u64, kind: CommodityKind) -> DemandObservation {
        DemandObservation {
            commodity: kind,
            quantity: Quantity(1),
            place: entity(99),
            tick: Tick(tick),
            counterparty: Some(entity(77)),
            reason: DemandObservationReason::WantedToBuyButNoSeller,
        }
    }

    fn seed_agent(
        world: &mut World,
        name: &str,
        memory: Option<DemandMemory>,
        retention_ticks: Option<u32>,
    ) -> worldwake_core::EntityId {
        let agent = {
            let mut txn = new_txn(world, 1);
            let agent = txn.create_agent(name, ControlSource::Ai).unwrap();
            commit_txn(txn);
            agent
        };
        let mut txn = new_txn(world, 2);
        if let Some(memory) = memory {
            txn.set_component_demand_memory(agent, memory).unwrap();
        }
        if let Some(retention_ticks) = retention_ticks {
            txn.set_component_trade_disposition_profile(
                agent,
                TradeDispositionProfile {
                    negotiation_round_ticks: nz(1),
                    initial_offer_bias: pm(500),
                    concession_rate: pm(100),
                    demand_memory_retention_ticks: retention_ticks,
                },
            )
            .unwrap();
        }
        commit_txn(txn);
        agent
    }

    fn sale_profile(kinds: &[CommodityKind]) -> MerchandiseProfile {
        MerchandiseProfile {
            sale_kinds: kinds.iter().copied().collect::<BTreeSet<_>>(),
            home_market: None,
        }
    }

    fn set_merchandise_profile(
        world: &mut World,
        agent: worldwake_core::EntityId,
        profile: MerchandiseProfile,
    ) {
        let mut txn = new_txn(world, 2);
        txn.set_component_merchandise_profile(agent, profile).unwrap();
        commit_txn(txn);
    }

    fn grant_stock(
        world: &mut World,
        holder: worldwake_core::EntityId,
        place: worldwake_core::EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> worldwake_core::EntityId {
        let mut txn = new_txn(world, 3);
        let lot = txn.create_item_lot(commodity, quantity).unwrap();
        txn.set_ground_location(lot, place).unwrap();
        txn.set_possessor(lot, holder).unwrap();
        txn.set_owner(lot, holder).unwrap();
        commit_txn(txn);
        lot
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
            system_id: SystemId::Trade,
        }
    }

    #[test]
    fn restock_candidates_returns_missing_sale_kinds_for_merchants() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = seed_agent(&mut world, "Merchant", None, None);
        set_merchandise_profile(
            &mut world,
            agent,
            sale_profile(&[CommodityKind::Bread, CommodityKind::Water]),
        );

        let candidates = restock_candidates(agent, &world);

        assert_eq!(candidates, vec![CommodityKind::Bread, CommodityKind::Water]);
    }

    #[test]
    fn restock_candidates_excludes_sale_kinds_with_available_stock() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = seed_agent(&mut world, "Stocked", None, None);
        set_merchandise_profile(
            &mut world,
            agent,
            sale_profile(&[CommodityKind::Bread, CommodityKind::Water]),
        );
        let _bread_lot = grant_stock(&mut world, agent, place, CommodityKind::Bread, Quantity(2));

        let candidates = restock_candidates(agent, &world);

        assert_eq!(candidates, vec![CommodityKind::Water]);
    }

    #[test]
    fn restock_candidates_returns_empty_without_merchandise_profile() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = seed_agent(
            &mut world,
            "MemoryOnly",
            Some(DemandMemory {
                observations: vec![observation(4, CommodityKind::Bread)],
            }),
            Some(10),
        );

        let candidates = restock_candidates(agent, &world);

        assert!(candidates.is_empty());
    }

    #[test]
    fn restock_candidates_ignore_demand_memory_when_sale_intent_and_stock_match() {
        let mut left = World::new(build_prototype_world()).unwrap();
        let mut right = World::new(build_prototype_world()).unwrap();
        let place = left.topology().place_ids().next().unwrap();
        let left_agent = seed_agent(&mut left, "Left", None, None);
        let right_agent = seed_agent(
            &mut right,
            "Right",
            Some(DemandMemory {
                observations: vec![observation(4, CommodityKind::Bread)],
            }),
            Some(10),
        );
        let profile = sale_profile(&[CommodityKind::Bread, CommodityKind::Water]);
        set_merchandise_profile(&mut left, left_agent, profile.clone());
        set_merchandise_profile(&mut right, right_agent, profile);
        let _left_lot = grant_stock(&mut left, left_agent, place, CommodityKind::Water, Quantity(1));
        let _right_lot =
            grant_stock(&mut right, right_agent, place, CommodityKind::Water, Quantity(1));

        assert_eq!(
            restock_candidates(left_agent, &left),
            restock_candidates(right_agent, &right)
        );
    }

    #[test]
    fn restock_candidates_do_not_use_partial_stock_thresholds() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = seed_agent(&mut world, "Thresholdless", None, None);
        set_merchandise_profile(&mut world, agent, sale_profile(&[CommodityKind::Bread]));
        let _lot = grant_stock(&mut world, agent, place, CommodityKind::Bread, Quantity(1));

        let candidates = restock_candidates(agent, &world);

        assert!(candidates.is_empty());
    }

    #[test]
    fn restock_candidates_are_read_only() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = seed_agent(
            &mut world,
            "Readonly",
            Some(DemandMemory {
                observations: vec![observation(2, CommodityKind::Bread)],
            }),
            Some(10),
        );
        let profile = sale_profile(&[CommodityKind::Bread, CommodityKind::Water]);
        set_merchandise_profile(&mut world, agent, profile.clone());
        let _lot = grant_stock(&mut world, agent, place, CommodityKind::Water, Quantity(1));
        let before_memory = world.get_component_demand_memory(agent).unwrap().clone();

        let candidates = restock_candidates(agent, &world);

        assert_eq!(candidates, vec![CommodityKind::Bread]);
        assert_eq!(world.get_component_merchandise_profile(agent), Some(&profile));
        assert_eq!(world.get_component_demand_memory(agent), Some(&before_memory));
        assert_eq!(
            world.controlled_commodity_quantity(agent, CommodityKind::Water),
            Quantity(1)
        );
    }

    #[test]
    fn trade_system_tick_prunes_observations_older_than_retention() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = seed_agent(
            &mut world,
            "Aster",
            Some(DemandMemory {
                observations: vec![
                    observation(4, CommodityKind::Bread),
                    observation(8, CommodityKind::Coin),
                    observation(9, CommodityKind::Apple),
                ],
            }),
            Some(2),
        );
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([1; 32]));

        trade_system_tick(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            10,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_demand_memory(agent).unwrap().observations,
            vec![
                observation(8, CommodityKind::Coin),
                observation(9, CommodityKind::Apple),
            ]
        );
        assert_eq!(event_log.len(), 1);
        let record = event_log.get(worldwake_core::EventId(0)).unwrap();
        assert!(record.tags.contains(&EventTag::System));
        assert!(record.tags.contains(&EventTag::WorldMutation));
        assert_eq!(record.target_ids, vec![agent]);
    }

    #[test]
    fn trade_system_tick_respects_per_agent_retention_windows() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let cautious = seed_agent(
            &mut world,
            "Cautious",
            Some(DemandMemory {
                observations: vec![observation(5, CommodityKind::Bread)],
            }),
            Some(5),
        );
        let forgetful = seed_agent(
            &mut world,
            "Forgetful",
            Some(DemandMemory {
                observations: vec![observation(5, CommodityKind::Bread)],
            }),
            Some(3),
        );
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([2; 32]));

        trade_system_tick(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            10,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_demand_memory(cautious).unwrap().observations,
            vec![observation(5, CommodityKind::Bread)]
        );
        assert!(world
            .get_component_demand_memory(forgetful)
            .unwrap()
            .observations
            .is_empty());
    }

    #[test]
    fn trade_system_tick_preserves_observations_at_retention_boundary() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = seed_agent(
            &mut world,
            "Boundary",
            Some(DemandMemory {
                observations: vec![
                    observation(7, CommodityKind::Bread),
                    observation(8, CommodityKind::Apple),
                ],
            }),
            Some(3),
        );
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([3; 32]));

        trade_system_tick(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            10,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_demand_memory(agent).unwrap().observations,
            vec![
                observation(7, CommodityKind::Bread),
                observation(8, CommodityKind::Apple),
            ]
        );
        assert!(event_log.is_empty());
    }

    #[test]
    fn trade_system_tick_skips_agents_without_trade_disposition_profile() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = seed_agent(
            &mut world,
            "Unprofiled",
            Some(DemandMemory {
                observations: vec![observation(1, CommodityKind::Bread)],
            }),
            None,
        );
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([4; 32]));

        trade_system_tick(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            10,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_demand_memory(agent).unwrap().observations,
            vec![observation(1, CommodityKind::Bread)]
        );
        assert!(event_log.is_empty());
    }

    #[test]
    fn trade_system_tick_skips_agents_without_demand_memory() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([5; 32]));
        let _ = seed_agent(&mut world, "NoMemory", None, Some(1));

        trade_system_tick(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            10,
        ))
        .unwrap();

        assert!(event_log.is_empty());
    }

    #[test]
    fn dispatch_table_routes_trade_slot_to_trade_system_tick() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = seed_agent(
            &mut world,
            "Aster",
            Some(DemandMemory {
                observations: vec![observation(1, CommodityKind::Bread)],
            }),
            Some(1),
        );
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([6; 32]));
        let systems = dispatch_table();

        systems.get(SystemId::Trade)(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            10,
        ))
        .unwrap();

        assert!(world
            .get_component_demand_memory(agent)
            .unwrap()
            .observations
            .is_empty());
        assert_eq!(event_log.len(), 1);
    }

    #[test]
    fn trade_system_tick_is_deterministic_for_same_world_and_tick() {
        let mut left = World::new(build_prototype_world()).unwrap();
        let mut right = World::new(build_prototype_world()).unwrap();
        let left_agent = seed_agent(
            &mut left,
            "Aster",
            Some(DemandMemory {
                observations: vec![
                    observation(1, CommodityKind::Bread),
                    observation(9, CommodityKind::Apple),
                ],
            }),
            Some(2),
        );
        let right_agent = seed_agent(
            &mut right,
            "Aster",
            Some(DemandMemory {
                observations: vec![
                    observation(1, CommodityKind::Bread),
                    observation(9, CommodityKind::Apple),
                ],
            }),
            Some(2),
        );
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut left_log = EventLog::new();
        let mut right_log = EventLog::new();
        let mut left_rng = DeterministicRng::new(Seed([7; 32]));
        let mut right_rng = DeterministicRng::new(Seed([7; 32]));

        trade_system_tick(system_context(
            &mut left,
            &mut left_log,
            &mut left_rng,
            &active_actions,
            &action_defs,
            10,
        ))
        .unwrap();
        trade_system_tick(system_context(
            &mut right,
            &mut right_log,
            &mut right_rng,
            &active_actions,
            &action_defs,
            10,
        ))
        .unwrap();

        assert_eq!(
            left.get_component_demand_memory(left_agent),
            right.get_component_demand_memory(right_agent)
        );
        assert_eq!(left_log.len(), right_log.len());
    }
}
