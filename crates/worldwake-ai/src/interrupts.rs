use crate::{
    goal_switching::{compare_goal_switch, GoalSwitchKind},
    AgentDecisionRuntime, GoalPriorityClass, RankedGoal,
};
use worldwake_core::{CommodityPurpose, GoalKey, GoalKind};
use worldwake_sim::Interruptibility;

use crate::PlanningBudget;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum InterruptDecision {
    NoInterrupt,
    InterruptForReplan { trigger: InterruptTrigger },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum InterruptTrigger {
    CriticalSurvival,
    CriticalDanger,
    HigherPriorityGoal,
    SuperiorSameClassPlan,
    PlanInvalid,
    OpportunisticLoot,
}

pub fn evaluate_interrupt(
    runtime: &AgentDecisionRuntime,
    current_action_interruptibility: Interruptibility,
    ranked_candidates: &[RankedGoal],
    plan_valid: bool,
    budget: &PlanningBudget,
) -> InterruptDecision {
    if current_action_interruptibility == Interruptibility::NonInterruptible {
        return InterruptDecision::NoInterrupt;
    }

    if !plan_valid {
        return InterruptDecision::InterruptForReplan {
            trigger: InterruptTrigger::PlanInvalid,
        };
    }

    let Some(challenger) = best_challenger(runtime.current_goal, ranked_candidates) else {
        return InterruptDecision::NoInterrupt;
    };

    match current_action_interruptibility {
        Interruptibility::NonInterruptible => InterruptDecision::NoInterrupt,
        Interruptibility::InterruptibleWithPenalty => interrupt_with_penalty(challenger),
        Interruptibility::FreelyInterruptible => {
            interrupt_freely(runtime, challenger, ranked_candidates, budget)
        }
    }
}

fn interrupt_with_penalty(challenger: &RankedGoal) -> InterruptDecision {
    if challenger.priority_class != GoalPriorityClass::Critical {
        return InterruptDecision::NoInterrupt;
    }

    if is_critical_survival_goal(&challenger.grounded.key.kind) {
        InterruptDecision::InterruptForReplan {
            trigger: InterruptTrigger::CriticalSurvival,
        }
    } else if matches!(challenger.grounded.key.kind, GoalKind::ReduceDanger) {
        InterruptDecision::InterruptForReplan {
            trigger: InterruptTrigger::CriticalDanger,
        }
    } else {
        InterruptDecision::NoInterrupt
    }
}

fn interrupt_freely(
    runtime: &AgentDecisionRuntime,
    challenger: &RankedGoal,
    ranked_candidates: &[RankedGoal],
    budget: &PlanningBudget,
) -> InterruptDecision {
    if matches!(challenger.grounded.key.kind, GoalKind::LootCorpse { .. }) {
        return if no_medium_or_above_self_care_or_danger(ranked_candidates) {
            InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::OpportunisticLoot,
            }
        } else {
            InterruptDecision::NoInterrupt
        };
    }

    let Some((current_class, current_motive)) = current_priority(runtime, ranked_candidates) else {
        return InterruptDecision::NoInterrupt;
    };
    let Some(switch_kind) = compare_goal_switch(
        current_class,
        current_motive,
        challenger.priority_class,
        challenger.motive_score,
        budget.switch_margin_permille,
    ) else {
        return InterruptDecision::NoInterrupt;
    };

    match switch_kind {
        GoalSwitchKind::HigherPriorityGoal if is_reactive_goal(&challenger.grounded.key.kind) => {
            InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::HigherPriorityGoal,
            }
        }
        GoalSwitchKind::SameClassMargin => InterruptDecision::InterruptForReplan {
            trigger: InterruptTrigger::SuperiorSameClassPlan,
        },
        GoalSwitchKind::HigherPriorityGoal => InterruptDecision::NoInterrupt,
    }
}

fn best_challenger(
    current_goal: Option<GoalKey>,
    ranked_candidates: &[RankedGoal],
) -> Option<&RankedGoal> {
    ranked_candidates
        .iter()
        .find(|candidate| Some(candidate.grounded.key) != current_goal)
}

fn current_priority(
    runtime: &AgentDecisionRuntime,
    ranked_candidates: &[RankedGoal],
) -> Option<(GoalPriorityClass, Option<u32>)> {
    if let Some(current_goal) = runtime.current_goal {
        if let Some(current) = ranked_candidates
            .iter()
            .find(|candidate| candidate.grounded.key == current_goal)
        {
            return Some((current.priority_class, Some(current.motive_score)));
        }
    }

    runtime.last_priority_class.map(|class| (class, None))
}

fn is_critical_survival_goal(kind: &GoalKind) -> bool {
    matches!(
        kind,
        GoalKind::ConsumeOwnedCommodity { .. }
            | GoalKind::AcquireCommodity {
                purpose: CommodityPurpose::SelfConsume,
                ..
            }
            | GoalKind::Sleep
            | GoalKind::Relieve
            | GoalKind::Wash
    )
}

fn is_reactive_goal(kind: &GoalKind) -> bool {
    is_critical_survival_goal(kind)
        || matches!(kind, GoalKind::ReduceDanger | GoalKind::Heal { .. })
}

fn no_medium_or_above_self_care_or_danger(ranked_candidates: &[RankedGoal]) -> bool {
    ranked_candidates.iter().all(|candidate| {
        if candidate.priority_class < GoalPriorityClass::Medium {
            return true;
        }

        !matches!(
            candidate.grounded.key.kind,
            GoalKind::ConsumeOwnedCommodity { .. }
                | GoalKind::AcquireCommodity {
                    purpose: CommodityPurpose::SelfConsume,
                    ..
                }
                | GoalKind::Sleep
                | GoalKind::Relieve
                | GoalKind::Wash
                | GoalKind::ReduceDanger
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{evaluate_interrupt, InterruptDecision, InterruptTrigger};
    use crate::{
        AgentDecisionRuntime, CommodityPurpose, GoalKey, GoalPriorityClass, GroundedGoal,
        PlanningBudget, RankedGoal,
    };
    use std::collections::BTreeSet;
    use worldwake_core::{CommodityKind, EntityId, GoalKind};
    use worldwake_sim::Interruptibility;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn ranked(kind: GoalKind, priority_class: GoalPriorityClass, motive_score: u32) -> RankedGoal {
        RankedGoal {
            grounded: GroundedGoal {
                key: GoalKey::from(kind),
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
            },
            priority_class,
            motive_score,
        }
    }

    fn runtime(
        current_goal: GoalKind,
        last_priority_class: GoalPriorityClass,
    ) -> AgentDecisionRuntime {
        AgentDecisionRuntime {
            current_goal: Some(GoalKey::from(current_goal)),
            current_plan: None,
            dirty: false,
            last_priority_class: Some(last_priority_class),
            ..AgentDecisionRuntime::default()
        }
    }

    #[test]
    fn non_interruptible_actions_ignore_even_critical_challengers() {
        let current_goal = GoalKind::RestockCommodity {
            commodity: CommodityKind::Bread,
        };
        let challengers = vec![
            ranked(current_goal, GoalPriorityClass::Medium, 100),
            ranked(GoalKind::ReduceDanger, GoalPriorityClass::Critical, 1_000),
        ];

        let decision = evaluate_interrupt(
            &runtime(current_goal, GoalPriorityClass::Medium),
            Interruptibility::NonInterruptible,
            &challengers,
            true,
            &PlanningBudget::default(),
        );

        assert_eq!(decision, InterruptDecision::NoInterrupt);
    }

    #[test]
    fn interruptible_with_penalty_interrupts_for_critical_danger() {
        let current_goal = GoalKind::RestockCommodity {
            commodity: CommodityKind::Bread,
        };
        let challengers = vec![
            ranked(current_goal, GoalPriorityClass::Medium, 100),
            ranked(GoalKind::ReduceDanger, GoalPriorityClass::Critical, 1_000),
        ];

        let decision = evaluate_interrupt(
            &runtime(current_goal, GoalPriorityClass::Medium),
            Interruptibility::InterruptibleWithPenalty,
            &challengers,
            true,
            &PlanningBudget::default(),
        );

        assert_eq!(
            decision,
            InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::CriticalDanger,
            }
        );
    }

    #[test]
    fn interruptible_with_penalty_does_not_interrupt_for_high_danger() {
        let current_goal = GoalKind::RestockCommodity {
            commodity: CommodityKind::Bread,
        };
        let challengers = vec![
            ranked(current_goal, GoalPriorityClass::Medium, 100),
            ranked(GoalKind::ReduceDanger, GoalPriorityClass::High, 950),
        ];

        let decision = evaluate_interrupt(
            &runtime(current_goal, GoalPriorityClass::Medium),
            Interruptibility::InterruptibleWithPenalty,
            &challengers,
            true,
            &PlanningBudget::default(),
        );

        assert_eq!(decision, InterruptDecision::NoInterrupt);
    }

    #[test]
    fn interruptible_with_penalty_interrupts_for_invalid_plan() {
        let current_goal = GoalKind::RestockCommodity {
            commodity: CommodityKind::Bread,
        };
        let decision = evaluate_interrupt(
            &runtime(current_goal, GoalPriorityClass::Medium),
            Interruptibility::InterruptibleWithPenalty,
            &[ranked(current_goal, GoalPriorityClass::Medium, 100)],
            false,
            &PlanningBudget::default(),
        );

        assert_eq!(
            decision,
            InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::PlanInvalid,
            }
        );
    }

    #[test]
    fn freely_interruptible_interrupts_for_higher_priority_reactive_goal() {
        let current_goal = GoalKind::RestockCommodity {
            commodity: CommodityKind::Bread,
        };
        let challengers = vec![
            ranked(current_goal, GoalPriorityClass::Medium, 100),
            ranked(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                },
                GoalPriorityClass::High,
                900,
            ),
        ];

        let decision = evaluate_interrupt(
            &runtime(current_goal, GoalPriorityClass::Medium),
            Interruptibility::FreelyInterruptible,
            &challengers,
            true,
            &PlanningBudget::default(),
        );

        assert_eq!(
            decision,
            InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::HigherPriorityGoal,
            }
        );
    }

    #[test]
    fn freely_interruptible_requires_margin_for_same_class_switch() {
        let current_goal = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        };
        let runtime = runtime(current_goal, GoalPriorityClass::High);
        let below_margin = vec![
            ranked(current_goal, GoalPriorityClass::High, 1_000),
            ranked(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                },
                GoalPriorityClass::High,
                1_099,
            ),
        ];
        let at_margin = vec![
            ranked(current_goal, GoalPriorityClass::High, 1_000),
            ranked(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                },
                GoalPriorityClass::High,
                1_100,
            ),
        ];

        assert_eq!(
            evaluate_interrupt(
                &runtime,
                Interruptibility::FreelyInterruptible,
                &below_margin,
                true,
                &PlanningBudget::default(),
            ),
            InterruptDecision::NoInterrupt
        );
        assert_eq!(
            evaluate_interrupt(
                &runtime,
                Interruptibility::FreelyInterruptible,
                &at_margin,
                true,
                &PlanningBudget::default(),
            ),
            InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::SuperiorSameClassPlan,
            }
        );
    }

    #[test]
    fn freely_interruptible_allows_loot_only_without_medium_self_care_or_danger() {
        let current_goal = GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        };
        let no_pressure = vec![
            ranked(
                GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread,
                },
                GoalPriorityClass::Background,
                100,
            ),
            ranked(
                GoalKind::LootCorpse { corpse: entity(9) },
                GoalPriorityClass::Low,
                50,
            ),
        ];
        let blocked_by_hunger = vec![
            ranked(current_goal, GoalPriorityClass::Medium, 700),
            ranked(
                GoalKind::LootCorpse { corpse: entity(9) },
                GoalPriorityClass::Low,
                50,
            ),
        ];

        assert_eq!(
            evaluate_interrupt(
                &runtime(
                    GoalKind::RestockCommodity {
                        commodity: CommodityKind::Bread,
                    },
                    GoalPriorityClass::Background,
                ),
                Interruptibility::FreelyInterruptible,
                &no_pressure,
                true,
                &PlanningBudget::default(),
            ),
            InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::OpportunisticLoot,
            }
        );
        assert_eq!(
            evaluate_interrupt(
                &runtime(current_goal, GoalPriorityClass::Medium),
                Interruptibility::FreelyInterruptible,
                &blocked_by_hunger,
                true,
                &PlanningBudget::default(),
            ),
            InterruptDecision::NoInterrupt
        );
    }

    #[test]
    fn freely_interruptible_does_not_switch_for_higher_priority_enterprise_goal() {
        let current_goal = GoalKind::LootCorpse { corpse: entity(1) };
        let challengers = vec![
            ranked(current_goal, GoalPriorityClass::Low, 20),
            ranked(
                GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread,
                },
                GoalPriorityClass::Medium,
                900,
            ),
        ];

        let decision = evaluate_interrupt(
            &runtime(current_goal, GoalPriorityClass::Low),
            Interruptibility::FreelyInterruptible,
            &challengers,
            true,
            &PlanningBudget::default(),
        );

        assert_eq!(decision, InterruptDecision::NoInterrupt);
    }
}
