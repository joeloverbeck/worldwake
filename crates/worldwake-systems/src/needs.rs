use std::collections::BTreeMap;
use worldwake_core::{
    BodyCostPerTick, CauseRef, DeprivationExposure, EventTag, HomeostaticNeeds, VisibilitySpec,
    WitnessData, WorldTxn,
};
use worldwake_sim::{
    ActionDefRegistry, ActionInstance, ActionInstanceId, SystemDispatchTable, SystemError,
    SystemExecutionContext,
};

pub fn needs_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let SystemExecutionContext {
        world,
        event_log,
        rng: _rng,
        active_actions,
        action_defs,
        tick,
        system_id: _system_id,
    } = ctx;
    let updates = collect_updates(world, action_defs, active_actions)?;
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
    txn.add_tag(EventTag::System).add_tag(EventTag::WorldMutation);

    for update in updates {
        txn.set_component_homeostatic_needs(update.entity, update.needs)
            .map_err(|error| SystemError::new(error.to_string()))?;
        txn.set_component_deprivation_exposure(update.entity, update.exposure)
            .map_err(|error| SystemError::new(error.to_string()))?;
    }

    let _ = txn.commit(event_log);
    Ok(())
}

pub fn dispatch_table() -> SystemDispatchTable {
    SystemDispatchTable::from_handlers([
        needs_system,
        noop_system,
        noop_system,
        noop_system,
        noop_system,
        noop_system,
    ])
}

struct PendingUpdate {
    entity: worldwake_core::EntityId,
    needs: HomeostaticNeeds,
    exposure: DeprivationExposure,
}

fn collect_updates(
    world: &worldwake_core::World,
    action_defs: &ActionDefRegistry,
    active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,
) -> Result<Vec<PendingUpdate>, SystemError> {
    let body_costs = aggregate_body_costs(action_defs, active_actions)?;
    let mut updates = Vec::new();

    for (entity, needs) in world.query_homeostatic_needs() {
        if world.get_component_agent_data(entity).is_none() {
            continue;
        }

        let Some(profile) = world.get_component_metabolism_profile(entity).copied() else {
            continue;
        };
        let Some(thresholds) = world.get_component_drive_thresholds(entity).copied() else {
            continue;
        };
        let Some(exposure) = world.get_component_deprivation_exposure(entity).copied() else {
            continue;
        };

        let mut next_needs = apply_body_cost(*needs, basal_body_cost(profile));
        if let Some(cost) = body_costs.get(&entity).copied() {
            next_needs = apply_body_cost(next_needs, cost);
        }
        let next_exposure = update_exposure(exposure, next_needs, thresholds);

        if next_needs != *needs || next_exposure != exposure {
            updates.push(PendingUpdate {
                entity,
                needs: next_needs,
                exposure: next_exposure,
            });
        }
    }

    Ok(updates)
}

fn aggregate_body_costs(
    action_defs: &ActionDefRegistry,
    active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,
) -> Result<BTreeMap<worldwake_core::EntityId, BodyCostPerTick>, SystemError> {
    let mut costs = BTreeMap::new();

    for action in active_actions.values() {
        let def = action_defs
            .get(action.def_id)
            .ok_or_else(|| SystemError::new(format!("missing action def for {}", action.def_id)))?;
        costs
            .entry(action.actor)
            .and_modify(|aggregated| {
                *aggregated = combine_body_costs(*aggregated, def.body_cost_per_tick);
            })
            .or_insert(def.body_cost_per_tick);
    }

    Ok(costs)
}

fn basal_body_cost(profile: worldwake_core::MetabolismProfile) -> BodyCostPerTick {
    BodyCostPerTick::new(
        profile.hunger_rate,
        profile.thirst_rate,
        profile.fatigue_rate,
        profile.dirtiness_rate,
    )
}

fn combine_body_costs(lhs: BodyCostPerTick, rhs: BodyCostPerTick) -> BodyCostPerTick {
    BodyCostPerTick::new(
        lhs.hunger_delta.saturating_add(rhs.hunger_delta),
        lhs.thirst_delta.saturating_add(rhs.thirst_delta),
        lhs.fatigue_delta.saturating_add(rhs.fatigue_delta),
        lhs.dirtiness_delta.saturating_add(rhs.dirtiness_delta),
    )
}

fn apply_body_cost(needs: HomeostaticNeeds, cost: BodyCostPerTick) -> HomeostaticNeeds {
    HomeostaticNeeds::new(
        needs.hunger.saturating_add(cost.hunger_delta),
        needs.thirst.saturating_add(cost.thirst_delta),
        needs.fatigue.saturating_add(cost.fatigue_delta),
        needs.bladder,
        needs.dirtiness.saturating_add(cost.dirtiness_delta),
    )
}

fn update_exposure(
    exposure: DeprivationExposure,
    needs: HomeostaticNeeds,
    thresholds: worldwake_core::DriveThresholds,
) -> DeprivationExposure {
    DeprivationExposure {
        hunger_critical_ticks: critical_ticks(
            exposure.hunger_critical_ticks,
            needs.hunger,
            thresholds.hunger.critical(),
        ),
        thirst_critical_ticks: critical_ticks(
            exposure.thirst_critical_ticks,
            needs.thirst,
            thresholds.thirst.critical(),
        ),
        fatigue_critical_ticks: critical_ticks(
            exposure.fatigue_critical_ticks,
            needs.fatigue,
            thresholds.fatigue.critical(),
        ),
        bladder_critical_ticks: critical_ticks(
            exposure.bladder_critical_ticks,
            needs.bladder,
            thresholds.bladder.critical(),
        ),
    }
}

fn critical_ticks(
    current: u32,
    value: worldwake_core::Permille,
    critical: worldwake_core::Permille,
) -> u32 {
    if value >= critical {
        current.saturating_add(1)
    } else {
        0
    }
}

#[allow(clippy::unnecessary_wraps)]
fn noop_system(_ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{dispatch_table, needs_system};
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, BodyCostPerTick, CauseRef, ControlSource, DeprivationExposure,
        DriveThresholds, EventLog, EventTag, HomeostaticNeeds, MetabolismProfile, Permille, Seed,
        Tick, VisibilitySpec, WitnessData, World, WorldTxn,
    };
    use worldwake_sim::{
        ActionDef, ActionDefId, ActionDefRegistry, ActionHandlerId, ActionInstance,
        ActionInstanceId, ActionState, ActionStatus, DeterministicRng, DurationExpr,
        Interruptibility, SystemExecutionContext, SystemId,
    };

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

    fn spawn_agent(world: &mut World, tick: u64, name: &str) -> worldwake_core::EntityId {
        let mut txn = new_txn(world, tick);
        let agent = txn.create_agent(name, ControlSource::Ai).unwrap();
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
        agent
    }

    fn seed_agent(
        world: &mut World,
        agent: worldwake_core::EntityId,
        needs: HomeostaticNeeds,
        exposure: DeprivationExposure,
        profile: MetabolismProfile,
        thresholds: DriveThresholds,
    ) {
        let mut txn = new_txn(world, 2);
        txn.set_component_homeostatic_needs(agent, needs).unwrap();
        txn.set_component_deprivation_exposure(agent, exposure).unwrap();
        txn.set_component_metabolism_profile(agent, profile).unwrap();
        txn.set_component_drive_thresholds(agent, thresholds).unwrap();
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    fn metabolism(
        hunger: u16,
        thirst: u16,
        fatigue: u16,
        bladder: u16,
        dirtiness: u16,
    ) -> MetabolismProfile {
        MetabolismProfile::new(
            pm(hunger),
            pm(thirst),
            pm(fatigue),
            pm(bladder),
            pm(dirtiness),
            pm(20),
            nz(100),
            nz(100),
            nz(100),
            nz(100),
            nz(5),
            nz(5),
        )
    }

    fn register_action(registry: &mut ActionDefRegistry, body_cost_per_tick: BodyCostPerTick) {
        let _ = registry.register(ActionDef {
            id: ActionDefId(0),
            name: "travel".to_string(),
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::MIN),
            body_cost_per_tick,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::Hidden,
            causal_event_tags: BTreeSet::from([EventTag::System]),
            handler: ActionHandlerId(0),
        });
    }

    fn system_context<'a>(
        world: &'a mut World,
        event_log: &'a mut EventLog,
        rng: &'a mut DeterministicRng,
        active_actions: &'a BTreeMap<ActionInstanceId, ActionInstance>,
        action_defs: &'a ActionDefRegistry,
    ) -> SystemExecutionContext<'a> {
        SystemExecutionContext {
            world,
            event_log,
            rng,
            active_actions,
            action_defs,
            tick: Tick(7),
            system_id: SystemId::Needs,
        }
    }

    #[test]
    fn needs_system_applies_basal_progression_and_records_system_event() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = spawn_agent(&mut world, 1, "Aster");
        seed_agent(
            &mut world,
            agent,
            HomeostaticNeeds::new(pm(10), pm(20), pm(30), pm(40), pm(50)),
            DeprivationExposure::default(),
            metabolism(2, 3, 4, 5, 6),
            DriveThresholds::default(),
        );
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([1; 32]));

        needs_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_homeostatic_needs(agent),
            Some(&HomeostaticNeeds::new(pm(12), pm(23), pm(34), pm(40), pm(56)))
        );
        let record = event_log.get(worldwake_core::EventId(0)).unwrap();
        assert!(record.tags.contains(&EventTag::System));
        assert!(record.tags.contains(&EventTag::WorldMutation));
    }

    #[test]
    fn needs_system_applies_active_action_body_costs() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = spawn_agent(&mut world, 1, "Aster");
        seed_agent(
            &mut world,
            agent,
            HomeostaticNeeds::new(pm(100), pm(100), pm(100), pm(0), pm(100)),
            DeprivationExposure::default(),
            metabolism(1, 1, 1, 0, 1),
            DriveThresholds::default(),
        );

        let mut action_defs = ActionDefRegistry::new();
        register_action(
            &mut action_defs,
            BodyCostPerTick::new(pm(2), pm(3), pm(5), pm(4)),
        );
        let active_actions = BTreeMap::from([(
            ActionInstanceId(0),
            ActionInstance {
                instance_id: ActionInstanceId(0),
                def_id: ActionDefId(0),
                actor: agent,
                targets: Vec::new(),
                start_tick: Tick(6),
                remaining_ticks: 2,
                status: ActionStatus::Active,
                reservation_ids: Vec::new(),
                local_state: Some(ActionState::Empty),
            },
        )]);
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([2; 32]));

        needs_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_homeostatic_needs(agent),
            Some(&HomeostaticNeeds::new(pm(103), pm(104), pm(106), pm(0), pm(105)))
        );
    }

    #[test]
    fn needs_system_increments_deprivation_exposure_at_critical_thresholds() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = spawn_agent(&mut world, 1, "Aster");
        let thresholds = DriveThresholds::default();
        seed_agent(
            &mut world,
            agent,
            HomeostaticNeeds::new(
                thresholds.hunger.critical(),
                thresholds.thirst.critical(),
                thresholds.fatigue.critical(),
                thresholds.bladder.critical(),
                pm(0),
            ),
            DeprivationExposure {
                hunger_critical_ticks: 4,
                thirst_critical_ticks: 5,
                fatigue_critical_ticks: 6,
                bladder_critical_ticks: 7,
            },
            metabolism(0, 0, 0, 0, 0),
            thresholds,
        );
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([3; 32]));

        needs_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_deprivation_exposure(agent),
            Some(&DeprivationExposure {
                hunger_critical_ticks: 5,
                thirst_critical_ticks: 6,
                fatigue_critical_ticks: 7,
                bladder_critical_ticks: 8,
            })
        );
    }

    #[test]
    fn needs_system_resets_deprivation_exposure_when_pressure_drops_below_critical() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = spawn_agent(&mut world, 1, "Aster");
        let thresholds = DriveThresholds::default();
        seed_agent(
            &mut world,
            agent,
            HomeostaticNeeds::new(pm(10), pm(10), pm(10), pm(10), pm(0)),
            DeprivationExposure {
                hunger_critical_ticks: 4,
                thirst_critical_ticks: 5,
                fatigue_critical_ticks: 6,
                bladder_critical_ticks: 7,
            },
            metabolism(0, 0, 0, 0, 0),
            thresholds,
        );
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([4; 32]));

        needs_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_deprivation_exposure(agent),
            Some(&DeprivationExposure::default())
        );
    }

    #[test]
    fn dispatch_table_registers_needs_system_without_changing_other_slots() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = spawn_agent(&mut world, 1, "Aster");
        seed_agent(
            &mut world,
            agent,
            HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(0), pm(0)),
            DeprivationExposure::default(),
            metabolism(1, 0, 0, 0, 0),
            DriveThresholds::default(),
        );
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([5; 32]));
        let systems = dispatch_table();

        systems
            .get(SystemId::Needs)(system_context(
                &mut world,
                &mut event_log,
                &mut rng,
                &active_actions,
                &action_defs,
            ))
            .unwrap();

        assert_eq!(
            world.get_component_homeostatic_needs(agent),
            Some(&HomeostaticNeeds::new(pm(1), pm(0), pm(0), pm(0), pm(0)))
        );

        systems
            .get(SystemId::Trade)(SystemExecutionContext {
                world: &mut world,
                event_log: &mut event_log,
                rng: &mut rng,
                active_actions: &active_actions,
                action_defs: &action_defs,
                tick: Tick(8),
                system_id: SystemId::Trade,
            })
            .unwrap();

        assert_eq!(event_log.len(), 1);
    }
}
