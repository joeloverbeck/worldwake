use crate::{GoalKey, GoalKindTag};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldwake_core::EntityId;
use worldwake_sim::{ActionDef, ActionDefId, ActionDefRegistry, ActionDomain, ActionPayload, InputKind};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum PlannerOpKind {
    Travel,
    Consume,
    Sleep,
    Relieve,
    Wash,
    Trade,
    Harvest,
    Craft,
    MoveCargo,
    Heal,
    Loot,
    Attack,
    Defend,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PlannerOpSemantics {
    pub op_kind: PlannerOpKind,
    pub may_appear_mid_plan: bool,
    pub is_materialization_barrier: bool,
    pub relevant_goal_kinds: &'static [GoalKindTag],
}

const GOALS_CONSUME: &[GoalKindTag] = &[GoalKindTag::ConsumeOwnedCommodity];
const GOALS_ACQUIRE: &[GoalKindTag] = &[
    GoalKindTag::ConsumeOwnedCommodity,
    GoalKindTag::AcquireCommodity,
    GoalKindTag::Heal,
    GoalKindTag::SellCommodity,
    GoalKindTag::RestockCommodity,
];
const GOALS_PRODUCE: &[GoalKindTag] = &[
    GoalKindTag::ConsumeOwnedCommodity,
    GoalKindTag::AcquireCommodity,
    GoalKindTag::Heal,
    GoalKindTag::ProduceCommodity,
    GoalKindTag::RestockCommodity,
];
const GOALS_MOVE_CARGO: &[GoalKindTag] = &[
    GoalKindTag::ConsumeOwnedCommodity,
    GoalKindTag::AcquireCommodity,
    GoalKindTag::Wash,
    GoalKindTag::ProduceCommodity,
    GoalKindTag::SellCommodity,
    GoalKindTag::RestockCommodity,
    GoalKindTag::MoveCargo,
];
const GOALS_HEAL: &[GoalKindTag] = &[GoalKindTag::ReduceDanger, GoalKindTag::Heal];
const GOALS_LOOT: &[GoalKindTag] = &[GoalKindTag::LootCorpse];
const GOALS_ATTACK: &[GoalKindTag] = &[GoalKindTag::ReduceDanger];
const GOALS_DEFEND: &[GoalKindTag] = &[GoalKindTag::ReduceDanger];

#[must_use]
pub fn build_semantics_table(registry: &ActionDefRegistry) -> BTreeMap<ActionDefId, PlannerOpSemantics> {
    registry
        .iter()
        .filter_map(|def| classify_action_def(def).map(|op_kind| (def.id, semantics_for(op_kind))))
        .collect()
}

fn classify_action_def(def: &ActionDef) -> Option<PlannerOpKind> {
    match (def.domain, def.name.as_str(), &def.payload) {
        (ActionDomain::Travel, "travel", _) => Some(PlannerOpKind::Travel),
        (ActionDomain::Needs, "eat" | "drink", _) => Some(PlannerOpKind::Consume),
        (ActionDomain::Needs, "sleep", _) => Some(PlannerOpKind::Sleep),
        (ActionDomain::Needs, "toilet", _) => Some(PlannerOpKind::Relieve),
        (ActionDomain::Needs, "wash", _) => Some(PlannerOpKind::Wash),
        (ActionDomain::Trade, "trade", _) => Some(PlannerOpKind::Trade),
        (ActionDomain::Production, name, ActionPayload::Harvest(_)) if name.starts_with("harvest:") => {
            Some(PlannerOpKind::Harvest)
        }
        (ActionDomain::Production, name, ActionPayload::Craft(_)) if name.starts_with("craft:") => {
            Some(PlannerOpKind::Craft)
        }
        (ActionDomain::Transport, "pick_up" | "put_down", _) => Some(PlannerOpKind::MoveCargo),
        (ActionDomain::Care, "heal", _) => Some(PlannerOpKind::Heal),
        (ActionDomain::Loot, "loot", _) => Some(PlannerOpKind::Loot),
        (ActionDomain::Combat, "attack", _) => Some(PlannerOpKind::Attack),
        (ActionDomain::Combat, "defend", _) => Some(PlannerOpKind::Defend),
        _ => None,
    }
}

const fn semantics_for(op_kind: PlannerOpKind) -> PlannerOpSemantics {
    match op_kind {
        PlannerOpKind::Travel => PlannerOpSemantics {
            op_kind,
            may_appear_mid_plan: true,
            is_materialization_barrier: false,
            relevant_goal_kinds: &[
                GoalKindTag::ConsumeOwnedCommodity,
                GoalKindTag::AcquireCommodity,
                GoalKindTag::Sleep,
                GoalKindTag::Relieve,
                GoalKindTag::Wash,
                GoalKindTag::ReduceDanger,
                GoalKindTag::Heal,
                GoalKindTag::ProduceCommodity,
                GoalKindTag::SellCommodity,
                GoalKindTag::RestockCommodity,
                GoalKindTag::MoveCargo,
                GoalKindTag::LootCorpse,
            ],
        },
        PlannerOpKind::Consume => PlannerOpSemantics {
            op_kind,
            may_appear_mid_plan: true,
            is_materialization_barrier: false,
            relevant_goal_kinds: GOALS_CONSUME,
        },
        PlannerOpKind::Sleep => PlannerOpSemantics {
            op_kind,
            may_appear_mid_plan: true,
            is_materialization_barrier: false,
            relevant_goal_kinds: &[GoalKindTag::Sleep],
        },
        PlannerOpKind::Relieve => PlannerOpSemantics {
            op_kind,
            may_appear_mid_plan: true,
            is_materialization_barrier: false,
            relevant_goal_kinds: &[GoalKindTag::Relieve],
        },
        PlannerOpKind::Wash => PlannerOpSemantics {
            op_kind,
            may_appear_mid_plan: true,
            is_materialization_barrier: false,
            relevant_goal_kinds: &[GoalKindTag::Wash],
        },
        PlannerOpKind::Trade => PlannerOpSemantics {
            op_kind,
            may_appear_mid_plan: true,
            is_materialization_barrier: true,
            relevant_goal_kinds: GOALS_ACQUIRE,
        },
        PlannerOpKind::Harvest => PlannerOpSemantics {
            op_kind,
            may_appear_mid_plan: true,
            is_materialization_barrier: true,
            relevant_goal_kinds: &[
                GoalKindTag::ConsumeOwnedCommodity,
                GoalKindTag::AcquireCommodity,
                GoalKindTag::RestockCommodity,
            ],
        },
        PlannerOpKind::Craft => PlannerOpSemantics {
            op_kind,
            may_appear_mid_plan: true,
            is_materialization_barrier: true,
            relevant_goal_kinds: GOALS_PRODUCE,
        },
        PlannerOpKind::MoveCargo => PlannerOpSemantics {
            op_kind,
            may_appear_mid_plan: true,
            is_materialization_barrier: false,
            relevant_goal_kinds: GOALS_MOVE_CARGO,
        },
        PlannerOpKind::Heal => PlannerOpSemantics {
            op_kind,
            may_appear_mid_plan: true,
            is_materialization_barrier: false,
            relevant_goal_kinds: GOALS_HEAL,
        },
        PlannerOpKind::Loot => PlannerOpSemantics {
            op_kind,
            may_appear_mid_plan: true,
            is_materialization_barrier: true,
            relevant_goal_kinds: GOALS_LOOT,
        },
        PlannerOpKind::Attack => PlannerOpSemantics {
            op_kind,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            relevant_goal_kinds: GOALS_ATTACK,
        },
        PlannerOpKind::Defend => PlannerOpSemantics {
            op_kind,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            relevant_goal_kinds: GOALS_DEFEND,
        },
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PlannedStep {
    pub def_id: ActionDefId,
    pub targets: Vec<EntityId>,
    pub payload_override: Option<ActionPayload>,
    pub op_kind: PlannerOpKind,
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
    CombatCommitment,
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
    use super::{
        build_semantics_table, PlanTerminalKind, PlannedPlan, PlannedStep, PlannerOpKind,
    };
    use crate::{CommodityPurpose, GoalKey, GoalKind};
    use std::num::NonZeroU32;
    use worldwake_core::{
        BodyCostPerTick, CommodityKind, EntityId, Quantity, UniqueItemKind, WorkstationTag,
    };
    use worldwake_sim::{
        ActionDefId, ActionDefRegistry, ActionHandlerRegistry, ActionPayload, InputKind,
        RecipeDefinition, RecipeRegistry, TradeActionPayload,
    };
    use worldwake_systems::{
        register_attack_action, register_craft_actions, register_defend_action, register_harvest_actions,
        register_heal_action, register_loot_action, register_needs_actions, register_trade_action,
        register_transport_actions, register_travel_actions,
    };

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
            op_kind: PlannerOpKind::Trade,
            estimated_ticks: 5,
            is_materialization_barrier: true,
        }
    }

    fn build_phase_two_registry() -> ActionDefRegistry {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        let mut recipes = RecipeRegistry::new();
        recipes.register(RecipeDefinition {
            name: "Harvest Apples".to_string(),
            inputs: Vec::new(),
            outputs: vec![(CommodityKind::Apple, Quantity(2))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::OrchardRow),
            required_tool_kinds: vec![UniqueItemKind::SimpleTool],
            body_cost_per_tick: BodyCostPerTick::zero(),
        });
        recipes.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(4).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: vec![UniqueItemKind::SimpleTool],
            body_cost_per_tick: BodyCostPerTick::zero(),
        });

        register_needs_actions(&mut defs, &mut handlers);
        let _ = register_travel_actions(&mut defs, &mut handlers);
        let _ = register_transport_actions(&mut defs, &mut handlers);
        let _ = register_trade_action(&mut defs, &mut handlers);
        let _ = register_harvest_actions(&mut defs, &mut handlers, &recipes);
        let _ = register_craft_actions(&mut defs, &mut handlers, &recipes);
        let _ = register_attack_action(&mut defs, &mut handlers);
        let _ = register_defend_action(&mut defs, &mut handlers);
        let _ = register_loot_action(&mut defs, &mut handlers);
        let _ = register_heal_action(&mut defs, &mut handlers);

        defs
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
                op_kind: PlannerOpKind::Sleep,
                estimated_ticks: 1,
                is_materialization_barrier: false,
            }],
            PlanTerminalKind::GoalSatisfied,
        );

        let bytes = bincode::serialize(&plan).unwrap();
        let roundtrip: PlannedPlan = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, plan);
    }

    #[test]
    fn planner_op_kind_covers_exactly_current_phase_two_families() {
        let all = [
            PlannerOpKind::Travel,
            PlannerOpKind::Consume,
            PlannerOpKind::Sleep,
            PlannerOpKind::Relieve,
            PlannerOpKind::Wash,
            PlannerOpKind::Trade,
            PlannerOpKind::Harvest,
            PlannerOpKind::Craft,
            PlannerOpKind::MoveCargo,
            PlannerOpKind::Heal,
            PlannerOpKind::Loot,
            PlannerOpKind::Attack,
            PlannerOpKind::Defend,
        ];

        assert_eq!(all.len(), 13);
    }

    #[test]
    fn build_semantics_table_classifies_all_registered_phase_two_defs() {
        let defs = build_phase_two_registry();

        let table = build_semantics_table(&defs);

        assert_eq!(table.len(), defs.len());
        assert_eq!(table.get(&ActionDefId(0)).unwrap().op_kind, PlannerOpKind::Consume);
        assert_eq!(table.get(&ActionDefId(2)).unwrap().op_kind, PlannerOpKind::Sleep);
        assert_eq!(table.get(&ActionDefId(3)).unwrap().op_kind, PlannerOpKind::Relieve);
        assert_eq!(table.get(&ActionDefId(5)).unwrap().op_kind, PlannerOpKind::Travel);
        assert_eq!(table.get(&ActionDefId(6)).unwrap().op_kind, PlannerOpKind::MoveCargo);
        assert_eq!(table.get(&ActionDefId(8)).unwrap().op_kind, PlannerOpKind::Trade);
        assert_eq!(table.get(&ActionDefId(9)).unwrap().op_kind, PlannerOpKind::Harvest);
        assert_eq!(table.get(&ActionDefId(10)).unwrap().op_kind, PlannerOpKind::Craft);
        assert_eq!(table.get(&ActionDefId(11)).unwrap().op_kind, PlannerOpKind::Attack);
        assert_eq!(table.get(&ActionDefId(12)).unwrap().op_kind, PlannerOpKind::Defend);
        assert_eq!(table.get(&ActionDefId(13)).unwrap().op_kind, PlannerOpKind::Loot);
        assert_eq!(table.get(&ActionDefId(14)).unwrap().op_kind, PlannerOpKind::Heal);
    }

    #[test]
    fn build_semantics_table_marks_barriers_and_leaf_only_ops() {
        let defs = build_phase_two_registry();
        let table = build_semantics_table(&defs);

        for id in [ActionDefId(8), ActionDefId(9), ActionDefId(10), ActionDefId(13)] {
            assert!(table.get(&id).unwrap().is_materialization_barrier);
        }
        for id in [
            ActionDefId(0),
            ActionDefId(1),
            ActionDefId(2),
            ActionDefId(3),
            ActionDefId(4),
            ActionDefId(5),
            ActionDefId(6),
            ActionDefId(7),
            ActionDefId(11),
            ActionDefId(12),
            ActionDefId(14),
        ] {
            assert!(!table.get(&id).unwrap().is_materialization_barrier);
        }
        assert!(!table.get(&ActionDefId(11)).unwrap().may_appear_mid_plan);
        assert!(!table.get(&ActionDefId(12)).unwrap().may_appear_mid_plan);
    }
}
