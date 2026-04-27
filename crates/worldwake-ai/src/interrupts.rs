use crate::{
    AgendaEntry, AgentDecisionRuntime, DecisionContext, FramePlanRelation, GoalDispatchKey,
    GoalKey, GoalPriorityClass, classify_frame_plan_relation,
    frame_switch_policy::compare_relation_aware_goal_switch,
    goal_policy::{FreeInterruptRole, PenaltyInterruptEligibility},
    goal_switching::{GoalSwitchKind, compare_goal_switch},
    plan_selection::SelectionCandidatePlan,
    ranking::OrderedRanked,
};
use std::collections::BTreeMap;
use worldwake_core::{IntentionFrame, Permille};
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
    active_goal: Option<GoalKey>,
    jc: Option<&IntentionFrame>,
    current_action_interruptibility: Interruptibility,
    ranked_candidates: &OrderedRanked<'_>,
    planned_candidates: Option<&[SelectionCandidatePlan]>,
    plan_valid: bool,
    default_switch_margin: Permille,
    frame_switch_margin: Permille,
    decision_context: &DecisionContext,
) -> InterruptDecision {
    if current_action_interruptibility == Interruptibility::NonInterruptible {
        return InterruptDecision::NoInterrupt;
    }

    let effective_active_goal = effective_active_goal(runtime, active_goal, jc);

    if !plan_valid {
        if ranked_candidates
            .first()
            .is_some_and(|candidate| Some(candidate.offer.key) == effective_active_goal)
        {
            return InterruptDecision::NoInterrupt;
        }
        return InterruptDecision::InterruptForReplan {
            trigger: InterruptTrigger::PlanInvalid,
        };
    }

    let Some(challenger) = best_challenger(effective_active_goal, ranked_candidates) else {
        return InterruptDecision::NoInterrupt;
    };

    match current_action_interruptibility {
        Interruptibility::NonInterruptible => InterruptDecision::NoInterrupt,
        Interruptibility::InterruptibleWithPenalty => {
            if penalty_interrupt_trigger(&challenger.offer.key.kind)
                == Some(InterruptTrigger::CriticalSurvival)
                && effective_active_goal.is_some_and(|goal| {
                    penalty_interrupt_trigger(&goal.kind)
                        == Some(InterruptTrigger::CriticalSurvival)
                })
            {
                InterruptDecision::NoInterrupt
            } else {
                interrupt_with_penalty(challenger)
            }
        }
        Interruptibility::FreelyInterruptible => interrupt_freely(
            effective_active_goal,
            runtime,
            jc,
            challenger,
            ranked_candidates,
            planned_candidates,
            default_switch_margin,
            frame_switch_margin,
            *decision_context,
        ),
    }
}

fn effective_active_goal(
    runtime: &AgentDecisionRuntime,
    active_goal: Option<GoalKey>,
    jc: Option<&IntentionFrame>,
) -> Option<GoalKey> {
    active_goal
        .or_else(|| runtime.current_plan.as_ref().map(|plan| plan.goal))
        .or_else(|| jc.map(|frame| frame.goal))
}

fn interrupt_with_penalty(challenger: &AgendaEntry) -> InterruptDecision {
    if challenger.priority_class != GoalPriorityClass::Critical {
        return InterruptDecision::NoInterrupt;
    }
    let policy = GoalDispatchKey::from_goal_kind(&challenger.offer.key.kind)
        .declaration()
        .family_policy;
    match policy.penalty_interrupt {
        PenaltyInterruptEligibility::WhenCritical { trigger } => {
            InterruptDecision::InterruptForReplan { trigger }
        }
        PenaltyInterruptEligibility::Never => InterruptDecision::NoInterrupt,
    }
}

fn penalty_interrupt_trigger(kind: &worldwake_core::GoalKind) -> Option<InterruptTrigger> {
    match GoalDispatchKey::from_goal_kind(kind)
        .declaration()
        .family_policy
        .penalty_interrupt
    {
        PenaltyInterruptEligibility::WhenCritical { trigger } => Some(trigger),
        PenaltyInterruptEligibility::Never => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn interrupt_freely(
    active_goal: Option<GoalKey>,
    runtime: &AgentDecisionRuntime,
    jc: Option<&IntentionFrame>,
    challenger: &AgendaEntry,
    ranked_candidates: &OrderedRanked<'_>,
    planned_candidates: Option<&[SelectionCandidatePlan]>,
    default_switch_margin: Permille,
    frame_switch_margin: Permille,
    decision_context: DecisionContext,
) -> InterruptDecision {
    let policy = GoalDispatchKey::from_goal_kind(&challenger.offer.key.kind)
        .declaration()
        .family_policy;
    if policy.free_interrupt == FreeInterruptRole::Opportunistic {
        return if decision_context.is_stressed_at_or_above(GoalPriorityClass::Medium) {
            InterruptDecision::NoInterrupt
        } else {
            InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::OpportunisticLoot,
            }
        };
    }

    let Some((current_class, current_motive)) =
        current_priority(active_goal, runtime, ranked_candidates)
    else {
        return InterruptDecision::NoInterrupt;
    };

    if let Some((challenger, switch_kind)) = relation_aware_interrupt_candidate(
        active_goal,
        jc,
        ranked_candidates,
        planned_candidates,
        current_class,
        current_motive,
        default_switch_margin,
        frame_switch_margin,
    ) {
        return match switch_kind {
            GoalSwitchKind::HigherPriorityGoal
                if GoalDispatchKey::from_goal_kind(&challenger.offer.key.kind)
                    .declaration()
                    .family_policy
                    .free_interrupt
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

#[allow(clippy::too_many_arguments)]
fn relation_aware_interrupt_candidate<'a>(
    active_goal: Option<GoalKey>,
    jc: Option<&IntentionFrame>,
    ranked_candidates: &'a OrderedRanked<'_>,
    planned_candidates: Option<&'a [SelectionCandidatePlan]>,
    current_class: GoalPriorityClass,
    current_motive: Option<u32>,
    default_switch_margin: Permille,
    frame_switch_margin: Permille,
) -> Option<(&'a AgendaEntry, GoalSwitchKind)> {
    let planned_candidates = planned_candidates?;
    let planned_by_goal = planned_candidates
        .iter()
        .filter_map(|selection_plan| {
            selection_plan
                .found_plan
                .as_ref()
                .map(|plan| (selection_plan.searched_opportunity.goal_key, plan))
        })
        .collect::<BTreeMap<_, _>>();

    for challenger in ranked_candidates {
        if Some(challenger.offer.key) == active_goal {
            continue;
        }

        let Some(plan) = planned_by_goal.get(&challenger.offer.key) else {
            continue;
        };
        let relation = classify_frame_plan_relation(jc, plan);
        let Some(switch_kind) = compare_relation_aware_goal_switch(
            current_class,
            current_motive,
            challenger.priority_class,
            challenger.motive_score,
            relation,
            default_switch_margin,
            frame_switch_margin,
        ) else {
            continue;
        };

        if relation == FramePlanRelation::RefreshesFrame
            && matches!(
                switch_kind,
                GoalSwitchKind::HigherPriorityGoal | GoalSwitchKind::SameClassMargin
            )
        {
            return Some((challenger, switch_kind));
        }

        if relation != FramePlanRelation::NoFrame {
            return Some((challenger, switch_kind));
        }
    }

    None
}

fn best_challenger<'a>(
    current_goal: Option<GoalKey>,
    ranked_candidates: &'a OrderedRanked<'a>,
) -> Option<&'a AgendaEntry> {
    ranked_candidates
        .iter()
        .find(|candidate| Some(candidate.offer.key) != current_goal)
}

fn current_priority(
    active_goal: Option<GoalKey>,
    runtime: &AgentDecisionRuntime,
    ranked_candidates: &OrderedRanked<'_>,
) -> Option<(GoalPriorityClass, Option<u32>)> {
    if let Some(current_goal) = active_goal
        && let Some(current) = ranked_candidates
            .iter()
            .find(|candidate| candidate.offer.key == current_goal)
    {
        return Some((current.priority_class, Some(current.motive_score)));
    }

    runtime.last_priority_class.map(|class| (class, None))
}

#[cfg(test)]
mod tests {
    use super::{InterruptDecision, InterruptTrigger, evaluate_interrupt};
    use crate::plan_selection::SelectionCandidatePlan;
    use crate::{
        AgendaEntry, AgentDecisionRuntime, CommodityPurpose, DecisionContext, GoalKey, GoalOffer,
        GoalPriorityClass, PlannedPlan,
    };
    use std::collections::BTreeSet;
    use worldwake_core::{
        AcquisitionQuantity, ActionDefId, CommodityKind, EntityId, GoalKind, Permille, Tick,
    };
    use worldwake_sim::Interruptibility;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn ranked(kind: GoalKind, priority_class: GoalPriorityClass, motive_score: u32) -> AgendaEntry {
        AgendaEntry {
            key: worldwake_core::OpportunityKey {
                goal_key: GoalKey::from(kind),
                anchor: worldwake_core::OpportunityAnchor::None,
            },
            offer: GoalOffer {
                anchor: worldwake_core::OpportunityAnchor::None,
                key: GoalKey::from(kind),
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                acquisition_quantity: None,
            },
            priority_class,
            motive_score,
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            feasibility: crate::feasibility::FeasibilityHint::Uncertain,
            phase: crate::AgendaPhase::Pending,
            origin: crate::AgendaOrigin::NeedDrive,
            introduced_tick: Tick(0),
            last_reconsidered_tick: Tick(0),
            revival_trigger: None,
            kill_condition: crate::KillCondition::External,
        }
    }

    fn runtime(
        _current_goal: GoalKind,
        last_priority_class: GoalPriorityClass,
    ) -> AgentDecisionRuntime {
        AgentDecisionRuntime {
            current_plan: None,
            dirty: crate::DirtySet::default(),
            last_priority_class: Some(last_priority_class),
            ..AgentDecisionRuntime::default()
        }
    }

    fn opportunity(goal: GoalKey) -> worldwake_core::OpportunityKey {
        worldwake_core::OpportunityKey {
            goal_key: goal,
            anchor: worldwake_core::OpportunityAnchor::None,
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

    fn selection_plan(goal: GoalKey, plan: Option<PlannedPlan>) -> SelectionCandidatePlan {
        SelectionCandidatePlan {
            searched_opportunity: opportunity(goal),
            perceived_cost: plan.as_ref().map(|plan| plan.total_estimated_ticks),
            found_plan: plan,
        }
    }

    fn ordered(ranked: &[AgendaEntry]) -> crate::ranking::OrderedRanked<'_> {
        crate::ranking::OrderedRanked::from_sorted_for_test(ranked)
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
            Some(GoalKey::from(current_goal)),
            None,
            Interruptibility::NonInterruptible,
            &ordered(&challengers),
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
            Some(GoalKey::from(current_goal)),
            None,
            Interruptibility::InterruptibleWithPenalty,
            &ordered(&challengers),
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
            Some(GoalKey::from(current_goal)),
            None,
            Interruptibility::InterruptibleWithPenalty,
            &ordered(&challengers),
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
            Some(GoalKey::from(current_goal)),
            None,
            Interruptibility::InterruptibleWithPenalty,
            &ordered(&[ranked(
                GoalKind::ReduceDanger,
                GoalPriorityClass::Critical,
                1_000,
            )]),
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
    fn interruptible_with_penalty_keeps_running_same_critical_goal_even_if_plan_marked_invalid() {
        let current_goal = GoalKind::Relieve;
        let challengers = vec![ranked(current_goal, GoalPriorityClass::Critical, 1_000)];

        let decision = evaluate_interrupt(
            &runtime(current_goal, GoalPriorityClass::Critical),
            Some(GoalKey::from(current_goal)),
            None,
            Interruptibility::InterruptibleWithPenalty,
            &ordered(&challengers),
            None,
            false,
            default_switch_margin(),
            default_switch_margin(),
            &dummy_context(),
        );

        assert_eq!(decision, InterruptDecision::NoInterrupt);
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
            Some(GoalKey::from(current_goal)),
            None,
            Interruptibility::InterruptibleWithPenalty,
            &ordered(&challengers),
            None,
            true,
            default_switch_margin(),
            default_switch_margin(),
            &dummy_context(),
        );

        assert_eq!(decision, InterruptDecision::NoInterrupt);
    }

    #[test]
    fn interruptible_with_penalty_does_not_rotate_between_critical_self_care_goals() {
        let current_goal = GoalKind::Relieve;
        let challengers = vec![
            ranked(current_goal, GoalPriorityClass::Critical, 1_000),
            ranked(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::Critical,
                1_100,
            ),
        ];

        let decision = evaluate_interrupt(
            &runtime(current_goal, GoalPriorityClass::Critical),
            Some(GoalKey::from(current_goal)),
            None,
            Interruptibility::InterruptibleWithPenalty,
            &ordered(&challengers),
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
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::High,
                900,
            ),
        ];

        let decision = evaluate_interrupt(
            &runtime(current_goal, GoalPriorityClass::Medium),
            Some(GoalKey::from(current_goal)),
            None,
            Interruptibility::FreelyInterruptible,
            &ordered(&challengers),
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
        use worldwake_core::{FrameState, IntentionDomain, IntentionFrame, Tick};
        let current_goal = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        };
        let jc = Some(IntentionFrame {
            goal: GoalKey::from(current_goal),
            domain: IntentionDomain::Travel {
                destination: entity(1),
            },
            assumptions: Vec::new(),
            state: FrameState::Active,
            established_at: Tick(1),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 10,
        });
        let active_goal = Some(GoalKey::from(current_goal));
        let runtime = AgentDecisionRuntime {
            current_plan: Some(PlannedPlan::new(
                opportunity(GoalKey::from(current_goal)),
                GoalKey::from(current_goal),
                vec![crate::PlannedStep {
                    def_id: ActionDefId(1),
                    targets: vec![crate::PlanningEntityRef::Authoritative(entity(1))],
                    target_place: None,
                    payload_override: None,
                    op_kind: crate::PlannerOpKind::Travel,
                    estimated_ticks: 2,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                    guard: None,
                    expectations: Vec::new(),
                }],
                crate::PlanTerminalKind::GoalSatisfied,
            )),
            dirty: crate::DirtySet::default(),
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };
        let below_margin = vec![
            ranked(current_goal, GoalPriorityClass::High, 1_000),
            ranked(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
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
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::High,
                1_100,
            ),
        ];

        assert_eq!(
            evaluate_interrupt(
                &runtime,
                active_goal,
                jc.as_ref(),
                Interruptibility::FreelyInterruptible,
                &ordered(&below_margin),
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
                active_goal,
                jc.as_ref(),
                Interruptibility::FreelyInterruptible,
                &ordered(&at_margin),
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
                Some(GoalKey::from(GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread,
                })),
                None,
                Interruptibility::FreelyInterruptible,
                &ordered(&no_pressure),
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
                Some(GoalKey::from(GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Bread,
                })),
                None,
                Interruptibility::FreelyInterruptible,
                &ordered(&blocked_by_hunger),
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
            Some(GoalKey::from(current_goal)),
            None,
            Interruptibility::FreelyInterruptible,
            &ordered(&challengers),
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
        use worldwake_core::{FrameState, IntentionDomain, IntentionFrame, Tick};
        let current_goal = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        };
        let current_goal_key = GoalKey::from(current_goal);
        let challenger_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let jc = Some(IntentionFrame {
            goal: current_goal_key,
            domain: IntentionDomain::Travel {
                destination: entity(1),
            },
            assumptions: Vec::new(),
            state: FrameState::Active,
            established_at: Tick(1),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 10,
        });
        let runtime = AgentDecisionRuntime {
            current_plan: Some(PlannedPlan::new(
                opportunity(current_goal_key),
                current_goal_key,
                vec![crate::PlannedStep {
                    def_id: ActionDefId(1),
                    targets: vec![crate::PlanningEntityRef::Authoritative(entity(1))],
                    target_place: None,
                    payload_override: None,
                    op_kind: crate::PlannerOpKind::Travel,
                    estimated_ticks: 2,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                    guard: None,
                    expectations: Vec::new(),
                }],
                crate::PlanTerminalKind::GoalSatisfied,
            )),
            dirty: crate::DirtySet::default(),
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };
        let challengers = vec![
            ranked(current_goal, GoalPriorityClass::High, 1_000),
            ranked(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::High,
                1_350,
            ),
        ];
        let planned_candidates = vec![selection_plan(
            challenger_goal,
            Some(PlannedPlan::new(
                opportunity(challenger_goal),
                challenger_goal,
                vec![crate::PlannedStep {
                    def_id: ActionDefId(2),
                    targets: vec![crate::PlanningEntityRef::Authoritative(entity(2))],
                    target_place: None,
                    payload_override: None,
                    op_kind: crate::PlannerOpKind::Travel,
                    estimated_ticks: 2,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                    guard: None,
                    expectations: Vec::new(),
                }],
                crate::PlanTerminalKind::GoalSatisfied,
            )),
        )];

        let conservative = evaluate_interrupt(
            &runtime,
            Some(current_goal_key),
            jc.as_ref(),
            Interruptibility::FreelyInterruptible,
            &ordered(&challengers),
            Some(&planned_candidates),
            true,
            default_switch_margin(),
            Permille::new(400).unwrap(),
            &dummy_context(),
        );
        let permissive = evaluate_interrupt(
            &runtime,
            Some(current_goal_key),
            jc.as_ref(),
            Interruptibility::FreelyInterruptible,
            &ordered(&challengers),
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
    fn penalty_interrupt_ignores_current_plan_goal_when_active_goal_component_is_missing() {
        let current_goal = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        };
        let current_goal_key = GoalKey::from(current_goal);
        let runtime = AgentDecisionRuntime {
            current_plan: Some(PlannedPlan::new(
                opportunity(current_goal_key),
                current_goal_key,
                vec![crate::PlannedStep {
                    def_id: ActionDefId(1),
                    targets: vec![crate::PlanningEntityRef::Authoritative(entity(1))],
                    target_place: None,
                    payload_override: None,
                    op_kind: crate::PlannerOpKind::Harvest,
                    estimated_ticks: 6,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                    guard: None,
                    expectations: Vec::new(),
                }],
                crate::PlanTerminalKind::GoalSatisfied,
            )),
            dirty: crate::DirtySet::default(),
            last_priority_class: Some(GoalPriorityClass::Critical),
            ..AgentDecisionRuntime::default()
        };
        let challengers = vec![
            ranked(current_goal, GoalPriorityClass::Critical, 1_000),
            ranked(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Apple,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::Low,
                10,
            ),
        ];

        assert_eq!(
            evaluate_interrupt(
                &runtime,
                None,
                None,
                Interruptibility::InterruptibleWithPenalty,
                &ordered(&challengers),
                None,
                true,
                default_switch_margin(),
                default_switch_margin(),
                &dummy_context(),
            ),
            InterruptDecision::NoInterrupt
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
            Some(GoalKey::from(current_goal)),
            None,
            Interruptibility::FreelyInterruptible,
            &ordered(&challengers),
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
            Some(GoalKey::from(current_goal)),
            None,
            Interruptibility::FreelyInterruptible,
            &ordered(&challengers_higher),
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
            Some(GoalKey::from(current_goal)),
            None,
            Interruptibility::InterruptibleWithPenalty,
            &ordered(&challengers_critical),
            None,
            true,
            default_switch_margin(),
            default_switch_margin(),
            &dummy_context(),
        );
        assert_eq!(decision_penalty, InterruptDecision::NoInterrupt);
    }

    #[test]
    fn reduce_danger_interrupts_raid_target_but_raid_does_not_interrupt_reduce_danger() {
        let raid_goal = GoalKind::RaidTarget { target: entity(77) };
        let danger_goal = GoalKind::ReduceDanger;

        let raid_interrupted = evaluate_interrupt(
            &runtime(raid_goal, GoalPriorityClass::Medium),
            Some(GoalKey::from(raid_goal)),
            None,
            Interruptibility::FreelyInterruptible,
            &ordered(&[
                ranked(raid_goal, GoalPriorityClass::Medium, 100),
                ranked(danger_goal, GoalPriorityClass::High, 900),
            ]),
            None,
            true,
            default_switch_margin(),
            default_switch_margin(),
            &dummy_context(),
        );
        assert_eq!(
            raid_interrupted,
            InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::HigherPriorityGoal,
            }
        );

        let danger_not_interrupted = evaluate_interrupt(
            &runtime(danger_goal, GoalPriorityClass::High),
            Some(GoalKey::from(danger_goal)),
            None,
            Interruptibility::FreelyInterruptible,
            &ordered(&[
                ranked(danger_goal, GoalPriorityClass::High, 900),
                ranked(raid_goal, GoalPriorityClass::Medium, 1000),
            ]),
            None,
            true,
            default_switch_margin(),
            default_switch_margin(),
            &dummy_context(),
        );
        assert_eq!(danger_not_interrupted, InterruptDecision::NoInterrupt);
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn frame_interrupt_allows_detour_without_route_margin_when_plan_is_local() {
        use worldwake_core::{FrameState, IntentionDomain, IntentionFrame, Tick};
        let committed_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let destination = entity(40);
        let detour_goal = GoalKey::from(GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Water,
        });
        let abandon_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let jc = Some(IntentionFrame {
            goal: committed_goal,
            domain: IntentionDomain::Travel { destination },
            assumptions: Vec::new(),
            state: FrameState::Active,
            established_at: Tick(1),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 10,
        });
        let runtime = AgentDecisionRuntime {
            current_plan: Some(PlannedPlan::new(
                opportunity(committed_goal),
                committed_goal,
                vec![crate::PlannedStep {
                    def_id: ActionDefId(1),
                    targets: vec![crate::PlanningEntityRef::Authoritative(destination)],
                    target_place: None,
                    payload_override: None,
                    op_kind: crate::PlannerOpKind::Travel,
                    estimated_ticks: 2,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                    guard: None,
                    expectations: Vec::new(),
                }],
                crate::PlanTerminalKind::GoalSatisfied,
            )),
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };
        let challengers = vec![
            ranked(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
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
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::High,
                1_000,
            ),
        ];
        let planned_candidates = vec![
            selection_plan(
                abandon_goal,
                Some(PlannedPlan::new(
                    opportunity(abandon_goal),
                    abandon_goal,
                    vec![crate::PlannedStep {
                        def_id: ActionDefId(2),
                        targets: vec![crate::PlanningEntityRef::Authoritative(entity(99))],
                        target_place: None,
                        payload_override: None,
                        op_kind: crate::PlannerOpKind::Travel,
                        estimated_ticks: 1,
                        is_materialization_barrier: false,
                        expected_materializations: Vec::new(),
                        guard: None,
                        expectations: Vec::new(),
                    }],
                    crate::PlanTerminalKind::GoalSatisfied,
                )),
            ),
            selection_plan(
                detour_goal,
                Some(PlannedPlan::new(
                    opportunity(detour_goal),
                    detour_goal,
                    vec![crate::PlannedStep {
                        def_id: ActionDefId(3),
                        targets: vec![crate::PlanningEntityRef::Authoritative(entity(3))],
                        target_place: None,
                        payload_override: None,
                        op_kind: crate::PlannerOpKind::Consume,
                        estimated_ticks: 1,
                        is_materialization_barrier: false,
                        expected_materializations: Vec::new(),
                        guard: None,
                        expectations: Vec::new(),
                    }],
                    crate::PlanTerminalKind::GoalSatisfied,
                )),
            ),
        ];

        let decision = evaluate_interrupt(
            &runtime,
            Some(committed_goal),
            jc.as_ref(),
            Interruptibility::FreelyInterruptible,
            &ordered(&challengers),
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
