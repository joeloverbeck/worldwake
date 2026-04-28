use std::{collections::BTreeSet, num::NonZeroU32};

use worldwake_core::{
    ExpectationBasis, ExpectationState, FrameAssumption, HomeostaticNeedId, Permille, Tick,
    WakeCondition, WorldTxn,
};

pub fn synthesize_wake_conditions(
    agent: worldwake_core::EntityId,
    intended_max_ticks: NonZeroU32,
    target_recovery: Permille,
    current_tick: Tick,
    txn: &WorldTxn<'_>,
) -> Vec<WakeCondition> {
    let mut conditions = vec![WakeCondition::IntendedDurationReached];
    let horizon = Tick(
        current_tick
            .0
            .saturating_add(u64::from(intended_max_ticks.get())),
    );

    if let Some(frame) = txn.get_component_intention_frame(agent) {
        for need in HomeostaticNeedId::ALL {
            if need == HomeostaticNeedId::Fatigue {
                continue;
            }
            if frame.assumptions.iter().any(|assumption| {
                matches!(
                    assumption,
                    FrameAssumption::NeedSafeUntilTick {
                        need: assumed_need,
                        until_tick
                    } if *assumed_need == need && *until_tick < horizon
                )
            }) {
                conditions.push(WakeCondition::ProjectedNeedBreach { need });
            }
        }
    }

    if let Some(store) = txn.get_component_expectation_store(agent) {
        let mut due_ticks = BTreeSet::new();
        for record in store.records.values() {
            if record.owner != agent {
                continue;
            }
            if !matches!(
                record.state,
                ExpectationState::Active | ExpectationState::Overdue
            ) {
                continue;
            }
            if matches!(record.basis, ExpectationBasis::PlanStepCompletion { .. }) {
                continue;
            }
            if record.deadline_tick <= horizon {
                due_ticks.insert(record.deadline_tick);
            }
        }
        conditions.extend(
            due_ticks
                .into_iter()
                .map(|tick| WakeCondition::ScheduledCommitmentDue { tick }),
        );
    }

    conditions.push(WakeCondition::LocalDisturbance);
    if target_recovery < Permille::new_unchecked(1000) {
        conditions.push(WakeCondition::TargetRecoveryReached);
    }

    conditions
}

#[cfg(test)]
mod tests {
    use super::synthesize_wake_conditions;
    use std::num::NonZeroU32;
    use worldwake_core::{
        CauseRef, ControlSource, ExpectationBasis, ExpectationId, ExpectationRecord,
        ExpectationState, ExpectationStore, FrameAssumption, FrameState, GoalKey, GoalKind,
        HomeostaticNeedId, IntentionDomain, IntentionFrame, Permille, Quantity, Tick,
        VisibilitySpec, WakeCondition, WitnessData, World, WorldTxn, build_prototype_world,
    };

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

    fn sleep_goal() -> GoalKey {
        GoalKey::from(GoalKind::Sleep)
    }

    fn frame_with(assumptions: Vec<FrameAssumption>) -> IntentionFrame {
        IntentionFrame {
            goal: sleep_goal(),
            domain: IntentionDomain::Generic,
            assumptions,
            state: FrameState::Active,
            established_at: Tick(1),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 10,
        }
    }

    fn expectation_store(records: impl IntoIterator<Item = ExpectationRecord>) -> ExpectationStore {
        let mut store = ExpectationStore::default();
        for record in records {
            store.records.insert(record.id, record);
        }
        store
    }

    fn commitment_due(
        id: u64,
        owner: worldwake_core::EntityId,
        deadline_tick: Tick,
        state: ExpectationState,
    ) -> ExpectationRecord {
        ExpectationRecord {
            id: ExpectationId(id),
            owner,
            subject: owner,
            expected_place: owner,
            deadline_tick,
            grace_ticks: 0,
            basis: ExpectationBasis::DeliveryCommitment {
                commodity: worldwake_core::CommodityKind::Bread,
                quantity: Quantity(1),
            },
            state,
            created_tick: Tick(1),
        }
    }

    #[test]
    fn test_sleep_synthesis_defaults_to_duration_disturbance_and_target_recovery() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut txn = new_txn(&mut world, 1);
        let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();

        let conditions =
            synthesize_wake_conditions(agent, NonZeroU32::new(8).unwrap(), pm(0), Tick(10), &txn);

        assert_eq!(
            conditions,
            vec![
                WakeCondition::IntendedDurationReached,
                WakeCondition::LocalDisturbance,
                WakeCondition::TargetRecoveryReached,
            ]
        );
    }

    #[test]
    fn test_sleep_synthesis_includes_projected_need_breach_inside_window() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut txn = new_txn(&mut world, 1);
        let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_component_intention_frame(
            agent,
            frame_with(vec![
                FrameAssumption::NeedSafeUntilTick {
                    need: HomeostaticNeedId::Hunger,
                    until_tick: Tick(15),
                },
                FrameAssumption::NeedSafeUntilTick {
                    need: HomeostaticNeedId::Thirst,
                    until_tick: Tick(25),
                },
            ]),
        )
        .unwrap();

        let conditions = synthesize_wake_conditions(
            agent,
            NonZeroU32::new(10).unwrap(),
            pm(1000),
            Tick(10),
            &txn,
        );

        assert_eq!(
            conditions,
            vec![
                WakeCondition::IntendedDurationReached,
                WakeCondition::ProjectedNeedBreach {
                    need: HomeostaticNeedId::Hunger,
                },
                WakeCondition::LocalDisturbance,
            ]
        );
    }

    #[test]
    fn test_sleep_synthesis_includes_scheduled_commitments_due_by_horizon() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut txn = new_txn(&mut world, 1);
        let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        let other = txn.create_agent("Bryn", ControlSource::Ai).unwrap();
        txn.set_component_expectation_store(
            agent,
            expectation_store([
                commitment_due(0, agent, Tick(12), ExpectationState::Active),
                commitment_due(1, agent, Tick(25), ExpectationState::Active),
                commitment_due(2, agent, Tick(9), ExpectationState::Overdue),
                commitment_due(3, other, Tick(12), ExpectationState::Active),
                ExpectationRecord {
                    id: ExpectationId(4),
                    owner: agent,
                    subject: agent,
                    expected_place: agent,
                    deadline_tick: Tick(13),
                    grace_ticks: 0,
                    basis: ExpectationBasis::PlanStepCompletion {
                        step_index: 0,
                        kind_tag: worldwake_core::ExpectationKindTag::Immediate,
                    },
                    state: ExpectationState::Active,
                    created_tick: Tick(1),
                },
            ]),
        )
        .unwrap();

        let conditions = synthesize_wake_conditions(
            agent,
            NonZeroU32::new(10).unwrap(),
            pm(1000),
            Tick(10),
            &txn,
        );

        assert_eq!(
            conditions,
            vec![
                WakeCondition::IntendedDurationReached,
                WakeCondition::ScheduledCommitmentDue { tick: Tick(9) },
                WakeCondition::ScheduledCommitmentDue { tick: Tick(12) },
                WakeCondition::LocalDisturbance,
            ]
        );
    }
}
