use crate::{
    goal_switching::GoalSwitchKind, journey_switch_policy::compare_relation_aware_goal_switch,
    AgentDecisionRuntime, GoalKey, GoalPriorityClass, JourneyPlanRelation, PlannedPlan, RankedGoal,
};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use worldwake_core::Permille;

pub fn select_best_plan(
    candidates: &[RankedGoal],
    plans: &[(GoalKey, Option<PlannedPlan>)],
    current: &AgentDecisionRuntime,
    default_switch_margin: Permille,
    journey_switch_margin: Permille,
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
    let best_plan = available.first()?.2.clone();
    let has_current_goal_plan = current.current_goal.is_some_and(|goal| {
        plans
            .iter()
            .any(|(key, plan)| *key == goal && plan.is_some())
    });

    let Some(current_plan) = current.current_plan.clone() else {
        return Some(best_plan);
    };
    let Some((current_class, current_motive)) = candidate_scores.get(&current_plan.goal).copied()
    else {
        return Some(best_plan);
    };

    for (challenger_class, challenger_motive, challenger_plan) in available {
        let relation = current.classify_journey_plan_relation(&challenger_plan);
        if relation == JourneyPlanRelation::RefreshesCommitment
            || challenger_plan.goal == current_plan.goal
        {
            return Some(challenger_plan);
        }

        if matches!(
            compare_relation_aware_goal_switch(
                current_class,
                Some(current_motive),
                challenger_class,
                challenger_motive,
                relation,
                default_switch_margin,
                journey_switch_margin,
            ),
            Some(GoalSwitchKind::HigherPriorityGoal | GoalSwitchKind::SameClassMargin)
        ) {
            return Some(challenger_plan);
        }
    }

    if !has_current_goal_plan {
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
        .then_with(|| {
            left.2
                .total_estimated_ticks
                .cmp(&right.2.total_estimated_ticks)
        })
        .then_with(|| left.2.steps.cmp(&right.2.steps))
        .then_with(|| left.2.goal.cmp(&right.2.goal))
}

#[cfg(test)]
mod tests {
    use super::select_best_plan;
    use crate::{
        AgentDecisionRuntime, CommodityPurpose, GoalKey, GoalPriorityClass, GroundedGoal,
        PlanTerminalKind, PlannedPlan, PlannedStep, PlannerOpKind, PlanningEntityRef, RankedGoal,
    };
    use std::collections::BTreeSet;
    use worldwake_core::ActionDefId;
    use worldwake_core::{CommodityKind, EntityId, Permille};

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn ranked(goal: worldwake_core::GoalKind, class: GoalPriorityClass, motive: u32) -> RankedGoal {
        RankedGoal {
            grounded: GroundedGoal {
                key: GoalKey::from(goal),
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
            },
            priority_class: class,
            motive_score: motive,
            provenance: None,
        }
    }

    fn plan(goal: GoalKey, def_id: u32, ticks: u32) -> PlannedPlan {
        PlannedPlan::new(
            goal,
            vec![PlannedStep {
                def_id: ActionDefId(def_id),
                targets: vec![PlanningEntityRef::Authoritative(entity(def_id))],
                payload_override: None,
                op_kind: PlannerOpKind::Travel,
                estimated_ticks: ticks,
                is_materialization_barrier: false,
                expected_materializations: Vec::new(),
            }],
            PlanTerminalKind::GoalSatisfied,
        )
    }

    fn default_switch_margin() -> Permille {
        Permille::new(100).unwrap()
    }

    fn route_switch_margin() -> Permille {
        Permille::new(300).unwrap()
    }

    #[test]
    fn selection_prefers_higher_priority_class_before_cost() {
        let sleep_goal = GoalKey::from(worldwake_core::GoalKind::Sleep);
        let eat_goal = GoalKey::from(worldwake_core::GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        });
        let candidates = vec![
            ranked(
                worldwake_core::GoalKind::Sleep,
                GoalPriorityClass::Medium,
                900,
            ),
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
            default_switch_margin(),
            default_switch_margin(),
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

        let selected = select_best_plan(
            &candidates,
            &plans,
            &runtime,
            default_switch_margin(),
            default_switch_margin(),
        )
        .unwrap();

        assert_eq!(selected.goal, current_goal);
    }

    #[test]
    fn deterministic_tie_break_uses_cost_then_step_order() {
        let first_goal = GoalKey::from(worldwake_core::GoalKind::Sleep);
        let second_goal = GoalKey::from(worldwake_core::GoalKind::Relieve);
        let candidates = vec![
            ranked(
                worldwake_core::GoalKind::Sleep,
                GoalPriorityClass::Medium,
                500,
            ),
            ranked(
                worldwake_core::GoalKind::Relieve,
                GoalPriorityClass::Medium,
                500,
            ),
        ];
        let slower = plan(first_goal, 4, 3);
        let faster = plan(second_goal, 3, 2);
        let plans = vec![
            (first_goal, Some(slower)),
            (second_goal, Some(faster.clone())),
        ];

        let first = select_best_plan(
            &candidates,
            &plans,
            &AgentDecisionRuntime::default(),
            default_switch_margin(),
            default_switch_margin(),
        )
        .unwrap();
        let second = select_best_plan(
            &candidates,
            &plans,
            &AgentDecisionRuntime::default(),
            default_switch_margin(),
            default_switch_margin(),
        )
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.goal, faster.goal);
    }

    #[test]
    fn same_goal_replanning_replaces_stale_in_progress_plan() {
        let goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
        });
        let stale_plan = PlannedPlan::new(
            goal,
            vec![
                PlannedStep {
                    def_id: ActionDefId(1),
                    targets: vec![PlanningEntityRef::Authoritative(entity(11))],
                    payload_override: None,
                    op_kind: PlannerOpKind::Travel,
                    estimated_ticks: 5,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                },
                PlannedStep {
                    def_id: ActionDefId(2),
                    targets: vec![PlanningEntityRef::Authoritative(entity(12))],
                    payload_override: None,
                    op_kind: PlannerOpKind::Travel,
                    estimated_ticks: 4,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                },
            ],
            PlanTerminalKind::ProgressBarrier,
        );
        let refreshed_plan = plan(goal, 3, 2);
        let candidates = vec![ranked(
            worldwake_core::GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
            },
            GoalPriorityClass::High,
            900,
        )];
        let plans = vec![(goal, Some(refreshed_plan.clone()))];
        let runtime = AgentDecisionRuntime {
            current_goal: Some(goal),
            current_plan: Some(stale_plan),
            current_step_index: 1,
            dirty: true,
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };

        let selected = select_best_plan(
            &candidates,
            &plans,
            &runtime,
            default_switch_margin(),
            default_switch_margin(),
        )
        .unwrap();

        assert_eq!(selected, refreshed_plan);
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

        let selected = select_best_plan(
            &candidates,
            &plans,
            &runtime,
            default_switch_margin(),
            default_switch_margin(),
        )
        .unwrap();

        assert_eq!(selected.goal, eat_goal);
        assert_eq!(
            selected.steps.len(),
            1,
            "should adopt the actionable plan, not the empty one"
        );
    }

    #[test]
    fn nonempty_current_plan_is_replaced_by_refreshed_plan_for_same_goal() {
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
        let plans = vec![(eat_goal, Some(challenger.clone()))];
        let runtime = AgentDecisionRuntime {
            current_goal: Some(eat_goal),
            current_plan: Some(current.clone()),
            dirty: false,
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };

        let selected = select_best_plan(
            &candidates,
            &plans,
            &runtime,
            default_switch_margin(),
            default_switch_margin(),
        )
        .unwrap();

        assert_eq!(
            selected, challenger,
            "same-goal replanning should adopt the refreshed plan from current world state"
        );
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

        let selected = select_best_plan(
            &candidates,
            &plans,
            &runtime,
            default_switch_margin(),
            default_switch_margin(),
        )
        .unwrap();

        assert_eq!(selected.goal, eat_goal);
        assert!(
            selected.steps.is_empty(),
            "both plans are empty — best is selected but also empty"
        );
    }

    #[test]
    fn higher_effective_margin_raises_plan_switch_threshold() {
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
                1_000,
            ),
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                },
                GoalPriorityClass::High,
                1_350,
            ),
        ];
        let plans = vec![
            (current_goal, Some(current_plan.clone())),
            (challenger_goal, Some(challenger_plan.clone())),
        ];
        let runtime = AgentDecisionRuntime {
            current_goal: Some(current_goal),
            current_plan: Some(current_plan),
            journey_committed_goal: Some(current_goal),
            journey_committed_destination: Some(entity(1)),
            dirty: false,
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };

        let conservative = select_best_plan(
            &candidates,
            &plans,
            &runtime,
            default_switch_margin(),
            Permille::new(400).unwrap(),
        )
        .unwrap();
        let permissive = select_best_plan(
            &candidates,
            &plans,
            &runtime,
            default_switch_margin(),
            Permille::new(300).unwrap(),
        )
        .unwrap();

        assert_eq!(conservative.goal, current_goal);
        assert_eq!(permissive.goal, challenger_goal);
    }

    #[test]
    fn stale_current_plan_is_not_retained_when_current_goal_has_no_plan() {
        let current_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let fallback_goal = GoalKey::from(worldwake_core::GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Water,
        });
        let current_plan = plan(current_goal, 1, 3);
        let fallback_plan = plan(fallback_goal, 2, 2);
        let candidates = vec![
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: CommodityPurpose::SelfConsume,
                },
                GoalPriorityClass::High,
                1_000,
            ),
            ranked(
                worldwake_core::GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Water,
                },
                GoalPriorityClass::Medium,
                400,
            ),
        ];
        let plans = vec![
            (current_goal, None),
            (fallback_goal, Some(fallback_plan.clone())),
        ];
        let runtime = AgentDecisionRuntime {
            current_goal: Some(current_goal),
            current_plan: Some(current_plan),
            dirty: true,
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };

        let selected = select_best_plan(
            &candidates,
            &plans,
            &runtime,
            default_switch_margin(),
            default_switch_margin(),
        )
        .unwrap();

        assert_eq!(
            selected, fallback_plan,
            "fresh search should not retain a stale current plan when the current goal has no viable plan"
        );
    }

    #[test]
    fn suspended_detour_can_replace_current_plan_without_paying_route_margin() {
        let committed_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let detour_goal = GoalKey::from(worldwake_core::GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Water,
        });
        let destination = entity(44);
        let current_plan = PlannedPlan::new(
            committed_goal,
            vec![PlannedStep {
                targets: vec![PlanningEntityRef::Authoritative(destination)],
                ..PlannedStep {
                    def_id: ActionDefId(1),
                    targets: Vec::new(),
                    payload_override: None,
                    op_kind: PlannerOpKind::Travel,
                    estimated_ticks: 3,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                }
            }],
            PlanTerminalKind::GoalSatisfied,
        );
        let detour_plan = PlannedPlan::new(
            detour_goal,
            vec![PlannedStep {
                def_id: ActionDefId(2),
                targets: vec![PlanningEntityRef::Authoritative(entity(2))],
                payload_override: None,
                op_kind: PlannerOpKind::Consume,
                estimated_ticks: 1,
                is_materialization_barrier: false,
                expected_materializations: Vec::new(),
            }],
            PlanTerminalKind::GoalSatisfied,
        );
        let abandon_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
        });
        let abandon_plan = plan(abandon_goal, 3, 1);
        let candidates = vec![
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                },
                GoalPriorityClass::High,
                1_150,
            ),
            ranked(
                worldwake_core::GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Water,
                },
                GoalPriorityClass::High,
                1_120,
            ),
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: CommodityPurpose::SelfConsume,
                },
                GoalPriorityClass::High,
                1_000,
            ),
        ];
        let plans = vec![
            (abandon_goal, Some(abandon_plan)),
            (detour_goal, Some(detour_plan.clone())),
            (committed_goal, Some(current_plan.clone())),
        ];
        let runtime = AgentDecisionRuntime {
            current_goal: Some(committed_goal),
            current_plan: Some(current_plan),
            journey_committed_goal: Some(committed_goal),
            journey_committed_destination: Some(destination),
            ..AgentDecisionRuntime::default()
        };

        let selected = select_best_plan(
            &candidates,
            &plans,
            &runtime,
            default_switch_margin(),
            route_switch_margin(),
        )
        .unwrap();

        assert_eq!(selected.goal, detour_goal);
    }
}
