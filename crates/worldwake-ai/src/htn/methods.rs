use crate::htn::{
    ArtifactTemplate, BeliefPredicate, CommodityTemplate, EntityCriterion, EntityTemplate,
    ExplanationTemplateId, LocationTemplate, MethodPrecondition, MethodSchema, MotiveBias,
    PayloadTemplate, PayloadValueTemplate, RecipeTemplate, SubgoalTemplate, TopicTemplate,
};
use crate::planner_ops::PlannerOpKind;
use worldwake_core::{
    CommodityKind, GoalKindDiscriminant, GoalPlanningBudget, MethodSchemaId,
    MotiveSourceDiscriminant, Permille, Quantity, WorkstationTag,
};

const GOAL_RECIPE: RecipeTemplate = RecipeTemplate::GoalRecipe;
const FIRST_RECIPE_INPUT: CommodityTemplate = CommodityTemplate::RecipeInput {
    recipe: GOAL_RECIPE,
    ordinal: 0,
};

fn bias(motive_variant: MotiveSourceDiscriminant, weight: u16) -> MotiveBias {
    MotiveBias {
        motive_variant,
        weight: Permille::new_unchecked(weight),
    }
}

struct MethodParts {
    id: u32,
    goal_kind: GoalKindDiscriminant,
    preconditions: Vec<MethodPrecondition>,
    subgoals: Vec<SubgoalTemplate>,
    explanation_template: u32,
    motive_bias: Vec<MotiveBias>,
    planning_budget_hint: Option<GoalPlanningBudget>,
}

fn schema(parts: MethodParts) -> MethodSchema {
    MethodSchema {
        id: MethodSchemaId(parts.id),
        goal_kind: parts.goal_kind,
        preconditions: parts.preconditions,
        subgoals: parts
            .subgoals
            .into_iter()
            .map(crate::htn::MethodSubgoal::stage_hint)
            .collect(),
        explanation_template: ExplanationTemplateId(parts.explanation_template),
        motive_bias: parts.motive_bias,
        planning_budget_hint: parts.planning_budget_hint,
    }
}

macro_rules! method_schema {
    (
        $id:expr,
        $goal_kind:expr,
        $preconditions:expr,
        $subgoals:expr,
        $explanation_template:expr,
        $motive_bias:expr,
        $planning_budget_hint:expr $(,)?
    ) => {
        schema(MethodParts {
            id: $id,
            goal_kind: $goal_kind,
            preconditions: $preconditions,
            subgoals: $subgoals,
            explanation_template: $explanation_template,
            motive_bias: $motive_bias,
            planning_budget_hint: $planning_budget_hint,
        })
    };
}

pub fn fulfill_bounty_direct() -> MethodSchema {
    method_schema!(
        1,
        GoalKindDiscriminant::FulfillBounty,
        vec![
            MethodPrecondition::BeliefHolds(BeliefPredicate::BountyRecordExists {
                bounty: EntityTemplate::GoalPrimaryEntity,
            }),
            MethodPrecondition::BeliefHolds(BeliefPredicate::TargetLastSeenKnown {
                target: EntityTemplate::BountyTarget,
            }),
        ],
        vec![
            SubgoalTemplate::AcquireCommodity {
                commodity: CommodityTemplate::Fixed(CommodityKind::Sword),
                min_quantity: Quantity(1),
            },
            SubgoalTemplate::TravelTo(LocationTemplate::LastKnownTargetPlace {
                target: EntityTemplate::BountyTarget,
            }),
            SubgoalTemplate::ObserveTarget(EntityCriterion::Target(EntityTemplate::BountyTarget)),
            SubgoalTemplate::PerformAction(
                PlannerOpKind::Attack,
                PayloadTemplate::Explicit(PayloadValueTemplate::Attack {
                    target: EntityTemplate::BountyTarget,
                }),
            ),
            SubgoalTemplate::TravelTo(LocationTemplate::BountyIssuerPlace {
                bounty: EntityTemplate::GoalPrimaryEntity,
            }),
            SubgoalTemplate::PerformAction(
                PlannerOpKind::ClaimBounty,
                PayloadTemplate::Explicit(PayloadValueTemplate::ClaimBounty {
                    bounty: EntityTemplate::GoalPrimaryEntity,
                }),
            ),
        ],
        1,
        vec![bias(MotiveSourceDiscriminant::Greed, 450)],
        Some(GoalPlanningBudget::BOUNTY_ESCORT),
    )
}

pub fn fulfill_bounty_investigation() -> MethodSchema {
    method_schema!(
        2,
        GoalKindDiscriminant::FulfillBounty,
        vec![
            MethodPrecondition::BeliefHolds(BeliefPredicate::BountyRecordExists {
                bounty: EntityTemplate::GoalPrimaryEntity,
            }),
            MethodPrecondition::BeliefHolds(BeliefPredicate::WitnessNamesKnown {
                violation: EntityTemplate::Violation,
            }),
        ],
        vec![
            SubgoalTemplate::AskWitness(TopicTemplate::TargetWhereabouts {
                target: EntityTemplate::BountyTarget,
            }),
            SubgoalTemplate::InspectArtifact(ArtifactTemplate::ViolationEvidence {
                violation: EntityTemplate::Violation,
            }),
        ],
        2,
        vec![bias(MotiveSourceDiscriminant::OfficeDuty, 600)],
        Some(GoalPlanningBudget::INVESTIGATION),
    )
}

pub fn fulfill_bounty_support_declared_direct() -> MethodSchema {
    method_schema!(
        3,
        GoalKindDiscriminant::FulfillBounty,
        vec![
            MethodPrecondition::BeliefHolds(BeliefPredicate::TargetBelievedDangerous {
                target: EntityTemplate::BountyTarget,
            }),
            MethodPrecondition::BeliefHolds(BeliefPredicate::AllyOrBountyOfficeAvailable),
        ],
        vec![
            // DeclareSupport is a real social signal; current execution then
            // pursues the target directly without enforced group coordination.
            SubgoalTemplate::PerformAction(
                PlannerOpKind::DeclareSupport,
                PayloadTemplate::FromContext,
            ),
            SubgoalTemplate::TravelTo(LocationTemplate::StagingPlaceForConfrontation {
                target: EntityTemplate::BountyTarget,
            }),
            SubgoalTemplate::PerformAction(
                PlannerOpKind::Attack,
                PayloadTemplate::Explicit(PayloadValueTemplate::Attack {
                    target: EntityTemplate::BountyTarget,
                }),
            ),
        ],
        3,
        vec![
            bias(MotiveSourceDiscriminant::Loyalty, 850),
            bias(MotiveSourceDiscriminant::Revenge, 650),
        ],
        Some(GoalPlanningBudget::BOUNTY_ESCORT),
    )
}

pub fn produce_from_owned_stock() -> MethodSchema {
    method_schema!(
        4,
        GoalKindDiscriminant::ProduceCommodity,
        vec![
            MethodPrecondition::BeliefHolds(BeliefPredicate::OwnsInputsForRecipe {
                recipe: GOAL_RECIPE,
            }),
            MethodPrecondition::LocationKnown(EntityCriterion::Workstation(WorkstationTag::Mill)),
        ],
        vec![SubgoalTemplate::PerformAction(
            PlannerOpKind::Craft,
            PayloadTemplate::Explicit(PayloadValueTemplate::Craft {
                recipe: GOAL_RECIPE,
            }),
        )],
        4,
        vec![bias(MotiveSourceDiscriminant::NeedPressure, 500)],
        Some(GoalPlanningBudget::PRODUCTION),
    )
}

pub fn produce_with_gather() -> MethodSchema {
    method_schema!(
        5,
        GoalKindDiscriminant::ProduceCommodity,
        vec![MethodPrecondition::BeliefHolds(
            BeliefPredicate::ResourceSourceKnown {
                commodity: FIRST_RECIPE_INPUT,
            },
        )],
        vec![
            SubgoalTemplate::AcquireCommodity {
                commodity: FIRST_RECIPE_INPUT,
                min_quantity: Quantity(1),
            },
            SubgoalTemplate::TravelTo(LocationTemplate::KnownWorkstationFor {
                recipe: GOAL_RECIPE,
            }),
            SubgoalTemplate::PerformAction(
                PlannerOpKind::Craft,
                PayloadTemplate::Explicit(PayloadValueTemplate::Craft {
                    recipe: GOAL_RECIPE,
                }),
            ),
        ],
        5,
        vec![bias(MotiveSourceDiscriminant::NeedPressure, 400)],
        Some(GoalPlanningBudget::PRODUCTION),
    )
}

pub fn produce_with_purchase() -> MethodSchema {
    method_schema!(
        6,
        GoalKindDiscriminant::ProduceCommodity,
        vec![MethodPrecondition::BeliefHolds(
            BeliefPredicate::SellerKnown {
                commodity: FIRST_RECIPE_INPUT,
            },
        )],
        vec![
            SubgoalTemplate::TravelTo(LocationTemplate::NearestSellerOf {
                commodity: FIRST_RECIPE_INPUT,
            }),
            SubgoalTemplate::PerformAction(
                PlannerOpKind::Trade,
                PayloadTemplate::Explicit(PayloadValueTemplate::Trade {
                    commodity: FIRST_RECIPE_INPUT,
                    quantity: Quantity(1),
                }),
            ),
            SubgoalTemplate::TravelTo(LocationTemplate::KnownWorkstationFor {
                recipe: GOAL_RECIPE,
            }),
            SubgoalTemplate::PerformAction(
                PlannerOpKind::Craft,
                PayloadTemplate::Explicit(PayloadValueTemplate::Craft {
                    recipe: GOAL_RECIPE,
                }),
            ),
        ],
        6,
        vec![bias(MotiveSourceDiscriminant::Greed, 350)],
        Some(GoalPlanningBudget::PRODUCTION),
    )
}

pub fn restock_from_harvest() -> MethodSchema {
    method_schema!(
        7,
        GoalKindDiscriminant::RestockCommodity,
        vec![
            MethodPrecondition::BeliefHolds(BeliefPredicate::OwnedCommodityBelowThreshold {
                commodity: CommodityTemplate::GoalCommodity,
                threshold: Quantity(3),
            }),
            MethodPrecondition::BeliefHolds(BeliefPredicate::ResourceSourceKnown {
                commodity: CommodityTemplate::GoalCommodity,
            }),
        ],
        vec![SubgoalTemplate::AcquireCommodity {
            commodity: CommodityTemplate::GoalCommodity,
            min_quantity: Quantity(3),
        }],
        7,
        vec![bias(MotiveSourceDiscriminant::Greed, 300)],
        Some(GoalPlanningBudget::PRODUCTION),
    )
}

pub fn restock_from_market() -> MethodSchema {
    method_schema!(
        8,
        GoalKindDiscriminant::RestockCommodity,
        vec![
            MethodPrecondition::BeliefHolds(BeliefPredicate::OwnedCommodityBelowThreshold {
                commodity: CommodityTemplate::GoalCommodity,
                threshold: Quantity(3),
            }),
            MethodPrecondition::BeliefHolds(BeliefPredicate::SellerKnown {
                commodity: CommodityTemplate::GoalCommodity,
            }),
        ],
        vec![
            SubgoalTemplate::TravelTo(LocationTemplate::NearestSellerOf {
                commodity: CommodityTemplate::GoalCommodity,
            }),
            SubgoalTemplate::PerformAction(
                PlannerOpKind::Trade,
                PayloadTemplate::Explicit(PayloadValueTemplate::Trade {
                    commodity: CommodityTemplate::GoalCommodity,
                    quantity: Quantity(3),
                }),
            ),
        ],
        8,
        vec![bias(MotiveSourceDiscriminant::Greed, 500)],
        Some(GoalPlanningBudget::PRODUCTION),
    )
}

pub fn investigate_by_witness() -> MethodSchema {
    method_schema!(
        10,
        GoalKindDiscriminant::InvestigateViolation,
        vec![MethodPrecondition::BeliefHolds(
            BeliefPredicate::WitnessNamesKnown {
                violation: EntityTemplate::Violation,
            },
        )],
        vec![
            SubgoalTemplate::AskWitness(TopicTemplate::ViolationCircumstances {
                violation: EntityTemplate::Violation,
            }),
            SubgoalTemplate::PerformAction(
                PlannerOpKind::Investigate,
                PayloadTemplate::FromContext,
            ),
        ],
        10,
        vec![bias(MotiveSourceDiscriminant::OfficeDuty, 650)],
        Some(GoalPlanningBudget::INVESTIGATION),
    )
}

pub fn investigate_by_ledger() -> MethodSchema {
    method_schema!(
        11,
        GoalKindDiscriminant::InvestigateViolation,
        vec![MethodPrecondition::BeliefHolds(
            BeliefPredicate::InstitutionalRecordBelievedExtant {
                violation: EntityTemplate::Violation,
            },
        )],
        vec![
            SubgoalTemplate::TravelTo(LocationTemplate::OfficePlace {
                institution: EntityTemplate::Institution,
            }),
            SubgoalTemplate::InspectArtifact(ArtifactTemplate::Ledger {
                institution: EntityTemplate::Institution,
            }),
            SubgoalTemplate::PerformAction(
                PlannerOpKind::Investigate,
                PayloadTemplate::FromContext,
            ),
        ],
        11,
        vec![bias(MotiveSourceDiscriminant::OfficeDuty, 700)],
        Some(GoalPlanningBudget::INVESTIGATION),
    )
}

pub fn escort_to_home() -> MethodSchema {
    method_schema!(
        12,
        GoalKindDiscriminant::EscortToSafety,
        vec![MethodPrecondition::BeliefHolds(
            BeliefPredicate::EscorteeBelievedSafeAt {
                escortee: EntityTemplate::Escortee,
            },
        )],
        vec![
            SubgoalTemplate::PerformAction(
                PlannerOpKind::EscortToSafety,
                PayloadTemplate::Explicit(PayloadValueTemplate::EscortToSafety {
                    escortee: EntityTemplate::Escortee,
                    destination: LocationTemplate::EscorteeHome {
                        escortee: EntityTemplate::Escortee,
                    },
                }),
            ),
            SubgoalTemplate::ObserveTarget(EntityCriterion::Target(EntityTemplate::Escortee)),
        ],
        12,
        vec![bias(MotiveSourceDiscriminant::Loyalty, 550)],
        Some(GoalPlanningBudget::BOUNTY_ESCORT),
    )
}
