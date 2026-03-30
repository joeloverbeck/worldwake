use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    AgentBeliefStore, CauseRef, EntityId, EventTag, PatrolProfile, PatrolRoute, PerceptionProfile,
    SocialObservationDetail, Tick, ViolationKind, ViolationMemory, VisibilitySpec, WitnessData,
    WorldTxn,
};
use worldwake_sim::{
    ActionInstance, ActionInstanceId, SystemError, SystemExecutionContext,
};

pub fn patrol_route_adaptation_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let SystemExecutionContext {
        world,
        event_log,
        rng: _rng,
        active_actions,
        action_defs: _action_defs,
        politics_trace: _,
        perception_trace: _,
        tick,
        system_id: _system_id,
    } = ctx;

    let updates = world
        .query_patrol_route()
        .filter_map(|(agent, route)| {
            if has_active_action(active_actions, agent) {
                return None;
            }
            let profile = world.get_component_patrol_profile(agent)?;
            adapt_patrol_route(
                route,
                profile,
                world.get_component_perception_profile(agent),
                world.get_component_agent_belief_store(agent),
                world.get_component_violation_memory(agent),
                tick,
            )
            .map(|updated_route| (agent, updated_route))
        })
        .collect::<Vec<_>>();

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

    for (agent, route) in updates {
        txn.add_target(agent);
        txn.set_component_patrol_route(agent, route)
            .map_err(|error| SystemError::new(error.to_string()))?;
    }

    let _ = txn.commit(event_log);
    Ok(())
}

fn has_active_action(
    active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,
    agent: EntityId,
) -> bool {
    active_actions.values().any(|instance| instance.actor == agent)
}

fn adapt_patrol_route(
    route: &PatrolRoute,
    profile: &PatrolProfile,
    perception_profile: Option<&PerceptionProfile>,
    beliefs: Option<&AgentBeliefStore>,
    violation_memory: Option<&ViolationMemory>,
    current_tick: Tick,
) -> Option<PatrolRoute> {
    if route.assigned_places.is_empty() || route.current_index >= route.assigned_places.len() {
        return None;
    }

    let active_places = collect_active_patrol_places(
        profile,
        perception_profile,
        beliefs,
        violation_memory,
        current_tick,
    );
    if active_places.is_empty() {
        return None;
    }

    let mut active_existing = Vec::new();
    let mut inactive_existing = Vec::new();
    for &place in &route.assigned_places {
        if active_places.contains_key(&place) {
            active_existing.push(place);
        } else {
            inactive_existing.push(place);
        }
    }

    let existing_places = route.assigned_places.iter().copied().collect::<BTreeSet<_>>();
    let mut appended_new = active_places
        .iter()
        .filter_map(|(place, observed_tick)| {
            (!existing_places.contains(place)).then_some((*place, *observed_tick))
        })
        .collect::<Vec<_>>();
    appended_new.sort_by(|(left_place, left_tick), (right_place, right_tick)| {
        right_tick
            .cmp(left_tick)
            .then_with(|| left_place.cmp(right_place))
    });

    let mut assigned_places =
        Vec::with_capacity(active_existing.len() + inactive_existing.len() + appended_new.len());
    assigned_places.extend(active_existing);
    assigned_places.extend(appended_new.into_iter().map(|(place, _)| place));
    assigned_places.extend(inactive_existing);

    let updated = PatrolRoute {
        assigned_places,
        current_index: 0,
    };

    (updated != *route).then_some(updated)
}

fn collect_active_patrol_places(
    profile: &PatrolProfile,
    perception_profile: Option<&PerceptionProfile>,
    beliefs: Option<&AgentBeliefStore>,
    violation_memory: Option<&ViolationMemory>,
    current_tick: Tick,
) -> BTreeMap<EntityId, Tick> {
    let mut places = BTreeMap::new();

    if let Some(memory) = violation_memory {
        for record in memory.unresolved_records(current_tick) {
            let ViolationKind::SuspectedTheft { theft, .. } = &record.kind else {
                continue;
            };
            let freshness_window = record
                .expires_tick
                .0
                .saturating_sub(record.observed_tick.0);
            if within_route_reactive_window(
                current_tick,
                record.observed_tick,
                freshness_window,
                profile.route_adaptation_sensitivity,
            ) {
                record_recent_place(&mut places, theft.expected_place, record.observed_tick);
            }
        }
    }

    if let (Some(perception_profile), Some(beliefs)) = (perception_profile, beliefs) {
        for observation in &beliefs.social_observations {
            let SocialObservationDetail::SuspectedTheft { theft, .. } = observation.detail else {
                continue;
            };
            if within_route_reactive_window(
                current_tick,
                observation.observed_tick,
                perception_profile.memory_retention_ticks,
                profile.route_adaptation_sensitivity,
            ) {
                record_recent_place(&mut places, theft.expected_place, observation.observed_tick);
            }
        }
    }

    places
}

fn record_recent_place(
    places: &mut BTreeMap<EntityId, Tick>,
    place: EntityId,
    observed_tick: Tick,
) {
    places
        .entry(place)
        .and_modify(|existing| {
            if *existing < observed_tick {
                *existing = observed_tick;
            }
        })
        .or_insert(observed_tick);
}

fn within_route_reactive_window(
    current_tick: Tick,
    observed_tick: Tick,
    base_window_ticks: u64,
    sensitivity: worldwake_core::Permille,
) -> bool {
    let reactive_window = scaled_window(base_window_ticks, sensitivity);
    reactive_window > 0 && current_tick.0.saturating_sub(observed_tick.0) < reactive_window
}

fn scaled_window(base_window_ticks: u64, sensitivity: worldwake_core::Permille) -> u64 {
    let sensitivity = u64::from(sensitivity.value());
    if base_window_ticks == 0 || sensitivity == 0 {
        return 0;
    }

    base_window_ticks
        .saturating_mul(sensitivity)
        .saturating_add(999)
        / 1000
}

#[cfg(test)]
mod tests {
    use super::patrol_route_adaptation_system;
    use crate::dispatch_table;
    use std::collections::BTreeMap;
    use worldwake_core::{
        build_prototype_world, prototype_place_entity, ActionDefId, AgentBeliefStore,
        BeliefConfidencePolicy, CauseRef, CommodityKind, ControlSource, EventLog, PatrolProfile,
        PatrolRoute, PerceptionProfile, PerceptionSource, Permille, PrototypePlace, Quantity,
        RecordedViolation, Seed, SocialObservation, SocialObservationDetail, TheftFacts, Tick,
        ViolationId, ViolationKind, ViolationMemory, VisibilitySpec, WitnessData, World,
        WorldTxn,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionDuration, ActionInstance, ActionInstanceId, ActionPayload,
        ActionStatus, DeterministicRng, SystemExecutionContext, SystemId,
    };

    fn entity(slot: u32) -> worldwake_core::EntityId {
        worldwake_core::EntityId {
            slot,
            generation: 0,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

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

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    fn test_rng() -> DeterministicRng {
        DeterministicRng::new(Seed([0x41; 32]))
    }

    fn patrol_profile(sensitivity: u16) -> PatrolProfile {
        PatrolProfile {
            base_dwell_ticks: 4,
            dwell_vigilance_scale_ticks: 4,
            vigilance: pm(500),
            route_adaptation_sensitivity: pm(sensitivity),
            patrol_motive_weight: pm(550),
        }
    }

    fn perception_profile(retention: u64) -> PerceptionProfile {
        PerceptionProfile {
            memory_capacity: 12,
            memory_retention_ticks: retention,
            observation_fidelity: pm(1000),
            confidence_policy: BeliefConfidencePolicy::default(),
            institutional_memory_capacity: 20,
            consultation_speed_factor: pm(500),
            contradiction_tolerance: pm(300),
        }
    }

    fn theft(place: worldwake_core::EntityId) -> TheftFacts {
        TheftFacts {
            missing_entity: entity(place.slot + 1000),
            expected_place: place,
            commodity: CommodityKind::Bread,
            quantity: Quantity(1),
        }
    }

    fn seed_guard(
        world: &mut World,
        actor_place: worldwake_core::EntityId,
        route: PatrolRoute,
        profile: PatrolProfile,
        perception: Option<PerceptionProfile>,
    ) -> worldwake_core::EntityId {
        let mut txn = new_txn(world, 1);
        let guard = txn.create_agent("Guard", ControlSource::Ai).unwrap();
        txn.set_ground_location(guard, actor_place).unwrap();
        txn.set_component_patrol_route(guard, route).unwrap();
        txn.set_component_patrol_profile(guard, profile).unwrap();
        txn.set_component_agent_belief_store(guard, AgentBeliefStore::default())
            .unwrap();
        txn.set_component_violation_memory(guard, ViolationMemory::default())
            .unwrap();
        if let Some(perception) = perception {
            txn.set_component_perception_profile(guard, perception).unwrap();
        }
        commit_txn(txn);
        guard
    }

    fn add_social_observation(
        world: &mut World,
        guard: worldwake_core::EntityId,
        place: worldwake_core::EntityId,
        observed_tick: u64,
    ) {
        let mut store = world.get_component_agent_belief_store(guard).unwrap().clone();
        store.record_social_observation(SocialObservation {
            detail: SocialObservationDetail::SuspectedTheft {
                theft: theft(place),
                suspect: None,
            },
            place,
            observed_tick: Tick(observed_tick),
            source: PerceptionSource::DirectObservation,
        });
        let mut txn = new_txn(world, observed_tick + 1);
        txn.set_component_agent_belief_store(guard, store).unwrap();
        commit_txn(txn);
    }

    fn add_violation_record(
        world: &mut World,
        guard: worldwake_core::EntityId,
        place: worldwake_core::EntityId,
        observed_tick: u64,
        expires_tick: u64,
    ) {
        let mut memory = world.get_component_violation_memory(guard).unwrap().clone();
        let next_id = memory.next_violation_id();
        memory.violations.push(RecordedViolation {
            id: next_id,
            kind: ViolationKind::SuspectedTheft {
                theft: theft(place),
                suspect: None,
            },
            observed_tick: Tick(observed_tick),
            resolved_tick: None,
            expires_tick: Tick(expires_tick),
        });
        memory.next_violation_id = ViolationId(next_id.0 + 1);
        let mut txn = new_txn(world, observed_tick + 1);
        txn.set_component_violation_memory(guard, memory).unwrap();
        commit_txn(txn);
    }

    fn system_context<'a>(
        world: &'a mut World,
        log: &'a mut EventLog,
        rng: &'a mut DeterministicRng,
        active_actions: &'a BTreeMap<ActionInstanceId, ActionInstance>,
        defs: &'a ActionDefRegistry,
    ) -> SystemExecutionContext<'a> {
        SystemExecutionContext {
            world,
            event_log: log,
            rng,
            active_actions,
            action_defs: defs,
            politics_trace: None,
            perception_trace: None,
            tick: Tick(10),
            system_id: SystemId::Patrol,
        }
    }

    #[test]
    fn social_report_adds_new_patrol_waypoint() {
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let gate = prototype_place_entity(PrototypePlace::SouthGate);
        let market = prototype_place_entity(PrototypePlace::GeneralStore);
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = seed_guard(
            &mut world,
            square,
            PatrolRoute {
                assigned_places: vec![square, gate],
                current_index: 0,
            },
            patrol_profile(1000),
            Some(perception_profile(12)),
        );
        add_social_observation(&mut world, guard, market, 9);
        let defs = ActionDefRegistry::new();
        let mut log = EventLog::new();
        let mut rng = test_rng();

        patrol_route_adaptation_system(system_context(
            &mut world,
            &mut log,
            &mut rng,
            &BTreeMap::new(),
            &defs,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_patrol_route(guard).unwrap(),
            &PatrolRoute {
                assigned_places: vec![market, square, gate],
                current_index: 0,
            }
        );
    }

    #[test]
    fn violation_memory_record_adds_new_patrol_waypoint() {
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let gate = prototype_place_entity(PrototypePlace::SouthGate);
        let hall = prototype_place_entity(PrototypePlace::RulersHall);
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = seed_guard(
            &mut world,
            square,
            PatrolRoute {
                assigned_places: vec![square, gate],
                current_index: 1,
            },
            patrol_profile(1000),
            Some(perception_profile(12)),
        );
        add_violation_record(&mut world, guard, hall, 8, 18);
        let defs = ActionDefRegistry::new();
        let mut log = EventLog::new();
        let mut rng = test_rng();

        patrol_route_adaptation_system(system_context(
            &mut world,
            &mut log,
            &mut rng,
            &BTreeMap::new(),
            &defs,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_patrol_route(guard).unwrap(),
            &PatrolRoute {
                assigned_places: vec![hall, square, gate],
                current_index: 0,
            }
        );
    }

    #[test]
    fn low_sensitivity_ignores_stale_report() {
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let gate = prototype_place_entity(PrototypePlace::SouthGate);
        let market = prototype_place_entity(PrototypePlace::GeneralStore);
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = seed_guard(
            &mut world,
            square,
            PatrolRoute {
                assigned_places: vec![square, gate],
                current_index: 0,
            },
            patrol_profile(200),
            Some(perception_profile(10)),
        );
        add_social_observation(&mut world, guard, market, 5);
        let defs = ActionDefRegistry::new();
        let mut log = EventLog::new();
        let mut rng = test_rng();

        patrol_route_adaptation_system(system_context(
            &mut world,
            &mut log,
            &mut rng,
            &BTreeMap::new(),
            &defs,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_patrol_route(guard).unwrap(),
            &PatrolRoute {
                assigned_places: vec![square, gate],
                current_index: 0,
            }
        );
    }

    #[test]
    fn active_places_are_promoted_without_removing_existing_waypoints() {
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let gate = prototype_place_entity(PrototypePlace::SouthGate);
        let hall = prototype_place_entity(PrototypePlace::RulersHall);
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = seed_guard(
            &mut world,
            square,
            PatrolRoute {
                assigned_places: vec![square, gate, hall],
                current_index: 0,
            },
            patrol_profile(1000),
            Some(perception_profile(12)),
        );
        add_social_observation(&mut world, guard, hall, 9);
        let defs = ActionDefRegistry::new();
        let mut log = EventLog::new();
        let mut rng = test_rng();

        patrol_route_adaptation_system(system_context(
            &mut world,
            &mut log,
            &mut rng,
            &BTreeMap::new(),
            &defs,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_patrol_route(guard).unwrap(),
            &PatrolRoute {
                assigned_places: vec![hall, square, gate],
                current_index: 0,
            }
        );
    }

    #[test]
    fn guard_does_not_adapt_from_other_agents_reports() {
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let gate = prototype_place_entity(PrototypePlace::SouthGate);
        let market = prototype_place_entity(PrototypePlace::GeneralStore);
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = seed_guard(
            &mut world,
            square,
            PatrolRoute {
                assigned_places: vec![square, gate],
                current_index: 0,
            },
            patrol_profile(1000),
            Some(perception_profile(12)),
        );
        let other = seed_guard(
            &mut world,
            square,
            PatrolRoute {
                assigned_places: vec![square],
                current_index: 0,
            },
            patrol_profile(1000),
            Some(perception_profile(12)),
        );
        add_social_observation(&mut world, other, market, 9);
        let defs = ActionDefRegistry::new();
        let mut log = EventLog::new();
        let mut rng = test_rng();

        patrol_route_adaptation_system(system_context(
            &mut world,
            &mut log,
            &mut rng,
            &BTreeMap::new(),
            &defs,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_patrol_route(guard).unwrap(),
            &PatrolRoute {
                assigned_places: vec![square, gate],
                current_index: 0,
            }
        );
    }

    #[test]
    fn active_action_skips_route_adaptation() {
        let square = prototype_place_entity(PrototypePlace::VillageSquare);
        let gate = prototype_place_entity(PrototypePlace::SouthGate);
        let market = prototype_place_entity(PrototypePlace::GeneralStore);
        let mut world = World::new(build_prototype_world()).unwrap();
        let guard = seed_guard(
            &mut world,
            square,
            PatrolRoute {
                assigned_places: vec![square, gate],
                current_index: 0,
            },
            patrol_profile(1000),
            Some(perception_profile(12)),
        );
        add_social_observation(&mut world, guard, market, 9);
        let defs = ActionDefRegistry::new();
        let mut active_actions = BTreeMap::new();
        active_actions.insert(
            ActionInstanceId(1),
            ActionInstance {
                instance_id: ActionInstanceId(1),
                def_id: ActionDefId(0),
                payload: ActionPayload::None,
                actor: guard,
                targets: vec![square],
                start_tick: Tick(9),
                remaining_duration: ActionDuration::new(2),
                status: ActionStatus::Active,
                reservation_ids: Vec::new(),
                local_state: None,
                body_cost_override: None,
            },
        );
        let mut log = EventLog::new();
        let mut rng = test_rng();

        patrol_route_adaptation_system(system_context(
            &mut world,
            &mut log,
            &mut rng,
            &active_actions,
            &defs,
        ))
        .unwrap();

        assert_eq!(
            world.get_component_patrol_route(guard).unwrap(),
            &PatrolRoute {
                assigned_places: vec![square, gate],
                current_index: 0,
            }
        );
    }

    #[test]
    fn dispatch_table_uses_patrol_system_for_patrol_slot() {
        let systems = dispatch_table();
        let defs = ActionDefRegistry::new();
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut log = EventLog::new();
        let mut rng = test_rng();

        systems
            .get(SystemId::Patrol)(system_context(
                &mut world,
                &mut log,
                &mut rng,
                &BTreeMap::new(),
                &defs,
            ))
            .unwrap();
    }
}
