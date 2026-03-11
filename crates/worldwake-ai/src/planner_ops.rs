use crate::GoalKey;
use serde::{Deserialize, Serialize};
use worldwake_core::EntityId;
use worldwake_sim::{ActionDefId, ActionPayload, InputKind};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PlannedStep {
    pub def_id: ActionDefId,
    pub targets: Vec<EntityId>,
    pub payload_override: Option<ActionPayload>,
    pub estimated_ticks: u32,
    pub is_materialization_barrier: bool,
}

impl PlannedStep {
    #[must_use]
    pub fn to_request_action(&self, actor: EntityId) -> InputKind {
        InputKind::RequestAction {
            actor,
            def_id: self.def_id,
            targets: self.targets.clone(),
            payload_override: self.payload_override.clone(),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum PlanTerminalKind {
    GoalSatisfied,
    ProgressBarrier,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PlannedPlan {
    pub goal: GoalKey,
    pub steps: Vec<PlannedStep>,
    pub total_estimated_ticks: u32,
    pub terminal_kind: PlanTerminalKind,
}

impl PlannedPlan {
    #[must_use]
    pub fn new(goal: GoalKey, steps: Vec<PlannedStep>, terminal_kind: PlanTerminalKind) -> Self {
        Self {
            goal,
            total_estimated_ticks: total_estimated_ticks(&steps),
            steps,
            terminal_kind,
        }
    }
}

fn total_estimated_ticks(steps: &[PlannedStep]) -> u32 {
    steps.iter().fold(0u32, |acc, step| {
        acc.checked_add(step.estimated_ticks)
            .expect("planned step ticks overflow u32")
    })
}

#[cfg(test)]
mod tests {
    use super::{PlanTerminalKind, PlannedPlan, PlannedStep};
    use crate::{CommodityPurpose, GoalKey, GoalKind};
    use worldwake_core::{CommodityKind, EntityId, Quantity};
    use worldwake_sim::{ActionDefId, ActionPayload, InputKind, TradeActionPayload};

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn sample_step() -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(7),
            targets: vec![entity(3), entity(4)],
            payload_override: Some(ActionPayload::Trade(TradeActionPayload {
                counterparty: entity(3),
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(2),
                requested_commodity: CommodityKind::Bread,
                requested_quantity: Quantity(1),
            })),
            estimated_ticks: 5,
            is_materialization_barrier: false,
        }
    }

    #[test]
    fn planned_step_to_request_action_preserves_exact_execution_identity() {
        let actor = entity(1);
        let step = sample_step();

        let request = step.to_request_action(actor);

        assert_eq!(
            request,
            InputKind::RequestAction {
                actor,
                def_id: step.def_id,
                targets: step.targets.clone(),
                payload_override: step.payload_override.clone(),
            }
        );
    }

    #[test]
    fn planned_plan_new_derives_total_estimated_ticks_from_steps() {
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let mut second = sample_step();
        second.estimated_ticks = 9;
        second.is_materialization_barrier = true;

        let plan = PlannedPlan::new(
            goal,
            vec![sample_step(), second],
            PlanTerminalKind::ProgressBarrier,
        );

        assert_eq!(plan.total_estimated_ticks, 14);
    }

    #[test]
    fn planned_plan_new_uses_zero_ticks_for_empty_steps() {
        let plan = PlannedPlan::new(
            GoalKey::from(GoalKind::ReduceDanger),
            Vec::new(),
            PlanTerminalKind::ProgressBarrier,
        );

        assert_eq!(plan.total_estimated_ticks, 0);
    }

    #[test]
    fn planned_plan_roundtrips_through_bincode() {
        let plan = PlannedPlan::new(
            GoalKey::from(GoalKind::Sleep),
            vec![PlannedStep {
                def_id: ActionDefId(2),
                targets: vec![entity(6)],
                payload_override: None,
                estimated_ticks: 1,
                is_materialization_barrier: false,
            }],
            PlanTerminalKind::GoalSatisfied,
        );

        let bytes = bincode::serialize(&plan).unwrap();
        let roundtrip: PlannedPlan = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, plan);
    }
}
