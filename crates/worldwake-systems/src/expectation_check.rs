use worldwake_core::{
    CauseRef, EventTag, ExpectationState, ExpectationStore, Tick, VisibilitySpec, WitnessData,
    World, WorldTxn,
};
use worldwake_sim::{SystemError, SystemExecutionContext};

pub fn check_overdue_expectations(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
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

    for (agent, store) in updates {
        txn.add_target(agent);
        txn.set_component_expectation_store(agent, store)
            .map_err(|error| SystemError::new(error.to_string()))?;
    }

    let _ = txn.commit(event_log);
    Ok(())
}

fn collect_updates(world: &World, tick: Tick) -> Vec<(worldwake_core::EntityId, ExpectationStore)> {
    world
        .query_expectation_store()
        .filter_map(|(agent, store)| overdue_store_for_tick(store, tick).map(|next| (agent, next)))
        .collect()
}

fn overdue_store_for_tick(store: &ExpectationStore, tick: Tick) -> Option<ExpectationStore> {
    let mut next = store.clone();
    let mut changed = false;

    for record in next.records.values_mut() {
        if record.state != ExpectationState::Active {
            continue;
        }
        if tick.0 > record.deadline_tick.0.saturating_add(record.grace_ticks) {
            record.state = ExpectationState::Overdue;
            changed = true;
        }
    }

    changed.then_some(next)
}

#[cfg(test)]
mod tests {
    use super::{check_overdue_expectations, overdue_store_for_tick};
    use crate::dispatch_table;
    use std::collections::BTreeMap;
    use worldwake_core::{
        CauseRef, ControlSource, EntityId, EventLog, EventTag, ExpectationBasis, ExpectationId,
        ExpectationRecord, ExpectationState, ExpectationStore, Seed, Tick, VisibilitySpec,
        WitnessData, World, WorldTxn, build_prototype_world,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionInstance, ActionInstanceId, DeterministicRng,
        SystemExecutionContext, SystemId,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
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

    fn expectation_record(id: u64, owner: EntityId, state: ExpectationState) -> ExpectationRecord {
        ExpectationRecord {
            id: ExpectationId(id),
            owner,
            subject: entity(100 + id as u32),
            expected_place: entity(200 + id as u32),
            deadline_tick: Tick(10),
            grace_ticks: 2,
            basis: ExpectationBasis::RoutineReturn,
            state,
            created_tick: Tick(1),
        }
    }

    fn expectation_store(
        records: impl IntoIterator<Item = (ExpectationId, ExpectationRecord)>,
    ) -> ExpectationStore {
        let mut store = ExpectationStore::default();
        store.records = records.into_iter().collect();
        store
    }

    fn seed_agent_with_store(world: &mut World, store: ExpectationStore) -> EntityId {
        let mut txn = new_txn(world, 1);
        let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_component_expectation_store(agent, store).unwrap();
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
            politics_trace: None,
            perception_trace: None,
            tick: Tick(tick),
            system_id: SystemId::ExpectationCheck,
        }
    }

    #[test]
    fn overdue_store_for_tick_marks_only_active_records_past_grace() {
        let owner = entity(1);
        let store = expectation_store([
            (
                ExpectationId(1),
                expectation_record(1, owner, ExpectationState::Active),
            ),
            (
                ExpectationId(2),
                ExpectationRecord {
                    grace_ticks: 3,
                    ..expectation_record(2, owner, ExpectationState::Active)
                },
            ),
            (
                ExpectationId(3),
                expectation_record(3, owner, ExpectationState::Overdue),
            ),
            (
                ExpectationId(4),
                expectation_record(
                    4,
                    owner,
                    ExpectationState::Resolved {
                        outcome: worldwake_core::ExpectationOutcome::ReturnedLate,
                    },
                ),
            ),
            (
                ExpectationId(5),
                expectation_record(5, owner, ExpectationState::Expired),
            ),
        ]);

        let updated = overdue_store_for_tick(&store, Tick(13)).unwrap();

        assert_eq!(
            updated.records.get(&ExpectationId(1)).unwrap().state,
            ExpectationState::Overdue
        );
        assert_eq!(
            updated.records.get(&ExpectationId(2)).unwrap().state,
            ExpectationState::Active
        );
        assert_eq!(
            updated.records.get(&ExpectationId(3)).unwrap().state,
            ExpectationState::Overdue
        );
        assert_eq!(
            updated.records.get(&ExpectationId(4)).unwrap().state,
            ExpectationState::Resolved {
                outcome: worldwake_core::ExpectationOutcome::ReturnedLate,
            }
        );
        assert_eq!(
            updated.records.get(&ExpectationId(5)).unwrap().state,
            ExpectationState::Expired
        );
    }

    #[test]
    fn overdue_store_for_tick_returns_none_when_no_records_change() {
        let owner = entity(1);
        let store = expectation_store([(
            ExpectationId(1),
            ExpectationRecord {
                grace_ticks: 3,
                ..expectation_record(1, owner, ExpectationState::Active)
            },
        )]);

        assert_eq!(overdue_store_for_tick(&store, Tick(13)), None);
    }

    #[test]
    fn system_transitions_overdue_expectations_and_emits_world_mutation_event() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = seed_agent_with_store(
            &mut world,
            expectation_store([(
                ExpectationId(1),
                expectation_record(1, entity(1), ExpectationState::Active),
            )]),
        );
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([7; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        check_overdue_expectations(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            13,
        ))
        .unwrap();

        assert_eq!(
            world
                .get_component_expectation_store(agent)
                .unwrap()
                .records
                .get(&ExpectationId(1))
                .unwrap()
                .state,
            ExpectationState::Overdue
        );
        let world_mutations = event_log.events_by_tag(EventTag::WorldMutation);
        assert_eq!(world_mutations.len(), 1);
    }

    #[test]
    fn system_leaves_within_grace_expectations_active_without_emitting_event() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = seed_agent_with_store(
            &mut world,
            expectation_store([(
                ExpectationId(1),
                ExpectationRecord {
                    grace_ticks: 3,
                    ..expectation_record(1, entity(1), ExpectationState::Active)
                },
            )]),
        );
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([8; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        check_overdue_expectations(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            13,
        ))
        .unwrap();

        assert_eq!(
            world
                .get_component_expectation_store(agent)
                .unwrap()
                .records
                .get(&ExpectationId(1))
                .unwrap()
                .state,
            ExpectationState::Active
        );
        assert!(event_log.is_empty());
    }

    #[test]
    fn dispatch_table_routes_expectation_check_system() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let agent = seed_agent_with_store(
            &mut world,
            expectation_store([(
                ExpectationId(1),
                expectation_record(1, entity(1), ExpectationState::Active),
            )]),
        );
        let mut event_log = EventLog::new();
        let mut rng = DeterministicRng::new(Seed([9; 32]));
        let active_actions = BTreeMap::new();
        let action_defs = ActionDefRegistry::new();

        dispatch_table().get(SystemId::ExpectationCheck)(system_context(
            &mut world,
            &mut event_log,
            &mut rng,
            &active_actions,
            &action_defs,
            13,
        ))
        .unwrap();

        assert_eq!(
            world
                .get_component_expectation_store(agent)
                .unwrap()
                .records
                .get(&ExpectationId(1))
                .unwrap()
                .state,
            ExpectationState::Overdue
        );
    }
}
