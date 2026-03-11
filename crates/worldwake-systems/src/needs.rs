use std::collections::BTreeMap;
use worldwake_core::{
    BodyCostPerTick, BodyPart, CauseRef, CommodityKind, DeprivationExposure, DeprivationKind,
    EventTag, HomeostaticNeeds, Quantity, Tick, VisibilitySpec, WitnessData, WorldTxn, Wound,
    WoundCause, WoundList,
};
use worldwake_sim::{
    ActionDefRegistry, ActionInstance, ActionInstanceId, SystemError, SystemExecutionContext,
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
    let updates = collect_updates(world, action_defs, active_actions, tick)?;
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

    for update in updates {
        txn.set_component_homeostatic_needs(update.entity, update.needs)
            .map_err(|error| SystemError::new(error.to_string()))?;
        txn.set_component_deprivation_exposure(update.entity, update.exposure)
            .map_err(|error| SystemError::new(error.to_string()))?;
        if let Some(wound_list) = update.wound_list {
            txn.set_component_wound_list(update.entity, wound_list)
                .map_err(|error| SystemError::new(error.to_string()))?;
        }
        if let Some(place) = update.waste_place {
            let waste = txn
                .create_item_lot(CommodityKind::Waste, Quantity(1))
                .map_err(|error| SystemError::new(error.to_string()))?;
            txn.set_ground_location(waste, place)
                .map_err(|error| SystemError::new(error.to_string()))?;
        }
    }

    let _ = txn.commit(event_log);
    Ok(())
}

struct PendingUpdate {
    entity: worldwake_core::EntityId,
    needs: HomeostaticNeeds,
    exposure: DeprivationExposure,
    wound_list: Option<WoundList>,
    waste_place: Option<worldwake_core::EntityId>,
}

fn collect_updates(
    world: &worldwake_core::World,
    action_defs: &ActionDefRegistry,
    active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,
    tick: Tick,
) -> Result<Vec<PendingUpdate>, SystemError> {
    let body_costs = aggregate_body_costs(action_defs, active_actions)?;
    let mut updates = Vec::new();

    for (entity, needs) in world.query_homeostatic_needs() {
        if world.get_component_agent_data(entity).is_none() {
            continue;
        }
        if world.get_component_dead_at(entity).is_some() {
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

        let mut next_needs = apply_basal_progression(*needs, profile);
        if let Some(cost) = body_costs.get(&entity).copied() {
            next_needs = apply_action_body_cost(next_needs, cost);
        }
        let mut next_exposure = update_exposure(exposure, next_needs, thresholds);
        let (wound_list, waste_place) = apply_deprivation_consequences(
            world,
            entity,
            tick,
            profile,
            &mut next_needs,
            &mut next_exposure,
        )?;

        if next_needs != *needs
            || next_exposure != exposure
            || wound_list.is_some()
            || waste_place.is_some()
        {
            updates.push(PendingUpdate {
                entity,
                needs: next_needs,
                exposure: next_exposure,
                wound_list,
                waste_place,
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

fn combine_body_costs(lhs: BodyCostPerTick, rhs: BodyCostPerTick) -> BodyCostPerTick {
    BodyCostPerTick::new(
        lhs.hunger_delta.saturating_add(rhs.hunger_delta),
        lhs.thirst_delta.saturating_add(rhs.thirst_delta),
        lhs.fatigue_delta.saturating_add(rhs.fatigue_delta),
        lhs.dirtiness_delta.saturating_add(rhs.dirtiness_delta),
    )
}

fn apply_basal_progression(
    needs: HomeostaticNeeds,
    profile: worldwake_core::MetabolismProfile,
) -> HomeostaticNeeds {
    HomeostaticNeeds::new(
        needs.hunger.saturating_add(profile.hunger_rate),
        needs.thirst.saturating_add(profile.thirst_rate),
        needs.fatigue.saturating_add(profile.fatigue_rate),
        needs.bladder.saturating_add(profile.bladder_rate),
        needs.dirtiness.saturating_add(profile.dirtiness_rate),
    )
}

fn apply_action_body_cost(needs: HomeostaticNeeds, cost: BodyCostPerTick) -> HomeostaticNeeds {
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

fn apply_deprivation_consequences(
    world: &worldwake_core::World,
    entity: worldwake_core::EntityId,
    tick: Tick,
    profile: worldwake_core::MetabolismProfile,
    needs: &mut HomeostaticNeeds,
    exposure: &mut DeprivationExposure,
) -> Result<(Option<WoundList>, Option<worldwake_core::EntityId>), SystemError> {
    let mut wound_list = None;
    let mut wounds_changed = false;

    if exposure.hunger_critical_ticks >= profile.starvation_tolerance_ticks.get() {
        append_deprivation_wound(
            &mut wound_list,
            world.get_component_wound_list(entity),
            DeprivationKind::Starvation,
            needs.hunger,
            tick,
        );
        exposure.hunger_critical_ticks = 0;
        wounds_changed = true;
    }

    if exposure.thirst_critical_ticks >= profile.dehydration_tolerance_ticks.get() {
        append_deprivation_wound(
            &mut wound_list,
            world.get_component_wound_list(entity),
            DeprivationKind::Dehydration,
            needs.thirst,
            tick,
        );
        exposure.thirst_critical_ticks = 0;
        wounds_changed = true;
    }

    let waste_place =
        if exposure.bladder_critical_ticks >= profile.bladder_accident_tolerance_ticks.get() {
            let bladder_pressure = needs.bladder;
            let place = world.effective_place(entity).ok_or_else(|| {
                SystemError::new(format!(
                    "agent {entity} cannot create waste without an effective place"
                ))
            })?;
            *needs = HomeostaticNeeds::new(
                needs.hunger,
                needs.thirst,
                needs.fatigue,
                worldwake_core::Permille::new(0).expect("zero is a valid permille"),
                needs.dirtiness.saturating_add(bladder_pressure),
            );
            exposure.bladder_critical_ticks = 0;
            Some(place)
        } else {
            None
        };

    Ok((
        wounds_changed.then_some(wound_list.unwrap_or_default()),
        waste_place,
    ))
}

fn append_deprivation_wound(
    wound_list: &mut Option<WoundList>,
    existing: Option<&WoundList>,
    kind: DeprivationKind,
    severity: worldwake_core::Permille,
    tick: Tick,
) {
    let list = wound_list.get_or_insert_with(|| existing.cloned().unwrap_or_default());
    let wound_id = list.next_wound_id();
    list.wounds.push(Wound {
        id: wound_id,
        body_part: BodyPart::Torso,
        cause: WoundCause::Deprivation(kind),
        severity,
        inflicted_at: tick,
        bleed_rate_per_tick: worldwake_core::Permille::new(0).expect("zero is a valid permille"),
    });
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

#[cfg(test)]
mod tests {
    use super::needs_system;
    use crate::dispatch_table;
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, BodyCostPerTick, CauseRef, CommodityKind, ControlSource, DeadAt,
        DemandMemory, DemandObservation, DemandObservationReason, DeprivationExposure,
        DeprivationKind, DriveThresholds, EventLog, EventTag, HomeostaticNeeds, MetabolismProfile,
        Permille, Quantity, Seed, Tick, TradeDispositionProfile, VisibilitySpec, WitnessData,
        World, WorldTxn, WoundCause,
    };
    use worldwake_sim::{
        ActionDef, ActionDefId, ActionDefRegistry, ActionDuration, ActionHandlerId,
        ActionInstance, ActionInstanceId, ActionPayload, ActionState, ActionStatus,
        DeterministicRng, DurationExpr, Interruptibility, SystemExecutionContext, SystemId,
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

    fn first_place(world: &World) -> worldwake_core::EntityId {
        world.topology().place_ids().next().unwrap()
    }

    fn place_agent(
        world: &mut World,
        agent: worldwake_core::EntityId,
        place: worldwake_core::EntityId,
    ) {
        let mut txn = new_txn(world, 2);
        txn.set_ground_location(agent, place).unwrap();
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
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
        txn.set_component_deprivation_exposure(agent, exposure)
            .unwrap();
        txn.set_component_metabolism_profile(agent, profile)
            .unwrap();
        txn.set_component_drive_thresholds(agent, thresholds)
            .unwrap();
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
            payload: worldwake_sim::ActionPayload::None,
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

    fn seed_trade_memory(world: &mut World) -> worldwake_core::EntityId {
        let place = first_place(world);
        let mut txn = new_txn(world, 2);
        let agent = txn.create_agent("Trader", ControlSource::Ai).unwrap();
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
            Some(&HomeostaticNeeds::new(
                pm(12),
                pm(23),
                pm(34),
                pm(45),
                pm(56)
            ))
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
                payload: ActionPayload::None,
                actor: agent,
                targets: Vec::new(),
                start_tick: Tick(6),
                remaining_duration: ActionDuration::Finite(2),
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
            Some(&HomeostaticNeeds::new(
                pm(103),
                pm(104),
                pm(106),
                pm(0),
                pm(105)
            ))
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
    fn needs_system_applies_bladder_basal_progression() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = spawn_agent(&mut world, 1, "Aster");
        seed_agent(
            &mut world,
            agent,
            HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(10), pm(0)),
            DeprivationExposure::default(),
            metabolism(0, 0, 0, 7, 0),
            DriveThresholds::default(),
        );
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([9; 32]));

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
            Some(&HomeostaticNeeds::new(pm(0), pm(0), pm(0), pm(17), pm(0)))
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
    fn needs_system_skips_dead_agents_entirely() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = spawn_agent(&mut world, 1, "Dead");
        let place = first_place(&world);
        place_agent(&mut world, agent, place);
        seed_agent(
            &mut world,
            agent,
            HomeostaticNeeds::new(pm(100), pm(120), pm(140), pm(0), pm(0)),
            DeprivationExposure::default(),
            metabolism(10, 12, 14, 0, 0),
            DriveThresholds::default(),
        );
        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_dead_at(agent, DeadAt(Tick(3))).unwrap();
            let _ = txn.commit(&mut EventLog::new());
        }
        let original_needs = *world.get_component_homeostatic_needs(agent).unwrap();
        let original_exposure = *world.get_component_deprivation_exposure(agent).unwrap();
        let mut log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([9; 32]));
        let active_actions = BTreeMap::new();
        let defs = ActionDefRegistry::new();

        needs_system(SystemExecutionContext {
            world: &mut world,
            event_log: &mut log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &defs,
            tick: Tick(4),
            system_id: SystemId::Needs,
        })
        .unwrap();

        assert_eq!(world.get_component_homeostatic_needs(agent), Some(&original_needs));
        assert_eq!(
            world.get_component_deprivation_exposure(agent),
            Some(&original_exposure)
        );
        assert!(log.is_empty());
    }

    #[test]
    fn needs_system_adds_starvation_wound_and_resets_hunger_exposure() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = spawn_agent(&mut world, 1, "Aster");
        let thresholds = DriveThresholds::default();
        seed_agent(
            &mut world,
            agent,
            HomeostaticNeeds::new(thresholds.hunger.critical(), pm(0), pm(0), pm(0), pm(0)),
            DeprivationExposure {
                hunger_critical_ticks: 99,
                ..DeprivationExposure::default()
            },
            metabolism(0, 0, 0, 0, 0),
            thresholds,
        );
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([10; 32]));

        needs_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
        ))
        .unwrap();

        let wounds = world.get_component_wound_list(agent).unwrap();
        assert_eq!(wounds.wounds.len(), 1);
        assert_eq!(
            wounds.wounds[0].cause,
            WoundCause::Deprivation(DeprivationKind::Starvation)
        );
        assert_eq!(wounds.wounds[0].severity, thresholds.hunger.critical());
        assert_eq!(
            wounds.wounds[0].bleed_rate_per_tick,
            Permille::new(0).unwrap()
        );
        assert_eq!(
            world
                .get_component_deprivation_exposure(agent)
                .unwrap()
                .hunger_critical_ticks,
            0
        );
        let record = event_log.get(worldwake_core::EventId(0)).unwrap();
        assert!(record.tags.contains(&EventTag::System));
        assert!(record.tags.contains(&EventTag::WorldMutation));
    }

    #[test]
    fn needs_system_adds_dehydration_wound_and_resets_thirst_exposure() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = spawn_agent(&mut world, 1, "Aster");
        let thresholds = DriveThresholds::default();
        seed_agent(
            &mut world,
            agent,
            HomeostaticNeeds::new(pm(0), thresholds.thirst.critical(), pm(0), pm(0), pm(0)),
            DeprivationExposure {
                thirst_critical_ticks: 99,
                ..DeprivationExposure::default()
            },
            metabolism(0, 0, 0, 0, 0),
            thresholds,
        );
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([11; 32]));

        needs_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
        ))
        .unwrap();

        let wounds = world.get_component_wound_list(agent).unwrap();
        assert_eq!(wounds.wounds.len(), 1);
        assert_eq!(
            wounds.wounds[0].cause,
            WoundCause::Deprivation(DeprivationKind::Dehydration)
        );
        assert_eq!(wounds.wounds[0].severity, thresholds.thirst.critical());
        assert_eq!(
            wounds.wounds[0].bleed_rate_per_tick,
            Permille::new(0).unwrap()
        );
        assert_eq!(
            world
                .get_component_deprivation_exposure(agent)
                .unwrap()
                .thirst_critical_ticks,
            0
        );
    }

    #[test]
    fn needs_system_requires_another_full_tolerance_period_before_second_wound() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = spawn_agent(&mut world, 1, "Aster");
        let thresholds = DriveThresholds::default();
        seed_agent(
            &mut world,
            agent,
            HomeostaticNeeds::new(thresholds.hunger.critical(), pm(0), pm(0), pm(0), pm(0)),
            DeprivationExposure {
                hunger_critical_ticks: 99,
                ..DeprivationExposure::default()
            },
            metabolism(0, 0, 0, 0, 0),
            thresholds,
        );
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([12; 32]));

        needs_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
        ))
        .unwrap();
        needs_system(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
        ))
        .unwrap();

        let wounds = world.get_component_wound_list(agent).unwrap();
        assert_eq!(wounds.wounds.len(), 1);
        assert_eq!(
            world
                .get_component_deprivation_exposure(agent)
                .unwrap()
                .hunger_critical_ticks,
            1
        );
    }

    #[test]
    fn needs_system_triggers_bladder_accident_and_creates_waste_at_agent_place() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = first_place(&world);
        let agent = spawn_agent(&mut world, 1, "Aster");
        place_agent(&mut world, agent, place);
        let thresholds = DriveThresholds::default();
        seed_agent(
            &mut world,
            agent,
            HomeostaticNeeds::new(pm(0), pm(0), pm(0), thresholds.bladder.critical(), pm(30)),
            DeprivationExposure {
                bladder_critical_ticks: 99,
                ..DeprivationExposure::default()
            },
            metabolism(0, 0, 0, 0, 0),
            thresholds,
        );
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([13; 32]));

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
            Some(&HomeostaticNeeds::new(
                pm(0),
                pm(0),
                pm(0),
                pm(0),
                pm(30).saturating_add(thresholds.bladder.critical()),
            ))
        );
        assert_eq!(
            world
                .get_component_deprivation_exposure(agent)
                .unwrap()
                .bladder_critical_ticks,
            0
        );

        let waste_lots: Vec<_> = world
            .ground_entities_at(place)
            .into_iter()
            .filter(|entity| {
                world
                    .get_component_item_lot(*entity)
                    .is_some_and(|lot| lot.commodity == CommodityKind::Waste)
            })
            .collect();
        assert_eq!(waste_lots.len(), 1);
    }

    #[test]
    fn dispatch_table_registers_needs_system_and_trade_slot_ages_demand_memory() {
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

        systems.get(SystemId::Needs)(system_context(
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

        let trader = seed_trade_memory(&mut world);
        systems.get(SystemId::Trade)(SystemExecutionContext {
            world: &mut world,
            event_log: &mut event_log,
            rng: &mut rng,
            active_actions: &active_actions,
            action_defs: &action_defs,
            tick: Tick(8),
            system_id: SystemId::Trade,
        })
        .unwrap();

        assert_eq!(event_log.len(), 2);
        assert!(world
            .get_component_demand_memory(trader)
            .unwrap()
            .observations
            .is_empty());
    }
}
