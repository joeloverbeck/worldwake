use crate::{
    AgendaEntry, GoalPriorityClass, PlannedPlan, PlannerOpKind, PlanningEntityRef,
    ranking::OrderedRanked,
};
use std::collections::BTreeSet;
use worldwake_core::{EntityId, GoalKey, OpportunityAnchor, Permille};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SideBenefit {
    pub goal_key: GoalKey,
    pub at_place: EntityId,
    pub estimated_value: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanValue {
    pub plan: PlannedPlan,
    pub priority_class: GoalPriorityClass,
    pub primary_motive: u32,
    pub side_benefits: Vec<SideBenefit>,
    pub total_value: u32,
}

#[must_use]
pub fn detect_side_benefits(
    plan: &PlannedPlan,
    ranked_candidates: &OrderedRanked<'_>,
    primary_goal_key: &GoalKey,
    side_benefit_weight: Permille,
) -> Vec<SideBenefit> {
    if side_benefit_weight.value() == 0 {
        return Vec::new();
    }

    let visited_places = visited_places(plan);
    if visited_places.is_empty() {
        return Vec::new();
    }

    let mut seen_goals = BTreeSet::new();
    let mut side_benefits = Vec::new();

    for candidate in ranked_candidates {
        if &candidate.offer.key == primary_goal_key {
            continue;
        }

        let Some(at_place) = candidate_target_place(candidate) else {
            continue;
        };
        if !visited_places.contains(&at_place) {
            continue;
        }
        if !seen_goals.insert(candidate.offer.key) {
            continue;
        }

        let estimated_value =
            candidate.motive_score * u32::from(side_benefit_weight.value()) / 1000;
        if estimated_value == 0 {
            continue;
        }

        side_benefits.push(SideBenefit {
            goal_key: candidate.offer.key,
            at_place,
            estimated_value,
        });
        if side_benefits.len() == 3 {
            break;
        }
    }

    side_benefits
}

#[must_use]
pub fn build_plan_value(
    plan: PlannedPlan,
    priority_class: GoalPriorityClass,
    primary_motive: u32,
    ranked_candidates: &OrderedRanked<'_>,
    side_benefit_weight: Permille,
) -> PlanValue {
    let side_benefits =
        detect_side_benefits(&plan, ranked_candidates, &plan.goal, side_benefit_weight);
    let total_value = capped_total_value(primary_motive, &side_benefits);
    PlanValue {
        plan,
        priority_class,
        primary_motive,
        side_benefits,
        total_value,
    }
}

fn visited_places(plan: &PlannedPlan) -> BTreeSet<EntityId> {
    plan.steps
        .iter()
        .filter(|step| step.op_kind == PlannerOpKind::Travel)
        .flat_map(|step| step.targets.iter())
        .copied()
        .filter_map(|target| match target {
            PlanningEntityRef::Authoritative(entity) => Some(entity),
            PlanningEntityRef::Hypothetical(_) => None,
        })
        .collect()
}

fn candidate_target_place(candidate: &AgendaEntry) -> Option<EntityId> {
    match candidate.offer.anchor {
        OpportunityAnchor::Place(place) => Some(place),
        OpportunityAnchor::Entity(_) | OpportunityAnchor::None => candidate.offer.key.place,
    }
}

fn capped_total_value(primary_motive: u32, side_benefits: &[SideBenefit]) -> u32 {
    let side_benefit_total = side_benefits.iter().fold(0u32, |acc, benefit| {
        acc.saturating_add(benefit.estimated_value)
    });
    let cap = primary_motive.saturating_mul(3) / 2;
    primary_motive.saturating_add(side_benefit_total).min(cap)
}

#[cfg(test)]
mod tests {
    use super::{PlanValue, SideBenefit, build_plan_value, detect_side_benefits};
    use crate::{
        AgendaEntry, CommodityPurpose, GoalOffer, GoalPriorityClass, PlanTerminalKind, PlannedPlan,
        PlannedStep, PlannerOpKind, PlanningEntityRef,
    };
    use std::collections::BTreeSet;
    use worldwake_core::{
        AcquisitionQuantity, ActionDefId, CommodityKind, EntityId, GoalKey, GoalKind,
        OpportunityAnchor, OpportunityKey, Permille, Tick,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn ranked(goal: GoalKind, anchor: OpportunityAnchor, motive_score: u32) -> AgendaEntry {
        AgendaEntry {
            key: worldwake_core::OpportunityKey {
                goal_key: GoalKey::from(goal),
                anchor,
            },
            offer: GoalOffer {
                key: GoalKey::from(goal),
                anchor,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
                obligation_source: None,
                commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
                required_information_gaps: Vec::new(),
                invalidators: Vec::new(),
                learned_expectation_refs: Vec::new(),
                acquisition_quantity: None,
            },
            priority_class: GoalPriorityClass::High,
            motive_score,
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            source_composite: None,
            feasibility: crate::feasibility::FeasibilityHint::Uncertain,
            phase: crate::AgendaPhase::Pending,
            origin: crate::AgendaOrigin::NeedDrive,
            introduced_tick: Tick(0),
            last_reconsidered_tick: Tick(0),
            revival_trigger: None,
            kill_condition: crate::KillCondition::External,
        }
    }

    fn travel_step(place: EntityId) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(1),
            targets: vec![PlanningEntityRef::Authoritative(place)],
            target_place: None,
            payload_override: None,
            op_kind: PlannerOpKind::Travel,
            estimated_ticks: 3,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        }
    }

    fn hypothetical_travel_step() -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(2),
            targets: vec![PlanningEntityRef::Hypothetical(
                crate::HypotheticalEntityId(7),
            )],
            target_place: None,
            payload_override: None,
            op_kind: PlannerOpKind::Travel,
            estimated_ticks: 3,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        }
    }

    fn plan(goal: GoalKey, places: &[EntityId]) -> PlannedPlan {
        PlannedPlan::new(
            OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::None,
            },
            goal,
            places.iter().copied().map(travel_step).collect(),
            PlanTerminalKind::GoalSatisfied,
        )
    }

    fn ordered(ranked: &[AgendaEntry]) -> crate::ranking::OrderedRanked<'_> {
        crate::ranking::OrderedRanked::from_sorted_for_test(ranked)
    }

    #[test]
    fn detect_side_benefits_matches_candidate_place_on_plan_path() {
        let market = entity(10);
        let orchard = entity(11);
        let primary_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let plan = plan(primary_goal, &[market, orchard]);
        let candidates = vec![
            ranked(primary_goal.kind, OpportunityAnchor::None, 900),
            ranked(
                GoalKind::SellCommodity {
                    commodity: CommodityKind::Apple,
                },
                OpportunityAnchor::Place(market),
                400,
            ),
            ranked(
                GoalKind::Patrol { place: orchard },
                OpportunityAnchor::None,
                250,
            ),
        ];

        let benefits = detect_side_benefits(
            &plan,
            &ordered(&candidates),
            &primary_goal,
            Permille::new(100).unwrap(),
        );

        assert_eq!(
            benefits,
            vec![
                SideBenefit {
                    goal_key: GoalKey::from(GoalKind::SellCommodity {
                        commodity: CommodityKind::Apple
                    }),
                    at_place: market,
                    estimated_value: 40,
                },
                SideBenefit {
                    goal_key: GoalKey::from(GoalKind::Patrol { place: orchard }),
                    at_place: orchard,
                    estimated_value: 25,
                },
            ]
        );
    }

    #[test]
    fn detect_side_benefits_excludes_primary_goal_and_hypothetical_targets() {
        let market = entity(10);
        let primary_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let plan = PlannedPlan::new(
            OpportunityKey {
                goal_key: primary_goal,
                anchor: OpportunityAnchor::None,
            },
            primary_goal,
            vec![hypothetical_travel_step(), travel_step(market)],
            PlanTerminalKind::GoalSatisfied,
        );
        let candidates = vec![
            ranked(primary_goal.kind, OpportunityAnchor::Place(market), 900),
            ranked(
                GoalKind::SellCommodity {
                    commodity: CommodityKind::Apple,
                },
                OpportunityAnchor::Entity(entity(44)),
                500,
            ),
        ];

        let benefits = detect_side_benefits(
            &plan,
            &ordered(&candidates),
            &primary_goal,
            Permille::new(100).unwrap(),
        );

        assert!(benefits.is_empty());
    }

    #[test]
    fn detect_side_benefits_caps_at_three_unique_goals() {
        let market = entity(10);
        let primary_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let plan = plan(primary_goal, &[market]);
        let candidates = vec![
            ranked(primary_goal.kind, OpportunityAnchor::None, 900),
            ranked(
                GoalKind::SellCommodity {
                    commodity: CommodityKind::Apple,
                },
                OpportunityAnchor::Place(market),
                400,
            ),
            ranked(
                GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread,
                },
                OpportunityAnchor::Place(market),
                350,
            ),
            ranked(
                GoalKind::Patrol { place: market },
                OpportunityAnchor::None,
                300,
            ),
            ranked(
                GoalKind::Patrol { place: market },
                OpportunityAnchor::Place(market),
                275,
            ),
            ranked(
                GoalKind::InvestigateViolation {
                    violation_id: worldwake_core::ViolationId(1),
                    place: market,
                },
                OpportunityAnchor::Place(market),
                250,
            ),
        ];

        let benefits = detect_side_benefits(
            &plan,
            &ordered(&candidates),
            &primary_goal,
            Permille::new(100).unwrap(),
        );

        assert_eq!(benefits.len(), 3);
        assert_eq!(
            benefits
                .iter()
                .map(|benefit| benefit.goal_key)
                .collect::<Vec<_>>(),
            vec![
                GoalKey::from(GoalKind::SellCommodity {
                    commodity: CommodityKind::Apple
                }),
                GoalKey::from(GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread
                }),
                GoalKey::from(GoalKind::Patrol { place: market }),
            ]
        );
    }

    #[test]
    fn build_plan_value_caps_total_value_at_one_and_a_half_times_primary() {
        let market = entity(10);
        let primary_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let plan = plan(primary_goal, &[market]);
        let candidates = vec![
            ranked(primary_goal.kind, OpportunityAnchor::None, 1_000),
            ranked(
                GoalKind::SellCommodity {
                    commodity: CommodityKind::Apple,
                },
                OpportunityAnchor::Place(market),
                2_000,
            ),
            ranked(
                GoalKind::RestockCommodity {
                    commodity: CommodityKind::Bread,
                },
                OpportunityAnchor::Place(market),
                2_000,
            ),
        ];

        let value = build_plan_value(
            plan.clone(),
            GoalPriorityClass::High,
            1_000,
            &ordered(&candidates),
            Permille::new(500).unwrap(),
        );

        assert_eq!(
            value,
            PlanValue {
                plan,
                priority_class: GoalPriorityClass::High,
                primary_motive: 1_000,
                side_benefits: vec![
                    SideBenefit {
                        goal_key: GoalKey::from(GoalKind::SellCommodity {
                            commodity: CommodityKind::Apple
                        }),
                        at_place: market,
                        estimated_value: 1_000,
                    },
                    SideBenefit {
                        goal_key: GoalKey::from(GoalKind::RestockCommodity {
                            commodity: CommodityKind::Bread
                        }),
                        at_place: market,
                        estimated_value: 1_000,
                    },
                ],
                total_value: 1_500,
            }
        );
    }

    #[test]
    fn detect_side_benefits_returns_empty_when_weight_is_zero() {
        let market = entity(10);
        let primary_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let plan = plan(primary_goal, &[market]);
        let candidates = vec![ranked(
            GoalKind::SellCommodity {
                commodity: CommodityKind::Apple,
            },
            OpportunityAnchor::Place(market),
            400,
        )];

        let benefits = detect_side_benefits(
            &plan,
            &ordered(&candidates),
            &primary_goal,
            Permille::new(0).unwrap(),
        );

        assert!(benefits.is_empty());
    }
}
