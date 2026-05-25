use crate::{
    AgentDecisionRuntime, FramePlanRelation, GoalKey, GoalPriorityClass, PlanValue, PlannedPlan,
    classify_frame_plan_relation, frame_switch_policy::compare_relation_aware_goal_switch,
    goal_switching::GoalSwitchKind, ranking::OrderedRanked, side_benefit::build_plan_value,
};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use worldwake_core::{IntentionFrame, OpportunityKey, Permille};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionCandidatePlan {
    pub searched_opportunity: OpportunityKey,
    pub found_plan: Option<PlannedPlan>,
    pub perceived_cost: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectionPolicy {
    pub side_benefit_weight: Permille,
    pub default_switch_margin: Permille,
    pub frame_switch_margin: Permille,
}

pub fn select_best_plan(
    candidates: &OrderedRanked<'_>,
    plans: &[SelectionCandidatePlan],
    active_goal: Option<GoalKey>,
    current: &AgentDecisionRuntime,
    jc: Option<&IntentionFrame>,
    policy: SelectionPolicy,
) -> Option<PlannedPlan> {
    let candidate_scores = candidates
        .iter()
        .map(|candidate| {
            (
                OpportunityKey {
                    goal_key: candidate.offer.key,
                    anchor: candidate.offer.anchor,
                },
                (candidate.priority_class, candidate.motive_score),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut available = plans
        .iter()
        .filter_map(|selection_plan| {
            let plan = selection_plan.found_plan.as_ref()?;
            let (priority_class, motive_score) = candidate_scores
                .get(&selection_plan.searched_opportunity)
                .copied()?;
            debug_assert_eq!(plan.opportunity, selection_plan.searched_opportunity);
            Some((
                priority_class,
                motive_score,
                selection_plan
                    .perceived_cost
                    .unwrap_or(plan.total_estimated_ticks),
                build_plan_value(
                    plan.clone(),
                    priority_class,
                    motive_score,
                    candidates,
                    policy.side_benefit_weight,
                ),
            ))
        })
        .collect::<Vec<_>>();
    available.sort_by(compare_ranked_plans);
    let best_plan = available.first()?.3.plan.clone();
    let has_current_goal_plan = active_goal.is_some_and(|goal| {
        plans
            .iter()
            .any(|plan| plan.searched_opportunity.goal_key == goal && plan.found_plan.is_some())
    });

    let Some(current_plan) = current.current_plan.clone() else {
        return Some(best_plan);
    };
    if active_goal != Some(current_plan.goal) {
        return Some(best_plan);
    }
    let Some((current_class, current_motive)) =
        candidate_scores.get(&current_plan.opportunity).copied()
    else {
        return Some(best_plan);
    };

    for (challenger_class, challenger_motive, _challenger_cost, challenger_value) in available {
        let challenger_plan = challenger_value.plan.clone();
        let relation = classify_frame_plan_relation(jc, &challenger_plan);
        if relation == FramePlanRelation::RefreshesFrame
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
                policy.default_switch_margin,
                policy.frame_switch_margin,
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
    left: &(GoalPriorityClass, u32, u32, PlanValue),
    right: &(GoalPriorityClass, u32, u32, PlanValue),
) -> Ordering {
    right
        .0
        .cmp(&left.0)
        .then_with(|| right.1.cmp(&left.1))
        .then_with(|| right.3.total_value.cmp(&left.3.total_value))
        .then_with(|| left.2.cmp(&right.2))
        .then_with(|| {
            left.3
                .plan
                .total_estimated_ticks
                .cmp(&right.3.plan.total_estimated_ticks)
        })
        .then_with(|| left.3.plan.steps.cmp(&right.3.plan.steps))
        .then_with(|| left.3.plan.goal.cmp(&right.3.plan.goal))
}

#[cfg(test)]
mod tests {
    use super::{SelectionCandidatePlan, SelectionPolicy, select_best_plan};
    use crate::{
        AgendaEntry, AgentDecisionRuntime, CommodityPurpose, GoalKey, GoalOffer, GoalPriorityClass,
        PlanTerminalKind, PlannedPlan, PlannedStep, PlannerOpKind, PlanningEntityRef,
    };
    use std::collections::BTreeSet;
    use worldwake_core::ActionDefId;
    use worldwake_core::{
        AcquisitionQuantity, CommodityKind, EntityId, FrameState, IntentionDomain, IntentionFrame,
        OpportunityAnchor, Permille, Tick,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn ranked(
        goal: worldwake_core::GoalKind,
        class: GoalPriorityClass,
        motive: u32,
    ) -> AgendaEntry {
        AgendaEntry {
            key: worldwake_core::OpportunityKey {
                goal_key: GoalKey::from(goal),
                anchor: worldwake_core::OpportunityAnchor::None,
            },
            offer: GoalOffer {
                anchor: worldwake_core::OpportunityAnchor::None,
                key: GoalKey::from(goal),
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                motive_sources: Vec::new(),
                acquisition_quantity: None,
            },
            priority_class: class,
            motive_score: motive,
            motive_source_contributions: Vec::new(),
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            learned_opportunity_bonus: None,
            repair_memory_bonus: None,
            source_composite: None,
            feasibility: crate::feasibility::FeasibilityHint::Uncertain,
            partial_plan_segment: None,
            phase: crate::AgendaPhase::Pending,
            origin: crate::AgendaOrigin::NeedDrive,
            introduced_tick: Tick(0),
            last_reconsidered_tick: Tick(0),
            revival_trigger: None,
            kill_condition: crate::KillCondition::External,
        }
    }

    fn opportunity(goal: GoalKey) -> worldwake_core::OpportunityKey {
        worldwake_core::OpportunityKey {
            goal_key: goal,
            anchor: worldwake_core::OpportunityAnchor::None,
        }
    }

    fn plan(goal: GoalKey, def_id: u32, ticks: u32) -> PlannedPlan {
        PlannedPlan::new(
            opportunity(goal),
            goal,
            vec![PlannedStep {
                def_id: ActionDefId(def_id),
                targets: vec![PlanningEntityRef::Authoritative(entity(def_id))],
                target_place: None,
                payload_override: None,
                op_kind: PlannerOpKind::Travel,
                estimated_ticks: ticks,
                is_materialization_barrier: false,
                expected_materializations: Vec::new(),
                guard: None,
                expectations: Vec::new(),
            }],
            PlanTerminalKind::GoalSatisfied,
        )
    }

    fn plan_at(
        goal: GoalKey,
        anchor: worldwake_core::OpportunityAnchor,
        def_id: u32,
        ticks: u32,
    ) -> PlannedPlan {
        PlannedPlan::new(
            worldwake_core::OpportunityKey {
                goal_key: goal,
                anchor,
            },
            goal,
            vec![PlannedStep {
                def_id: ActionDefId(def_id),
                targets: vec![PlanningEntityRef::Authoritative(entity(def_id))],
                target_place: None,
                payload_override: None,
                op_kind: PlannerOpKind::Travel,
                estimated_ticks: ticks,
                is_materialization_barrier: false,
                expected_materializations: Vec::new(),
                guard: None,
                expectations: Vec::new(),
            }],
            PlanTerminalKind::GoalSatisfied,
        )
    }

    fn default_switch_margin() -> Permille {
        Permille::new(100).unwrap()
    }

    fn ordered(ranked: &[AgendaEntry]) -> crate::ranking::OrderedRanked<'_> {
        crate::ranking::OrderedRanked::from_sorted_for_test(ranked)
    }

    fn route_switch_margin() -> Permille {
        Permille::new(300).unwrap()
    }

    fn side_benefit_weight() -> Permille {
        Permille::new(100).unwrap()
    }

    fn selection_policy() -> SelectionPolicy {
        SelectionPolicy {
            side_benefit_weight: side_benefit_weight(),
            default_switch_margin: default_switch_margin(),
            frame_switch_margin: default_switch_margin(),
        }
    }

    fn selection_plan(goal: GoalKey, plan: Option<PlannedPlan>) -> SelectionCandidatePlan {
        SelectionCandidatePlan {
            searched_opportunity: opportunity(goal),
            perceived_cost: plan.as_ref().map(|plan| plan.total_estimated_ticks),
            found_plan: plan,
        }
    }

    fn selection_plan_at(
        goal: GoalKey,
        anchor: worldwake_core::OpportunityAnchor,
        plan: Option<PlannedPlan>,
    ) -> SelectionCandidatePlan {
        SelectionCandidatePlan {
            searched_opportunity: worldwake_core::OpportunityKey {
                goal_key: goal,
                anchor,
            },
            perceived_cost: plan.as_ref().map(|plan| plan.total_estimated_ticks),
            found_plan: plan,
        }
    }

    fn selection_plan_at_with_perceived_cost(
        goal: GoalKey,
        anchor: worldwake_core::OpportunityAnchor,
        plan: Option<PlannedPlan>,
        perceived_cost: Option<u32>,
    ) -> SelectionCandidatePlan {
        SelectionCandidatePlan {
            searched_opportunity: worldwake_core::OpportunityKey {
                goal_key: goal,
                anchor,
            },
            perceived_cost,
            found_plan: plan,
        }
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
            selection_plan(sleep_goal, Some(plan(sleep_goal, 1, 1))),
            selection_plan(eat_goal, Some(plan(eat_goal, 2, 9))),
        ];

        let selected = select_best_plan(
            &ordered(&candidates),
            &plans,
            None,
            &AgentDecisionRuntime::default(),
            None,
            selection_policy(),
        )
        .unwrap();

        assert_eq!(selected.goal, eat_goal);
    }

    #[test]
    fn same_goal_sibling_opportunity_selection_uses_opportunity_scoped_scores() {
        let goal = GoalKey::from(worldwake_core::GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        });
        let local_anchor = OpportunityAnchor::Place(entity(30));
        let remote_anchor = OpportunityAnchor::Place(entity(31));
        let candidates = vec![
            AgendaEntry {
                offer: GoalOffer {
                    anchor: local_anchor,
                    key: goal,
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                    obligation_source: None,
                    commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                    required_information_gaps: Vec::new(),
                    invalidators: Vec::new(),
                    learned_expectation_refs: Vec::new(),
                    motive_sources: Vec::new(),
                    acquisition_quantity: None,
                },
                priority_class: GoalPriorityClass::High,
                motive_score: 450,
                motive_source_contributions: Vec::new(),
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                learned_opportunity_bonus: None,
                repair_memory_bonus: None,
                source_composite: None,
                feasibility: crate::feasibility::FeasibilityHint::Uncertain,
                partial_plan_segment: None,
                key: worldwake_core::OpportunityKey {
                    goal_key: goal,
                    anchor: local_anchor,
                },

                phase: crate::AgendaPhase::Pending,
                origin: crate::AgendaOrigin::NeedDrive,
                introduced_tick: Tick(0),
                last_reconsidered_tick: Tick(0),
                revival_trigger: None,
                kill_condition: crate::KillCondition::External,
            },
            AgendaEntry {
                offer: GoalOffer {
                    anchor: remote_anchor,
                    key: goal,
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                    obligation_source: None,
                    commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                    required_information_gaps: Vec::new(),
                    invalidators: Vec::new(),
                    learned_expectation_refs: Vec::new(),
                    motive_sources: Vec::new(),
                    acquisition_quantity: None,
                },
                priority_class: GoalPriorityClass::High,
                motive_score: 900,
                motive_source_contributions: Vec::new(),
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                learned_opportunity_bonus: None,
                repair_memory_bonus: None,
                source_composite: None,
                feasibility: crate::feasibility::FeasibilityHint::Uncertain,
                partial_plan_segment: None,
                key: worldwake_core::OpportunityKey {
                    goal_key: goal,
                    anchor: remote_anchor,
                },

                phase: crate::AgendaPhase::Pending,
                origin: crate::AgendaOrigin::NeedDrive,
                introduced_tick: Tick(0),
                last_reconsidered_tick: Tick(0),
                revival_trigger: None,
                kill_condition: crate::KillCondition::External,
            },
        ];
        let plans = vec![
            selection_plan_at(goal, local_anchor, Some(plan_at(goal, local_anchor, 1, 1))),
            selection_plan_at(
                goal,
                remote_anchor,
                Some(plan_at(goal, remote_anchor, 2, 2)),
            ),
        ];

        let selected = select_best_plan(
            &ordered(&candidates),
            &plans,
            None,
            &AgentDecisionRuntime::default(),
            None,
            selection_policy(),
        )
        .unwrap();

        assert_eq!(selected.opportunity.anchor, remote_anchor);
    }

    #[test]
    fn same_goal_sibling_selection_prefers_lower_perceived_cost_over_shorter_raw_duration() {
        let goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let dangerous_anchor = OpportunityAnchor::Place(entity(40));
        let safe_anchor = OpportunityAnchor::Place(entity(41));
        let candidates = vec![
            AgendaEntry {
                offer: GoalOffer {
                    anchor: dangerous_anchor,
                    key: goal,
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                    obligation_source: None,
                    commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                    required_information_gaps: Vec::new(),
                    invalidators: Vec::new(),
                    learned_expectation_refs: Vec::new(),
                    motive_sources: Vec::new(),
                    acquisition_quantity: None,
                },
                priority_class: GoalPriorityClass::Critical,
                motive_score: 900,
                motive_source_contributions: Vec::new(),
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                learned_opportunity_bonus: None,
                repair_memory_bonus: None,
                source_composite: None,
                feasibility: crate::feasibility::FeasibilityHint::Uncertain,
                partial_plan_segment: None,
                key: worldwake_core::OpportunityKey {
                    goal_key: goal,
                    anchor: dangerous_anchor,
                },

                phase: crate::AgendaPhase::Pending,
                origin: crate::AgendaOrigin::NeedDrive,
                introduced_tick: Tick(0),
                last_reconsidered_tick: Tick(0),
                revival_trigger: None,
                kill_condition: crate::KillCondition::External,
            },
            AgendaEntry {
                offer: GoalOffer {
                    anchor: safe_anchor,
                    key: goal,
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                    obligation_source: None,
                    commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                    required_information_gaps: Vec::new(),
                    invalidators: Vec::new(),
                    learned_expectation_refs: Vec::new(),
                    motive_sources: Vec::new(),
                    acquisition_quantity: None,
                },
                priority_class: GoalPriorityClass::Critical,
                motive_score: 900,
                motive_source_contributions: Vec::new(),
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                learned_opportunity_bonus: None,
                repair_memory_bonus: None,
                source_composite: None,
                feasibility: crate::feasibility::FeasibilityHint::Uncertain,
                partial_plan_segment: None,
                key: worldwake_core::OpportunityKey {
                    goal_key: goal,
                    anchor: safe_anchor,
                },

                phase: crate::AgendaPhase::Pending,
                origin: crate::AgendaOrigin::NeedDrive,
                introduced_tick: Tick(0),
                last_reconsidered_tick: Tick(0),
                revival_trigger: None,
                kill_condition: crate::KillCondition::External,
            },
        ];
        let plans = vec![
            selection_plan_at_with_perceived_cost(
                goal,
                dangerous_anchor,
                Some(plan_at(goal, dangerous_anchor, 1, 2)),
                Some(4),
            ),
            selection_plan_at_with_perceived_cost(
                goal,
                safe_anchor,
                Some(plan_at(goal, safe_anchor, 2, 3)),
                Some(3),
            ),
        ];

        let selected = select_best_plan(
            &ordered(&candidates),
            &plans,
            None,
            &AgentDecisionRuntime::default(),
            None,
            selection_policy(),
        )
        .unwrap();

        assert_eq!(selected.opportunity.anchor, safe_anchor);
    }

    #[test]
    fn same_class_replacement_requires_switch_margin() {
        let current_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let challenger_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let current_plan = plan(current_goal, 1, 3);
        let challenger_plan = plan(challenger_goal, 2, 2);
        let candidates = [
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::High,
                1000,
            ),
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::High,
                1099,
            ),
        ];
        let plans = vec![
            selection_plan(current_goal, Some(current_plan.clone())),
            selection_plan(challenger_goal, Some(challenger_plan)),
        ];
        let runtime = AgentDecisionRuntime {
            current_plan: Some(current_plan.clone()),
            dirty: crate::DirtySet::default(),
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };

        let selected = select_best_plan(
            &ordered(&candidates),
            &plans,
            Some(current_goal),
            &runtime,
            None,
            selection_policy(),
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
            selection_plan(first_goal, Some(slower)),
            selection_plan(second_goal, Some(faster.clone())),
        ];

        let first = select_best_plan(
            &ordered(&candidates),
            &plans,
            None,
            &AgentDecisionRuntime::default(),
            None,
            selection_policy(),
        )
        .unwrap();
        let second = select_best_plan(
            &ordered(&candidates),
            &plans,
            None,
            &AgentDecisionRuntime::default(),
            None,
            selection_policy(),
        )
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.goal, faster.goal);
    }

    #[test]
    fn same_priority_and_primary_motive_can_break_tie_on_side_benefits() {
        let market = entity(20);
        let market_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let orchard_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let market_plan = plan(market_goal, 1, 3);
        let orchard_plan = plan(orchard_goal, 2, 3);
        let candidates = [
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::High,
                700,
            ),
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::High,
                700,
            ),
            ranked(
                worldwake_core::GoalKind::SellCommodity {
                    commodity: CommodityKind::Apple,
                },
                GoalPriorityClass::Low,
                400,
            ),
        ];
        let plans = vec![
            selection_plan(market_goal, Some(market_plan.clone())),
            selection_plan(orchard_goal, Some(orchard_plan)),
        ];

        let selected = select_best_plan(
            &ordered(&[
                candidates[0].clone(),
                candidates[1].clone(),
                AgendaEntry {
                    offer: GoalOffer {
                        anchor: OpportunityAnchor::Place(market),
                        key: candidates[2].offer.key,
                        evidence_entities: BTreeSet::new(),
                        evidence_places: BTreeSet::new(),
                        obligation_source: None,
                        commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                        required_information_gaps: Vec::new(),
                        invalidators: Vec::new(),
                        learned_expectation_refs: Vec::new(),
                        motive_sources: Vec::new(),
                        acquisition_quantity: None,
                    },
                    ..candidates[2].clone()
                },
            ]),
            &plans,
            None,
            &AgentDecisionRuntime::default(),
            None,
            selection_policy(),
        )
        .unwrap();

        assert_eq!(selected.goal, market_goal);
    }

    #[test]
    fn side_benefits_do_not_override_higher_priority_class() {
        let market = entity(30);
        let orchard = entity(31);
        let high_goal = GoalKey::from(worldwake_core::GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        });
        let lower_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let candidates = vec![
            ranked(
                worldwake_core::GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Bread,
                },
                GoalPriorityClass::Critical,
                200,
            ),
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::High,
                200,
            ),
            AgendaEntry {
                offer: GoalOffer {
                    anchor: OpportunityAnchor::Place(market),
                    key: GoalKey::from(worldwake_core::GoalKind::Patrol { place: market }),
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                    obligation_source: None,
                    commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                    required_information_gaps: Vec::new(),
                    invalidators: Vec::new(),
                    learned_expectation_refs: Vec::new(),
                    motive_sources: Vec::new(),
                    acquisition_quantity: None,
                },
                priority_class: GoalPriorityClass::Low,
                motive_score: 2_000,
                motive_source_contributions: Vec::new(),
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                learned_opportunity_bonus: None,
                repair_memory_bonus: None,
                source_composite: None,
                feasibility: crate::feasibility::FeasibilityHint::Uncertain,
                partial_plan_segment: None,
                key: worldwake_core::OpportunityKey {
                    goal_key: GoalKey::from(worldwake_core::GoalKind::Patrol { place: market }),
                    anchor: OpportunityAnchor::Place(market),
                },

                phase: crate::AgendaPhase::Pending,
                origin: crate::AgendaOrigin::NeedDrive,
                introduced_tick: Tick(0),
                last_reconsidered_tick: Tick(0),
                revival_trigger: None,
                kill_condition: crate::KillCondition::External,
            },
        ];
        let plans = vec![
            selection_plan(high_goal, Some(plan(high_goal, 3, 4))),
            selection_plan(lower_goal, Some(plan(lower_goal, 4, 4))),
        ];

        let selected = select_best_plan(
            &ordered(&candidates),
            &plans,
            None,
            &AgentDecisionRuntime::default(),
            None,
            SelectionPolicy {
                side_benefit_weight: Permille::new(500).unwrap(),
                ..selection_policy()
            },
        )
        .unwrap();

        assert_eq!(selected.goal, high_goal);
        assert_ne!(
            selected.goal,
            GoalKey::from(worldwake_core::GoalKind::Patrol { place: orchard })
        );
    }

    #[test]
    fn same_class_goal_switch_uses_primary_motive_not_total_value() {
        let market = entity(40);
        let current_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let challenger_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let current_plan = plan(current_goal, 5, 3);
        let challenger_plan = plan(challenger_goal, 6, 3);
        let candidates = vec![
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::High,
                1_000,
            ),
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::High,
                1_000,
            ),
            AgendaEntry {
                key: worldwake_core::OpportunityKey {
                    goal_key: GoalKey::from(worldwake_core::GoalKind::SellCommodity {
                        commodity: CommodityKind::Apple,
                    }),
                    anchor: OpportunityAnchor::Place(market),
                },
                offer: GoalOffer {
                    anchor: OpportunityAnchor::Place(market),
                    key: GoalKey::from(worldwake_core::GoalKind::SellCommodity {
                        commodity: CommodityKind::Apple,
                    }),
                    evidence_entities: BTreeSet::new(),
                    evidence_places: BTreeSet::new(),
                    obligation_source: None,
                    commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                    required_information_gaps: Vec::new(),
                    invalidators: Vec::new(),
                    learned_expectation_refs: Vec::new(),
                    motive_sources: Vec::new(),
                    acquisition_quantity: None,
                },
                priority_class: GoalPriorityClass::Low,
                motive_score: 800,
                motive_source_contributions: Vec::new(),
                provenance: None,
                source_reliability_discount: None,
                competition_discount: None,
                learned_opportunity_bonus: None,
                repair_memory_bonus: None,
                source_composite: None,
                feasibility: crate::feasibility::FeasibilityHint::Uncertain,
                partial_plan_segment: None,

                phase: crate::AgendaPhase::Pending,
                origin: crate::AgendaOrigin::NeedDrive,
                introduced_tick: Tick(0),
                last_reconsidered_tick: Tick(0),
                revival_trigger: None,
                kill_condition: crate::KillCondition::External,
            },
        ];
        let plans = vec![
            selection_plan(current_goal, Some(current_plan.clone())),
            selection_plan(challenger_goal, Some(challenger_plan)),
        ];
        let runtime = AgentDecisionRuntime {
            current_plan: Some(current_plan),
            dirty: crate::DirtySet::default(),
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };

        let selected = select_best_plan(
            &ordered(&candidates),
            &plans,
            Some(current_goal),
            &runtime,
            None,
            SelectionPolicy {
                side_benefit_weight: Permille::new(500).unwrap(),
                ..selection_policy()
            },
        )
        .unwrap();

        assert_eq!(selected.goal, current_goal);
    }

    #[test]
    fn same_goal_replanning_replaces_stale_in_progress_plan() {
        let goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let stale_plan = PlannedPlan::new(
            opportunity(goal),
            goal,
            vec![
                PlannedStep {
                    def_id: ActionDefId(1),
                    targets: vec![PlanningEntityRef::Authoritative(entity(11))],
                    target_place: None,
                    payload_override: None,
                    op_kind: PlannerOpKind::Travel,
                    estimated_ticks: 5,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                    guard: None,
                    expectations: Vec::new(),
                },
                PlannedStep {
                    def_id: ActionDefId(2),
                    targets: vec![PlanningEntityRef::Authoritative(entity(12))],
                    target_place: None,
                    payload_override: None,
                    op_kind: PlannerOpKind::Travel,
                    estimated_ticks: 4,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                    guard: None,
                    expectations: Vec::new(),
                },
            ],
            PlanTerminalKind::SearchBudgetExhausted {
                budget_consumed: 0,
                budget_total: 0,
            },
        );
        let refreshed_plan = plan(goal, 3, 2);
        let candidates = vec![ranked(
            worldwake_core::GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            },
            GoalPriorityClass::High,
            900,
        )];
        let plans = vec![selection_plan(goal, Some(refreshed_plan.clone()))];
        let runtime = AgentDecisionRuntime {
            current_plan: Some(stale_plan),
            current_step_index: 1,
            dirty: crate::DirtySet::NO_PLAN,
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };

        let selected = select_best_plan(
            &ordered(&candidates),
            &plans,
            Some(goal),
            &runtime,
            None,
            selection_policy(),
        )
        .unwrap();

        assert_eq!(selected, refreshed_plan);
    }

    fn empty_plan(goal: GoalKey) -> PlannedPlan {
        PlannedPlan::new(
            opportunity(goal),
            goal,
            Vec::new(),
            PlanTerminalKind::GoalSatisfied,
        )
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
        let plans = vec![selection_plan(eat_goal, Some(actionable.clone()))];
        let runtime = AgentDecisionRuntime {
            current_plan: Some(empty_plan(eat_goal)),
            dirty: crate::DirtySet::default(),
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };

        let selected = select_best_plan(
            &ordered(&candidates),
            &plans,
            Some(eat_goal),
            &runtime,
            None,
            selection_policy(),
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
        let plans = vec![selection_plan(eat_goal, Some(challenger.clone()))];
        let runtime = AgentDecisionRuntime {
            current_plan: Some(current.clone()),
            dirty: crate::DirtySet::default(),
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };

        let selected = select_best_plan(
            &ordered(&candidates),
            &plans,
            Some(eat_goal),
            &runtime,
            None,
            selection_policy(),
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
        let plans = vec![selection_plan(eat_goal, Some(empty_plan(eat_goal)))];
        let runtime = AgentDecisionRuntime {
            current_plan: Some(empty_plan(eat_goal)),
            dirty: crate::DirtySet::default(),
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };

        let selected = select_best_plan(
            &ordered(&candidates),
            &plans,
            Some(eat_goal),
            &runtime,
            None,
            selection_policy(),
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
            quantity: AcquisitionQuantity::single(),
        });
        let challenger_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let current_plan = plan(current_goal, 1, 3);
        let challenger_plan = plan(challenger_goal, 2, 2);
        let candidates = vec![
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::High,
                1_000,
            ),
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::High,
                1_350,
            ),
        ];
        let plans = vec![
            selection_plan(current_goal, Some(current_plan.clone())),
            selection_plan(challenger_goal, Some(challenger_plan.clone())),
        ];
        let jc = Some(IntentionFrame {
            goal: current_goal,
            domain: IntentionDomain::Travel {
                destination: entity(1),
            },
            assumptions: Vec::new(),
            state: FrameState::Active,
            established_at: Tick(1),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 10,
            motive_refs: Vec::new(),
            resume_conditions: Vec::new(),
            abandon_conditions: Vec::new(),
            explicit_claims: Vec::new(),
            causal_links: Vec::new(),
        });
        let runtime = AgentDecisionRuntime {
            current_plan: Some(current_plan),
            dirty: crate::DirtySet::default(),
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };

        let conservative = select_best_plan(
            &ordered(&candidates),
            &plans,
            Some(current_goal),
            &runtime,
            jc.as_ref(),
            SelectionPolicy {
                frame_switch_margin: Permille::new(400).unwrap(),
                ..selection_policy()
            },
        )
        .unwrap();
        let permissive = select_best_plan(
            &ordered(&candidates),
            &plans,
            Some(current_goal),
            &runtime,
            jc.as_ref(),
            SelectionPolicy {
                frame_switch_margin: Permille::new(300).unwrap(),
                ..selection_policy()
            },
        )
        .unwrap();

        assert_eq!(conservative.goal, current_goal);
        assert_eq!(permissive.goal, challenger_goal);
    }

    #[test]
    fn current_plan_is_not_retained_when_active_goal_differs() {
        let active_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let stale_current_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let fallback_plan = plan(active_goal, 1, 3);
        let stale_current_plan = plan(stale_current_goal, 2, 2);
        let candidates = vec![
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Apple,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::High,
                900,
            ),
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::High,
                800,
            ),
        ];
        let plans = vec![
            selection_plan(active_goal, Some(fallback_plan.clone())),
            selection_plan(stale_current_goal, Some(stale_current_plan.clone())),
        ];
        let runtime = AgentDecisionRuntime {
            current_plan: Some(stale_current_plan),
            dirty: crate::DirtySet::NO_PLAN,
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };

        let selected = select_best_plan(
            &ordered(&candidates),
            &plans,
            Some(active_goal),
            &runtime,
            None,
            selection_policy(),
        )
        .unwrap();

        assert_eq!(
            selected, fallback_plan,
            "selection must not retain a current plan whose goal no longer matches the committed active goal"
        );
    }

    #[test]
    fn stale_current_plan_is_not_retained_when_current_goal_has_no_plan() {
        let current_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
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
                    quantity: AcquisitionQuantity::single(),
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
            selection_plan(current_goal, None),
            selection_plan(fallback_goal, Some(fallback_plan.clone())),
        ];
        let runtime = AgentDecisionRuntime {
            current_plan: Some(current_plan),
            dirty: crate::DirtySet::NO_PLAN,
            last_priority_class: Some(GoalPriorityClass::High),
            ..AgentDecisionRuntime::default()
        };

        let selected = select_best_plan(
            &ordered(&candidates),
            &plans,
            Some(current_goal),
            &runtime,
            None,
            selection_policy(),
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
            quantity: AcquisitionQuantity::single(),
        });
        let detour_goal = GoalKey::from(worldwake_core::GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Water,
        });
        let destination = entity(44);
        let current_plan = PlannedPlan::new(
            opportunity(committed_goal),
            committed_goal,
            vec![PlannedStep {
                targets: vec![PlanningEntityRef::Authoritative(destination)],
                ..PlannedStep {
                    def_id: ActionDefId(1),
                    targets: Vec::new(),
                    target_place: None,
                    payload_override: None,
                    op_kind: PlannerOpKind::Travel,
                    estimated_ticks: 3,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                    guard: None,
                    expectations: Vec::new(),
                }
            }],
            PlanTerminalKind::GoalSatisfied,
        );
        let detour_plan = PlannedPlan::new(
            opportunity(detour_goal),
            detour_goal,
            vec![PlannedStep {
                def_id: ActionDefId(2),
                targets: vec![PlanningEntityRef::Authoritative(entity(2))],
                target_place: None,
                payload_override: None,
                op_kind: PlannerOpKind::Consume,
                estimated_ticks: 1,
                is_materialization_barrier: false,
                expected_materializations: Vec::new(),
                guard: None,
                expectations: Vec::new(),
            }],
            PlanTerminalKind::GoalSatisfied,
        );
        let abandon_goal = GoalKey::from(worldwake_core::GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let abandon_plan = plan(abandon_goal, 3, 1);
        let candidates = vec![
            ranked(
                worldwake_core::GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
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
                    quantity: AcquisitionQuantity::single(),
                },
                GoalPriorityClass::High,
                1_000,
            ),
        ];
        let plans = vec![
            selection_plan(abandon_goal, Some(abandon_plan)),
            selection_plan(detour_goal, Some(detour_plan.clone())),
            selection_plan(committed_goal, Some(current_plan.clone())),
        ];
        let jc = Some(IntentionFrame {
            goal: committed_goal,
            domain: IntentionDomain::Travel { destination },
            assumptions: Vec::new(),
            state: FrameState::Active,
            established_at: Tick(1),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 10,
            motive_refs: Vec::new(),
            resume_conditions: Vec::new(),
            abandon_conditions: Vec::new(),
            explicit_claims: Vec::new(),
            causal_links: Vec::new(),
        });
        let runtime = AgentDecisionRuntime {
            current_plan: Some(current_plan),
            ..AgentDecisionRuntime::default()
        };

        let selected = select_best_plan(
            &ordered(&candidates),
            &plans,
            Some(committed_goal),
            &runtime,
            jc.as_ref(),
            SelectionPolicy {
                frame_switch_margin: route_switch_margin(),
                ..selection_policy()
            },
        )
        .unwrap();

        assert_eq!(selected.goal, detour_goal);
    }
}
