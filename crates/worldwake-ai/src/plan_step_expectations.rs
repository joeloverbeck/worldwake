use crate::{ExpectationKind, PlanExpectation, PlannedPlan};
use worldwake_core::{
    CauseRef, EntityId, EventLog, ExpectationBasis, ExpectationKindTag, ExpectationOutcome,
    ExpectationRecord, ExpectationState, ExpectationStore, Tick, VisibilitySpec, WitnessData,
    World, WorldTxn,
};
use worldwake_sim::TickInputError;

fn expectation_kind_tag(expectation: &PlanExpectation) -> ExpectationKindTag {
    match &expectation.kind {
        ExpectationKind::Immediate { .. } => ExpectationKindTag::Immediate,
        ExpectationKind::State { .. } => ExpectationKindTag::State,
        ExpectationKind::Informed { .. } => ExpectationKindTag::Informed,
        ExpectationKind::Regression { .. } => ExpectationKindTag::Regression,
    }
}

pub(crate) fn write_plan_step_expectations(
    store: &mut ExpectationStore,
    agent: EntityId,
    current_place: EntityId,
    plan: &PlannedPlan,
    adoption_tick: Tick,
    expectation_tolerance_ticks: u32,
) -> bool {
    let mut changed = false;
    let grace_ticks = u64::from(expectation_tolerance_ticks);
    for (index, step) in plan.steps.iter().enumerate() {
        let step_index = index
            .try_into()
            .expect("planned step index exceeds u16 for expectation basis");
        let subject = step.primary_target().unwrap_or(agent);
        let expected_place = step.target_place().unwrap_or(current_place);
        let default_deadline = step.expected_complete_tick(adoption_tick);
        for expectation in &step.expectations {
            store.allocate_record(|id| ExpectationRecord {
                id,
                owner: agent,
                subject,
                expected_place,
                deadline_tick: expectation.observe_by.unwrap_or(default_deadline),
                grace_ticks,
                basis: ExpectationBasis::PlanStepCompletion {
                    step_index,
                    kind_tag: expectation_kind_tag(expectation),
                },
                state: ExpectationState::Active,
                created_tick: adoption_tick,
            });
            changed = true;
        }
    }
    changed
}

pub(crate) fn expire_plan_step_expectations(store: &mut ExpectationStore) -> bool {
    let mut changed = false;
    for record in store.records.values_mut() {
        if !matches!(record.basis, ExpectationBasis::PlanStepCompletion { .. }) {
            continue;
        }
        if matches!(
            record.state,
            ExpectationState::Active | ExpectationState::Overdue
        ) {
            record.state = ExpectationState::Expired;
            changed = true;
        }
    }
    changed
}

pub(crate) fn fulfill_plan_step_expectations(
    store: &mut ExpectationStore,
    step_index: u16,
) -> bool {
    let mut changed = false;
    for record in store.records.values_mut() {
        let matches_step = matches!(
            record.basis,
            ExpectationBasis::PlanStepCompletion {
                step_index: record_step_index,
                ..
            } if record_step_index == step_index
        );
        if !matches_step {
            continue;
        }
        if matches!(
            record.state,
            ExpectationState::Active | ExpectationState::Overdue
        ) {
            record.state = ExpectationState::Resolved {
                outcome: ExpectationOutcome::Fulfilled,
            };
            changed = true;
        }
    }
    changed
}

pub(crate) fn persist_expectation_store_update<F>(
    world: &mut World,
    event_log: &mut EventLog,
    agent: EntityId,
    tick: Tick,
    update: F,
) -> Result<bool, TickInputError>
where
    F: FnOnce(&mut ExpectationStore) -> bool,
{
    let Some(existing) = world.get_component_expectation_store(agent).cloned() else {
        return Ok(false);
    };
    let mut next = existing.clone();
    if !update(&mut next) || next == existing {
        return Ok(false);
    }

    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        Some(agent),
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.set_component_expectation_store(agent, next)
        .map_err(|error| TickInputError::new(error.to_string()))?;
    let _ = txn.commit(event_log);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{
        expire_plan_step_expectations, fulfill_plan_step_expectations, write_plan_step_expectations,
    };
    use crate::{
        ExpectationKind, GoalKey, GoalKind, PlanExpectation, PlanTerminalKind, PlannedPlan,
        PlannedStep, PlannerOpKind, PlanningEntityRef,
    };
    use worldwake_core::{
        ActionDefId, EntityId, EventTag, ExpectationBasis, ExpectationKindTag, ExpectationOutcome,
        ExpectationState, Tick,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn plan_with_expectations() -> PlannedPlan {
        let goal = GoalKey::from(GoalKind::Sleep);
        PlannedPlan::new(
            worldwake_core::OpportunityKey {
                goal_key: goal,
                anchor: worldwake_core::OpportunityAnchor::None,
            },
            goal,
            vec![
                PlannedStep {
                    def_id: ActionDefId(1),
                    targets: vec![PlanningEntityRef::Authoritative(entity(10))],
                    target_place: Some(entity(20)),
                    payload_override: None,
                    op_kind: PlannerOpKind::Travel,
                    estimated_ticks: 3,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                    guard: None,
                    expectations: vec![
                        PlanExpectation {
                            kind: ExpectationKind::Immediate {
                                event_tag: EventTag::ExpectationMismatch,
                            },
                            observe_by: None,
                        },
                        PlanExpectation {
                            kind: ExpectationKind::State {
                                predicate: worldwake_core::StatePredicate::EntityAtPlace {
                                    entity: entity(1),
                                    place: entity(20),
                                },
                            },
                            observe_by: Some(Tick(99)),
                        },
                    ],
                },
                PlannedStep {
                    def_id: ActionDefId(2),
                    targets: Vec::new(),
                    target_place: None,
                    payload_override: None,
                    op_kind: PlannerOpKind::Sleep,
                    estimated_ticks: 2,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                    guard: None,
                    expectations: vec![PlanExpectation {
                        kind: ExpectationKind::Immediate {
                            event_tag: EventTag::GoalCommitted,
                        },
                        observe_by: None,
                    }],
                },
            ],
            PlanTerminalKind::GoalSatisfied,
        )
    }

    #[test]
    fn plan_adoption_writes_one_record_per_expectation() {
        let mut store = worldwake_core::ExpectationStore::default();
        let changed = write_plan_step_expectations(
            &mut store,
            entity(1),
            entity(30),
            &plan_with_expectations(),
            Tick(10),
            4,
        );

        assert!(changed);
        assert_eq!(store.records.len(), 3);

        let records = store.records.values().copied().collect::<Vec<_>>();
        assert_eq!(
            records[0].basis,
            ExpectationBasis::PlanStepCompletion {
                step_index: 0,
                kind_tag: ExpectationKindTag::Immediate,
            }
        );
        assert_eq!(records[0].subject, entity(10));
        assert_eq!(records[0].expected_place, entity(20));
        assert_eq!(records[0].deadline_tick, Tick(13));
        assert_eq!(records[0].state, ExpectationState::Active);

        assert_eq!(
            records[1].basis,
            ExpectationBasis::PlanStepCompletion {
                step_index: 0,
                kind_tag: ExpectationKindTag::State,
            }
        );
        assert_eq!(records[1].deadline_tick, Tick(99));

        assert_eq!(
            records[2].basis,
            ExpectationBasis::PlanStepCompletion {
                step_index: 1,
                kind_tag: ExpectationKindTag::Immediate,
            }
        );
        assert_eq!(records[2].subject, entity(1));
        assert_eq!(records[2].expected_place, entity(30));
        assert_eq!(records[2].deadline_tick, Tick(12));
    }

    #[test]
    fn plan_adoption_sets_grace_ticks_from_profile() {
        let mut store = worldwake_core::ExpectationStore::default();
        write_plan_step_expectations(
            &mut store,
            entity(1),
            entity(30),
            &plan_with_expectations(),
            Tick(10),
            4,
        );

        assert!(store.records.values().all(|record| record.grace_ticks == 4));
    }

    #[test]
    fn plan_adoption_advances_counter_past_allocated_records() {
        let mut store = worldwake_core::ExpectationStore::default();
        write_plan_step_expectations(
            &mut store,
            entity(1),
            entity(30),
            &plan_with_expectations(),
            Tick(10),
            4,
        );

        assert_eq!(
            store.next_expectation_id(),
            worldwake_core::ExpectationId(3)
        );
    }

    #[test]
    fn plan_adoption_with_empty_expectations_writes_no_records() {
        let goal = GoalKey::from(GoalKind::Sleep);
        let plan = PlannedPlan::new(
            worldwake_core::OpportunityKey {
                goal_key: goal,
                anchor: worldwake_core::OpportunityAnchor::None,
            },
            goal,
            vec![PlannedStep {
                def_id: ActionDefId(1),
                targets: Vec::new(),
                target_place: None,
                payload_override: None,
                op_kind: PlannerOpKind::Sleep,
                estimated_ticks: 1,
                is_materialization_barrier: false,
                expected_materializations: Vec::new(),
                guard: None,
                expectations: Vec::new(),
            }],
            PlanTerminalKind::GoalSatisfied,
        );
        let mut store = worldwake_core::ExpectationStore::default();

        let changed =
            write_plan_step_expectations(&mut store, entity(1), entity(30), &plan, Tick(10), 4);

        assert!(!changed);
        assert!(store.records.is_empty());
    }

    #[test]
    fn plan_replacement_expires_prior_records() {
        let mut store = worldwake_core::ExpectationStore::default();
        write_plan_step_expectations(
            &mut store,
            entity(1),
            entity(30),
            &plan_with_expectations(),
            Tick(10),
            4,
        );

        assert!(expire_plan_step_expectations(&mut store));
        assert!(
            store
                .records
                .values()
                .all(|record| record.state == ExpectationState::Expired)
        );
    }

    #[test]
    fn step_completion_resolves_record_as_fulfilled() {
        let mut store = worldwake_core::ExpectationStore::default();
        write_plan_step_expectations(
            &mut store,
            entity(1),
            entity(30),
            &plan_with_expectations(),
            Tick(10),
            4,
        );

        assert!(fulfill_plan_step_expectations(&mut store, 0));
        let records = store.records.values().copied().collect::<Vec<_>>();
        assert_eq!(
            records[0].state,
            ExpectationState::Resolved {
                outcome: ExpectationOutcome::Fulfilled,
            }
        );
        assert_eq!(
            records[1].state,
            ExpectationState::Resolved {
                outcome: ExpectationOutcome::Fulfilled,
            }
        );
        assert_eq!(records[2].state, ExpectationState::Active);
    }
}
