use crate::{
    goal_policy::{goal_family_policy, FreeInterruptRole, PenaltyInterruptEligibility},
    goal_switching::{compare_goal_switch, GoalSwitchKind},
    journey_switch_policy::compare_relation_aware_goal_switch,
    AgentDecisionRuntime, DecisionContext, GoalKey, GoalPriorityClass, JourneyPlanRelation,
    PlannedPlan, RankedGoal,
};
use std::collections::BTreeMap;
use worldwake_core::Permille;
use worldwake_sim::Interruptibility;

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

#[allow(clippy::too_many_arguments)]
pub fn evaluate_interrupt(
    runtime: &AgentDecisionRuntime,
    current_action_interruptibility: Interruptibility,
    ranked_candidates: &[RankedGoal],
    planned_candidates: Option<&[(GoalKey, Option<PlannedPlan>)]>,
    plan_valid: bool,
    default_switch_margin: Permille,
    journey_switch_margin: Permille,
    decision_context: &DecisionContext,
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
        Interruptibility::FreelyInterruptible => interrupt_freely(
            runtime,
            challenger,
            ranked_candidates,
            planned_candidates,
            default_switch_margin,
            journey_switch_margin,
            *decision_context,
        ),
    }
}

fn interrupt_with_penalty(challenger: &RankedGoal) -> InterruptDecision {
    if challenger.priority_class != GoalPriorityClass::Critical {
        return InterruptDecision::NoInterrupt;
    }
    let policy = goal_family_policy(&challenger.grounded.key.kind);
    match policy.penalty_interrupt {
        PenaltyInterruptEligibility::WhenCritical { trigger } => {
            InterruptDecision::InterruptForReplan { trigger }
        }
        PenaltyInterruptEligibility::Never => InterruptDecision::NoInterrupt,
    }
}

fn interrupt_freely(
    runtime: &AgentDecisionRuntime,
    challenger: &RankedGoal,
    ranked_candidates: &[RankedGoal],
    planned_candidates: Option<&[(GoalKey, Option<PlannedPlan>)]>,
    default_switch_margin: Permille,
    journey_switch_margin: Permille,
    decision_context: DecisionContext,
) -> InterruptDecision {
    let policy = goal_family_policy(&challenger.grounded.key.kind);
    if policy.free_interrupt == FreeInterruptRole::Opportunistic {
        return if decision_context.is_stressed_at_or_above(GoalPriorityClass::Medium) {
            InterruptDecision::NoInterrupt
        } else {
            InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::OpportunisticLoot,
            }
        };
    }

    let Some((current_class, current_motive)) = current_priority(runtime, ranked_candidates) else {
        return InterruptDecision::NoInterrupt;
    };

    if let Some((challenger, switch_kind)) = relation_aware_interrupt_candidate(
        runtime,
        ranked_candidates,
        planned_candidates,
        current_class,
        current_motive,
        default_switch_margin,
        journey_switch_margin,
    ) {
        return match switch_kind {
            GoalSwitchKind::HigherPriorityGoal
                if goal_family_policy(&challenger.grounded.key.kind).free_interrupt
                    == FreeInterruptRole::Reactive =>
            {
                InterruptDecision::InterruptForReplan {
                    trigger: InterruptTrigger::HigherPriorityGoal,
                }
            }
            GoalSwitchKind::SameClassMargin => InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::SuperiorSameClassPlan,
            },
            GoalSwitchKind::HigherPriorityGoal => InterruptDecision::NoInterrupt,
        };
    }

    if planned_candidates.is_some() {
        return InterruptDecision::NoInterrupt;
    }

    let Some(switch_kind) = compare_goal_switch(
        current_class,
        current_motive,
        challenger.priority_class,
        challenger.motive_score,
        default_switch_margin,
    ) else {
        return InterruptDecision::NoInterrupt;
    };

    match switch_kind {
        GoalSwitchKind::HigherPriorityGoal
            if policy.free_interrupt == FreeInterruptRole::Reactive =>
        {
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

fn relation_aware_interrupt_candidate<'a>(
    runtime: &AgentDecisionRuntime,
    ranked_candidates: &'a [RankedGoal],
    planned_candidates: Option<&'a [(GoalKey, Option<PlannedPlan>)]>,
    current_class: GoalPriorityClass,
    current_motive: Option<u32>,
    default_switch_margin: Permille,
    journey_switch_margin: Permille,
) -> Option<(&'a RankedGoal, GoalSwitchKind)> {
    let planned_candidates = planned_candidates?;
    let planned_by_goal = planned_candidates
        .iter()
        .filter_map(|(goal, plan)| plan.as_ref().map(|plan| (*goal, plan)))
        .collect::<BTreeMap<_, _>>();

    for challenger in ranked_candidates {
        if Some(challenger.grounded.key) == runtime.current_goal {
            continue;
        }

        let Some(plan) = planned_by_goal.get(&challenger.grounded.key) else {
            continue;
        };
        let relation = runtime.classify_journey_plan_relation(plan);
        let Some(switch_kind) = compare_relation_aware_goal_switch(
            current_class,
            current_motive,
            challenger.priority_class,
            challenger.motive_score,
            relation,
            default_switch_margin,
            journey_switch_margin,
        ) else {
            continue;
        };

        if relation == JourneyPlanRelation::RefreshesCommitment
            && matches!(
                switch_kind,
                GoalSwitchKind::HigherPriorityGoal | GoalSwitchKind::SameClassMargin
            )
        {
            return Some((challenger, switch_kind));
        }

        if relation != JourneyPlanRelation::NoCommitment {
            return Some((challenger, switch_kind));
        }
    }

    None
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

#[cfg(test)]
mod tests {
    use super::{evaluate_interrupt, InterruptDecision, InterruptTrigger};
    use crate::{
        AgentDecisionRuntime, CommodityPurpose, DecisionContext, GoalKey, GoalPriorityClass,
        GroundedGoal, PlannedPlan, RankedGoal,
    };
    use std::collections::BTreeSet;
    use worldwake_core::{ActionDefId, CommodityKind, EntityId, GoalKind, Permille};
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

    fn default_switch_margin() -> Permille {
        Permille::new(100).unwrap()
    }

    fn route_switch_margin() -> Permille {
        Permille::new(300).unwrap()
    }

    fn dummy_context() -> DecisionContext {
        DecisionContext {
            max_self_care_class: GoalPriorityClass::Background,
            danger_class: GoalPriorityClass::Background,
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
            None,
            true,
            default_switch_margin(),
            default_switch_margin(),
            &dummy_context(),
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
            None,
            true,
            default_switch_margin(),
            default_switch_margin(),
            &dummy_context(),
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
            None,
            true,
            default_switch_margin(),
            default_switch_margin(),
            &dummy_context(),
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
            None,
            false,
            default_switch_margin(),
            default_switch_margin(),
            &dummy_context(),
        );

        assert_eq!(
            decision,
            InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::PlanInvalid,
            }
        );
    }

    #[test]
    fn interruptible_with_penalty_does_not_interrupt_for_critical_heal() {
        let current_goal = GoalKind::RestockCommodity {
            commodity: CommodityKind::Bread,
        };
        let challengers = vec![
            ranked(current_goal, GoalPriorityClass::Medium, 100),
            ranked(
                GoalKind::TreatWounds {
                    patient: entity(99),
                },
                GoalPriorityClass::Critical,
                1_000,
            ),
        ];

        let decision = evaluate_interrupt(
            &runtime(current_goal, GoalPriorityClass::Medium),
            Interruptibility::InterruptibleWithPenalty,
            &challengers,
            None,
            true,
            default_switch_margin(),
            default_switch_margin(),
            &dummy_context(),
        );

        assert_eq!(decision, InterruptDecision::NoInterrupt);
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
            None,
            true,
            default_switch_margin(),
            default_switch_margin(),
            &dummy_context(),
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
        let runtime = AgentDecisionRuntime {
            current_goal: Some(GoalKey::from(current_goal)),
            current_plan: Some(PlannedPlan::new(
                GoalKey::from(current_goal),
                vec![crate::PlannedStep {
                    def_id: ActionDefId(1),
                    targets: vec![crate::PlanningEntityRef::Authoritative(entity(1))],
                    payload_override: None,
                    op_kind: crate::PlannerOpKind::Travel,
                    estimated_ticks: 2,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                }],
                crate::PlanTerminalKind::GoalSatisfied,
            )),
            journey_committed_goal: Some(GoalKey::from(current_goal)),
            journey_committed_destination: Some(entity(1)),
            dirty: false,
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };
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
                None,
                true,
                default_switch_margin(),
                default_switch_margin(),
                &dummy_context(),
            ),
            InterruptDecision::NoInterrupt
        );
        assert_eq!(
            evaluate_interrupt(
                &runtime,
                Interruptibility::FreelyInterruptible,
                &at_margin,
                None,
                true,
                default_switch_margin(),
                default_switch_margin(),
                &dummy_context(),
            ),
            InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::SuperiorSameClassPlan,
            }
        );
    }

    #[test]
    fn freely_interruptible_allows_loot_only_without_medium_stress() {
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
            ranked(
                GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Bread,
                },
                GoalPriorityClass::Medium,
                700,
            ),
            ranked(
                GoalKind::LootCorpse { corpse: entity(9) },
                GoalPriorityClass::Low,
                50,
            ),
        ];
        let stressed_context = DecisionContext {
            max_self_care_class: GoalPriorityClass::Medium,
            danger_class: GoalPriorityClass::Background,
        };

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
                None,
                true,
                default_switch_margin(),
                default_switch_margin(),
                &dummy_context(),
            ),
            InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::OpportunisticLoot,
            }
        );
        assert_eq!(
            evaluate_interrupt(
                &runtime(
                    GoalKind::ConsumeOwnedCommodity {
                        commodity: CommodityKind::Bread,
                    },
                    GoalPriorityClass::Medium,
                ),
                Interruptibility::FreelyInterruptible,
                &blocked_by_hunger,
                None,
                true,
                default_switch_margin(),
                default_switch_margin(),
                &stressed_context,
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
            None,
            true,
            default_switch_margin(),
            default_switch_margin(),
            &dummy_context(),
        );

        assert_eq!(decision, InterruptDecision::NoInterrupt);
    }

    #[test]
    fn higher_effective_margin_raises_interrupt_switch_threshold() {
        let current_goal = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        };
        let current_goal_key = GoalKey::from(current_goal);
        let challenger_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
        });
        let runtime = AgentDecisionRuntime {
            current_goal: Some(current_goal_key),
            current_plan: Some(PlannedPlan::new(
                current_goal_key,
                vec![crate::PlannedStep {
                    def_id: ActionDefId(1),
                    targets: vec![crate::PlanningEntityRef::Authoritative(entity(1))],
                    payload_override: None,
                    op_kind: crate::PlannerOpKind::Travel,
                    estimated_ticks: 2,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                }],
                crate::PlanTerminalKind::GoalSatisfied,
            )),
            journey_committed_goal: Some(current_goal_key),
            journey_committed_destination: Some(entity(1)),
            dirty: false,
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };
        let challengers = vec![
            ranked(current_goal, GoalPriorityClass::High, 1_000),
            ranked(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                },
                GoalPriorityClass::High,
                1_350,
            ),
        ];
        let planned_candidates = vec![(
            challenger_goal,
            Some(PlannedPlan::new(
                challenger_goal,
                vec![crate::PlannedStep {
                    def_id: ActionDefId(2),
                    targets: vec![crate::PlanningEntityRef::Authoritative(entity(2))],
                    payload_override: None,
                    op_kind: crate::PlannerOpKind::Travel,
                    estimated_ticks: 2,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                }],
                crate::PlanTerminalKind::GoalSatisfied,
            )),
        )];

        let conservative = evaluate_interrupt(
            &runtime,
            Interruptibility::FreelyInterruptible,
            &challengers,
            Some(&planned_candidates),
            true,
            default_switch_margin(),
            Permille::new(400).unwrap(),
            &dummy_context(),
        );
        let permissive = evaluate_interrupt(
            &runtime,
            Interruptibility::FreelyInterruptible,
            &challengers,
            Some(&planned_candidates),
            true,
            default_switch_margin(),
            Permille::new(300).unwrap(),
            &dummy_context(),
        );

        assert_eq!(conservative, InterruptDecision::NoInterrupt);
        assert_eq!(
            permissive,
            InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::SuperiorSameClassPlan,
            }
        );
    }

    #[test]
    fn bury_corpse_does_not_get_opportunistic_interrupt() {
        let current_goal = GoalKind::RestockCommodity {
            commodity: CommodityKind::Bread,
        };
        let challengers = vec![
            ranked(current_goal, GoalPriorityClass::Background, 100),
            ranked(
                GoalKind::BuryCorpse {
                    corpse: entity(9),
                    burial_site: entity(10),
                },
                GoalPriorityClass::Low,
                50,
            ),
        ];

        let decision = evaluate_interrupt(
            &runtime(current_goal, GoalPriorityClass::Background),
            Interruptibility::FreelyInterruptible,
            &challengers,
            None,
            true,
            default_switch_margin(),
            default_switch_margin(),
            &dummy_context(),
        );

        // BuryCorpse has Normal free_interrupt role, not Opportunistic.
        // Since it's a lower-priority goal, it cannot interrupt via HigherPriorityGoal
        // (Normal goals are blocked) or SameClassMargin (same class, insufficient margin).
        assert_eq!(decision, InterruptDecision::NoInterrupt);
    }

    #[test]
    fn heal_interrupts_via_higher_priority_but_not_via_penalty() {
        let current_goal = GoalKind::RestockCommodity {
            commodity: CommodityKind::Bread,
        };
        let heal_goal = GoalKind::TreatWounds {
            patient: entity(99),
        };

        // Heal at higher priority class can interrupt freely (Reactive role).
        let challengers_higher = vec![
            ranked(current_goal, GoalPriorityClass::Medium, 100),
            ranked(heal_goal, GoalPriorityClass::High, 900),
        ];
        let decision_free = evaluate_interrupt(
            &runtime(current_goal, GoalPriorityClass::Medium),
            Interruptibility::FreelyInterruptible,
            &challengers_higher,
            None,
            true,
            default_switch_margin(),
            default_switch_margin(),
            &dummy_context(),
        );
        assert_eq!(
            decision_free,
            InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::HigherPriorityGoal,
            }
        );

        // Heal at Critical does NOT trigger penalty interrupt (PenaltyInterruptEligibility::Never).
        let challengers_critical = vec![
            ranked(current_goal, GoalPriorityClass::Medium, 100),
            ranked(heal_goal, GoalPriorityClass::Critical, 1_000),
        ];
        let decision_penalty = evaluate_interrupt(
            &runtime(current_goal, GoalPriorityClass::Medium),
            Interruptibility::InterruptibleWithPenalty,
            &challengers_critical,
            None,
            true,
            default_switch_margin(),
            default_switch_margin(),
            &dummy_context(),
        );
        assert_eq!(decision_penalty, InterruptDecision::NoInterrupt);
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn journey_interrupt_allows_detour_without_route_margin_when_plan_is_local() {
        let committed_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let destination = entity(40);
        let detour_goal = GoalKey::from(GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Water,
        });
        let abandon_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
        });
        let runtime = AgentDecisionRuntime {
            current_goal: Some(committed_goal),
            current_plan: Some(PlannedPlan::new(
                committed_goal,
                vec![crate::PlannedStep {
                    def_id: ActionDefId(1),
                    targets: vec![crate::PlanningEntityRef::Authoritative(destination)],
                    payload_override: None,
                    op_kind: crate::PlannerOpKind::Travel,
                    estimated_ticks: 2,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                }],
                crate::PlanTerminalKind::GoalSatisfied,
            )),
            journey_committed_goal: Some(committed_goal),
            journey_committed_destination: Some(destination),
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };
        let challengers = vec![
            ranked(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                },
                GoalPriorityClass::High,
                1_150,
            ),
            ranked(
                GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Water,
                },
                GoalPriorityClass::High,
                1_120,
            ),
            ranked(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: CommodityPurpose::SelfConsume,
                },
                GoalPriorityClass::High,
                1_000,
            ),
        ];
        let planned_candidates = vec![
            (
                abandon_goal,
                Some(PlannedPlan::new(
                    abandon_goal,
                    vec![crate::PlannedStep {
                        def_id: ActionDefId(2),
                        targets: vec![crate::PlanningEntityRef::Authoritative(entity(99))],
                        payload_override: None,
                        op_kind: crate::PlannerOpKind::Travel,
                        estimated_ticks: 1,
                        is_materialization_barrier: false,
                        expected_materializations: Vec::new(),
                    }],
                    crate::PlanTerminalKind::GoalSatisfied,
                )),
            ),
            (
                detour_goal,
                Some(PlannedPlan::new(
                    detour_goal,
                    vec![crate::PlannedStep {
                        def_id: ActionDefId(3),
                        targets: vec![crate::PlanningEntityRef::Authoritative(entity(3))],
                        payload_override: None,
                        op_kind: crate::PlannerOpKind::Consume,
                        estimated_ticks: 1,
                        is_materialization_barrier: false,
                        expected_materializations: Vec::new(),
                    }],
                    crate::PlanTerminalKind::GoalSatisfied,
                )),
            ),
        ];

        let decision = evaluate_interrupt(
            &runtime,
            Interruptibility::FreelyInterruptible,
            &challengers,
            Some(&planned_candidates),
            true,
            default_switch_margin(),
            route_switch_margin(),
            &dummy_context(),
        );

        assert_eq!(
            decision,
            InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::SuperiorSameClassPlan,
            }
        );
    }
}
