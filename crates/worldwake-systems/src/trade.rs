use worldwake_core::{
    CauseRef, DemandMemory, EventTag, Tick, VisibilitySpec, WitnessData, World, WorldTxn,
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
    use super::trade_system_tick;
    use crate::dispatch_table;
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, CauseRef, CommodityKind, ControlSource, DemandMemory,
        DemandObservation, DemandObservationReason, EventLog, EventTag, Permille, Quantity, Seed,
        Tick, TradeDispositionProfile, VisibilitySpec, WitnessData, World, WorldTxn,
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
