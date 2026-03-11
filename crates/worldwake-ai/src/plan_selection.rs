use crate::{
    goal_switching::{compare_goal_switch, GoalSwitchKind},
    AgentDecisionRuntime, GoalKey, GoalPriorityClass, PlannedPlan, PlanningBudget, RankedGoal,
};
use std::cmp::Ordering;
use std::collections::BTreeMap;

pub fn select_best_plan(
    candidates: &[RankedGoal],
    plans: &[(GoalKey, Option<PlannedPlan>)],
    current: &AgentDecisionRuntime,
    budget: &PlanningBudget,
) -> Option<PlannedPlan> {
    let candidate_scores = candidates
        .iter()
        .map(|candidate| {
            (
                candidate.grounded.key,
                (candidate.priority_class, candidate.motive_score),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut available = plans
        .iter()
        .filter_map(|(key, plan)| {
            let plan = plan.as_ref()?;
            let (priority_class, motive_score) = candidate_scores.get(key).copied()?;
            Some((priority_class, motive_score, plan.clone()))
        })
        .collect::<Vec<_>>();
    available.sort_by(compare_ranked_plans);
    let (_, best_motive, best_plan) = available.first()?.clone();

    let Some(current_plan) = current.current_plan.clone() else {
        return Some(best_plan);
    };
    let Some((current_class, current_motive)) = candidate_scores.get(&current_plan.goal).copied() else {
        return Some(best_plan);
    };
    let Some((best_class, _, _)) = available.first().cloned() else {
        return Some(current_plan);
    };

    if best_plan.goal == current_plan.goal {
        return Some(if current_plan.steps.is_empty() { best_plan } else { current_plan });
    }
    if matches!(
        compare_goal_switch(
            current_class,
            Some(current_motive),
            best_class,
            best_motive,
            budget
        ),
        Some(GoalSwitchKind::HigherPriorityGoal | GoalSwitchKind::SameClassMargin)
    ) {
        return Some(best_plan);
    }

    Some(current_plan)
}

fn compare_ranked_plans(
    left: &(GoalPriorityClass, u32, PlannedPlan),
    right: &(GoalPriorityClass, u32, PlannedPlan),
) -> Ordering {
    right
        .0
        .cmp(&left.0)
        .then_with(|| right.1.cmp(&left.1))
        .then_with(|| left.2.total_estimated_ticks.cmp(&right.2.total_estimated_ticks))
        .then_with(|| left.2.steps.cmp(&right.2.steps))
        .then_with(|| left.2.goal.cmp(&right.2.goal))
}

#[cfg(test)]
mod tests {
    use super::select_best_plan;
    use crate::{
        AgentDecisionRuntime, CommodityPurpose, GoalKey, GoalPriorityClass, GroundedGoal,
        PlanTerminalKind, PlannedPlan, PlannedStep, PlannerOpKind, PlanningBudget, RankedGoal,
    };
    use std::collections::BTreeSet;
    use worldwake_core::{CommodityKind, EntityId};
    use worldwake_sim::ActionDefId;

    fn entity(slot: u32) -> EntityId {
        EntityId { slot, generation: 1 }
    }

    fn ranked(
        goal: worldwake_core::GoalKind,
        class: GoalPriorityClass,
        motive: u32,
    ) -> RankedGoal {
        RankedGoal {
            grounded: GroundedGoal {
                key: GoalKey::from(goal),
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
            },
            priority_class: class,
            motive_score: motive,
        }
    }

    fn plan(goal: GoalKey, def_id: u32, ticks: u32) -> PlannedPlan {
        PlannedPlan::new(
            goal,
            vec![PlannedStep {
                def_id: ActionDefId(def_id),
                targets: vec![entity(def_id)],
                payload_override: None,
                op_kind: PlannerOpKind::Travel,
                estimated_ticks: ticks,
                is_materialization_barrier: false,
            }],
            PlanTerminalKind::GoalSatisfied,
        )
    }

    #[test]
    fn selection_prefers_higher_priority_class_before_cost() {
        let sleep_goal = GoalKey::from(worldwake_core::GoalKind::Sleep);
        let eat_goal = GoalKey::from(worldwake_core::GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        });
        let candidates = vec![
            ranked(worldwake_core::GoalKind::Sleep, GoalPriorityClass::Medium, 900),
            ranked(
                worldwake_core::GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Bread,
                },
                GoalPriorityClass::Critical,
                1,
            ),
        ];
        let plans = vec![
            (sleep_goal, Some(plan(sleep_goal, 1, 1))),
            (eat_goal, Some(plan(eat_goal, 2, 9))),
        ];

        let selected = select_best_plan(
            &candidates,
            &plans,
            &AgentDecisionRuntime::default(),
            &PlanningBudget::default(),
        )
        .unwrap();

        assert_eq!(selected.goal, eat_goal);
    }

    #[test]
    fn same_class_replacement_requires_switch_margin() {
        let current_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let challenger_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
        });
        let current_plan = plan(current_goal, 1, 3);
        let challenger_plan = plan(challenger_goal, 2, 2);
        let candidates = vec![
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: CommodityPurpose::SelfConsume,
                },
                GoalPriorityClass::High,
                1000,
            ),
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                },
                GoalPriorityClass::High,
                1099,
            ),
        ];
        let plans = vec![
            (current_goal, Some(current_plan.clone())),
            (challenger_goal, Some(challenger_plan)),
        ];
        let runtime = AgentDecisionRuntime {
            current_goal: Some(current_goal),
            current_plan: Some(current_plan.clone()),
            dirty: false,
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };

        let selected = select_best_plan(&candidates, &plans, &runtime, &PlanningBudget::default()).unwrap();

        assert_eq!(selected.goal, current_goal);
    }

    #[test]
    fn deterministic_tie_break_uses_cost_then_step_order() {
        let first_goal = GoalKey::from(worldwake_core::GoalKind::Sleep);
        let second_goal = GoalKey::from(worldwake_core::GoalKind::Relieve);
        let candidates = vec![
            ranked(worldwake_core::GoalKind::Sleep, GoalPriorityClass::Medium, 500),
            ranked(worldwake_core::GoalKind::Relieve, GoalPriorityClass::Medium, 500),
        ];
        let slower = plan(first_goal, 4, 3);
        let faster = plan(second_goal, 3, 2);
        let plans = vec![(first_goal, Some(slower)), (second_goal, Some(faster.clone()))];

        let first = select_best_plan(
            &candidates,
            &plans,
            &AgentDecisionRuntime::default(),
            &PlanningBudget::default(),
        )
        .unwrap();
        let second = select_best_plan(
            &candidates,
            &plans,
            &AgentDecisionRuntime::default(),
            &PlanningBudget::default(),
        )
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.goal, faster.goal);
    }

    fn empty_plan(goal: GoalKey) -> PlannedPlan {
        PlannedPlan::new(goal, Vec::new(), PlanTerminalKind::GoalSatisfied)
    }

    #[test]
    fn empty_current_plan_replaced_by_actionable_plan_for_same_goal() {
        let eat_goal = GoalKey::from(worldwake_core::GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        });
        let candidates = vec![ranked(
            worldwake_core::GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalPriorityClass::High,
            800,
        )];
        let actionable = plan(eat_goal, 1, 3);
        let plans = vec![(eat_goal, Some(actionable.clone()))];
        let runtime = AgentDecisionRuntime {
            current_goal: Some(eat_goal),
            current_plan: Some(empty_plan(eat_goal)),
            dirty: false,
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };

        let selected = select_best_plan(&candidates, &plans, &runtime, &PlanningBudget::default()).unwrap();

        assert_eq!(selected.goal, eat_goal);
        assert_eq!(selected.steps.len(), 1, "should adopt the actionable plan, not the empty one");
    }

    #[test]
    fn nonempty_current_plan_retained_over_new_plan_for_same_goal() {
        let eat_goal = GoalKey::from(worldwake_core::GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        });
        let candidates = vec![ranked(
            worldwake_core::GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalPriorityClass::High,
            800,
        )];
        let current = plan(eat_goal, 1, 3);
        let challenger = plan(eat_goal, 2, 1);
        let plans = vec![(eat_goal, Some(challenger))];
        let runtime = AgentDecisionRuntime {
            current_goal: Some(eat_goal),
            current_plan: Some(current.clone()),
            dirty: false,
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };

        let selected = select_best_plan(&candidates, &plans, &runtime, &PlanningBudget::default()).unwrap();

        assert_eq!(selected, current, "non-empty current plan should be retained for same goal");
    }

    #[test]
    fn both_empty_plans_same_goal_selects_best() {
        let eat_goal = GoalKey::from(worldwake_core::GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        });
        let candidates = vec![ranked(
            worldwake_core::GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalPriorityClass::High,
            800,
        )];
        let plans = vec![(eat_goal, Some(empty_plan(eat_goal)))];
        let runtime = AgentDecisionRuntime {
            current_goal: Some(eat_goal),
            current_plan: Some(empty_plan(eat_goal)),
            dirty: false,
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };

        let selected = select_best_plan(&candidates, &plans, &runtime, &PlanningBudget::default()).unwrap();

        assert_eq!(selected.goal, eat_goal);
        assert!(selected.steps.is_empty(), "both plans are empty — best is selected but also empty");
    }
}
