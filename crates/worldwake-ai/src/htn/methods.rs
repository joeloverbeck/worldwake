use crate::htn::{
    ArtifactTemplate, BeliefPredicate, ClaimRequirement, CommodityTemplate, EntityCriterion,
    EntityTemplate, ExplanationTemplateId, LocationTemplate, MethodFailureMode, MethodPrecondition,
    MethodSchema, MotiveBias, PayloadTemplate, PayloadValueTemplate, RecipeTemplate, RoleTag,
    SubgoalTemplate, TopicTemplate,
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

fn timeout(ticks: u32) -> MethodFailureMode {
    MethodFailureMode::Timeout(ticks)
}

struct MethodParts {
    id: u32,
    goal_kind: GoalKindDiscriminant,
    preconditions: Vec<MethodPrecondition>,
    subgoals: Vec<SubgoalTemplate>,
    expected_artifacts: Vec<ArtifactTemplate>,
    required_claims: Vec<ClaimRequirement>,
    failure_modes: Vec<MethodFailureMode>,
    explanation_template: u32,
    motive_bias: Vec<MotiveBias>,
    planning_budget_hint: Option<GoalPlanningBudget>,
}

fn schema(parts: MethodParts) -> MethodSchema {
    MethodSchema {
        id: MethodSchemaId(parts.id),
        goal_kind: parts.goal_kind,
        preconditions: parts.preconditions,
        subgoals: parts.subgoals,
        expected_artifacts: parts.expected_artifacts,
        required_claims: parts.required_claims,
        failure_modes: parts.failure_modes,
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
        $expected_artifacts:expr,
        $required_claims:expr,
        $failure_modes:expr,
        $explanation_template:expr,
        $motive_bias:expr,
        $planning_budget_hint:expr $(,)?
    ) => {
        schema(MethodParts {
            id: $id,
            goal_kind: $goal_kind,
            preconditions: $preconditions,
            subgoals: $subgoals,
            expected_artifacts: $expected_artifacts,
            required_claims: $required_claims,
            failure_modes: $failure_modes,
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
        vec![ArtifactTemplate::BountyProof {
            bounty: EntityTemplate::GoalPrimaryEntity,
            target: EntityTemplate::BountyTarget,
        }],
        vec![ClaimRequirement::BountyIssuance {
            bounty: EntityTemplate::GoalPrimaryEntity,
        }],
        vec![
            MethodFailureMode::PreconditionLost(BeliefPredicate::BountyExpired {
                bounty: EntityTemplate::GoalPrimaryEntity,
            }),
            MethodFailureMode::SubgoalUnachievable(2),
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
        vec![ArtifactTemplate::ViolationEvidence {
            violation: EntityTemplate::Violation,
        }],
        vec![],
        vec![timeout(120), MethodFailureMode::SubgoalUnachievable(0)],
        2,
        vec![bias(MotiveSourceDiscriminant::OfficeDuty, 600)],
        Some(GoalPlanningBudget::INVESTIGATION),
    )
}

pub fn fulfill_bounty_group_hunt() -> MethodSchema {
    method_schema!(
        3,
        GoalKindDiscriminant::FulfillBounty,
        vec![
            MethodPrecondition::BeliefHolds(BeliefPredicate::TargetBelievedDangerous {
                target: EntityTemplate::BountyTarget,
            }),
            MethodPrecondition::BeliefHolds(BeliefPredicate::AllyOrBountyOfficeAvailable),
            MethodPrecondition::AgentRole(RoleTag::Hunter),
        ],
        vec![
            // Existing planner ops have no RecruitAlly leaf. DeclareSupport is
            // the first-ship social signal for assembling a lawful group hunt.
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
        vec![ArtifactTemplate::BountyProof {
            bounty: EntityTemplate::GoalPrimaryEntity,
            target: EntityTemplate::BountyTarget,
        }],
        vec![ClaimRequirement::BountyIssuance {
            bounty: EntityTemplate::GoalPrimaryEntity,
        }],
        vec![timeout(180), MethodFailureMode::SubgoalUnachievable(1)],
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
        vec![],
        vec![ClaimRequirement::FacilityQueueSlot {
            facility: EntityTemplate::GoalPlace,
        }],
        vec![timeout(80), MethodFailureMode::SubgoalUnachievable(0)],
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
        vec![],
        vec![ClaimRequirement::ResourceSourceAccess {
            commodity: FIRST_RECIPE_INPUT,
            place: EntityTemplate::GoalPlace,
        }],
        vec![timeout(140), MethodFailureMode::SubgoalUnachievable(0)],
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
        vec![],
        vec![],
        vec![timeout(140), MethodFailureMode::SubgoalUnachievable(1)],
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
        vec![],
        vec![ClaimRequirement::ResourceSourceAccess {
            commodity: CommodityTemplate::GoalCommodity,
            place: EntityTemplate::GoalPlace,
        }],
        vec![timeout(100), MethodFailureMode::SubgoalUnachievable(0)],
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
        vec![],
        vec![],
        vec![timeout(100), MethodFailureMode::SubgoalUnachievable(1)],
        8,
        vec![bias(MotiveSourceDiscriminant::Greed, 500)],
        Some(GoalPlanningBudget::PRODUCTION),
    )
}

pub fn investigate_on_scene() -> MethodSchema {
    method_schema!(
        9,
        GoalKindDiscriminant::InvestigateViolation,
        vec![MethodPrecondition::LocationKnown(
            EntityCriterion::ViolationEvidence {
                violation: EntityTemplate::Violation,
            },
        )],
        vec![
            SubgoalTemplate::InspectArtifact(ArtifactTemplate::ViolationEvidence {
                violation: EntityTemplate::Violation,
            }),
            SubgoalTemplate::PerformAction(
                PlannerOpKind::Investigate,
                PayloadTemplate::FromContext,
            ),
        ],
        vec![ArtifactTemplate::ViolationEvidence {
            violation: EntityTemplate::Violation,
        }],
        vec![],
        vec![timeout(100), MethodFailureMode::SubgoalUnachievable(0)],
        9,
        vec![bias(MotiveSourceDiscriminant::OfficeDuty, 750)],
        Some(GoalPlanningBudget::INVESTIGATION),
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
        vec![],
        vec![],
        vec![timeout(120), MethodFailureMode::SubgoalUnachievable(0)],
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
        vec![ArtifactTemplate::Ledger {
            institution: EntityTemplate::Institution,
        }],
        vec![ClaimRequirement::OfficeAuthority {
            office: EntityTemplate::Institution,
        }],
        vec![timeout(140), MethodFailureMode::SubgoalUnachievable(1)],
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
        vec![],
        vec![],
        vec![timeout(120), MethodFailureMode::SubgoalUnachievable(0)],
        12,
        vec![bias(MotiveSourceDiscriminant::Loyalty, 550)],
        Some(GoalPlanningBudget::BOUNTY_ESCORT),
    )
}

pub fn escort_to_office() -> MethodSchema {
    method_schema!(
        13,
        GoalKindDiscriminant::EscortToSafety,
        vec![MethodPrecondition::LocationKnown(EntityCriterion::Ledger {
            institution: EntityTemplate::Institution,
        })],
        vec![SubgoalTemplate::PerformAction(
            PlannerOpKind::EscortToSafety,
            PayloadTemplate::Explicit(PayloadValueTemplate::EscortToSafety {
                escortee: EntityTemplate::Escortee,
                destination: LocationTemplate::OfficePlace {
                    institution: EntityTemplate::Institution,
                },
            }),
        )],
        vec![],
        vec![ClaimRequirement::OfficeAuthority {
            office: EntityTemplate::Institution,
        }],
        vec![
            timeout(120),
            MethodFailureMode::ClaimDenied(ClaimRequirement::OfficeAuthority {
                office: EntityTemplate::Institution,
            }),
        ],
        13,
        vec![bias(MotiveSourceDiscriminant::OfficeDuty, 550)],
        Some(GoalPlanningBudget::BOUNTY_ESCORT),
    )
}
